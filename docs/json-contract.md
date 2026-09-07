# Omakeeper JSON contract (draft)

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
      "selected": true
    }
  ],
  "total_bytes": 123456,
  "freed_bytes": 0,
  "removed": 0
}
```

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

Array of installer files (name, path, source, bytes, selected).

## `omakeeper history --json`

Array of log entries (ts, command, dry_run, freed_bytes, items, detail).

## Planned

- `omakeeper status --json` / `--watch`
- `omakeeper analyze --json <path>`
- `omakeeper clean apply --json` with explicit item ids
