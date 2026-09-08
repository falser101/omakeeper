use crate::history;
use crate::ui::picker::{self, PickItem};
use crate::util;
use anyhow::Result;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const ARTIFACT_NAMES: &[&str] = &[
    "node_modules",
    "target",
    ".next",
    "dist",
    "build",
    ".build",
    "__pycache__",
    ".tox",
    ".venv",
    "venv",
    ".turbo",
    ".cache",
];

#[derive(Debug, Clone, Serialize)]
pub struct Artifact {
    pub project: String,
    pub name: String,
    pub path: String,
    pub bytes: u64,
    pub age_days: i64,
    pub selected: bool,
}

fn default_roots() -> Result<Vec<PathBuf>> {
    let home = util::home_dir()?;
    let candidates = [
        home.join("Projects"),
        home.join("projects"),
        home.join("dev"),
        home.join("Dev"),
        home.join("src"),
        home.join("code"),
        home.join("GitHub"),
        home.join("github"),
        home.join("Work"),
        home.join("work"),
    ];
    Ok(candidates.into_iter().filter(|p| p.is_dir()).collect())
}

fn paths_file() -> Result<PathBuf> {
    Ok(util::config_dir()?.join("purge_paths"))
}

fn configured_roots() -> Result<Vec<PathBuf>> {
    let file = paths_file()?;
    if !file.exists() {
        return default_roots();
    }
    let text = fs::read_to_string(&file)?;
    let roots: Vec<PathBuf> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(util::expand_user)
        .filter(|p| p.is_dir())
        .collect();
    if roots.is_empty() {
        default_roots()
    } else {
        Ok(roots)
    }
}

pub fn manage_paths(json: bool) -> Result<()> {
    let file = paths_file()?;
    if !file.exists() {
        let defaults = default_roots()?;
        let body = defaults
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(&file, format!("# One scan root per line\n{body}\n"))?;
    }
    if json {
        let roots = configured_roots()?;
        let list: Vec<_> = roots.iter().map(|p| p.display().to_string()).collect();
        println!("{}", serde_json::to_string_pretty(&list)?);
        return Ok(());
    }
    println!("Purge scan roots file: {}", file.display());
    println!("{}", fs::read_to_string(&file)?);
    println!("Edit this file, then re-run: omakeeper purge");
    Ok(())
}

fn dir_mtime_days(path: &Path) -> i64 {
    let meta = match fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return 999,
    };
    let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    let now = SystemTime::now();
    match now.duration_since(modified) {
        Ok(d) => (d.as_secs() / 86400) as i64,
        Err(_) => 0,
    }
}

fn scan_root(root: &Path, out: &mut Vec<Artifact>) {
    let walker = walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            // Do not descend into artifact dirs themselves
            let name = e.file_name().to_string_lossy();
            if e.depth() > 0 && ARTIFACT_NAMES.contains(&name.as_ref()) {
                return false;
            }
            // Skip hidden dirs except known artifacts at discovery time
            if e.depth() > 0 && name.starts_with('.') && !ARTIFACT_NAMES.contains(&name.as_ref()) {
                return false;
            }
            true
        });

    for entry in walker.filter_map(|e| e.ok()) {
        if !entry.file_type().is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !ARTIFACT_NAMES.contains(&name.as_str()) {
            continue;
        }
        // Skip root-level .cache under home tooling — only project-local
        if name == ".cache" && entry.depth() <= 1 {
            continue;
        }
        let path = entry.path().to_path_buf();
        let bytes = util::path_size(&path);
        if bytes == 0 {
            continue;
        }
        let age_days = dir_mtime_days(&path);
        let project = path
            .parent()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| path.display().to_string());
        // Recent (<7d) unselected by default
        let selected = age_days >= 7;
        out.push(Artifact {
            project,
            name,
            path: path.display().to_string(),
            bytes,
            age_days,
            selected,
        });
    }
}

fn scan() -> Result<Vec<Artifact>> {
    let roots = configured_roots()?;
    if roots.is_empty() {
        anyhow::bail!(
            "No project roots found. Create ~/Projects or run: omakeeper purge --paths"
        );
    }
    let mut artifacts = Vec::new();
    for root in &roots {
        scan_root(root, &mut artifacts);
    }
    artifacts.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    Ok(artifacts)
}

fn age_label(days: i64) -> String {
    if days < 1 {
        "<1d".into()
    } else if days < 7 {
        format!("{days}d")
    } else if days < 30 {
        format!("{}w", days / 7)
    } else if days < 365 {
        format!("{}mo", days / 30)
    } else {
        format!("{}y", days / 365)
    }
}

fn print_artifacts(artifacts: &[Artifact]) {
    println!("Purge project artifacts\n");
    if artifacts.is_empty() {
        println!("No artifacts found under configured roots.");
        return;
    }
    for a in artifacts {
        let mark = if a.selected { "●" } else { "○" };
        println!(
            "  {mark} {:<28} {:>10} | {:<12} | {}",
            truncate_path(&a.project, 28),
            util::format_bytes(a.bytes),
            a.name,
            age_label(a.age_days)
        );
    }
    let selected_bytes: u64 = artifacts
        .iter()
        .filter(|a| a.selected)
        .map(|a| a.bytes)
        .sum();
    let selected_count = artifacts.iter().filter(|a| a.selected).count();
    println!(
        "\nSelected: {} · {}",
        util::format_bytes(selected_bytes),
        selected_count
    );
    println!("(○ = modified in last 7 days, skipped by default)");
}

pub fn run(dry_run: bool, yes: bool, json: bool) -> Result<()> {
    let mut artifacts = scan()?;
    let selected_bytes: u64 = artifacts
        .iter()
        .filter(|a| a.selected)
        .map(|a| a.bytes)
        .sum();
    let selected_count = artifacts.iter().filter(|a| a.selected).count();

    if json && dry_run {
        println!("{}", serde_json::to_string_pretty(&artifacts)?);
        history::log_operation("purge", true, selected_bytes, selected_count, "scan")?;
        return Ok(());
    }

    if artifacts.is_empty() {
        println!("No artifacts found under configured roots.");
        return Ok(());
    }

    if !yes && !json {
        if util::is_tty() {
            let mut rows: Vec<PickItem> = artifacts
                .iter()
                .map(|a| PickItem {
                    selected: a.selected,
                    locked: false,
                    title: format!("{}  {}", a.name, truncate_path(&a.project, 28)),
                    detail: format!("{} · {}", a.project, age_label(a.age_days)),
                    bytes: a.bytes,
                })
                .collect();
            if !picker::select_items("Purge project artifacts", &mut rows)? {
                println!("Aborted.");
                return Ok(());
            }
            for (artifact, row) in artifacts.iter_mut().zip(rows) {
                artifact.selected = row.selected;
            }
        } else {
            print_artifacts(&artifacts);
            println!("\nPass --yes to delete the default selection, or run in a terminal to pick.");
            return Ok(());
        }
    } else if !json {
        print_artifacts(&artifacts);
    }

    let selected_bytes: u64 = artifacts
        .iter()
        .filter(|a| a.selected)
        .map(|a| a.bytes)
        .sum();
    let selected_count = artifacts.iter().filter(|a| a.selected).count();

    if dry_run {
        if json {
            println!("{}", serde_json::to_string_pretty(&artifacts)?);
        } else if yes || !util::is_tty() {
            // already printed
        } else {
            print_artifacts(&artifacts);
        }
        history::log_operation("purge", true, selected_bytes, selected_count, "scan")?;
        println!("\nRe-run without --dry-run to delete selected artifacts.");
        return Ok(());
    }

    if selected_count == 0 {
        println!("Nothing selected.");
        return Ok(());
    }

    if !yes
        && !util::confirm(&format!(
            "Permanently delete {} across {} artifacts?",
            util::format_bytes(selected_bytes),
            selected_count
        ))?
    {
        println!("Aborted.");
        return Ok(());
    }

    let mut freed = 0u64;
    let mut removed = 0usize;
    for a in &artifacts {
        if !a.selected {
            continue;
        }
        match util::remove_path(Path::new(&a.path)) {
            Ok(()) => {
                freed += a.bytes;
                removed += 1;
                println!("  ✓ {} ({})", a.name, a.project);
            }
            Err(e) => eprintln!("  ✗ {}: {e}", a.path),
        }
    }
    println!(
        "\nPurge complete\nFreed {} · {} items",
        util::format_bytes(freed),
        removed
    );
    history::log_operation("purge", false, freed, removed, "apply")?;
    Ok(())
}

fn truncate_path(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let t: String = s.chars().skip(s.chars().count() - (max - 1)).collect();
    format!("…{t}")
}
