// kimi:score-ignore=unsafe,unwrap
//! MCP (Model Context Protocol) server over stdio.
//!
//! Exposes `cargo kimi check` as an MCP tool so other agents (Claude Code,
//! Codex, etc.) can invoke contract checking natively without shell exec.

use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, Write};

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<serde_json::Value>,
    method: String,
    #[serde(default)]
    params: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct InitializeResult {
    protocol_version: String,
    capabilities: serde_json::Value,
    server_info: ServerInfo,
}

#[derive(Debug, Serialize)]
struct ServerInfo {
    name: String,
    version: String,
}

#[derive(Debug, Serialize)]
struct ToolsListResult {
    tools: Vec<Tool>,
}

#[derive(Debug, Serialize)]
struct Tool {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct ToolCallResult {
    content: Vec<ToolContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_error: Option<bool>,
}

#[derive(Debug, Serialize)]
struct ToolContent {
    #[serde(rename = "type")]
    ty: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct CheckContractsArgs {
    #[serde(default = "default_path")]
    path: String,
    #[serde(default = "default_strictness")]
    strictness: String,
}

fn default_path() -> String {
    "src/".into()
}

fn default_strictness() -> String {
    "standard".into()
}

fn error_response(id: Option<serde_json::Value>, code: i32, message: String) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".into(),
        id,
        result: None,
        error: Some(JsonRpcError {
            code,
            message,
            data: None,
        }),
    }
}

fn success_response(id: Option<serde_json::Value>, result: serde_json::Value) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".into(),
        id,
        result: Some(result),
        error: None,
    }
}

fn handle_initialize(id: Option<serde_json::Value>) -> JsonRpcResponse {
    let result = InitializeResult {
        protocol_version: "2024-11-05".into(),
        capabilities: serde_json::json!({"tools": {}}),
        server_info: ServerInfo {
            name: "cargo-kimi".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
    };
    let value = match serde_json::to_value(result) {
        Ok(v) => v,
        Err(e) => return error_response(id, -32602, format!("JSON serialization error: {e}")),
    };
    success_response(id, value)
}

fn handle_tools_list(id: Option<serde_json::Value>) -> JsonRpcResponse {
    let result = ToolsListResult {
        tools: vec![Tool {
            name: "check_contracts".into(),
            description: "Run Hoare triple, unwrap, and unsafe checker on Rust source files.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to Rust source files or directory to check"
                    },
                    "strictness": {
                        "type": "string",
                        "enum": ["relaxed", "standard", "strict"],
                        "description": "Contract checker strictness level"
                    }
                }
            }),
        }],
    };
    match serde_json::to_value(result) {
        Ok(v) => success_response(id, v),
        Err(e) => error_response(id, -32602, format!("JSON serialization error: {e}")),
    }
}

fn handle_tool_call(id: Option<serde_json::Value>, params: &serde_json::Value) -> JsonRpcResponse {
    let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let arguments = params.get("arguments").cloned().unwrap_or(serde_json::json!({}));

    match name {
        "check_contracts" => {
            let args: CheckContractsArgs = match serde_json::from_value(arguments) {
                Ok(a) => a,
                Err(e) => {
                    return error_response(id, -32602, format!("Invalid arguments: {e}"));
                }
            };

            let result = run_check_contracts(&args.path, &args.strictness);
            match result {
                Ok(text) => {
                    let tool_result = ToolCallResult {
                        content: vec![ToolContent {
                            ty: "text".into(),
                            text,
                        }],
                        is_error: None,
                    };
                    match serde_json::to_value(tool_result) {
                        Ok(v) => success_response(id, v),
                        Err(e) => error_response(id, -32602, format!("JSON serialization error: {e}")),
                    }
                }
                Err(e) => {
                    let tool_result = ToolCallResult {
                        content: vec![ToolContent {
                            ty: "text".into(),
                            text: format!("Error: {e}"),
                        }],
                        is_error: Some(true),
                    };
                    match serde_json::to_value(tool_result) {
                        Ok(v) => success_response(id, v),
                        Err(err) => error_response(id, -32602, format!("JSON serialization error: {err}")),
                    }
                }
            }
        }
        _ => error_response(id, -32601, format!("Unknown tool: {name}")),
    }
}

fn run_check_contracts(path: &str, strictness: &str) -> anyhow::Result<String> {
    use crate::{contracts, workspace};
    use std::path::PathBuf;

    let path_buf = PathBuf::from(path);
    if path_buf.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        anyhow::bail!("Path cannot contain parent directory references (..)");
    }
    let cwd = std::env::current_dir()?;
    let canonical_cwd = cwd.canonicalize()?;
    let canonical_path = path_buf.canonicalize().unwrap_or_else(|_| cwd.join(&path_buf));
    if !canonical_path.starts_with(&canonical_cwd) {
        anyhow::bail!("Path must be inside the current working directory");
    }

    let config = contracts::CheckConfig::from_strictness(strictness)?;
    let paths = if path_buf.is_file() {
        vec![path_buf]
    } else {
        workspace::find_workspace_crates()?
    };
    let reports = contracts::check_files(&paths, &config)?;

    let mut output = String::new();
    for report in &reports {
        output.push_str(&format!("\n{} (score: {})\n", report.file.display(), report.score));
        for issue in &report.issues {
            let sev = match issue.severity {
                contracts::Severity::Critical => "CRITICAL",
                contracts::Severity::Major => "MAJOR",
                contracts::Severity::Minor => "MINOR",
                contracts::Severity::Info => "INFO",
            };
            output.push_str(&format!("  [{}] L{}: {}\n", sev, issue.line, issue.message));
        }
    }

    if reports.is_empty() {
        output.push_str("✅ All contracts satisfied.\n");
    }

    let total_score: u32 = reports.iter().map(|r| r.score).sum();
    let avg = if reports.is_empty() { 100 } else { total_score / reports.len() as u32 };
    output.push_str(&format!("\nAverage score: {}/100\n", avg));

    Ok(output)
}

    /// { stdin/stdout are available and not redirected in a breaking way }
    /// pub fn run_server() -> anyhow::Result<()>
    /// { starts MCP stdio server and handles JSON-RPC requests until EOF }
pub fn run_server() -> anyhow::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let req: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let resp = error_response(None, -32700, format!("Parse error: {e}"));
                writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                stdout.flush()?;
                continue;
            }
        };

        let resp = match req.method.as_str() {
            "initialize" => handle_initialize(req.id),
            "notifications/initialized" => {
                // No response for notifications
                continue;
            }
            "tools/list" => handle_tools_list(req.id),
            "tools/call" => {
                if let Some(params) = req.params {
                    handle_tool_call(req.id, &params)
                } else {
                    error_response(req.id, -32602, "Missing params".into())
                }
            }
            _ => error_response(req.id, -32601, format!("Method not found: {}", req.method)),
        };

        writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
        stdout.flush()?;
    }

    Ok(())
}
