use anyhow::{Context, Result};
use humansize::{format_size, BINARY};
use std::collections::HashSet;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub fn load_id_file(path: &Path) -> Result<HashSet<String>> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let ids: Vec<String> = serde_json::from_str(&text)
        .with_context(|| format!("parse select file {}", path.display()))?;
    Ok(ids.into_iter().filter(|s| !s.is_empty()).collect())
}

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

pub fn is_tty() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal()
}

pub fn command_exists(bin: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths).any(|dir| dir.join(bin).is_file())
        })
        .unwrap_or(false)
}

pub fn sudo_cached() -> bool {
    Command::new("sudo")
        .args(["-n", "true"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn path_size(path: &Path) -> u64 {
    if path.is_file() {
        return path.metadata().map(|m| m.len()).unwrap_or(0);
    }
    if !path.is_dir() {
        return 0;
    }
    du_size(path).unwrap_or_else(|| walkdir_size(path))
}

fn du_size(path: &Path) -> Option<u64> {
    let out = Command::new("du")
        .args(["-sbP"])
        .arg(path)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let (n, _) = text.split_once(char::is_whitespace)?;
    n.parse().ok()
}

fn walkdir_size(path: &Path) -> u64 {
    walkdir::WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
        .sum()
}

pub fn path_sizes(paths: &[PathBuf]) -> Vec<u64> {
    std::thread::scope(|scope| {
        let handles: Vec<_> = paths.iter().map(|p| scope.spawn(|| path_size(p))).collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap_or(0))
            .collect()
    })
}

pub fn confirm(prompt: &str) -> Result<bool> {
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

pub fn running_comms() -> HashSet<String> {
    let mut set = HashSet::new();
    let Ok(entries) = fs::read_dir("/proc") else {
        return set;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(pid) = name.to_str() else { continue };
        if !pid.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        if let Ok(comm) = fs::read_to_string(entry.path().join("comm")) {
            let comm = comm.trim().to_ascii_lowercase();
            if !comm.is_empty() {
                set.insert(comm);
            }
        }
    }
    set
}

pub fn any_running(running: &HashSet<String>, names: &[&str]) -> bool {
    names
        .iter()
        .any(|n| running.contains(&n.to_ascii_lowercase()))
}

pub fn is_dangerous_delete(path: &Path) -> bool {
    let Ok(home) = home_dir() else {
        return true;
    };
    let canon = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if canon == Path::new("/") || canon == home {
        return true;
    }
    const BLOCKED: &[&str] = &[
        "/usr", "/etc", "/boot", "/dev", "/proc", "/sys", "/run", "/var", "/opt", "/root",
    ];
    if BLOCKED.iter().any(|p| canon == Path::new(p) || canon.starts_with(p)) {
        return true;
    }
    !canon.starts_with(&home)
}

pub fn move_to_trash(path: &Path) -> Result<()> {
    if is_dangerous_delete(path) {
        anyhow::bail!("refusing to trash protected path {}", path.display());
    }
    let home = home_dir()?;
    let files_dir = home.join(".local/share/Trash/files");
    let info_dir = home.join(".local/share/Trash/info");
    fs::create_dir_all(&files_dir)?;
    fs::create_dir_all(&info_dir)?;

    let base = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "item".into());
    let mut dest = files_dir.join(&base);
    let mut info = info_dir.join(format!("{base}.trashinfo"));
    let mut n = 1u32;
    while dest.exists() || info.exists() {
        n += 1;
        dest = files_dir.join(format!("{base}.{n}"));
        info = info_dir.join(format!("{base}.{n}.trashinfo"));
    }

    let abs = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let stamp = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S");
    let body = format!("[Trash Info]\nPath={}\nDeletionDate={stamp}\n", abs.display());
    fs::write(&info, body)?;
    fs::rename(path, &dest).with_context(|| {
        format!("move {} to trash", path.display())
    })?;
    Ok(())
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DiskStat {
    pub total: u64,
    pub used: u64,
    pub available: u64,
}

pub fn disk_stat(path: &Path) -> Option<DiskStat> {
    let out = Command::new("df")
        .args(["-B1", "-P"])
        .arg(path)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().nth(1)?;
    let cols: Vec<_> = line.split_whitespace().collect();
    if cols.len() < 4 {
        return None;
    }
    Some(DiskStat {
        total: cols[1].parse().ok()?,
        used: cols[2].parse().ok()?,
        available: cols[3].parse().ok()?,
    })
}

pub fn free_space(path: &Path) -> Option<u64> {
    disk_stat(path).map(|s| s.available)
}

pub fn truncate_left(s: &str, max: usize) -> String {
    let count = s.chars().count();
    if count <= max {
        return s.to_string();
    }
    let take = max.saturating_sub(1);
    let t: String = s.chars().skip(count - take).collect();
    format!("…{t}")
}

pub fn truncate_right(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let t: String = s.chars().take(max.saturating_sub(1)).collect();
    format!("{t}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_helpers() {
        assert_eq!(truncate_right("abcd", 4), "abcd");
        assert_eq!(truncate_right("abcde", 4), "abc…");
        assert_eq!(truncate_left("abcdef", 4), "…def");
    }

    #[test]
    fn dangerous_paths() {
        assert!(is_dangerous_delete(Path::new("/")));
        assert!(is_dangerous_delete(Path::new("/usr")));
        assert!(is_dangerous_delete(Path::new("/etc/passwd")));
    }
}
