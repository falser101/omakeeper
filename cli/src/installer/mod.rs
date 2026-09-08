use crate::history;
use crate::ui::picker::{self, PickItem};
use crate::util;
use anyhow::Result;
use serde::Serialize;
use std::path::{Path, PathBuf};

const EXTENSIONS: &[&str] = &[
    "iso", "img", "dmg", "pkg", "mpkg", "appimage", "deb", "rpm", "msi", "exe", "xip", "run",
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
    let home = util::home_dir()?;
    Ok(vec![
        ("Downloads".into(), home.join("Downloads")),
        ("Desktop".into(), home.join("Desktop")),
        ("Documents".into(), home.join("Documents")),
        ("Public".into(), home.join("Public")),
        ("Telegram".into(), home.join("Downloads/Telegram Desktop")),
        ("Telegram".into(), home.join(".local/share/TelegramDesktop")),
        ("WeChat".into(), home.join("Documents/xwechat_files")),
    ])
}

fn matches_installer(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    EXTENSIONS.iter().any(|ext| name.ends_with(&format!(".{ext}")))
}

fn source_for(path: &Path, roots: &[(String, PathBuf)]) -> String {
    for (name, root) in roots {
        if path.starts_with(root) {
            return name.clone();
        }
    }
    path.parent()
        .and_then(|p| p.file_name())
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "other".into())
}

fn scan() -> Result<Vec<InstallerFile>> {
    let roots = search_roots()?;
    let mut files = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (source, root) in &roots {
        if !root.is_dir() {
            continue;
        }
        let depth = if source == "Telegram" { 3 } else { 2 };
        for entry in walkdir::WalkDir::new(root)
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
            let canon = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
            if !seen.insert(canon) {
                continue;
            }
            let bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
            if bytes == 0 {
                continue;
            }
            files.push(InstallerFile {
                name: entry.file_name().to_string_lossy().to_string(),
                path: path.display().to_string(),
                source: source_for(path, &roots),
                bytes,
                selected: false,
            });
        }
    }
    files.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    Ok(files)
}

fn print_human(files: &[InstallerFile]) {
    println!("Installer files\n");
    if files.is_empty() {
        println!("No installer files found.");
        return;
    }
    for f in files {
        let mark = if f.selected { "●" } else { "○" };
        println!(
            "  {mark} {:<36} {:>10} | {}",
            util::truncate_right(&f.name, 36),
            util::format_bytes(f.bytes),
            f.source
        );
    }
    let total: u64 = files.iter().filter(|f| f.selected).map(|f| f.bytes).sum();
    let count = files.iter().filter(|f| f.selected).count();
    println!(
        "\nSelected: {} · {count} files",
        util::format_bytes(total)
    );
}

fn pick(files: &mut [InstallerFile]) -> Result<bool> {
    let mut rows: Vec<PickItem> = files
        .iter()
        .map(|f| PickItem {
            selected: f.selected,
            locked: false,
            title: f.name.clone(),
            detail: format!("{}  {}", f.source, f.path),
            bytes: f.bytes,
        })
        .collect();
    let ok = picker::select_items("Select installers to remove", &mut rows)?;
    if ok {
        for (file, row) in files.iter_mut().zip(rows) {
            file.selected = row.selected;
        }
    }
    Ok(ok)
}

pub fn run(dry_run: bool, yes: bool, json: bool) -> Result<()> {
    let mut files = scan()?;

    if json && dry_run {
        println!("{}", serde_json::to_string_pretty(&files)?);
        history::log_operation("installer", true, 0, files.len(), "scan")?;
        return Ok(());
    }

    if files.is_empty() {
        println!("No installer files found.");
        return Ok(());
    }

    if yes {
        for f in &mut files {
            f.selected = true;
        }
    } else if !json {
        if util::is_tty() {
            if !pick(&mut files)? {
                println!("Aborted.");
                return Ok(());
            }
        } else {
            print_human(&files);
            println!("\nPass --yes to delete all found installers, or run in a terminal to pick.");
            return Ok(());
        }
    }

    let total: u64 = files.iter().filter(|f| f.selected).map(|f| f.bytes).sum();
    let count = files.iter().filter(|f| f.selected).count();

    if dry_run {
        if !json {
            print_human(&files);
            println!("\nRe-run without --dry-run to delete selected files.");
        } else {
            println!("{}", serde_json::to_string_pretty(&files)?);
        }
        history::log_operation("installer", true, total, count, "scan")?;
        return Ok(());
    }

    if count == 0 {
        println!("Nothing selected.");
        return Ok(());
    }

    if !yes
        && !util::confirm(&format!(
            "Delete {} across {count} installer files?",
            util::format_bytes(total)
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
        "\nInstallers cleaned\nFreed {} · {removed} files",
        util::format_bytes(freed)
    );
    history::log_operation("installer", false, freed, removed, "apply")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_linux_and_generic_installers() {
        assert!(matches_installer(Path::new("/tmp/Foo.AppImage")));
        assert!(matches_installer(Path::new("/tmp/archlinux.iso")));
        assert!(matches_installer(Path::new("/tmp/pkg.pkg.tar.zst")));
        assert!(matches_installer(Path::new("/tmp/setup.run")));
        assert!(!matches_installer(Path::new("/tmp/notes.zip")));
        assert!(!matches_installer(Path::new("/tmp/photo.png")));
    }
}
