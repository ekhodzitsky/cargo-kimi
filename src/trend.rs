use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::Path;

use crate::contracts::FileReport;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct HistoryEntry {
    pub timestamp: String,
    pub average_score: u32,
    pub files: HashMap<String, u32>,
}

    /// { reports are valid check results }
    /// pub fn append_history(reports: &[FileReport]) -> anyhow::Result<()>
    /// { appends a JSONL entry to .kimi/score-history.jsonl with timestamp and scores }
pub fn append_history(reports: &[FileReport]) -> anyhow::Result<()> {
    fs::create_dir_all(".kimi")?;

    let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let average_score = if reports.is_empty() {
        0
    } else {
        reports.iter().map(|r| r.score).sum::<u32>() / reports.len() as u32
    };

    let files: HashMap<String, u32> = reports
        .iter()
        .map(|r| {
            let path = r.file.to_string_lossy().to_string();
            (path, r.score)
        })
        .collect();

    let entry = HistoryEntry {
        timestamp,
        average_score,
        files,
    };

    let line = serde_json::to_string(&entry)?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(".kimi/score-history.jsonl")?;
    file.write_all(format!("{}\n", line).as_bytes())?;

    Ok(())
}

    /// { days > 0 }
    /// pub fn show_trend(days: u32) -> anyhow::Result<()>
    /// { prints ASCII bar chart of contract scores for the last `days` days }
pub fn show_trend(days: u32) -> anyhow::Result<()> {
    let path = Path::new(".kimi/score-history.jsonl");
    if !path.exists() {
        println!("No score history found. Run `cargo kimi check` to start tracking.");
        return Ok(());
    }

    let file = fs::File::open(path)?;
    let reader = io::BufReader::new(file);

    let cutoff = chrono::Utc::now() - chrono::Duration::days(days as i64);

    let mut entries = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let entry: HistoryEntry = serde_json::from_str(&line)?;
        if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(&entry.timestamp) {
            if ts.with_timezone(&chrono::Utc) >= cutoff {
                entries.push(entry);
            }
        }
    }

    if entries.is_empty() {
        println!("No score history found. Run `cargo kimi check` to start tracking.");
        return Ok(());
    }

    // Group by day, keeping the last entry per day.
    let mut by_day: HashMap<String, HistoryEntry> = HashMap::new();
    for entry in entries {
        let day = entry
            .timestamp
            .split('T')
            .next()
            .unwrap_or(&entry.timestamp)
            .to_string();
        by_day.insert(day, entry);
    }

    let mut days_vec: Vec<_> = by_day.into_iter().collect();
    days_vec.sort_by(|a, b| a.0.cmp(&b.0));

    println!("Score trend (last {} days):", days);
    for (day, entry) in days_vec {
        let score = entry.average_score.min(100);
        let bar_len = (score as usize) / 10;
        let bar = "█".repeat(bar_len) + &"░".repeat(10usize.saturating_sub(bar_len));
        println!("{}  {} {}/100", day, bar, entry.average_score);
    }

    Ok(())
}
