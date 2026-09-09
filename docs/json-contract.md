# Omakeeper JSON contract

Machine-readable output for the Omarchy plugin and scripts. Prefer `--json` with `--dry-run` for scans.

## `omakeeper clean --dry-run --json`

```json
{
  "dry_run": true,
  "items": [
    {
      "id": "dev:/home/me/.cache/pip",
      "category": "dev",
      "label": "pip cache",
      "path": "/home/me/.cache/pip",
      "bytes": 123456,
      "selected": true,
      "skip_reason": null
    }
  ],
  "total_bytes": 123456,
  "freed_bytes": 0,
  "removed": 0
}
```

Busy items set `selected` to `false` and fill `skip_reason` (for example `"firefox busy"`). Optional `action` is set for command-backed items (`pacman-sc`, `flatpak-unused`) instead of a plain directory delete. Leftovers, old downloads, Steam shader cache, Gradle, and pacman package cache start unselected.

## `omakeeper purge --dry-run --json`

Array of artifacts:

```json
[
  {
    "project": "/home/me/Projects/app",
    "name": "node_modules",
    "path": "/home/me/Projects/app/node_modules",
    "bytes": 800000000,
    "age_days": 28,
    "selected": true
  }
]
```

## `omakeeper installer --dry-run --json`

Array of installer files (`name`, `path`, `source`, `bytes`, `selected`). Interactive defaults leave `selected` false; `--yes` selects all.

## `omakeeper analyze --json [PATH]`

Without `PATH`, `overview` is true and `entries` are the home overview roots. With `PATH`, `entries` are that directory's children.

```json
{
  "path": "/home/me",
  "overview": true,
  "entries": [
    { "name": "Home", "path": "/home/me", "size": 80939438080, "is_dir": true }
  ],
  "large_files": [],
  "total_size": 80939438080,
  "total_files": 0
}
```

## `omakeeper status --json`

One-shot snapshot. `omakeeper status --watch --json` streams NDJSON.

```json
{
  "host": "arch",
  "health_score": 92,
  "uptime": "3d 12h 45m",
  "cpu": { "usage": 45.2, "logical_cpu": 16, "load": [0.82, 1.05, 1.23] },
  "memory": { "total": 34359738368, "used": 20078972109, "available": 14280766259, "used_percent": 58.4 },
  "disks": [],
  "network": { "down_bps": 540000, "up_bps": 20000 },
  "processes": [],
  "zombie_count": 0,
  "zombie_parents": []
}
```

Piped stdout (not a TTY) also emits JSON for `status`.

## `omakeeper optimize --dry-run --json`

```json
{
  "dry_run": true,
  "diagnosis": ["Free space 240GiB", "User journal 3.5GiB (vacuum recommended)"],
  "tasks": [
    {
      "id": "journal-user",
      "label": "Vacuum user journal (7d)",
      "detail": "current 3.5GiB",
      "bytes": 3758096384,
      "selected": true,
      "status": "ready"
    }
  ],
  "applied": 0,
  "skipped": 1,
  "unavailable": 0,
  "freed_bytes": 0
}
```

`omakeeper optimize --whitelist --json` prints the skipped task id array.

## `omakeeper uninstall --dry-run --json [PACKAGES…]`

Without package names, lists removable explicit packages (`selected` false), each with `required_by`, plus leftover dirs for every listed package. With names, includes leftover dirs, reverse dependents (`required_by` on the report), and `pacman -Rns` targets.

Apply (`omakeeper uninstall --yes [--leftover PATH]… PACKAGES…`) only deletes leftover dirs passed with `--leftover`. Omitted dirs are kept.

```json
{
  "dry_run": true,
  "packages": [
    { "name": "vlc", "description": "media player", "bytes": 52428800, "selected": true }
  ],
  "leftovers": [
    { "package": "vlc", "path": "/home/me/.config/vlc", "bytes": 4096 }
  ],
  "required_by": [
    { "name": "vlc-plugin", "description": "plugin", "bytes": 1024, "protected": false }
  ],
  "pacman_targets": ["vlc"],
  "freed_bytes": 52432896
}
```

## `omakeeper autostart --json`

Merged XDG login autostart (`/etc/xdg/autostart` + `~/.config/autostart`). User `Hidden=true` stubs disable a system entry without deleting it.

```json
{
  "items": [
    {
      "id": "print-applet",
      "name": "Print Queue Applet",
      "description": "",
      "exec": "system-config-printer-applet",
      "icon": "printer",
      "enabled": false,
      "source": "system",
      "path": "/home/me/.config/autostart/print-applet.desktop",
      "user_added": false,
      "locked": false
    }
  ],
  "available": [
    { "id": "firefox", "name": "Firefox", "description": "Web Browser", "icon": "firefox" }
  ]
}
```

Mutations (all reprint the same JSON when `--json` is set):

- `omakeeper autostart --enable ID`
- `omakeeper autostart --disable ID`
- `omakeeper autostart --add DESKTOP-ID` — copy from `applications/`
- `omakeeper autostart --remove ID` — drop a user-added entry, or mask a system one

Takes effect on the next login (UWSM/XDG autostart). Session-required entries (`gnome-keyring`, `xdg-user-dirs`, …) are `locked`.

## `omakeeper history --json`

Array of log entries (`ts`, `command`, `dry_run`, `freed_bytes`, `items`, `detail`).
