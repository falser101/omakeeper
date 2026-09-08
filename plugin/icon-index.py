#!/usr/bin/env python3
"""Map desktop ids, names, execs, and icon basenames to icon files on disk."""
import json
import os
import re
import sys

HOME = os.path.expanduser("~")
SIZES = ("512x512", "256x256", "128x128", "96x96", "64x64", "48x48", "32x32", "24x24", "22x22", "16x16", "scalable")
EXTS = ("png", "svg", "xpm")
SUFFIXES = ("bin", "git", "debug", "appimage", "nightly", "stable", "git-bin", "follow-system")


def normalize(value):
    s = str(value or "").strip().lower()
    s = re.sub(r"\.(desktop|png|svg|xpm|jpg|jpeg)$", "", s)
    changed = True
    while changed:
        changed = False
        for suf in SUFFIXES:
            tail = "-" + suf
            if s.endswith(tail):
                s = s[: -len(tail)]
                changed = True
    return re.sub(r"[^a-z0-9]+", "", s)


def first_file(paths):
    for path in paths:
        if path and os.path.isfile(path):
            return path
    return ""


def icon_roots():
    roots = [
        os.path.join(HOME, ".icons"),
        os.path.join(HOME, ".local/share/icons"),
        "/usr/local/share/icons",
        "/usr/share/icons",
    ]
    extra = os.environ.get("XDG_DATA_DIRS", "")
    for item in extra.split(":"):
        item = item.strip()
        if item:
            roots.append(os.path.join(item, "icons"))
    out = []
    seen = set()
    for root in roots:
        if root in seen or not os.path.isdir(root):
            continue
        seen.add(root)
        out.append(root)
    return out


def resolve_icon(name):
    raw = str(name or "").strip()
    if not raw:
        return ""
    if raw.startswith("/") and os.path.isfile(raw):
        return raw
    base = re.sub(r"\.(png|svg|xpm|jpg|jpeg)$", "", raw)
    pixmaps = [
        f"/usr/share/pixmaps/{base}.{ext}"
        for ext in EXTS
    ] + [
        os.path.join(HOME, f".local/share/pixmaps/{base}.{ext}")
        for ext in EXTS
    ]
    hit = first_file(pixmaps)
    if hit:
        return hit
    for root in icon_roots():
        for theme in ("hicolor", "Yaru-olive", "Yaru", "Papirus", "Adwaita"):
            base_dir = os.path.join(root, theme)
            if not os.path.isdir(base_dir):
                continue
            candidates = []
            for size in SIZES:
                for ext in EXTS:
                    candidates.append(os.path.join(base_dir, size, "apps", f"{base}.{ext}"))
            hit = first_file(candidates)
            if hit:
                return hit
        # pixmaps-style files dropped directly in a theme
        hit = first_file(os.path.join(root, f"{base}.{ext}") for ext in EXTS)
        if hit:
            return hit
    return ""


def add(index, key, path):
    if not path:
        return
    raw = str(key or "").strip()
    if not raw:
        return
    low = raw.lower()
    if low not in index:
        index[low] = path
    n = normalize(raw)
    if n and n not in index:
        index[n] = path


def index_icon_files(index):
    """Basename → file, so vscode.png is findable without a .desktop match."""
    files = []
    for ext in EXTS:
        files.append(f"/usr/share/pixmaps/*.{ext}")
        files.append(os.path.join(HOME, f".local/share/pixmaps/*.{ext}"))
        for root in icon_roots():
            files.append(os.path.join(root, "hicolor", "*", "apps", f"*.{ext}"))
    import glob as g

    # Smaller first so larger sizes overwrite when we skip-if-present is inverted:
    # we only add if missing, so scan large sizes first.
    ordered = []
    for pattern in files:
        ordered.extend(g.glob(pattern))
    # Prefer larger files (bytes) as a proxy for 256/512 over 16px.
    ordered.sort(key=lambda p: os.path.getsize(p) if os.path.isfile(p) else 0, reverse=True)
    for path in ordered:
        if not os.path.isfile(path):
            continue
        base = os.path.splitext(os.path.basename(path))[0]
        add(index, base, path)


def parse_desktop(path):
    fields = {}
    try:
        with open(path, errors="replace") as fh:
            in_entry = False
            for line in fh:
                line = line.rstrip("\n")
                if line.startswith("["):
                    if line.strip() == "[Desktop Entry]":
                        in_entry = True
                        continue
                    if in_entry:
                        break
                    continue
                if not in_entry or not line or line.startswith("#") or "=" not in line:
                    continue
                key, value = line.split("=", 1)
                fields[key.strip()] = value.strip()
    except OSError:
        return {}
    return fields


def index_desktops(index):
    dirs = [
        os.path.join(HOME, ".local/share/applications"),
        "/usr/local/share/applications",
        "/usr/share/applications",
    ]
    extra = os.environ.get("XDG_DATA_DIRS", "")
    for item in extra.split(":"):
        item = item.strip()
        if item:
            dirs.append(os.path.join(item, "applications"))
    seen = set()
    for folder in dirs:
        if folder in seen or not os.path.isdir(folder):
            continue
        seen.add(folder)
        try:
            names = os.listdir(folder)
        except OSError:
            continue
        for fname in names:
            if not fname.endswith(".desktop"):
                continue
            path = os.path.join(folder, fname)
            fields = parse_desktop(path)
            icon_name = fields.get("Icon") or ""
            icon_path = resolve_icon(icon_name)
            if not icon_path:
                icon_path = resolve_icon(os.path.splitext(fname)[0])
            if not icon_path:
                continue
            desktop_id = os.path.splitext(fname)[0]
            add(index, desktop_id, icon_path)
            add(index, fields.get("Name", ""), icon_path)
            add(index, fields.get("GenericName", ""), icon_path)
            add(index, fields.get("StartupWMClass", ""), icon_path)
            add(index, icon_name, icon_path)
            exec_line = fields.get("Exec", "")
            if exec_line:
                # strip env PREFIX=val
                parts = exec_line.split()
                cmd = ""
                for part in parts:
                    if "=" in part and not part.startswith("/"):
                        continue
                    cmd = part.strip("\"'")
                    break
                if cmd:
                    add(index, os.path.basename(cmd), icon_path)
            # reverse-DNS last segment: org.mozilla.firefox → firefox
            if "." in desktop_id:
                last = desktop_id.split(".")[-1]
                if last and last.lower() not in ("desktop", "app"):
                    add(index, last, icon_path)


def main():
    index = {}
    index_icon_files(index)
    index_desktops(index)
    payload = json.dumps(index, separators=(",", ":"), ensure_ascii=False)
    dest = sys.argv[1] if len(sys.argv) > 1 else ""
    if dest:
        tmp = dest + ".tmp"
        with open(tmp, "w", encoding="utf-8") as fh:
            fh.write(payload)
            fh.write("\n")
        os.replace(tmp, dest)
    else:
        sys.stdout.write(payload)
        sys.stdout.write("\n")


if __name__ == "__main__":
    main()
