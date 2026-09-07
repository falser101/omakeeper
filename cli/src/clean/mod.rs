use crate::history;
use crate::util;
use crate::whitelist;
use anyhow::Result;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct CleanItem {
    pub id: String,
    pub category: String,
    pub label: String,
    pub path: String,
    pub bytes: u64,
    pub selected: bool,
}

#[derive(Debug, Serialize)]
struct CleanReport {
    dry_run: bool,
    items: Vec<CleanItem>,
    total_bytes: u64,
    freed_bytes: u64,
    removed: usize,
}

fn candidate_paths() -> Result<Vec<(String, String, PathBuf)>> {
    let home = util::home_dir()?;
    let mut out = Vec::new();

    let push = |out: &mut Vec<(String, String, PathBuf)>, cat: &str, label: &str, p: PathBuf| {
        if p.exists() {
            out.push((cat.to_string(), label.to_string(), p));
        }
    };

    // User caches (conservative, regenerable)
    push(
        &mut out,
        "user",
        "User cache root (thumbnails & app caches)",
        home.join(".cache/thumbnails"),
    );
    push(
        &mut out,
        "user",
        "Trash files",
        home.join(".local/share/Trash/files"),
    );
    push(
        &mut out,
        "user",
        "Trash info",
        home.join(".local/share/Trash/info"),
    );

    // Browsers
    push(
        &mut out,
        "browser",
        "Firefox cache",
        home.join(".cache/mozilla/firefox"),
    );
    push(
        &mut out,
        "browser",
        "Chromium cache",
        home.join(".cache/chromium"),
    );
    push(
        &mut out,
        "browser",
        "Google Chrome cache",
        home.join(".cache/google-chrome"),
    );
    push(
        &mut out,
        "browser",
        "Brave cache",
        home.join(".cache/BraveSoftware"),
    );

    // Dev tool caches
    push(&mut out, "dev", "pip cache", home.join(".cache/pip"));
    push(&mut out, "dev", "npm cache", home.join(".npm/_cacache"));
    push(
        &mut out,
        "dev",
        "pnpm store (metadata cache)",
        home.join(".cache/pnpm"),
    );
    push(
        &mut out,
        "dev",
        "cargo registry cache",
        home.join(".cargo/registry/cache"),
    );
    push(
        &mut out,
        "dev",
        "cargo git db",
        home.join(".cargo/git/db"),
    );
    push(
        &mut out,
        "dev",
        "go build cache",
        home.join(".cache/go-build"),
    );
    push(
        &mut out,
        "dev",
        "yarn berry cache",
        home.join(".yarn/berry/cache"),
    );

    // Package manager user caches (not system /var/cache/pacman — that needs root)
    push(
        &mut out,
        "packages",
        "yay cache",
        home.join(".cache/yay"),
    );
    push(
        &mut out,
        "packages",
        "paru cache",
        home.join(".cache/paru"),
    );

    Ok(out)
}

fn scan() -> Result<Vec<CleanItem>> {
    let mut items = Vec::new();
    for (cat, label, path) in candidate_paths()? {
        if whitelist::is_protected(&path) {
            continue;
        }
        let bytes = util::path_size(&path);
        if bytes == 0 {
            continue;
        }
        items.push(CleanItem {
            id: format!("{cat}:{}", path.display()),
            category: cat,
            label,
            path: path.display().to_string(),
            bytes,
            selected: true,
        });
    }
    items.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    Ok(items)
}

fn print_human(items: &[CleanItem], dry_run: bool) {
    let total: u64 = items.iter().map(|i| i.bytes).sum();
    let mode = if dry_run { "Dry run" } else { "Clean" };
    println!("{mode} — regenerable caches\n");
    if items.is_empty() {
        println!("Nothing to clean.");
        return;
    }

    let mut last_cat = String::new();
    for item in items {
        if item.category != last_cat {
            println!("➤ {}", item.category);
            last_cat = item.category.clone();
        }
        let mark = if item.selected { "●" } else { "○" };
        println!(
            "  {mark} {:<40} {:>10}",
            truncate(&item.label, 40),
            util::format_bytes(item.bytes)
        );
        println!("      {}", item.path);
    }
    println!(
        "\n{}",
        "=".repeat(64)
    );
    println!(
        "Total: {} across {} items",
        util::format_bytes(total),
        items.len()
    );
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let t: String = s.chars().take(max.saturating_sub(1)).collect();
    format!("{t}…")
}

pub fn run(dry_run: bool, yes: bool, json: bool) -> Result<()> {
    let mut items = scan()?;
    let total: u64 = items.iter().map(|i| i.bytes).sum();

    if json && dry_run {
        let report = CleanReport {
            dry_run: true,
            items: items.clone(),
            total_bytes: total,
            freed_bytes: 0,
            removed: 0,
        };
        println!("{}", serde_json::to_string_pretty(&report)?);
        history::log_operation("clean", true, total, items.len(), "scan")?;
        return Ok(());
    }

    print_human(&items, dry_run);

    if dry_run || items.is_empty() {
        history::log_operation("clean", true, total, items.len(), "scan")?;
        if dry_run && !items.is_empty() {
            println!("\nRe-run without --dry-run to delete selected items.");
        }
        return Ok(());
    }

    if !yes && !util::confirm(&format!(
        "Delete {} across {} items?",
        util::format_bytes(total),
        items.len()
    ))? {
        println!("Aborted.");
        return Ok(());
    }

    let mut freed = 0u64;
    let mut removed = 0usize;
    for item in &mut items {
        if !item.selected {
            continue;
        }
        let path = Path::new(&item.path);
        match util::remove_path(path) {
            Ok(()) => {
                freed += item.bytes;
                removed += 1;
                println!("  ✓ {}", item.label);
            }
            Err(e) => eprintln!("  ✗ {}: {e}", item.label),
        }
    }

    println!(
        "\nCleanup complete\nFreed {} · {} items",
        util::format_bytes(freed),
        removed
    );
    history::log_operation("clean", false, freed, removed, "apply")?;

    if json {
        let report = CleanReport {
            dry_run: false,
            items,
            total_bytes: total,
            freed_bytes: freed,
            removed,
        };
        println!("{}", serde_json::to_string_pretty(&report)?);
    }
    Ok(())
}
