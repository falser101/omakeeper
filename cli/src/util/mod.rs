use anyhow::{Context, Result};
use humansize::{format_size, BINARY};
use std::fs;
use std::path::{Path, PathBuf};

pub fn home_dir() -> Result<PathBuf> {
    dirs::home_dir().context("HOME is not set")
}

pub fn config_dir() -> Result<PathBuf> {
    let dir = home_dir()?.join(".config").join("omakeeper");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn data_dir() -> Result<PathBuf> {
    let dir = home_dir()?.join(".local").join("share").join("omakeeper");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn format_bytes(bytes: u64) -> String {
    format_size(bytes, BINARY)
}

pub fn path_size(path: &Path) -> u64 {
    if path.is_file() {
        return path.metadata().map(|m| m.len()).unwrap_or(0);
    }
    if !path.is_dir() {
        return 0;
    }
    walkdir::WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
        .sum()
}

pub fn confirm(prompt: &str) -> Result<bool> {
    use std::io::{self, Write};
    print!("{prompt} [y/N] ");
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    let t = line.trim().to_ascii_lowercase();
    Ok(t == "y" || t == "yes")
}

pub fn remove_path(path: &Path) -> Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path).with_context(|| format!("remove dir {}", path.display()))?;
    } else if path.exists() {
        fs::remove_file(path).with_context(|| format!("remove file {}", path.display()))?;
    }
    Ok(())
}

pub fn expand_user(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = home_dir() {
            return home.join(rest);
        }
    }
    if path == "~" {
        if let Ok(home) = home_dir() {
            return home;
        }
    }
    PathBuf::from(path)
}

pub fn is_under(child: &Path, parent: &Path) -> bool {
    child.starts_with(parent)
}
