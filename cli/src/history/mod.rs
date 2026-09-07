use crate::util;
use anyhow::Result;
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub ts: String,
    pub command: String,
    pub dry_run: bool,
    pub freed_bytes: u64,
    pub items: usize,
    pub detail: String,
}

pub fn log_operation(
    command: &str,
    dry_run: bool,
    freed_bytes: u64,
    items: usize,
    detail: &str,
) -> Result<()> {
    let path = util::data_dir()?.join("operations.log");
    let entry = Entry {
        ts: Local::now().format("%Y-%m-%dT%H:%M:%S%z").to_string(),
        command: command.to_string(),
        dry_run,
        freed_bytes,
        items,
        detail: detail.to_string(),
    };
    let line = serde_json::to_string(&entry)?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    writeln!(file, "{line}")?;
    Ok(())
}

pub fn show(limit: usize, json: bool) -> Result<()> {
    let path = util::data_dir()?.join("operations.log");
    if !path.exists() {
        if json {
            println!("[]");
        } else {
            println!("No operations logged yet.");
        }
        return Ok(());
    }

    let text = fs::read_to_string(&path)?;
    let mut entries: Vec<Entry> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    entries.reverse();
    entries.truncate(limit);

    if json {
        println!("{}", serde_json::to_string_pretty(&entries)?);
        return Ok(());
    }

    if entries.is_empty() {
        println!("No operations logged yet.");
        return Ok(());
    }

    println!("Recent Omakeeper operations\n");
    for e in &entries {
        let mode = if e.dry_run { "dry-run" } else { "apply" };
        println!(
            "  {}  {} [{}]  {} · {} items",
            e.ts,
            e.command,
            mode,
            util::format_bytes(e.freed_bytes),
            e.items
        );
        if !e.detail.is_empty() {
            println!("    {}", e.detail);
        }
    }
    Ok(())
}
