use crate::history;
use crate::ui::picker::{self, PickItem};
use crate::util;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
enum TaskStatus {
    Ready,
    Unchanged,
    Unavailable,
    Skipped,
    Applied,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
struct Task {
    id: String,
    label: String,
    detail: String,
    bytes: u64,
    selected: bool,
    status: TaskStatus,
}

#[derive(Debug, Serialize)]
struct OptimizeReport {
    dry_run: bool,
    diagnosis: Vec<String>,
    tasks: Vec<Task>,
    applied: usize,
    skipped: usize,
    unavailable: usize,
    freed_bytes: u64,
}

#[derive(Default, Serialize, Deserialize)]
struct SkipStore {
    ids: Vec<String>,
}

fn skip_path() -> Result<PathBuf> {
    Ok(util::config_dir()?.join("optimize_skip.json"))
}

fn load_skips() -> Result<Vec<String>> {
    let path = skip_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let store: SkipStore = serde_json::from_str(&fs::read_to_string(path)?).unwrap_or_default();
    Ok(store.ids)
}

fn save_skips(ids: Vec<String>) -> Result<()> {
    let path = skip_path()?;
    fs::write(path, serde_json::to_string_pretty(&SkipStore { ids })?)?;
    Ok(())
}

fn parse_si_token(token: &str) -> Option<u64> {
    let token = token.trim().trim_end_matches('.');
    let split = token.find(|c: char| c.is_ascii_alphabetic())?;
    let (num, unit) = token.split_at(split);
    let n: f64 = num.parse().ok()?;
    let mul = match unit.to_ascii_uppercase().as_str() {
        "B" => 1.0,
        "K" | "KB" | "KIB" => 1024.0,
        "M" | "MB" | "MIB" => 1024.0 * 1024.0,
        "G" | "GB" | "GIB" => 1024.0 * 1024.0 * 1024.0,
        "T" | "TB" | "TIB" => 1024.0_f64.powi(4),
        _ => return None,
    };
    Some((n * mul) as u64)
}

fn journal_usage(user: bool) -> Option<u64> {
    let mut cmd = Command::new("journalctl");
    if user {
        cmd.arg("--user");
    }
    let out = cmd.arg("--disk-usage").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let after = text.split("take up ").nth(1)?;
    let token = after.split_whitespace().next()?;
    parse_si_token(token)
}

fn diagnosis() -> Vec<String> {
    let mut lines = Vec::new();
    if let Ok(home) = util::home_dir() {
        if let Some(avail) = util::free_space(&home) {
            lines.push(format!("Free space {}", util::format_bytes(avail)));
        }
    }
    if let Some(j) = journal_usage(true) {
        let note = if j > 64 * 1024 * 1024 {
            " (vacuum recommended)"
        } else {
            ""
        };
        lines.push(format!("User journal {}{note}", util::format_bytes(j)));
    }
    if let Ok(load) = fs::read_to_string("/proc/loadavg") {
        if let Some(one) = load.split_whitespace().next() {
            lines.push(format!("Load {one}"));
        }
    }
    lines
}

fn run_ok(bin: &str, args: &[&str]) -> Result<String> {
    let out = Command::new(bin).args(args).output()?;
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if !out.status.success() {
        anyhow::bail!(if stderr.is_empty() { stdout } else { stderr });
    }
    Ok(if stdout.is_empty() { stderr } else { stdout })
}

fn catalog(skips: &[String]) -> Vec<Task> {
    let mut tasks = Vec::new();
    let skipped = |id: &str| skips.iter().any(|s| s == id);

    let user_journal = journal_usage(true).unwrap_or(0);
    tasks.push(Task {
        id: "journal-user".into(),
        label: "Vacuum user journal (7d)".into(),
        detail: if user_journal == 0 {
            "journalctl --user not available".into()
        } else if user_journal < 8 * 1024 * 1024 {
            "already small".into()
        } else {
            format!("current {}", util::format_bytes(user_journal))
        },
        bytes: user_journal,
        selected: user_journal >= 8 * 1024 * 1024 && !skipped("journal-user"),
        status: if !util::command_exists("journalctl") {
            TaskStatus::Unavailable
        } else if skipped("journal-user") {
            TaskStatus::Skipped
        } else if user_journal < 8 * 1024 * 1024 {
            TaskStatus::Unchanged
        } else {
            TaskStatus::Ready
        },
    });

    let sys_journal = journal_usage(false).unwrap_or(0);
    let sudo = util::sudo_cached();
    tasks.push(Task {
        id: "journal-system".into(),
        label: "Vacuum system journal (7d)".into(),
        detail: if !sudo {
            "needs sudo (run sudo -v first)".into()
        } else if sys_journal < 8 * 1024 * 1024 {
            "already small".into()
        } else {
            format!("current {}", util::format_bytes(sys_journal))
        },
        bytes: sys_journal,
        selected: sudo && sys_journal >= 8 * 1024 * 1024 && !skipped("journal-system"),
        status: if !util::command_exists("journalctl") {
            TaskStatus::Unavailable
        } else if skipped("journal-system") {
            TaskStatus::Skipped
        } else if !sudo {
            TaskStatus::Skipped
        } else if sys_journal < 8 * 1024 * 1024 {
            TaskStatus::Unchanged
        } else {
            TaskStatus::Ready
        },
    });

    push_cmd_task(
        &mut tasks,
        skips,
        "fontconfig",
        "Rebuild font cache",
        "fc-cache -f",
        "fc-cache",
        true,
    );
    let home = util::home_dir().ok();
    let apps = home.as_ref().map(|h| h.join(".local/share/applications"));
    push_cmd_task(
        &mut tasks,
        skips,
        "desktop-database",
        "Refresh desktop database",
        "update-desktop-database",
        "update-desktop-database",
        apps.as_ref().map(|p| p.is_dir()).unwrap_or(false),
    );
    let mime = home.as_ref().map(|h| h.join(".local/share/mime"));
    push_cmd_task(
        &mut tasks,
        skips,
        "mime-database",
        "Refresh MIME database",
        "update-mime-database",
        "update-mime-database",
        mime.as_ref().map(|p| p.is_dir()).unwrap_or(false),
    );

    let icon_bin = if util::command_exists("gtk4-update-icon-cache") {
        Some("gtk4-update-icon-cache")
    } else if util::command_exists("gtk-update-icon-cache") {
        Some("gtk-update-icon-cache")
    } else {
        None
    };
    let icon_dirs = user_icon_dirs();
    tasks.push(Task {
        id: "icon-cache".into(),
        label: "Rebuild icon caches".into(),
        detail: if icon_bin.is_none() {
            "gtk-update-icon-cache not installed".into()
        } else if icon_dirs.is_empty() {
            "no user icon themes".into()
        } else {
            format!("{} theme(s)", icon_dirs.len())
        },
        bytes: 0,
        selected: icon_bin.is_some() && !icon_dirs.is_empty() && !skipped("icon-cache"),
        status: if icon_bin.is_none() || icon_dirs.is_empty() {
            TaskStatus::Unavailable
        } else if skipped("icon-cache") {
            TaskStatus::Skipped
        } else {
            TaskStatus::Ready
        },
    });

    push_cmd_task(
        &mut tasks,
        skips,
        "resolved-flush",
        "Flush DNS caches",
        "resolvectl flush-caches",
        "resolvectl",
        true,
    );
    push_cmd_task(
        &mut tasks,
        skips,
        "tmpfiles-user",
        "Clean user temp files",
        "systemd-tmpfiles --user --clean",
        "systemd-tmpfiles",
        true,
    );
    push_cmd_task(
        &mut tasks,
        skips,
        "sync",
        "Sync filesystems",
        "sync",
        "sync",
        true,
    );
    push_cmd_task(
        &mut tasks,
        skips,
        "flatpak-repair",
        "Repair user Flatpak",
        "flatpak repair --user",
        "flatpak",
        true,
    );

    tasks
}

fn push_cmd_task(
    tasks: &mut Vec<Task>,
    skips: &[String],
    id: &str,
    label: &str,
    detail: &str,
    bin: &str,
    extra_ok: bool,
) {
    let exists = util::command_exists(bin);
    let skipped = skips.iter().any(|s| s == id);
    tasks.push(Task {
        id: id.into(),
        label: label.into(),
        detail: if !exists {
            format!("{bin} not installed")
        } else if !extra_ok {
            "nothing to do".into()
        } else {
            detail.into()
        },
        bytes: 0,
        selected: exists && extra_ok && !skipped,
        status: if !exists || !extra_ok {
            TaskStatus::Unavailable
        } else if skipped {
            TaskStatus::Skipped
        } else {
            TaskStatus::Ready
        },
    });
}

fn user_icon_dirs() -> Vec<PathBuf> {
    let Ok(home) = util::home_dir() else {
        return Vec::new();
    };
    let mut dirs = Vec::new();
    for root in [home.join(".local/share/icons"), home.join(".icons")] {
        let Ok(entries) = fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() && p.join("index.theme").is_file() {
                dirs.push(p);
            }
        }
    }
    dirs
}

fn apply_task(task: &Task, dry_run: bool) -> (TaskStatus, String, u64) {
    if dry_run {
        return (TaskStatus::Ready, format!("would {}", task.detail), 0);
    }
    match task.id.as_str() {
        "journal-user" => {
            let before = journal_usage(true).unwrap_or(0);
            match run_ok("journalctl", &["--user", "--vacuum-time=7d"]) {
                Ok(msg) => {
                    let after = journal_usage(true).unwrap_or(before);
                    let freed = before.saturating_sub(after);
                    (TaskStatus::Applied, msg, freed)
                }
                Err(e) => (TaskStatus::Failed, e.to_string(), 0),
            }
        }
        "journal-system" => {
            let before = journal_usage(false).unwrap_or(0);
            match run_ok("sudo", &["journalctl", "--vacuum-time=7d"]) {
                Ok(msg) => {
                    let after = journal_usage(false).unwrap_or(before);
                    (TaskStatus::Applied, msg, before.saturating_sub(after))
                }
                Err(e) => (TaskStatus::Failed, e.to_string(), 0),
            }
        }
        "fontconfig" => match run_ok("fc-cache", &["-f"]) {
            Ok(_) => (TaskStatus::Applied, "font cache rebuilt".into(), 0),
            Err(e) => (TaskStatus::Failed, e.to_string(), 0),
        },
        "desktop-database" => {
            let Ok(home) = util::home_dir() else {
                return (TaskStatus::Failed, "HOME unset".into(), 0);
            };
            let dir = home.join(".local/share/applications");
            match run_ok("update-desktop-database", &[dir.to_str().unwrap_or("")]) {
                Ok(_) => (TaskStatus::Applied, "desktop database updated".into(), 0),
                Err(e) => (TaskStatus::Failed, e.to_string(), 0),
            }
        }
        "mime-database" => {
            let Ok(home) = util::home_dir() else {
                return (TaskStatus::Failed, "HOME unset".into(), 0);
            };
            let dir = home.join(".local/share/mime");
            match run_ok("update-mime-database", &[dir.to_str().unwrap_or("")]) {
                Ok(_) => (TaskStatus::Applied, "MIME database updated".into(), 0),
                Err(e) => (TaskStatus::Failed, e.to_string(), 0),
            }
        }
        "icon-cache" => {
            let bin = if util::command_exists("gtk4-update-icon-cache") {
                "gtk4-update-icon-cache"
            } else {
                "gtk-update-icon-cache"
            };
            let mut ok = 0usize;
            let mut last_err = String::new();
            for dir in user_icon_dirs() {
                match run_ok(bin, &["-f", "-t", dir.to_str().unwrap_or("")]) {
                    Ok(_) => ok += 1,
                    Err(e) => last_err = e.to_string(),
                }
            }
            if ok == 0 {
                (TaskStatus::Failed, last_err, 0)
            } else {
                (TaskStatus::Applied, format!("updated {ok} icon theme(s)"), 0)
            }
        }
        "resolved-flush" => match run_ok("resolvectl", &["flush-caches"]) {
            Ok(_) => (TaskStatus::Applied, "DNS caches flushed".into(), 0),
            Err(e) => (TaskStatus::Failed, e.to_string(), 0),
        },
        "tmpfiles-user" => match run_ok("systemd-tmpfiles", &["--user", "--clean"]) {
            Ok(_) => (TaskStatus::Applied, "user tmpfiles cleaned".into(), 0),
            Err(e) => (TaskStatus::Failed, e.to_string(), 0),
        },
        "sync" => match run_ok("sync", &[]) {
            Ok(_) => (TaskStatus::Applied, "filesystems synced".into(), 0),
            Err(e) => (TaskStatus::Failed, e.to_string(), 0),
        },
        "flatpak-repair" => match run_ok("flatpak", &["repair", "--user"]) {
            Ok(_) => (TaskStatus::Applied, "flatpak repaired".into(), 0),
            Err(e) => (TaskStatus::Failed, e.to_string(), 0),
        },
        other => (TaskStatus::Failed, format!("unknown task {other}"), 0),
    }
}

fn print_human(diag: &[String], tasks: &[Task]) {
    println!("Optimize\n");
    if !diag.is_empty() {
        println!("DIAGNOSIS");
        for line in diag {
            println!("  {line}");
        }
        println!();
    }
    for t in tasks {
        let mark = match t.status {
            TaskStatus::Ready if t.selected => "●",
            TaskStatus::Ready => "○",
            TaskStatus::Applied => "✓",
            TaskStatus::Unchanged => "·",
            TaskStatus::Skipped | TaskStatus::Unavailable => "◎",
            TaskStatus::Failed => "✗",
        };
        println!("  {mark} {:<32} {}", t.label, t.detail);
    }
}

pub fn manage_whitelist(json: bool) -> Result<()> {
    let mut tasks = catalog(&[]);
    let skips = load_skips()?;
    for t in &mut tasks {
        t.selected = skips.iter().any(|s| s == &t.id);
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&skips)?);
        return Ok(());
    }
    if util::is_tty() {
        let mut rows: Vec<PickItem> = tasks
            .iter()
            .map(|t| PickItem {
                selected: t.selected,
                locked: false,
                title: t.id.clone(),
                detail: t.label.clone(),
                bytes: 0,
            })
            .collect();
        if !picker::select_items("Optimize skip list (selected = skipped)", &mut rows)? {
            println!("Aborted.");
            return Ok(());
        }
        let ids: Vec<String> = tasks
            .iter()
            .zip(rows)
            .filter(|(_, r)| r.selected)
            .map(|(t, _)| t.id.clone())
            .collect();
        save_skips(ids.clone())?;
        println!("Skipping {} optimize task(s).", ids.len());
        for id in ids {
            println!("  {id}");
        }
    } else {
        if skips.is_empty() {
            println!("No skipped optimize tasks.");
        } else {
            println!("Skipped optimize tasks:");
            for id in &skips {
                println!("  {id}");
            }
        }
        println!("\nKnown tasks:");
        for t in &tasks {
            println!("  {}  {}", t.id, t.label);
        }
    }
    Ok(())
}

pub fn run(dry_run: bool, yes: bool, json: bool, select_file: Option<&str>) -> Result<()> {
    let skips = load_skips()?;
    let mut tasks = catalog(&skips);
    if let Some(file) = select_file {
        let wanted = util::load_id_file(std::path::Path::new(file))?;
        for t in &mut tasks {
            t.selected = wanted.contains(&t.id) && matches!(t.status, TaskStatus::Ready);
        }
    }
    let diag = diagnosis();

    if json && dry_run {
        let skipped = tasks
            .iter()
            .filter(|t| matches!(t.status, TaskStatus::Skipped | TaskStatus::Unchanged))
            .count();
        let unavailable = tasks
            .iter()
            .filter(|t| matches!(t.status, TaskStatus::Unavailable))
            .count();
        let report = OptimizeReport {
            dry_run: true,
            diagnosis: diag,
            tasks,
            applied: 0,
            skipped,
            unavailable,
            freed_bytes: 0,
        };
        println!("{}", serde_json::to_string_pretty(&report)?);
        history::log_operation("optimize", true, 0, report.skipped, "scan")?;
        return Ok(());
    }

    if select_file.is_none() && !yes && !json && util::is_tty() {
        let mut rows: Vec<PickItem> = tasks
            .iter()
            .map(|t| {
                let locked = !matches!(t.status, TaskStatus::Ready);
                PickItem {
                    selected: t.selected && !locked,
                    locked,
                    title: t.label.clone(),
                    detail: t.detail.clone(),
                    bytes: t.bytes,
                }
            })
            .collect();
        if !picker::select_items("Optimize", &mut rows)? {
            println!("Aborted.");
            return Ok(());
        }
        for (t, r) in tasks.iter_mut().zip(rows) {
            t.selected = r.selected && matches!(t.status, TaskStatus::Ready);
        }
    } else if !json {
        print_human(&diag, &tasks);
        if !yes && !util::is_tty() {
            println!("\nPass --yes to apply the default selection, or run in a terminal.");
            return Ok(());
        }
    }

    let selected: Vec<usize> = tasks
        .iter()
        .enumerate()
        .filter(|(_, t)| t.selected && matches!(t.status, TaskStatus::Ready))
        .map(|(i, _)| i)
        .collect();

    if selected.is_empty() {
        println!("Nothing to optimize.");
        return Ok(());
    }

    if dry_run {
        for i in &selected {
            let (status, detail, _) = apply_task(&tasks[*i], true);
            tasks[*i].status = status;
            tasks[*i].detail = detail;
        }
        if json {
            let skipped = tasks
                .iter()
                .filter(|t| matches!(t.status, TaskStatus::Skipped | TaskStatus::Unchanged))
                .count();
            let unavailable = tasks
                .iter()
                .filter(|t| matches!(t.status, TaskStatus::Unavailable))
                .count();
            let report = OptimizeReport {
                dry_run: true,
                diagnosis: diag,
                tasks,
                applied: 0,
                skipped,
                unavailable,
                freed_bytes: 0,
            };
            println!("{}", serde_json::to_string_pretty(&report)?);
        } else {
            print_human(&diag, &tasks);
            println!("\nRe-run without --dry-run to apply selected tasks.");
        }
        history::log_operation("optimize", true, 0, selected.len(), "scan")?;
        return Ok(());
    }

    if !yes
        && !util::confirm(&format!("Apply {} optimize task(s)?", selected.len()))?
    {
        println!("Aborted.");
        return Ok(());
    }

    let mut freed = 0u64;
    let mut applied = 0usize;
    for i in selected {
        let (status, detail, bytes) = apply_task(&tasks[i], false);
        let mark = if matches!(status, TaskStatus::Applied) {
            applied += 1;
            freed += bytes;
            "✓"
        } else {
            "✗"
        };
        println!("  {mark} {} · {detail}", tasks[i].label);
        tasks[i].status = status;
        tasks[i].detail = detail;
    }

    println!(
        "\nOptimization complete\nApplied {applied} · freed {}",
        util::format_bytes(freed)
    );
    history::log_operation("optimize", false, freed, applied, "apply")?;
    if json {
        let skipped = tasks
            .iter()
            .filter(|t| matches!(t.status, TaskStatus::Skipped | TaskStatus::Unchanged))
            .count();
        let unavailable = tasks
            .iter()
            .filter(|t| matches!(t.status, TaskStatus::Unavailable))
            .count();
        let report = OptimizeReport {
            dry_run: false,
            diagnosis: diag,
            tasks,
            applied,
            skipped,
            unavailable,
            freed_bytes: freed,
        };
        println!("{}", serde_json::to_string_pretty(&report)?);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_journalctl_tokens() {
        assert_eq!(parse_si_token("3.5G"), Some(3_758_096_384));
        assert_eq!(parse_si_token("48.0M"), Some(50_331_648));
        assert_eq!(parse_si_token("512.0K"), Some(524_288));
    }
}
