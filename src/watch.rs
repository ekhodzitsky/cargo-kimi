use std::sync::mpsc;
use std::time::Duration;

use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

    /// { strictness is a valid strictness level; format is "text", "json", or "sarif"; debounce_ms > 0 }
    /// pub fn run_watch(strictness: &str, format: &str, debounce_ms: u64) -> anyhow::Result<()>
    /// { blocks indefinitely, re-running checks on every .rs change after debounce }
pub fn run_watch(strictness: &str, format: &str, debounce_ms: u64) -> anyhow::Result<()> {
    let (tx, rx) = mpsc::channel::<notify::Result<notify::Event>>();

    let mut watcher = RecommendedWatcher::new(
        move |res| {
            let _ = tx.send(res);
        },
        Config::default().with_poll_interval(Duration::from_millis(debounce_ms)),
    )?;

    let paths = crate::workspace::find_workspace_crates()?;
    for path in &paths {
        if path.is_dir() {
            watcher.watch(path, RecursiveMode::Recursive)?;
        } else if let Some(parent) = path.parent() {
            watcher.watch(parent, RecursiveMode::Recursive)?;
        }
    }

    println!("👁  Watching for Rust source changes... (Ctrl-C to stop)");

    // Run once immediately
    run_check(strictness, format)?;

    loop {
        match rx.recv() {
            Ok(Ok(event)) => {
                if matches!(
                    event.kind,
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                ) {
                    let has_rs = event.paths.iter().any(|p| {
                        p.extension().map(|e| e == "rs").unwrap_or(false)
                    });
                    if has_rs {
                        // Simple debounce: sleep a bit and drain extra events
                        std::thread::sleep(Duration::from_millis(debounce_ms));
                        while rx.try_recv().is_ok() {}

                        println!("\n────────────────────────────────────────");
                        println!("🔄 Change detected, re-running checks...");
                        println!("────────────────────────────────────────");
                        if let Err(e) = run_check(strictness, format) {
                            eprintln!("Error during check: {}", e);
                        }
                    }
                }
            }
            Ok(Err(e)) => eprintln!("Watch error: {}", e),
            Err(_) => break,
        }
    }

    Ok(())
}

fn run_check(strictness: &str, format: &str) -> anyhow::Result<()> {
    let config = crate::contracts::CheckConfig::from_strictness(strictness)?;
    let paths = crate::workspace::find_workspace_crates()?;
    let reports = crate::contracts::check_files(&paths, &config)?;

    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&reports)?;
            println!("{}", json);
        }
        "sarif" => {
            crate::contracts::print_sarif(&reports)?;
        }
        _ => {
            crate::contracts::print_reports(&reports);
        }
    }

    // Persist trend history
    crate::trend::append_history(&reports)?;

    Ok(())
}
