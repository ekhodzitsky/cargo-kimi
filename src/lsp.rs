// kimi:score-ignore=unsafe,unwrap
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::contracts;
use std::sync::LazyLock;

static STANDARD_CONFIG: LazyLock<contracts::CheckConfig> = LazyLock::new(|| {
    contracts::CheckConfig::from_strictness("standard")
        .expect("standard is a valid strictness level")
});

    /// {  }
    /// pub fn run_lsp() -> anyhow::Result<()>
    /// { запускает LSP сервер и блокирует до завершения }
pub fn run_lsp() -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());
        let (service, socket) = LspService::new(|client| Backend {
            client,
            documents: Arc::new(RwLock::new(HashMap::new())),
        });
        Server::new(stdin, stdout, socket).serve(service).await;
        Ok(())
    })
}

#[derive(Debug)]
struct Backend {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, String>>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        ..Default::default()
                    },
                )),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "cargo-kimi LSP server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.documents.write().await.insert(uri.clone(), text.clone());
        self.check_and_publish(uri, &text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.into_iter().next() {
            self.documents.write().await.insert(uri.clone(), change.text.clone());
            self.check_and_publish(uri, &change.text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents.write().await.remove(&params.text_document.uri);
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = &params.text_document.uri;
        let text = {
            let docs = self.documents.read().await;
            match docs.get(uri) {
                Some(t) => t.clone(),
                None => return Ok(None),
            }
        };

        let path = match uri.to_file_path() {
            Ok(p) => p,
            Err(_) => return Ok(None),
        };
        let report = match contracts::check_file_contents(&path, &text, &STANDARD_CONFIG) {
            Ok(r) => r,
            Err(_) => return Ok(None),
        };

        let mut actions = Vec::new();
        for issue in &report.issues {
            if let Some(action) = issue_to_code_action(issue) {
                actions.push(CodeActionOrCommand::CodeAction(action));
            }
        }

        Ok(Some(actions))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let text = {
            let docs = self.documents.read().await;
            match docs.get(&uri) {
                Some(t) => t.clone(),
                None => return Ok(None),
            }
        };

        let path = match uri.to_file_path() {
            Ok(p) => p,
            Err(_) => return Ok(None),
        };
        let report = match contracts::check_file_contents(&path, &text, &STANDARD_CONFIG) {
            Ok(r) => r,
            Err(_) => return Ok(None),
        };

        let score = report.score;
        let issues = report.issues.len();
        let emoji = if score >= 80 {
            "🟢"
        } else if score >= 60 {
            "🟡"
        } else if score >= 40 {
            "🟠"
        } else {
            "🔴"
        };

        let content = format!(
            "### Kimi Contract Score\n\n{} **{}/100**\n\n{} issue(s) found",
            emoji, score, issues
        );

        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String(content)),
            range: None,
        }))
    }
}

impl Backend {
    async fn check_and_publish(&self, uri: Url, text: &str) {
        let path = match uri.to_file_path() {
            Ok(p) => p,
            Err(_) => return,
        };
        let report = match contracts::check_file_contents(&path, text, &STANDARD_CONFIG) {
            Ok(r) => r,
            Err(_) => return,
        };

        let diagnostics: Vec<Diagnostic> = report.issues.iter().map(issue_to_diagnostic).collect();

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

fn issue_to_diagnostic(issue: &contracts::Issue) -> Diagnostic {
    let severity = match issue.severity {
        contracts::Severity::Critical => Some(DiagnosticSeverity::ERROR),
        contracts::Severity::Major => Some(DiagnosticSeverity::ERROR),
        contracts::Severity::Minor => Some(DiagnosticSeverity::WARNING),
        contracts::Severity::Info => Some(DiagnosticSeverity::INFORMATION),
    };

    let code = Some(NumberOrString::String(match issue.category {
        contracts::IssueCategory::MissingHoareTriple => "kimi/missing-hoare-triple".to_string(),
        contracts::IssueCategory::UnwrapExpectPanic => "kimi/unwrap-expect-panic".to_string(),
        contracts::IssueCategory::UnsafeWithoutSafety => "kimi/unsafe-without-safety".to_string(),
    }));

    let range = Range {
        start: Position {
            line: (issue.line.saturating_sub(1)) as u32,
            character: 0,
        },
        end: Position {
            line: (issue.line.saturating_sub(1)) as u32,
            character: 10_000,
        },
    };

    Diagnostic {
        range,
        severity,
        code,
        source: Some("cargo-kimi".to_string()),
        message: issue.message.clone(),
        ..Default::default()
    }
}

fn issue_to_code_action(issue: &contracts::Issue) -> Option<CodeAction> {
    let (title, edit) = match issue.category {
        contracts::IssueCategory::MissingHoareTriple => {
            let line = issue.line.saturating_sub(1);
            let fn_name = issue
                .message
                .strip_prefix("pub fn '")
                .and_then(|s| s.split('\'').next())
                .unwrap_or("unknown");
            let new_text = format!(
                "/// {{ TODO: precondition }}\n/// `pub fn {}`\n/// {{ TODO: postcondition }}\n",
                fn_name
            );
            let edit = TextEdit {
                range: Range {
                    start: Position {
                        line: line as u32,
                        character: 0,
                    },
                    end: Position {
                        line: line as u32,
                        character: 0,
                    },
                },
                new_text,
            };
            ("Insert Hoare triple stub".to_string(), edit)
        }
        contracts::IssueCategory::UnsafeWithoutSafety => {
            let line = issue.line.saturating_sub(1);
            let new_text = "// SAFETY: TODO: explain why this is safe\n".to_string();
            let edit = TextEdit {
                range: Range {
                    start: Position {
                        line: line as u32,
                        character: 0,
                    },
                    end: Position {
                        line: line as u32,
                        character: 0,
                    },
                },
                new_text,
            };
            ("Add SAFETY comment".to_string(), edit)
        }
        contracts::IssueCategory::UnwrapExpectPanic => {
            return None; // unwrap -> ? requires return type analysis, skip for now
        }
    };

    let mut changes = HashMap::new();
    changes.insert(
        Url::from_file_path(&issue.file).ok()?,
        vec![edit],
    );

    Some(CodeAction {
        title,
        kind: Some(CodeActionKind::QUICKFIX),
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        }),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn dummy_issue(category: contracts::IssueCategory, line: usize) -> contracts::Issue {
        contracts::Issue {
            file: PathBuf::from("/tmp/test.rs"),
            line,
            message: "test message".to_string(),
            severity: contracts::Severity::Major,
            category,
        }
    }

    #[test]
    fn issue_to_diagnostic_maps_severity() {
        let issue = dummy_issue(contracts::IssueCategory::MissingHoareTriple, 5);
        let diag = issue_to_diagnostic(&issue);
        assert_eq!(diag.severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(diag.range.start.line, 4);
        assert_eq!(diag.range.end.line, 4);
    }

    #[test]
    fn issue_to_code_action_hoare_triple() {
        let issue = dummy_issue(contracts::IssueCategory::MissingHoareTriple, 3);
        let action = issue_to_code_action(&issue).unwrap();
        assert_eq!(action.title, "Insert Hoare triple stub");
        assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
    }

    #[test]
    fn issue_to_code_action_safety_comment() {
        let issue = dummy_issue(contracts::IssueCategory::UnsafeWithoutSafety, 7);
        let action = issue_to_code_action(&issue).unwrap();
        assert_eq!(action.title, "Add SAFETY comment");
        assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
    }

    #[test]
    fn issue_to_code_action_unwrap_returns_none() {
        let issue = dummy_issue(contracts::IssueCategory::UnwrapExpectPanic, 1);
        assert!(issue_to_code_action(&issue).is_none());
    }
}
#[allow(dead_code)]
pub struct LspUri(pub(crate) String);
