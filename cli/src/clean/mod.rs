use crate::history;
use crate::ui::picker::{self, PickItem};
use crate::util;
use crate::whitelist;
use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::json;
use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

mod ai;

#[derive(Debug, Clone, Serialize)]
pub struct CleanItem {
    pub id: String,
    pub category: String,
    pub label: String,
    pub path: String,
    pub bytes: u64,
    pub selected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_key: Option<String>,
}

#[derive(Debug, Serialize)]
struct CleanReport {
    dry_run: bool,
    items: Vec<CleanItem>,
    total_bytes: u64,
    freed_bytes: u64,
    removed: usize,
}

struct Spec {
    category: &'static str,
    label: &'static str,
    rel: &'static str,
    busy: &'static [&'static str],
    default_on: bool,
}

struct Candidate {
    category: String,
    label: String,
    path: PathBuf,
    busy: Vec<&'static str>,
    default_on: bool,
    action: Option<&'static str>,
    group: Option<String>,
    group_label: Option<String>,
    label_key: Option<String>,
}

const CATALOG: &[Spec] = &[
    Spec { category: "user", label: "Thumbnails", rel: ".cache/thumbnails", busy: &[], default_on: true },
    Spec { category: "user", label: "Trash files", rel: ".local/share/Trash/files", busy: &[], default_on: true },
    Spec { category: "user", label: "Trash info", rel: ".local/share/Trash/info", busy: &[], default_on: true },
    Spec { category: "user", label: "fontconfig cache", rel: ".cache/fontconfig", busy: &[], default_on: true },
    Spec { category: "user", label: "Mesa shader cache", rel: ".cache/mesa_shader_cache", busy: &[], default_on: true },
    Spec { category: "user", label: "Mesa shader cache db", rel: ".cache/mesa_shader_cache_db", busy: &[], default_on: true },
    Spec { category: "user", label: "GStreamer cache", rel: ".cache/gstreamer-1.0", busy: &[], default_on: true },
    Spec { category: "user", label: "NVIDIA shader cache", rel: ".cache/nvidia", busy: &[], default_on: true },
    Spec { category: "user", label: "NV compute cache", rel: ".nv/ComputeCache", busy: &[], default_on: true },
    Spec { category: "user", label: "WebKitGTK cache", rel: ".cache/webkitgtk", busy: &[], default_on: true },
    Spec { category: "browser", label: "Firefox cache", rel: ".cache/mozilla/firefox", busy: &["firefox"], default_on: true },
    Spec { category: "browser", label: "LibreWolf cache", rel: ".cache/librewolf", busy: &["librewolf"], default_on: true },
    Spec { category: "browser", label: "Chromium cache", rel: ".cache/chromium", busy: &["chromium", "chromium-browser"], default_on: true },
    Spec { category: "browser", label: "Google Chrome cache", rel: ".cache/google-chrome", busy: &["chrome", "google-chrome"], default_on: true },
    Spec { category: "browser", label: "Brave cache", rel: ".cache/BraveSoftware", busy: &["brave"], default_on: true },
    Spec { category: "browser", label: "Zen cache", rel: ".cache/zen", busy: &["zen", "zen-browser"], default_on: true },
    Spec { category: "browser", label: "Vivaldi cache", rel: ".cache/vivaldi", busy: &["vivaldi"], default_on: true },
    Spec { category: "browser", label: "Microsoft Edge cache", rel: ".cache/microsoft-edge", busy: &["microsoft-edge", "msedge"], default_on: true },
    Spec { category: "browser", label: "Opera cache", rel: ".cache/opera", busy: &["opera"], default_on: true },
    Spec { category: "browser", label: "Floorp cache", rel: ".cache/floorp", busy: &["floorp"], default_on: true },
    Spec { category: "dev", label: "pip cache", rel: ".cache/pip", busy: &[], default_on: true },
    Spec { category: "dev", label: "uv cache", rel: ".cache/uv", busy: &[], default_on: true },
    Spec { category: "dev", label: "pypoetry cache", rel: ".cache/pypoetry", busy: &[], default_on: true },
    Spec { category: "dev", label: "npm cache", rel: ".npm/_cacache", busy: &["npm"], default_on: true },
    Spec { category: "dev", label: "pnpm store cache", rel: ".cache/pnpm", busy: &["pnpm"], default_on: true },
    Spec { category: "dev", label: "yarn cache", rel: ".cache/yarn", busy: &["yarn"], default_on: true },
    Spec { category: "dev", label: "yarn berry cache", rel: ".yarn/berry/cache", busy: &["yarn"], default_on: true },
    Spec { category: "dev", label: "bun cache", rel: ".bun/install/cache", busy: &["bun"], default_on: true },
    Spec { category: "dev", label: "cargo registry cache", rel: ".cargo/registry/cache", busy: &["cargo"], default_on: true },
    Spec { category: "dev", label: "cargo git db", rel: ".cargo/git/db", busy: &["cargo"], default_on: true },
    Spec { category: "dev", label: "go build cache", rel: ".cache/go-build", busy: &[], default_on: true },
    Spec { category: "dev", label: "ccache", rel: ".cache/ccache", busy: &[], default_on: true },
    Spec { category: "dev", label: "sccache", rel: ".cache/sccache", busy: &[], default_on: true },
    Spec { category: "dev", label: "gradle caches", rel: ".gradle/caches", busy: &[], default_on: false },
    Spec { category: "dev", label: "composer cache", rel: ".composer/cache", busy: &[], default_on: true },
    Spec { category: "dev", label: "electron cache", rel: ".cache/electron", busy: &[], default_on: true },
    Spec { category: "dev", label: "dart pub-cache", rel: ".pub-cache", busy: &[], default_on: false },
    Spec { category: "apps", label: "Spotify cache", rel: ".cache/spotify", busy: &["spotify"], default_on: true },
    Spec { category: "apps", label: "Steam shader cache", rel: ".local/share/Steam/steamapps/shadercache", busy: &["steam", "steamwebhelper"], default_on: false },
    Spec { category: "apps", label: "Steam shader cache", rel: ".steam/steam/steamapps/shadercache", busy: &["steam", "steamwebhelper"], default_on: false },
    Spec { category: "apps", label: "Steam logs", rel: ".local/share/Steam/logs", busy: &["steam"], default_on: true },
    Spec { category: "apps", label: "Telegram cache", rel: ".local/share/TelegramDesktop/tdata/user_data/cache", busy: &["telegram"], default_on: true },
    Spec { category: "packages", label: "yay cache", rel: ".cache/yay", busy: &["yay"], default_on: true },
    Spec { category: "packages", label: "paru cache", rel: ".cache/paru", busy: &["paru"], default_on: true },
    Spec { category: "packages", label: "pikaur cache", rel: ".cache/pikaur", busy: &["pikaur"], default_on: true },
];

const USER_DIR_KEEP: &[&str] = &[
    "omakeeper", "omarchy", "hypr", "hyprland", "systemd", "dconf", "fontconfig",
    "gtk-2.0", "gtk-3.0", "gtk-4.0", "qt5ct", "qt6ct", "pulse", "pipewire",
    "fcitx", "fcitx5", "ibus", "mozc", "mime", "applications", "icons", "themes",
    "Trash", "xorg", "keyrings", "keyring", "gnupg", "ssh", "pki", "containers",
    "flatpak", "fish", "zsh", "bash", "nvim", "vim", "git", "autostart",
    "environment.d", "user-dirs.dirs", "session", "mozilla", "chromium",
    "ibus-table", "Kvantum", "kitty", "alacritty", "foot", "ghostty",
    "mise", "asdf", "nvm", "fnm", "rbenv", "pyenv", "rustup", "cargo",
];

const MODEL_CACHE_NAMES: &[&str] = &[
    "huggingface",
    "torch",
    "whisper",
    "ollama",
    "llama.cpp",
    "llamacpp",
    "lm-studio",
    "modelscope",
    "openvino",
    "vllm",
    "comfyui",
];

const CACHE_SKIP_NAMES: &[&str] = &[
    "dconf",
    "keyring",
    "pulse",
    "pipewire",
    "session",
    "fcitx",
    "fcitx5",
    "ibus",
    "omarchy",
    "hyprland",
    "hypr",
    "tracker",
    "tracker3",
];

fn is_model_cache_name(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    MODEL_CACHE_NAMES
        .iter()
        .any(|k| n == *k || n.contains(k))
}

fn is_skipped_cache_name(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    CACHE_SKIP_NAMES.iter().any(|k| n == *k || n.starts_with(k))
        || is_model_cache_name(&n)
}

fn push_unique(
    seen: &mut HashSet<PathBuf>,
    out: &mut Vec<Candidate>,
    cat: &str,
    label: &str,
    path: PathBuf,
    busy: &[&'static str],
    default_on: bool,
    action: Option<&'static str>,
) {
    push_candidate(
        seen, out, cat, label, path, busy, default_on, action, None, None, None,
    );
}

fn push_ai(
    seen: &mut HashSet<PathBuf>,
    out: &mut Vec<Candidate>,
    group_id: &str,
    group_label: &str,
    label: &str,
    label_key: &str,
    path: PathBuf,
    busy: &[&'static str],
    default_on: bool,
) {
    push_candidate(
        seen,
        out,
        "ai",
        label,
        path,
        busy,
        default_on,
        None,
        Some(group_id.to_string()),
        Some(group_label.to_string()),
        Some(label_key.to_string()),
    );
}

fn push_candidate(
    seen: &mut HashSet<PathBuf>,
    out: &mut Vec<Candidate>,
    cat: &str,
    label: &str,
    path: PathBuf,
    busy: &[&'static str],
    default_on: bool,
    action: Option<&'static str>,
    group: Option<String>,
    group_label: Option<String>,
    label_key: Option<String>,
) {
    if action.is_none() && !path.exists() {
        return;
    }
    let key = path.canonicalize().unwrap_or_else(|_| path.clone());
    if seen
        .iter()
        .any(|s| s == &key || s.starts_with(&key) || key.starts_with(s))
    {
        return;
    }
    seen.insert(key);
    out.push(Candidate {
        category: cat.to_string(),
        label: label.to_string(),
        path,
        busy: busy.to_vec(),
        default_on,
        action,
        group,
        group_label,
        label_key,
    });
}

fn candidate_paths() -> Result<Vec<Candidate>> {
    let home = util::home_dir()?;
    let mut seen = HashSet::new();
    let mut out = Vec::new();

    for spec in CATALOG {
        push_unique(
            &mut seen,
            &mut out,
            spec.category,
            spec.label,
            home.join(spec.rel),
            spec.busy,
            spec.default_on,
            None,
        );
    }

    ai::scan(&home, &mut seen, &mut out);

    scan_cache_leftovers(&home, &mut seen, &mut out)?;
    scan_electron_caches(&home, &mut seen, &mut out)?;
    scan_flatpak_caches(&home, &mut seen, &mut out)?;
    scan_flatpak_unused(&mut seen, &mut out);
    scan_logs(&home, &mut seen, &mut out)?;
    scan_orphans(&home, &mut seen, &mut out)?;
    scan_old_downloads(&home, &mut seen, &mut out)?;
    scan_pacman_cache(&mut seen, &mut out);

    Ok(out)
}

fn scan_cache_leftovers(home: &Path, seen: &mut HashSet<PathBuf>, out: &mut Vec<Candidate>) -> Result<()> {
    let root = home.join(".cache");
    let Ok(entries) = fs::read_dir(&root) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || is_skipped_cache_name(&name) {
            continue;
        }
        let label = format!("{name} cache");
        push_unique(seen, out, "apps", &label, path, &[], true, None);
    }
    Ok(())
}

fn electron_busy(app: &str) -> &'static [&'static str] {
    match app.to_ascii_lowercase().as_str() {
        "code" | "code - oss" | "code-oss" | "vscodium" => &["code", "code-oss", "codium"],
        "discord" | "discordcanary" => &["discord"],
        "slack" => &["slack"],
        "spotify" => &["spotify"],
        "element" | "element-desktop" => &["element"],
        "obsidian" => &["obsidian"],
        "1password" => &["1password"],
        "signal" | "signal-desktop" => &["signal-desktop", "signal"],
        "telegramdesktop" | "telegram-desktop" => &["telegram"],
        _ => &[],
    }
}

fn scan_electron_caches(home: &Path, seen: &mut HashSet<PathBuf>, out: &mut Vec<Candidate>) -> Result<()> {
    let root = home.join(".config");
    let Ok(entries) = fs::read_dir(&root) else {
        return Ok(());
    };
    const NAMES: &[&str] = &["Cache", "Code Cache", "GPUCache", "CachedData", "DawnCache", "CachedExtensionVSIXs"];
    for entry in entries.flatten() {
        let app = entry.path();
        if !app.is_dir() {
            continue;
        }
        let app_name = entry.file_name().to_string_lossy().to_string();
        if is_skipped_cache_name(&app_name) {
            continue;
        }
        let busy = electron_busy(&app_name);
        for cache_name in NAMES {
            let path = app.join(cache_name);
            if path.is_dir() {
                let label = format!("{app_name} {cache_name}");
                push_unique(seen, out, "apps", &label, path, busy, true, None);
            }
        }
    }
    Ok(())
}

fn scan_flatpak_caches(home: &Path, seen: &mut HashSet<PathBuf>, out: &mut Vec<Candidate>) -> Result<()> {
    let root = home.join(".var/app");
    let Ok(entries) = fs::read_dir(&root) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let cache = entry.path().join("cache");
        if cache.is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            let label = format!("{name} cache");
            push_unique(seen, out, "flatpak", &label, cache, &[], true, None);
        }
    }
    Ok(())
}

fn scan_flatpak_unused(seen: &mut HashSet<PathBuf>, out: &mut Vec<Candidate>) {
    if !util::command_exists("flatpak") {
        return;
    }
    let Ok(cmd) = std::process::Command::new("flatpak")
        .args(["uninstall", "--unused", "--dry-run"])
        .output()
    else {
        return;
    };
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&cmd.stdout),
        String::from_utf8_lossy(&cmd.stderr)
    )
    .to_ascii_lowercase();
    if text.contains("nothing unused") || text.trim().is_empty() {
        return;
    }
    let path = PathBuf::from("flatpak://unused-runtimes");
    push_unique(
        seen,
        out,
        "flatpak",
        "Unused Flatpak runtimes",
        path,
        &[],
        false,
        Some("flatpak-unused"),
    );
}

fn scan_logs(home: &Path, seen: &mut HashSet<PathBuf>, out: &mut Vec<Candidate>) -> Result<()> {
    let xorg = home.join(".local/share/xorg");
    push_unique(seen, out, "logs", "Xorg logs", xorg, &[], true, None);

    for root in [
        home.join(".local/share"),
        home.join(".local/state"),
        home.join(".cache"),
    ] {
        let Ok(entries) = fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            if !entry.path().is_dir() {
                continue;
            }
            let app = entry.file_name().to_string_lossy().into_owned();
            if is_skipped_cache_name(&app) {
                continue;
            }
            for log_name in ["log", "logs", "Logs"] {
                let path = entry.path().join(log_name);
                if path.is_dir() {
                    let label = format!("{app} logs");
                    push_unique(seen, out, "logs", &label, path, &[], true, None);
                }
            }
        }
    }
    Ok(())
}

fn installed_names() -> HashSet<String> {
    let mut set = HashSet::new();
    if let Ok(out) = std::process::Command::new("pacman")
        .args(["-Qq"])
        .output()
    {
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            let n = line.trim().to_ascii_lowercase();
            if !n.is_empty() {
                set.insert(n);
            }
        }
    }
    for dir in [
        PathBuf::from("/usr/share/applications"),
        util::home_dir()
            .ok()
            .map(|h| h.join(".local/share/applications"))
            .unwrap_or_default(),
    ] {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            if let Some(stem) = name.strip_suffix(".desktop") {
                set.insert(stem.to_string());
                if let Some(short) = stem.rsplit('.').next() {
                    if short.len() >= 3 {
                        set.insert(short.to_string());
                    }
                }
            }
        }
    }
    set
}

fn dir_is_known(name: &str, installed: &HashSet<String>) -> bool {
    let n = name.to_ascii_lowercase();
    if USER_DIR_KEEP.iter().any(|k| n == *k || n.starts_with(&format!("{k}-"))) {
        return true;
    }
    if ai::keep_dir_name(&n) {
        return true;
    }
    if is_skipped_cache_name(&n) || is_model_cache_name(&n) {
        return true;
    }
    if installed.contains(&n) {
        return true;
    }
    for pkg in installed {
        if pkg.len() >= 4 && (n == *pkg || n.starts_with(&format!("{pkg}-")) || n.starts_with(&format!("{pkg}."))) {
            return true;
        }
    }
    false
}

fn scan_orphans(home: &Path, seen: &mut HashSet<PathBuf>, out: &mut Vec<Candidate>) -> Result<()> {
    let installed = installed_names();
    for base in [".config", ".local/share", ".cache", ".local/state"] {
        let root = home.join(base);
        let Ok(entries) = fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || dir_is_known(&name, &installed) {
                continue;
            }
            let label = format!("{base}/{name}");
            push_unique(seen, out, "leftovers", &label, path, &[], false, None);
        }
    }
    Ok(())
}

fn scan_old_downloads(home: &Path, seen: &mut HashSet<PathBuf>, out: &mut Vec<Candidate>) -> Result<()> {
    let root = home.join("Downloads");
    if !root.is_dir() {
        return Ok(());
    }
    let cutoff = std::time::SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(30 * 24 * 3600))
        .unwrap_or(std::time::UNIX_EPOCH);
    let mut found: Vec<(u64, PathBuf, String)> = Vec::new();
    for entry in walkdir::WalkDir::new(&root)
        .max_depth(2)
        .follow_links(false)
        .into_iter()
        .flatten()
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path().to_path_buf();
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let incomplete = name.ends_with(".crdownload")
            || name.ends_with(".part")
            || name.ends_with(".download");
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let bytes = meta.len();
        if !incomplete && bytes < 50 * 1024 * 1024 {
            continue;
        }
        let old = meta.modified().ok().map(|t| t < cutoff).unwrap_or(false);
        if !incomplete && !old {
            continue;
        }
        let label = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("download")
            .to_string();
        found.push((bytes, path, label));
    }
    found.sort_by(|a, b| b.0.cmp(&a.0));
    for (_, path, label) in found.into_iter().take(40) {
        push_unique(seen, out, "downloads", &label, path, &[], false, None);
    }
    Ok(())
}

fn scan_pacman_cache(seen: &mut HashSet<PathBuf>, out: &mut Vec<Candidate>) {
    let path = PathBuf::from("/var/cache/pacman/pkg");
    if !path.is_dir() {
        return;
    }
    push_unique(
        seen,
        out,
        "packages",
        "pacman package cache",
        path,
        &[],
        false,
        Some("pacman-sc"),
    );
}

fn privileged(bin: &str, args: &[&str]) -> Command {
    if !util::is_tty() && util::command_exists("pkexec") {
        let mut c = Command::new("pkexec");
        c.arg(bin).args(args);
        c
    } else {
        let mut c = Command::new("sudo");
        c.arg(bin).args(args);
        c
    }
}

fn apply_item(item: &CleanItem) -> Result<()> {
    match item.action.as_deref() {
        Some("pacman-sc") => {
            let mut cmd = if util::command_exists("paccache") {
                privileged("/usr/bin/paccache", &["-rk1"])
            } else {
                privileged("/usr/bin/pacman", &["-Sc", "--noconfirm"])
            };
            let status = cmd
                .stdin(Stdio::null())
                .status()
                .context("clean pacman cache")?;
            if !status.success() {
                anyhow::bail!("pacman cache clean failed ({status})");
            }
            Ok(())
        }
        Some("flatpak-unused") => {
            let mut cmd = if !util::is_tty() && util::command_exists("pkexec") {
                let mut c = Command::new("pkexec");
                c.args(["/usr/bin/flatpak", "uninstall", "--unused", "-y"]);
                c
            } else {
                let mut c = Command::new("flatpak");
                c.args(["uninstall", "--unused", "-y"]);
                c
            };
            let status = cmd
                .stdin(Stdio::null())
                .status()
                .context("flatpak uninstall --unused")?;
            if !status.success() {
                anyhow::bail!("flatpak unused cleanup failed ({status})");
            }
            Ok(())
        }
        _ => util::remove_path(Path::new(&item.path)),
    }
}

fn busy_for(path: &Path, names: &[&str], running: &HashSet<String>) -> Option<String> {
    if !names.is_empty() && util::any_running(running, names) {
        return Some(format!("{} busy", names[0]));
    }
    if let Some(app) = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
    {
        let needle = app.to_ascii_lowercase();
        if running.contains(&needle) {
            return Some(format!("{app} busy"));
        }
    }
    None
}

fn scan() -> Result<Vec<CleanItem>> {
    let running = util::running_comms();
    let candidates = candidate_paths()?;
    let paths: Vec<PathBuf> = candidates.iter().map(|c| c.path.clone()).collect();
    let sizes = util::path_sizes(&paths);

    let mut items = Vec::new();
    for (cand, bytes) in candidates.into_iter().zip(sizes) {
        if whitelist::is_protected(&cand.path) {
            continue;
        }
        let special = cand.action.is_some();
        if !special && bytes < 4096 {
            continue;
        }
        if cand.category == "apps" && !special && bytes < 1_048_576 {
            continue;
        }
        if cand.category == "leftovers" && bytes < 1_048_576 {
            continue;
        }
        if cand.category == "ai" && !special && bytes < 1_048_576 {
            continue;
        }
        let skip = if cand.category == "ai" {
            if !cand.busy.is_empty() && util::any_running(&running, &cand.busy) {
                Some(format!("{} busy", cand.busy[0]))
            } else {
                None
            }
        } else {
            busy_for(&cand.path, &cand.busy, &running)
        };
        items.push(CleanItem {
            id: format!("{}:{}", cand.category, cand.path.display()),
            category: cand.category,
            label: cand.label,
            path: cand.path.display().to_string(),
            bytes,
            selected: skip.is_none() && cand.default_on,
            skip_reason: skip,
            action: cand.action.map(|s| s.to_string()),
            group: cand.group,
            group_label: cand.group_label,
            label_key: cand.label_key,
        });
    }
    items.sort_by(|a, b| b.bytes.cmp(&a.bytes).then(a.category.cmp(&b.category)));
    Ok(items)
}

fn print_human(items: &[CleanItem], dry_run: bool) {
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
        let mark = if item.skip_reason.is_some() {
            "◎"
        } else if item.selected {
            "●"
        } else {
            "○"
        };
        let extra = item
            .skip_reason
            .as_deref()
            .map(|s| format!(" ({s})"))
            .unwrap_or_default();
        let title = match item.group_label.as_deref() {
            Some(g) => format!("{g} · {}", item.label),
            None => item.label.clone(),
        };
        println!(
            "  {mark} {:<40} {:>10}{extra}",
            util::truncate_right(&title, 40),
            util::format_bytes(item.bytes)
        );
        println!("      {}", item.path);
    }
    let total: u64 = items.iter().filter(|i| i.selected).map(|i| i.bytes).sum();
    let count = items.iter().filter(|i| i.selected).count();
    println!("\n{}", "=".repeat(64));
    println!(
        "Selected: {} across {count} items",
        util::format_bytes(total)
    );
}

fn pick(items: &mut [CleanItem]) -> Result<bool> {
    let mut rows: Vec<PickItem> = items
        .iter()
        .map(|i| PickItem {
            selected: i.selected,
            locked: i.skip_reason.is_some(),
            title: format!("{} · {}", i.category, i.label),
            detail: i
                .skip_reason
                .clone()
                .unwrap_or_else(|| i.path.clone()),
            bytes: i.bytes,
        })
        .collect();
    let ok = picker::select_items("Clean caches", &mut rows)?;
    if ok {
        for (item, row) in items.iter_mut().zip(rows) {
            item.selected = row.selected && item.skip_reason.is_none();
        }
    }
    Ok(ok)
}

pub fn run(dry_run: bool, yes: bool, json: bool, select_file: Option<&str>) -> Result<()> {
    let mut items = scan()?;
    if let Some(file) = select_file {
        let wanted = util::load_id_file(std::path::Path::new(file))?;
        for item in &mut items {
            item.selected = wanted.contains(&item.id) || wanted.contains(&item.path);
        }
    }
    let listed: u64 = items.iter().map(|i| i.bytes).sum();

    if json && dry_run {
        let report = CleanReport {
            dry_run: true,
            items: items.clone(),
            total_bytes: listed,
            freed_bytes: 0,
            removed: 0,
        };
        println!("{}", serde_json::to_string_pretty(&report)?);
        history::log_operation("clean", true, listed, items.len(), "scan")?;
        return Ok(());
    }

    if select_file.is_none() && !yes && !json {
        if items.is_empty() {
            println!("Nothing to clean.");
            return Ok(());
        }
        if util::is_tty() {
            if !pick(&mut items)? {
                println!("Aborted.");
                return Ok(());
            }
        } else {
            print_human(&items, dry_run);
            println!("\nPass --yes to delete the default selection, or run in a terminal to pick.");
            return Ok(());
        }
    } else if !json {
        print_human(&items, dry_run);
    }

    let selected: Vec<&CleanItem> = items.iter().filter(|i| i.selected).collect();
    let total: u64 = selected.iter().map(|i| i.bytes).sum();
    let count = selected.len();

    if dry_run {
        if json {
            let report = CleanReport {
                dry_run: true,
                items: items.clone(),
                total_bytes: listed,
                freed_bytes: 0,
                removed: 0,
            };
            println!("{}", serde_json::to_string_pretty(&report)?);
        } else if yes || !util::is_tty() {
            // already printed
        } else {
            print_human(&items, true);
        }
        history::log_operation("clean", true, total, count, "scan")?;
        if count > 0 {
            println!("\nRe-run without --dry-run to delete selected items.");
        }
        return Ok(());
    }

    if count == 0 {
        println!("Nothing selected.");
        return Ok(());
    }

    if !yes
        && !util::confirm(&format!(
            "Delete {} across {count} items?",
            util::format_bytes(total)
        ))?
    {
        println!("Aborted.");
        return Ok(());
    }

    let mut freed = 0u64;
    let mut removed = 0usize;
    let mut skipped = 0usize;
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for item in items.iter().filter(|i| i.selected) {
        match apply_item(item) {
            Ok(()) => {
                freed += item.bytes;
                removed += 1;
                if json {
                    let _ = writeln!(
                        out,
                        "{}",
                        json!({
                            "event": "item",
                            "id": item.id,
                            "label": item.label,
                            "path": item.path,
                            "bytes": item.bytes,
                            "ok": true,
                            "freed": freed,
                            "removed": removed,
                        })
                    );
                    let _ = out.flush();
                } else {
                    println!("  ✓ {}", item.label);
                }
            }
            Err(e) => {
                skipped += 1;
                if json {
                    let _ = writeln!(
                        out,
                        "{}",
                        json!({
                            "event": "item",
                            "id": item.id,
                            "label": item.label,
                            "path": item.path,
                            "bytes": item.bytes,
                            "ok": false,
                            "error": e.to_string(),
                            "freed": freed,
                            "removed": removed,
                        })
                    );
                    let _ = out.flush();
                } else {
                    eprintln!("  ✗ {}: {e}", item.label);
                }
            }
        }
    }

    if !json {
        println!(
            "\nCleanup complete\nFreed {} · {removed} items",
            util::format_bytes(freed)
        );
    }
    history::log_operation("clean", false, freed, removed, "apply")?;

    if json {
        let _ = writeln!(
            out,
            "{}",
            json!({
                "event": "done",
                "dry_run": false,
                "freed_bytes": freed,
                "removed": removed,
                "skipped": skipped,
                "total_bytes": listed,
            })
        );
        let _ = out.flush();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_caches_are_protected() {
        assert!(is_model_cache_name("huggingface"));
        assert!(is_model_cache_name("huggingface-hub"));
        assert!(is_model_cache_name("ollama"));
        assert!(!is_model_cache_name("pip"));
    }

    #[test]
    fn session_caches_are_skipped() {
        assert!(is_skipped_cache_name("fcitx5"));
        assert!(is_skipped_cache_name("keyring"));
        assert!(!is_skipped_cache_name("yay"));
        assert!(is_skipped_cache_name("tracker3"));
    }

    #[test]
    fn gradle_and_steam_default_off() {
        let gradle = CATALOG.iter().find(|s| s.label.contains("gradle")).unwrap();
        assert!(!gradle.default_on);
        let steam = CATALOG.iter().find(|s| s.label.contains("Steam shader")).unwrap();
        assert!(!steam.default_on);
    }

    #[test]
    fn orphans_keep_session_dirs() {
        let installed = HashSet::from(["firefox".into(), "code".into()]);
        assert!(dir_is_known("hypr", &installed));
        assert!(dir_is_known("firefox", &installed));
        assert!(dir_is_known("code", &installed));
        assert!(!dir_is_known("SomeDeadApp", &installed));
    }
}
