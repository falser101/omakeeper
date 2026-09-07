# Omakeeper

System maintenance for [Omarchy](https://omarchy.org/) — clean caches, purge build artifacts, remove leftover installers, and (soon) uninstall, optimize, analyze, and monitor.

Inspired by the workflow of [Mole](https://github.com/tw93/Mole), rebuilt for Arch/Omarchy with a Rust CLI and an Omarchy shell plugin. Branding uses Omarchy icon themes, not solar-system planets.

## Status

Phase 1 MVP in progress:

| Command | Status |
|---------|--------|
| `omakeeper` (menu) | scaffolding |
| `omakeeper clean` | dry-run + apply (user caches) |
| `omakeeper purge` | scan + confirm |
| `omakeeper installer` | scan Downloads installers |
| `omakeeper history` | operation log |
| `omakeeper whitelist` | protect paths from clean |
| uninstall / optimize / analyze / status | planned |
| Omarchy plugin UI | overlay shell |

## Install CLI

```bash
cd cli
cargo install --path .
```

Or for development:

```bash
cd cli && cargo build --release
./target/release/omakeeper --help
```

Optional short alias:

```bash
ln -s "$(which omakeeper)" ~/.local/bin/ok
```

## Install plugin

```bash
omarchy plugin add /home/falser/Projects/omakeeper/plugin --enable
# or from git once published:
# omarchy plugin add https://github.com/<you>/omakeeper.git --enable
```

Summon:

```bash
omarchy-shell shell summon io.github.falser.omakeeper
```

## Safety

- Prefer `omakeeper clean --dry-run` before deleting.
- Whitelist paths with `omakeeper whitelist add <path>`.
- Destructive actions ask for confirmation unless `--yes` is passed after a dry-run review.
- Operations are logged under `~/.local/share/omakeeper/operations.log`.

## Layout

```
omakeeper/
  cli/       Rust CLI (omakeeper)
  plugin/    Omarchy Quickshell overlay
  docs/      JSON contracts and design notes
```

## License

MIT
