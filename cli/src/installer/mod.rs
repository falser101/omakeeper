use crate::history;
use crate::util;
use anyhow::Result;
use serde::Serialize;
use std::path::{Path, PathBuf};

const EXTENSIONS: &[&str] = &[
    "iso", "img", "dmg", "pkg", "mpkg", "appimage", "AppImage", "deb", "rpm", "msi", "exe", "xip",
    "pkg.tar.zst", "pkg.tar.xz", "pkg.tar.gz", "tar.zst",
];

#[derive(Debug, Clone, Serialize)]
pub struct InstallerFile {
    pub name: String,
    pub path: String,
    pub source: String,
    pub bytes: u64,
    pub selected: bool,
}

fn search_roots() -> Result<Vec<(String, PathBuf)>> {
    // Package-manager caches are handled by `clean` (yay/paru). Installer
    // focuses on leftover downloads the user dropped on the desktop.
    let home = util::home_dir()?;
    Ok(vec![
        ("Downloads".into(), home.join("Downloads")),
        ("Desktop".into(), home.join("Desktop")),
    ])
}

fn matches_installer(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    EXTENSIONS.iter().any(|ext| {
        let e = ext.to_ascii_lowercase();
        name.ends_with(&format!(".{e}"))
    })
}

fn scan() -> Result<Vec<InstallerFile>> {
    let mut files = Vec::new();
    for (source, root) in search_roots()? {
        if !root.is_dir() {
            continue;
        }
        let depth = if source.contains("cache") { 3 } else { 2 };
        for entry in walkdir::WalkDir::new(&root)
            .max_depth(depth)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if !matches_installer(path) {
                continue;
            }
            let bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
            if bytes == 0 {
                continue;
            }
            files.push(InstallerFile {
                name: entry
                    .file_name()
                    .to_string_lossy()
                    .to_string(),
                path: path.display().to_string(),
                source: source.clone(),
                bytes,
                selected: true,
            });
        }
    }
    files.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    Ok(files)
}

pub fn run(dry_run: bool, yes: bool, json: bool) -> Result<()> {
    let files = scan()?;
    let total: u64 = files.iter().filter(|f| f.selected).map(|f| f.bytes).sum();
    let count = files.iter().filter(|f| f.selected).count();

    if json && dry_run {
        println!("{}", serde_json::to_string_pretty(&files)?);
        history::log_operation("installer", true, total, count, "scan")?;
        return Ok(());
    }

    println!("Installer files\n");
    if files.is_empty() {
        println!("No installer files found.");
        return Ok(());
    }

    for f in &files {
        let mark = if f.selected { "●" } else { "○" };
        println!(
            "  {mark} {:<36} {:>10} | {}",
            truncate(&f.name, 36),
            util::format_bytes(f.bytes),
            f.source
        );
    }
    println!(
        "\nSelected: {} · {} files",
        util::format_bytes(total),
        count
    );

    if dry_run {
        history::log_operation("installer", true, total, count, "scan")?;
        println!("\nRe-run without --dry-run to delete selected files.");
        return Ok(());
    }

    if !yes
        && !util::confirm(&format!(
            "Delete {} across {} installer files?",
            util::format_bytes(total),
            count
        ))?
    {
        println!("Aborted.");
        return Ok(());
    }

    let mut freed = 0u64;
    let mut removed = 0usize;
    for f in &files {
        if !f.selected {
            continue;
        }
        match util::remove_path(Path::new(&f.path)) {
            Ok(()) => {
                freed += f.bytes;
                removed += 1;
                println!("  ✓ {}", f.name);
            }
            Err(e) => eprintln!("  ✗ {}: {e}", f.name),
        }
    }
    println!(
        "\nInstallers cleaned\nFreed {} · {} files",
        util::format_bytes(freed),
        removed
    );
    history::log_operation("installer", false, freed, removed, "apply")?;
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let t: String = s.chars().take(max.saturating_sub(1)).collect();
    format!("{t}…")
}
