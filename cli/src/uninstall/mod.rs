use crate::history;
use crate::ui::picker::{self, PickItem};
use crate::util;
use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct Package {
    pub name: String,
    pub description: String,
    pub bytes: u64,
    pub selected: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Leftover {
    pub package: String,
    pub path: String,
    pub bytes: u64,
}

#[derive(Debug, Serialize)]
struct UninstallReport {
    dry_run: bool,
    packages: Vec<Package>,
    leftovers: Vec<Leftover>,
    pacman_targets: Vec<String>,
    freed_bytes: u64,
}

const PROTECTED_EXACT: &[&str] = &[
    "base",
    "base-devel",
    "bash",
    "zsh",
    "coreutils",
    "util-linux",
    "filesystem",
    "glibc",
    "gcc-libs",
    "pacman",
    "pacman-contrib",
    "archlinux-keyring",
    "sudo",
    "polkit",
    "shadow",
    "dbus",
    "sddm",
    "greetd",
    "networkmanager",
    "iwd",
    "quickshell",
    "hyprland",
    "wayland",
    "xorg-xwayland",
    "mesa",
    "libglvnd",
    "systemd",
    "systemd-libs",
    "systemd-sysvcompat",
    "linux",
    "linux-lts",
    "linux-zen",
    "linux-hardened",
    "linux-firmware",
    "linux-api-headers",
    "amd-ucode",
    "intel-ucode",
    "mkinitcpio",
    "limine",
    "grub",
    "efibootmgr",
    "openssl",
    "ca-certificates",
    "omarchy",
    "omarchy-keyring",
    "pipewire",
    "wireplumber",
];

fn is_protected_package(name: &str) -> bool {
    if PROTECTED_EXACT.contains(&name) {
        return true;
    }
    if name.starts_with("omarchy-settings") || name.starts_with("omarchy-keyring") {
        return true;
    }
    if name == "omarchy-dev" {
        return true;
    }
    if name.starts_with("linux-firmware") {
        return true;
    }
    if name.starts_with("linux-") && name.ends_with("-headers") {
        return true;
    }
    if name.starts_with("nvidia") {
        return true;
    }
    if name.starts_with("hypr") {
        return true;
    }
    if name.starts_with("systemd") {
        return true;
    }
    if name.starts_with("pipewire") || name.starts_with("wireplumber") {
        return true;
    }
    if name.starts_with("xdg-desktop-portal") {
        return true;
    }
    if name.starts_with("vulkan-") {
        return true;
    }
    false
}

fn parse_pacman_size(s: &str) -> u64 {
    let mut parts = s.split_whitespace();
    let Some(num) = parts.next() else {
        return 0;
    };
    let n: f64 = num.parse().unwrap_or(0.0);
    let unit = parts.next().unwrap_or("B");
    let mul = match unit {
        "B" => 1.0,
        "KiB" => 1024.0,
        "MiB" => 1024.0 * 1024.0,
        "GiB" => 1024.0 * 1024.0 * 1024.0,
        "TiB" => 1024.0_f64.powi(4),
        _ => 1.0,
    };
    (n * mul) as u64
}

fn pacman_qei() -> Result<String> {
    let out = Command::new("pacman")
        .args(["-Qei"])
        .env("LC_ALL", "C")
        .output()
        .context("run pacman -Qei")?;
    if !out.status.success() {
        anyhow::bail!("{}", String::from_utf8_lossy(&out.stderr));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn parse_packages(text: &str) -> Vec<Package> {
    let mut pkgs = Vec::new();
    let mut name = String::new();
    let mut description = String::new();
    let mut bytes = 0u64;
    let flush = |pkgs: &mut Vec<Package>, name: &mut String, description: &str, bytes: u64| {
        if name.is_empty() || is_protected_package(name) {
            name.clear();
            return;
        }
        pkgs.push(Package {
            name: name.clone(),
            description: description.to_string(),
            bytes,
            selected: false,
        });
        name.clear();
    };
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("Name") {
            flush(&mut pkgs, &mut name, &description, bytes);
            description.clear();
            bytes = 0;
            name = rest.split_once(':').map(|(_, v)| v.trim().to_string()).unwrap_or_default();
        } else if let Some(rest) = line.strip_prefix("Description") {
            description = rest
                .split_once(':')
                .map(|(_, v)| v.trim().to_string())
                .unwrap_or_default();
        } else if let Some(rest) = line.strip_prefix("Installed Size") {
            let raw = rest.split_once(':').map(|(_, v)| v.trim()).unwrap_or("");
            bytes = parse_pacman_size(raw);
        }
    }
    flush(&mut pkgs, &mut name, &description, bytes);
    pkgs.sort_by(|a, b| b.bytes.cmp(&a.bytes).then(a.name.cmp(&b.name)));
    pkgs
}

fn installed_names() -> Result<HashSet<String>> {
    let out = Command::new("pacman")
        .args(["-Qq"])
        .env("LC_ALL", "C")
        .output()
        .context("run pacman -Qq")?;
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

fn strip_pkg_suffix(name: &str) -> &str {
    for suffix in ["-git", "-bin", "-debug", "-appimage", "-nightly", "-stable"] {
        if let Some(stripped) = name.strip_suffix(suffix) {
            if stripped.len() >= 2 {
                return stripped;
            }
        }
    }
    name
}

fn leftover_names(pkg: &str) -> Vec<String> {
    let base = strip_pkg_suffix(pkg);
    let mut names = vec![pkg.to_string(), base.to_string()];
    match base {
        "firefox" | "librewolf" | "firedragon" => names.push("mozilla".into()),
        "google-chrome" => names.extend(["google-chrome".into(), "chrome".into()]),
        "code" | "code-oss" | "visual-studio-code-bin" => {
            names.extend(["Code".into(), "Code - OSS".into(), "code".into()]);
        }
        "vscodium" | "vscodium-bin" => names.extend(["VSCodium".into(), "vscodium".into()]),
        "zed" | "zed-editor" => names.extend(["zed".into(), "zed-editor".into()]),
        "telegram-desktop" => {
            names.extend(["TelegramDesktop".into(), "telegramdesktop".into()]);
        }
        "discord" => names.push("discord".into()),
        "steam" => names.extend(["Steam".into(), "steam".into()]),
        "obs-studio" => names.extend(["obs-studio".into(), "obs".into()]),
        "wechat" | "wechat-bin" | "wechat-universal" => {
            names.extend(["xwechat".into(), "wechat".into()]);
        }
        "feishu" | "lark" => names.extend(["lark".into(), "feishu".into()]),
        "qq" | "linuxqq" => names.extend(["QQ".into(), "qq".into()]),
        "spotify" => names.push("spotify".into()),
        "thunderbird" => names.extend(["thunderbird".into(), "Thunderbird".into()]),
        "qbittorrent" => names.extend(["qBittorrent".into(), "qbittorrent".into()]),
        "nextcloud-client" => names.extend(["Nextcloud".into(), "nextcloud".into()]),
        _ => {}
    }
    names.sort();
    names.dedup();
    names
}

fn leftover_dirs_for(pkg: &str, home: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let names = leftover_names(pkg);
    for base in [".config", ".local/share", ".cache", ".local/state"] {
        let root = home.join(base);
        for name in &names {
            let p = root.join(name);
            if p.is_dir() && !util::is_dangerous_delete(&p) {
                out.push(p);
            }
        }
    }
    out
}

fn collect_leftovers(packages: &[String], remaining: &HashSet<String>) -> Result<Vec<Leftover>> {
    let home = util::home_dir()?;
    let mut claimed: HashSet<PathBuf> = HashSet::new();
    for other in remaining {
        if packages.iter().any(|p| p == other) {
            continue;
        }
        for p in leftover_dirs_for(other, &home) {
            claimed.insert(p.canonicalize().unwrap_or(p));
        }
    }

    let mut leftovers = Vec::new();
    let mut seen = HashSet::new();
    let paths: Vec<(String, PathBuf)> = packages
        .iter()
        .flat_map(|pkg| {
            leftover_dirs_for(pkg, &home)
                .into_iter()
                .map(|p| (pkg.clone(), p))
        })
        .collect();
    let sizes = util::path_sizes(&paths.iter().map(|(_, p)| p.clone()).collect::<Vec<_>>());
    for ((pkg, path), bytes) in paths.into_iter().zip(sizes) {
        if bytes == 0 {
            continue;
        }
        let key = path.canonicalize().unwrap_or_else(|_| path.clone());
        if claimed.contains(&key) || !seen.insert(key) {
            continue;
        }
        leftovers.push(Leftover {
            package: pkg,
            path: path.display().to_string(),
            bytes,
        });
    }
    leftovers.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    Ok(leftovers)
}

fn pacman_print(packages: &[String]) -> Result<Vec<String>> {
    let mut cmd = Command::new("pacman");
    cmd.args(["-Rs", "--print", "--print-format", "%n"])
        .env("LC_ALL", "C");
    cmd.args(packages);
    let out = cmd.output().context("run pacman --print")?;
    if !out.status.success() {
        anyhow::bail!("{}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

fn pacman_remove(packages: &[String]) -> Result<()> {
    let mut cmd = if !util::is_tty() && util::command_exists("pkexec") {
        let mut c = Command::new("pkexec");
        c.args(["/usr/bin/pacman", "-Rns", "--noconfirm"]);
        c
    } else {
        let mut c = Command::new("sudo");
        c.args(["pacman", "-Rns", "--noconfirm"]);
        c
    };
    cmd.args(packages);
    let status = cmd.status().context("run privileged pacman -Rns")?;
    if !status.success() {
        anyhow::bail!("pacman -Rns failed with {status}");
    }
    Ok(())
}

fn pick_packages(pkgs: &mut [Package]) -> Result<bool> {
    let mut rows: Vec<PickItem> = pkgs
        .iter()
        .map(|p| PickItem {
            selected: p.selected,
            locked: false,
            title: p.name.clone(),
            detail: p.description.clone(),
            bytes: p.bytes,
        })
        .collect();
    let ok = picker::select_items("Select packages to uninstall  / search", &mut rows)?;
    if ok {
        for (pkg, row) in pkgs.iter_mut().zip(rows) {
            pkg.selected = row.selected;
        }
    }
    Ok(ok)
}

pub fn run(dry_run: bool, yes: bool, json: bool, names: &[String]) -> Result<()> {
    if !util::command_exists("pacman") {
        anyhow::bail!("pacman not found; uninstall is Arch-only");
    }
    let text = pacman_qei()?;
    let mut pkgs = parse_packages(&text);
    let installed = installed_names()?;

    if !names.is_empty() {
        for n in names {
            if is_protected_package(n) {
                anyhow::bail!("refusing to uninstall protected package: {n}");
            }
            if !installed.contains(n) {
                anyhow::bail!("package not installed: {n}");
            }
            if let Some(pkg) = pkgs.iter_mut().find(|p| p.name == *n) {
                pkg.selected = true;
            } else {
                pkgs.push(Package {
                    name: n.clone(),
                    description: String::new(),
                    bytes: 0,
                    selected: true,
                });
            }
        }
    } else if json && dry_run {
        println!(
            "{}",
            serde_json::to_string_pretty(&UninstallReport {
                dry_run: true,
                packages: pkgs,
                leftovers: Vec::new(),
                pacman_targets: Vec::new(),
                freed_bytes: 0,
            })?
        );
        return Ok(());
    } else if !yes {
        if pkgs.is_empty() {
            println!("No removable explicit packages found.");
            return Ok(());
        }
        if util::is_tty() {
            if !pick_packages(&mut pkgs)? {
                println!("Aborted.");
                return Ok(());
            }
        } else {
            println!("Pass package names, or run in a terminal to pick. Example:");
            println!("  omakeeper uninstall vlc --dry-run");
            return Ok(());
        }
    }

    let selected: Vec<Package> = pkgs.iter().filter(|p| p.selected).cloned().collect();
    if selected.is_empty() {
        println!("Nothing selected.");
        return Ok(());
    }
    for p in &selected {
        if is_protected_package(&p.name) {
            anyhow::bail!("refusing to uninstall protected package: {}", p.name);
        }
    }

    let selected_names: Vec<String> = selected.iter().map(|p| p.name.clone()).collect();
    let leftovers = collect_leftovers(&selected_names, &installed)?;
    let leftover_bytes: u64 = leftovers.iter().map(|l| l.bytes).sum();
    let pkg_bytes: u64 = selected.iter().map(|p| p.bytes).sum();
    let print_targets = match pacman_print(&selected_names) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("warning: pacman --print failed: {e}");
            selected_names.clone()
        }
    };

    if json && dry_run {
        let report = UninstallReport {
            dry_run: true,
            packages: selected,
            leftovers,
            pacman_targets: print_targets,
            freed_bytes: pkg_bytes + leftover_bytes,
        };
        println!("{}", serde_json::to_string_pretty(&report)?);
        history::log_operation("uninstall", true, pkg_bytes + leftover_bytes, report.packages.len(), "scan")?;
        return Ok(());
    }

    println!("Packages to remove:\n");
    for p in &selected {
        println!(
            "  ● {:<28} {:>10}  {}",
            p.name,
            util::format_bytes(p.bytes),
            util::truncate_right(&p.description, 40)
        );
    }
    if print_targets.len() > selected.len() {
        println!(
            "\n  pacman -Rns would also remove {} unused dependenc(y/ies)",
            print_targets.len() - selected.len()
        );
    }
    if !leftovers.is_empty() {
        println!("\nLeftover user data:");
        for l in &leftovers {
            println!(
                "  ● {:<28} {:>10}  {}",
                l.package,
                util::format_bytes(l.bytes),
                l.path
            );
        }
    }
    println!(
        "\nSelected: {} packages + {} leftovers",
        util::format_bytes(pkg_bytes),
        util::format_bytes(leftover_bytes)
    );

    if dry_run {
        println!("\nRe-run without --dry-run to uninstall.");
        history::log_operation(
            "uninstall",
            true,
            pkg_bytes + leftover_bytes,
            selected.len(),
            "scan",
        )?;
        return Ok(());
    }

    if !yes
        && !util::confirm(&format!(
            "Uninstall {} package(s) and delete leftover data?",
            selected.len()
        ))?
    {
        println!("Aborted.");
        return Ok(());
    }

    pacman_remove(&selected_names)?;
    println!("  ✓ pacman -Rns {}", selected_names.join(" "));

    let mut freed = pkg_bytes;
    let mut removed_left = 0usize;
    for l in &leftovers {
        match util::remove_path(Path::new(&l.path)) {
            Ok(()) => {
                freed += l.bytes;
                removed_left += 1;
                println!("  ✓ leftover {}", l.path);
            }
            Err(e) => eprintln!("  ✗ {}: {e}", l.path),
        }
    }

    println!(
        "\nUninstall complete\nRemoved {} package(s), {removed_left} leftover(s), {}",
        selected.len(),
        util::format_bytes(freed)
    );
    history::log_operation("uninstall", false, freed, selected.len(), "apply")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protects_core_and_allows_apps() {
        assert!(is_protected_package("linux"));
        assert!(is_protected_package("linux-lts"));
        assert!(is_protected_package("linux-firmware"));
        assert!(is_protected_package("hyprland"));
        assert!(is_protected_package("hyprlock"));
        assert!(is_protected_package("omarchy"));
        assert!(is_protected_package("omarchy-settings-dev"));
        assert!(is_protected_package("nvidia-dkms"));
        assert!(is_protected_package("sudo"));
        assert!(!is_protected_package("vlc"));
        assert!(!is_protected_package("omarchy-share-picker-git"));
        assert!(!is_protected_package("firefox"));
    }

    #[test]
    fn parses_pacman_iec_sizes() {
        assert_eq!(
            parse_pacman_size("55.96 MiB"),
            (55.96 * 1024.0 * 1024.0) as u64
        );
        assert_eq!(parse_pacman_size("602.93 KiB"), (602.93 * 1024.0) as u64);
        assert_eq!(parse_pacman_size("0.00 B"), 0);
    }

    #[test]
    fn leftover_aliases_cover_browsers() {
        let names = leftover_names("firefox");
        assert!(names.iter().any(|n| n == "mozilla"));
        let zed = leftover_names("zed-editor");
        assert!(zed.iter().any(|n| n == "zed"));
    }

    #[test]
    fn parse_qei_skips_protected() {
        let text = "\
Name            : sudo
Description     : give privileges
Installed Size  : 8.00 MiB
Name            : vlc
Description     : media player
Installed Size  : 50.00 MiB
";
        let pkgs = parse_packages(text);
        assert_eq!(pkgs.len(), 1);
        assert_eq!(pkgs[0].name, "vlc");
        assert!(pkgs[0].bytes > 0);
    }
}
