# Omakeeper

System maintenance for [Omarchy](https://omarchy.org/) — clean caches, purge build artifacts, remove leftover installers, and (soon) uninstall, optimize, analyze, and monitor.

Inspired by the workflow of [Mole](https://github.com/tw93/Mole), rebuilt for Arch/Omarchy with a **Rust CLI** and an **Omarchy shell plugin**. Branding uses Omarchy menu styling and icon semantics, not solar-system planets.

## Status (Phase 0–1)

| Command | Status |
|---------|--------|
| `omakeeper` (interactive menu) | ✅ |
| `omakeeper clean` | ✅ dry-run + apply (user caches) |
| `omakeeper purge` | ✅ scan + confirm |
| `omakeeper installer` | ✅ Downloads / Desktop leftovers |
| `omakeeper history` | ✅ |
| `omakeeper whitelist` | ✅ |
| uninstall / optimize / analyze / status | planned |
| Omarchy overlay launcher | ✅ (opens CLI in terminal for now) |

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

## Install plugin (local)

Omarchy rejects symlinked plugin folders; copy or use `omarchy plugin add` with a git URL once published.

```bash
PLUGIN="$HOME/.config/omarchy/plugins/io.github.falser101.omakeeper"
mkdir -p "$PLUGIN"
cp -a plugin/. "$PLUGIN/"
omarchy plugin enable io.github.falser101.omakeeper
```

Summon:

```bash
omarchy-shell shell summon io.github.falser101.omakeeper '{}'
# or toggle
omarchy-shell shell toggle io.github.falser101.omakeeper
```

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
omakeeper clean               # delete after confirm
omakeeper purge --dry-run
omakeeper installer --dry-run
omakeeper history
omakeeper whitelist add ~/.cache/something-to-keep
omakeeper clean --dry-run --json
```

## Safety

- Prefer `--dry-run` before deleting.
- Whitelist paths with `omakeeper whitelist add <path>`.
- Destructive actions ask for confirmation unless `--yes` is passed after review.
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
