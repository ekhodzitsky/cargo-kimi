use clap::{Parser, Subcommand};

/// Cargo subcommand for kimi-dotfiles — structured contracts for Rust
#[derive(Parser)]
#[command(name = "cargo-kimi")]
#[command(about = "Initialize, check, and verify Rust projects with kimi-dotfiles guidelines")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize kimi-dotfiles in the current project
    Init {
        /// Template to use
        #[arg(short, long, value_name = "NAME", default_value = "rust-only")]
        template: String,
        /// Strictness level for clippy config
        #[arg(short, long, value_name = "LEVEL", default_value = "standard")]
        strictness: String,
        /// Where to place AGENTS.md (root or .kimi)
        #[arg(short, long, value_name = "PATH", default_value = "auto")]
        location: String,
        /// Skip confirmation prompts
        #[arg(short, long)]
        yes: bool,
    },
    /// Run mechanized checks: contracts, clippy, tests
    Check {
        /// Strictness level for contract checker
        #[arg(short, long, value_name = "LEVEL", default_value = "standard")]
        strictness: String,
        /// Output format (text or json)
        #[arg(short, long, value_name = "FORMAT", default_value = "text")]
        format: String,
    },
    /// Run formal verification with Kani (if installed)
    Verify,
    /// Generate property tests for newtypes with arithmetic impls
    GenerateTests {
        /// Output file path (default: tests/property_tests.rs)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Show upgrade instructions
    Upgrade,
    /// Initialize a new skill with YAML frontmatter template
    InitSkill {
        /// Name of the skill (lowercase, hyphens allowed)
        name: String,
        /// Description of the skill
        #[arg(short, long)]
        description: Option<String>,
    },
    /// Run MCP server over stdio for integration with other agents
    Mcp,
    /// Show score trend over time
    Trend {
        /// Number of days to look back
        #[arg(short, long, default_value_t = 30)]
        days: u32,
    },
    /// Auto-fix contract issues
    Fix {
        /// Show what would be fixed without writing files
        #[arg(long)]
        dry_run: bool,
        /// Strictness level for contract checker
        #[arg(short, long, value_name = "LEVEL", default_value = "standard")]
        strictness: String,
    },
    /// Watch source files and re-run checks on change
    Watch {
        /// Strictness level for contract checker
        #[arg(short, long, value_name = "LEVEL", default_value = "standard")]
        strictness: String,
        /// Output format (text, json, or sarif)
        #[arg(short, long, value_name = "FORMAT", default_value = "text")]
        format: String,
        /// Debounce interval in milliseconds
        #[arg(short, long, default_value_t = 500)]
        debounce_ms: u64,
    },
}

pub fn run() -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    // When invoked as `cargo kimi`, cargo passes `kimi` as the first argument.
    // We need to strip it so clap sees only our subcommands.
    let args = if args.get(1).map(|s| s.as_str()) == Some("kimi") {
        let mut filtered = vec![args[0].clone()];
        filtered.extend(args.into_iter().skip(2));
        filtered
    } else {
        args
    };
    let cli = Cli::parse_from(args);
    match cli.command {
        Commands::Init {
            template,
            strictness,
            location,
            yes,
        } => crate::init::cmd_init(&template, &strictness, &location, yes),
        Commands::Check { strictness, format } => crate::cmd_check(&strictness, &format),
        Commands::Verify => crate::cmd_verify(),
        Commands::GenerateTests { output } => crate::cmd_generate_tests(output.as_deref()),
        Commands::Upgrade => crate::cmd_upgrade(),
        Commands::InitSkill { name, description } => {
            crate::skills::cmd_skill_init(&name, description.as_deref())
        }
        Commands::Mcp => crate::mcp::run_server(),
        Commands::Trend { days } => crate::trend::show_trend(days),
        Commands::Fix { dry_run, strictness } => crate::fix::run_fix(dry_run, &strictness),
        Commands::Watch {
            strictness,
            format,
            debounce_ms,
        } => crate::watch::run_watch(&strictness, &format, debounce_ms),
    }
}
