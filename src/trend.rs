// kimi:score-ignore=unwrap
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

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct Days(u32);

impl Days {
#[allow(dead_code)]
    pub(crate) fn new(value: u32) -> Self {
        Self(value)
    }

    pub(crate) fn get(self) -> u32 {
        self.0
    }
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

fn load_entries(days: Days) -> anyhow::Result<Vec<HistoryEntry>> {
    let path = Path::new(".kimi/score-history.jsonl");
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = fs::File::open(path)?;
    let reader = io::BufReader::new(file);

    let cutoff = chrono::Utc::now() - chrono::Duration::days(days.get() as i64);

    let mut entries = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let entry: HistoryEntry = match serde_json::from_str(&line) {
            Ok(e) => e,
            Err(_) => {
                eprintln!("Warning: skipping corrupted history entry");
                continue;
            }
        };
        if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(&entry.timestamp) {
            if ts.with_timezone(&chrono::Utc) >= cutoff {
                entries.push(entry);
            }
        }
    }

    Ok(entries)
}

fn group_by_day(entries: Vec<HistoryEntry>) -> Vec<(String, HistoryEntry)> {
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
    days_vec
}

fn print_chart(days_vec: &[(String, HistoryEntry)]) {
    let max_label = days_vec.iter().map(|(d, _)| d.len()).max().unwrap_or(0);
    let highest_score = days_vec
        .iter()
        .map(|(_, e)| e.average_score)
        .max()
        .unwrap_or(100)
        .max(1);

    for (day, entry) in days_vec {
        let bar_len = ((entry.average_score as f64 / highest_score as f64) * 20.0).round() as usize;
        let bar = "█".repeat(bar_len);
        println!(
            "{:>width$} │ {:3} {}",
            day,
            entry.average_score,
            bar,
            width = max_label
        );
    }

    println!();
    println!("Entries: {}", days_vec.len());
}

/// { days > 0 }
/// pub fn show_trend(days: u32) -> anyhow::Result<()>
/// { prints ASCII bar chart of contract scores for the last `days` days }
pub fn show_trend(days: u32) -> anyhow::Result<()> {
    let days = Days::new(days);
    let entries = load_entries(days)?;
    if entries.is_empty() {
        println!("No score history found. Run `cargo kimi check` to start tracking.");
        return Ok(());
    }
    let days_vec = group_by_day(entries);
    println!("📊 Score trend (last {} days):", days.get());
    println!();
    print_chart(&days_vec);
    Ok(())
}
