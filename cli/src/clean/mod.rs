use crate::history;
use crate::ui::picker::{self, PickItem};
use crate::util;
use crate::whitelist;
use anyhow::Result;
use serde::Serialize;
use serde_json::json;
use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

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
}

const CATALOG: &[Spec] = &[
    Spec { category: "user", label: "Thumbnails", rel: ".cache/thumbnails", busy: &[] },
    Spec { category: "user", label: "Trash files", rel: ".local/share/Trash/files", busy: &[] },
    Spec { category: "user", label: "Trash info", rel: ".local/share/Trash/info", busy: &[] },
    Spec { category: "user", label: "fontconfig cache", rel: ".cache/fontconfig", busy: &[] },
    Spec { category: "user", label: "Mesa shader cache", rel: ".cache/mesa_shader_cache", busy: &[] },
    Spec { category: "user", label: "Mesa shader cache db", rel: ".cache/mesa_shader_cache_db", busy: &[] },
    Spec { category: "user", label: "GStreamer cache", rel: ".cache/gstreamer-1.0", busy: &[] },
    Spec { category: "browser", label: "Firefox cache", rel: ".cache/mozilla/firefox", busy: &["firefox"] },
    Spec { category: "browser", label: "Chromium cache", rel: ".cache/chromium", busy: &["chromium", "chromium-browser"] },
    Spec { category: "browser", label: "Google Chrome cache", rel: ".cache/google-chrome", busy: &["chrome", "google-chrome"] },
    Spec { category: "browser", label: "Brave cache", rel: ".cache/BraveSoftware", busy: &["brave"] },
    Spec { category: "browser", label: "Zen cache", rel: ".cache/zen", busy: &["zen", "zen-browser"] },
    Spec { category: "browser", label: "Vivaldi cache", rel: ".cache/vivaldi", busy: &["vivaldi"] },
    Spec { category: "browser", label: "Microsoft Edge cache", rel: ".cache/microsoft-edge", busy: &["microsoft-edge", "msedge"] },
    Spec { category: "dev", label: "pip cache", rel: ".cache/pip", busy: &[] },
    Spec { category: "dev", label: "uv cache", rel: ".cache/uv", busy: &[] },
    Spec { category: "dev", label: "pypoetry cache", rel: ".cache/pypoetry", busy: &[] },
    Spec { category: "dev", label: "npm cache", rel: ".npm/_cacache", busy: &["npm"] },
    Spec { category: "dev", label: "pnpm store cache", rel: ".cache/pnpm", busy: &["pnpm"] },
    Spec { category: "dev", label: "yarn berry cache", rel: ".yarn/berry/cache", busy: &["yarn"] },
    Spec { category: "dev", label: "bun cache", rel: ".bun/install/cache", busy: &["bun"] },
    Spec { category: "dev", label: "cargo registry cache", rel: ".cargo/registry/cache", busy: &["cargo"] },
    Spec { category: "dev", label: "cargo git db", rel: ".cargo/git/db", busy: &["cargo"] },
    Spec { category: "dev", label: "go build cache", rel: ".cache/go-build", busy: &[] },
    Spec { category: "dev", label: "ccache", rel: ".cache/ccache", busy: &[] },
    Spec { category: "dev", label: "sccache", rel: ".cache/sccache", busy: &[] },
    Spec { category: "dev", label: "gradle caches", rel: ".gradle/caches", busy: &[] },
    Spec { category: "dev", label: "composer cache", rel: ".composer/cache", busy: &[] },
    Spec { category: "packages", label: "yay cache", rel: ".cache/yay", busy: &["yay"] },
    Spec { category: "packages", label: "paru cache", rel: ".cache/paru", busy: &["paru"] },
    Spec { category: "packages", label: "pikaur cache", rel: ".cache/pikaur", busy: &["pikaur"] },
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

fn push_unique(seen: &mut HashSet<PathBuf>, out: &mut Vec<(String, String, PathBuf, Vec<&'static str>)>, cat: &str, label: &str, path: PathBuf, busy: &[&'static str]) {
    if !path.exists() {
        return;
    }
    let key = path.canonicalize().unwrap_or_else(|_| path.clone());
    if seen.iter().any(|s| s == &key || s.starts_with(&key) || key.starts_with(s)) {
        return;
    }
    seen.insert(key);
    out.push((cat.to_string(), label.to_string(), path, busy.to_vec()));
}

fn candidate_paths() -> Result<Vec<(String, String, PathBuf, Vec<&'static str>)>> {
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
        );
    }

    scan_cache_leftovers(&home, &mut seen, &mut out)?;
    scan_electron_caches(&home, &mut seen, &mut out)?;
    scan_flatpak_caches(&home, &mut seen, &mut out)?;
    scan_logs(&home, &mut seen, &mut out)?;

    Ok(out)
}

fn scan_cache_leftovers(
    home: &Path,
    seen: &mut HashSet<PathBuf>,
    out: &mut Vec<(String, String, PathBuf, Vec<&'static str>)>,
) -> Result<()> {
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
        push_unique(seen, out, "apps", &label, path, &[]);
    }
    Ok(())
}

fn scan_electron_caches(
    home: &Path,
    seen: &mut HashSet<PathBuf>,
    out: &mut Vec<(String, String, PathBuf, Vec<&'static str>)>,
) -> Result<()> {
    let root = home.join(".config");
    let Ok(entries) = fs::read_dir(&root) else {
        return Ok(());
    };
    const NAMES: &[&str] = &["Cache", "Code Cache", "GPUCache", "CachedData", "DawnCache"];
    for entry in entries.flatten() {
        let app = entry.path();
        if !app.is_dir() {
            continue;
        }
        let app_name = entry.file_name().to_string_lossy().to_string();
        for cache_name in NAMES {
            let path = app.join(cache_name);
            if path.is_dir() {
                let label = format!("{app_name} {cache_name}");
                push_unique(seen, out, "apps", &label, path, &[]);
            }
        }
    }
    Ok(())
}

fn scan_flatpak_caches(
    home: &Path,
    seen: &mut HashSet<PathBuf>,
    out: &mut Vec<(String, String, PathBuf, Vec<&'static str>)>,
) -> Result<()> {
    let root = home.join(".var/app");
    let Ok(entries) = fs::read_dir(&root) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let cache = entry.path().join("cache");
        if cache.is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            let label = format!("flatpak {name} cache");
            push_unique(seen, out, "apps", &label, cache, &[]);
        }
    }
    Ok(())
}

fn scan_logs(
    home: &Path,
    seen: &mut HashSet<PathBuf>,
    out: &mut Vec<(String, String, PathBuf, Vec<&'static str>)>,
) -> Result<()> {
    let xorg = home.join(".local/share/xorg");
    push_unique(seen, out, "logs", "Xorg logs", xorg, &[]);

    for root in [home.join(".local/share"), home.join(".local/state")] {
        let Ok(entries) = fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            if !entry.path().is_dir() {
                continue;
            }
            for log_name in ["log", "logs", "Logs"] {
                let path = entry.path().join(log_name);
                if path.is_dir() {
                    let app = entry.file_name().to_string_lossy().into_owned();
                    let label = format!("{app} logs");
                    push_unique(seen, out, "logs", &label, path, &[]);
                }
            }
        }
    }
    Ok(())
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
    let paths: Vec<PathBuf> = candidates.iter().map(|c| c.2.clone()).collect();
    let sizes = util::path_sizes(&paths);

    let mut items = Vec::new();
    for ((cat, label, path, busy), bytes) in candidates.into_iter().zip(sizes) {
        if whitelist::is_protected(&path) {
            continue;
        }
        if bytes < 4096 {
            continue;
        }
        if cat == "apps" && bytes < 1_048_576 {
            continue;
        }
        let skip = busy_for(&path, &busy, &running);
        items.push(CleanItem {
            id: format!("{cat}:{}", path.display()),
            category: cat,
            label,
            path: path.display().to_string(),
            bytes,
            selected: skip.is_none(),
            skip_reason: skip,
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
        println!(
            "  {mark} {:<40} {:>10}{extra}",
            util::truncate_right(&item.label, 40),
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
        match util::remove_path(Path::new(&item.path)) {
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
}
