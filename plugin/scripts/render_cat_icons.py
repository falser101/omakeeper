#!/usr/bin/env python3
"""Category icons: one Lucide/SF-symbol stroke family, cream on transparent."""

import subprocess
from pathlib import Path

SIZE = 96
STROKE = "1.75"
COLOR = "#D6C9B0"
OUT = Path(__file__).resolve().parent.parent / "assets"

HEAD = f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="{COLOR}" stroke-width="{STROKE}" stroke-linecap="round" stroke-linejoin="round">'''
TAIL = "</svg>"

ICONS = {
    "cat-user": """
  <path d="M15 21v-8a1 1 0 0 0-1-1h-4a1 1 0 0 0-1 1v8"/>
  <path d="M3 10a2 2 0 0 1 .709-1.528l7-6a2 2 0 0 1 2.582 0l7 6A2 2 0 0 1 21 10v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>
""",
    "cat-apps": """
  <path d="M21 8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16Z"/>
  <path d="m3.3 7 8.7 5 8.7-5"/>
  <path d="M12 22V12"/>
""",
    "cat-packages": """
  <path d="M21 8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16Z"/>
  <path d="m3.3 7 8.7 5 8.7-5"/>
  <path d="M12 22V12"/>
  <path d="m7.5 4.27 9 5.15"/>
""",
    "cat-dev": """
  <path d="m15 12-8.373 8.373a1 1 0 1 1-3-3L12 9"/>
  <path d="m18 15 4-4"/>
  <path d="m21.5 11.5-1.914-1.914A2 2 0 0 1 19 8.172V7l-2.26-2.26a6 6 0 0 0-4.202-1.756L9 2.96l.92.82A6.18 6.18 0 0 1 12 8.4V10l2 2h1.172a2 2 0 0 1 1.414.586L18.5 14.5"/>
""",
    "cat-browser": """
  <circle cx="12" cy="12" r="10"/>
  <polygon points="16.24 7.76 14.12 14.12 7.76 16.24 9.88 9.88 16.24 7.76"/>
""",
    "cat-logs": """
  <path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/>
  <path d="M14 2v4a2 2 0 0 0 2 2h4"/>
  <path d="M10 13H8"/>
  <path d="M16 13H8"/>
  <path d="M16 17H8"/>
""",
    "cat-other": """
  <path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/>
""",
    "cat-folder": """
  <path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/>
""",
    "cat-ai": """
  <path d="M9.937 15.5A2 2 0 0 0 8.5 14.063l-6.135-1.582a.5.5 0 0 1 0-.962L8.5 9.936A2 2 0 0 0 9.937 8.5l1.582-6.135a.5.5 0 0 1 .963 0L14.063 8.5A2 2 0 0 0 15.5 9.937l6.135 1.581a.5.5 0 0 1 0 .964L15.5 14.063a2 2 0 0 0-1.437 1.437l-1.582 6.135a.5.5 0 0 1-.963 0z"/>
  <path d="M20 3v4"/>
  <path d="M22 5h-4"/>
  <path d="M4 17v2"/>
  <path d="M5 18H3"/>
""",
}


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    for name, body in ICONS.items():
        svg = OUT / f"{name}.svg"
        png = OUT / f"{name}.png"
        svg.write_text(HEAD + body + TAIL, encoding="utf-8")
        subprocess.check_call([
            "rsvg-convert", "-w", str(SIZE), "-h", str(SIZE),
            "--background-color=none", str(svg), "-o", str(png),
        ])
        print("wrote", png.name)


if __name__ == "__main__":
    main()
