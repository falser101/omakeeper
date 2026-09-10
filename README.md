# Omakeeper

System maintenance for [Omarchy](https://omarchy.org/) — clean caches, purge build artifacts, remove leftover installers, uninstall packages, run bounded maintenance, explore disk usage, and watch live system status.

Inspired by the workflow of [Mole](https://github.com/tw93/Mole), rebuilt for Arch/Omarchy with a **Rust CLI** and an **Omarchy shell plugin**. Branding uses Omarchy menu styling and icon semantics, not solar-system planets.

## Status

| Command | Status |
|---------|--------|
| `omakeeper` (interactive menu) | ✅ |
| `omakeeper clean` | ✅ scan + TUI pick + apply (user caches, browsers, apps, logs, dev) |
| `omakeeper purge` | ✅ scan + TUI pick + confirm |
| `omakeeper installer` | ✅ Downloads / Desktop / Documents / Telegram leftovers |
| `omakeeper analyze` | ✅ disk explorer (treemap, drill-in, reveal, trash) |
| `omakeeper status` | ✅ health dashboard (`--watch` for live) |
| `omakeeper history` | ✅ |
| `omakeeper whitelist` | ✅ |
| `omakeeper uninstall` | ✅ pacman -Rns; leftover user dirs are opt-in via `--leftover PATH` (denylist for core packages) |
| `omakeeper autostart` | ✅ list / enable / disable / add XDG login apps (`~/.config/autostart`) |
| `omakeeper optimize` | ✅ journal vacuum, font/icon/MIME caches, DNS flush, tmpfiles, flatpak repair |
| Omarchy overlay | ✅ five-tab app (清理 / 软件 / 优化 / 分析 / 状态), no terminal hop |

## Install CLI

Requires Rust (`rustup` or distro `rust` package).

```bash
cd cli
cargo install --path .
```

Ensure `~/.cargo/bin` is on your `PATH`. Optional short alias:

```bash
ln -sf "$(which omakeeper)" ~/.local/bin/ok
```

## Install plugin

From GitHub (recommended):

```bash
# Clone the repo first, then point plugin add at the plugin/ subdirectory
# after packaging — or copy plugin/ into the Omarchy plugins dir:

git clone https://github.com/falser101/omakeeper.git
PLUGIN="$HOME/.config/omarchy/plugins/io.github.falser101.omakeeper"
mkdir -p "$PLUGIN"
cp -a omakeeper/plugin/. "$PLUGIN/"
omarchy plugin enable io.github.falser101.omakeeper
```

> Note: `omarchy plugin add` expects the **plugin root** (with `manifest.json`) to be the repo root. This monorepo keeps the plugin under `plugin/`, so use the copy steps above until a dedicated plugin repo or release layout is published.

Summon the five-tab overlay (Clean / Software / Optimize / Analyze / Status):

```bash
omarchy-shell shell summon io.github.falser101.omakeeper '{}'
# or toggle
omarchy-shell shell toggle io.github.falser101.omakeeper
```

The overlay talks to the CLI over `--json` (and `--select-file` when applying). Uninstall uses `pkexec` for pacman when it is not attached to a TTY.

Suggested Hyprland binding (user config):

```lua
o.bind({ "SUPER", "SHIFT" }, "O", function()
  os.execute("omarchy-shell shell toggle io.github.falser101.omakeeper &")
end)
```

## Usage

```bash
omakeeper                     # interactive menu
omakeeper clean --dry-run     # preview caches
omakeeper clean               # pick items, then delete after confirm
omakeeper purge --dry-run
omakeeper installer --dry-run
omakeeper uninstall           # pick explicit packages (/ to search)
omakeeper uninstall vlc --dry-run
omakeeper optimize --dry-run  # preview maintenance tasks
omakeeper optimize            # apply selected tasks
omakeeper optimize --whitelist
omakeeper analyze             # disk explorer
omakeeper analyze ~/.cache
omakeeper analyze --json ~
omakeeper status              # one-shot dashboard
omakeeper status --watch      # live refresh
omakeeper status --json
omakeeper autostart --json
omakeeper autostart --disable print-applet
omakeeper autostart --add firefox
omakeeper history
omakeeper whitelist add ~/.cache/something-to-keep
omakeeper clean --dry-run --json
```

In a TTY, `clean` / `purge` / `installer` / `uninstall` / `optimize` open a selector:

- space toggle · `a` all · `n` none · `i` invert · `/` search · enter confirm · `q` quit

Busy processes (Firefox, Chrome, cargo, …) are skipped so live caches are not deleted. Installer items start unselected. Purge unselects artifacts touched in the last 7 days.

`analyze` moves selected items to the XDG trash (`~/.local/share/Trash`) after confirm, and refuses system paths (`/usr`, `/etc`, `$HOME` itself, …). The overlay treemap drills by folder, jumps from the path bar, and right-click reveals in the file manager.

## Safety

- Prefer `--dry-run` before deleting.
- Whitelist paths with `omakeeper whitelist add <path>`.
- Destructive actions ask for confirmation unless `--yes` is passed after review.
- Model caches (huggingface, ollama, torch, …) are never offered by `clean`.
- `uninstall` will not list kernel, Hyprland, sudo, pacman, omarchy settings, NVIDIA, or other core packages.
- `optimize` stays user-level unless `sudo -v` has cached credentials (system journal vacuum).
- Logs: `~/.local/share/omakeeper/operations.log`
- Config: `~/.config/omakeeper/`

## Layout

```
omakeeper/
  cli/       Rust CLI
  plugin/    Omarchy Quickshell overlay
  docs/      JSON contracts
```

## License

MIT
