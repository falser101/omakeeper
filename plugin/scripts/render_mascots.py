#!/usr/bin/env python3
"""Omarchy-wordmark mascots: pixel lattice + Bayer dither + vertical ramp."""

from __future__ import annotations

import math
from pathlib import Path

from PIL import Image, ImageDraw

COLS, ROWS = 32, 32
FILL = 13
GAP = 4
PITCH = FILL + GAP
PAD = 24
CANVAS = COLS * PITCH + PAD * 2

BAYER = [
    [0, 32, 8, 40, 2, 34, 10, 42],
    [48, 16, 56, 24, 50, 18, 58, 26],
    [12, 44, 4, 36, 14, 46, 6, 38],
    [60, 28, 52, 20, 62, 30, 54, 22],
    [3, 35, 11, 43, 1, 33, 9, 41],
    [51, 19, 59, 27, 49, 17, 57, 25],
    [15, 47, 7, 39, 13, 45, 5, 37],
    [63, 31, 55, 23, 61, 29, 53, 21],
]

# Ice → cobalt, matching omarchy.org hero wordmark
RAMP = [
    (236, 246, 255),
    (198, 224, 248),
    (154, 196, 236),
    (104, 160, 216),
    (58, 118, 186),
    (28, 78, 148),
    (14, 48, 104),
    (8, 28, 64),
]

FRAMES = 6
OUT = Path(__file__).resolve().parent.parent / "assets"


class Grid:
    def __init__(self) -> None:
        self.m = [[0] * COLS for _ in range(ROWS)]

    def on(self, x: float, y: float) -> None:
        xi, yi = int(round(x)), int(round(y))
        if 0 <= xi < COLS and 0 <= yi < ROWS:
            self.m[yi][xi] = 1

    def off(self, x: float, y: float) -> None:
        xi, yi = int(round(x)), int(round(y))
        if 0 <= xi < COLS and 0 <= yi < ROWS:
            self.m[yi][xi] = 0

    def disk(self, cx: float, cy: float, r: float, val: int = 1) -> None:
        rr = r * r
        x0, x1 = max(0, int(cx - r - 1)), min(COLS, int(cx + r + 2))
        y0, y1 = max(0, int(cy - r - 1)), min(ROWS, int(cy + r + 2))
        for y in range(y0, y1):
            for x in range(x0, x1):
                if (x + 0.5 - cx) ** 2 + (y + 0.5 - cy) ** 2 <= rr:
                    self.m[y][x] = val

    def ellipse(self, cx: float, cy: float, rx: float, ry: float, val: int = 1) -> None:
        x0, x1 = max(0, int(cx - rx - 1)), min(COLS, int(cx + rx + 2))
        y0, y1 = max(0, int(cy - ry - 1)), min(ROWS, int(cy + ry + 2))
        for y in range(y0, y1):
            for x in range(x0, x1):
                if ((x + 0.5 - cx) / rx) ** 2 + ((y + 0.5 - cy) / ry) ** 2 <= 1.0:
                    self.m[y][x] = val

    def rect(self, x0: float, y0: float, x1: float, y1: float, val: int = 1) -> None:
        xa, xb = sorted((int(round(x0)), int(round(x1))))
        ya, yb = sorted((int(round(y0)), int(round(y1))))
        for y in range(max(0, ya), min(ROWS, yb + 1)):
            for x in range(max(0, xa), min(COLS, xb + 1)):
                self.m[y][x] = val

    def tri(self, a, b, c, val: int = 1) -> None:
        xs = [a[0], b[0], c[0]]
        ys = [a[1], b[1], c[1]]
        x0, x1 = max(0, int(min(xs))), min(COLS, int(max(xs)) + 1)
        y0, y1 = max(0, int(min(ys))), min(ROWS, int(max(ys)) + 1)

        def cross(p, q, r):
            return (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0])

        s = cross(a, b, c)
        if s == 0:
            return
        for y in range(y0, y1):
            for x in range(x0, x1):
                p = (x + 0.5, y + 0.5)
                w1 = cross(a, b, p)
                w2 = cross(b, c, p)
                w3 = cross(c, a, p)
                if (w1 >= 0 and w2 >= 0 and w3 >= 0) or (w1 <= 0 and w2 <= 0 and w3 <= 0):
                    self.m[y][x] = val

    def thick(self, x0, y0, x1, y1, r: float = 1.1) -> None:
        n = max(1, int(math.hypot(x1 - x0, y1 - y0) * 2))
        for i in range(n + 1):
            t = i / n
            self.disk(x0 + (x1 - x0) * t, y0 + (y1 - y0) * t, r)


def cell_color(x: int, y: int) -> tuple[int, int, int]:
    t = y / (ROWS - 1)
    f = t * (len(RAMP) - 1)
    i = min(int(f), len(RAMP) - 2)
    frac = f - i
    thresh = (BAYER[y % 8][x % 8] + 0.5) / 64.0
    return RAMP[i + 1] if frac > thresh else RAMP[i]


def render(grid: Grid) -> Image.Image:
    img = Image.new("RGBA", (CANVAS, CANVAS), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    for y in range(ROWS):
        for x in range(COLS):
            if not grid.m[y][x]:
                continue
            px = PAD + x * PITCH + GAP // 2
            py = PAD + y * PITCH + GAP // 2
            rgb = cell_color(x, y)
            d.rectangle([px, py, px + FILL - 1, py + FILL - 1], fill=rgb + (255,))
    return img


# --- animals ---------------------------------------------------------------


def cat(frame: int) -> Grid:
    """Side-view run cycle facing right — readable silhouette, 6-frame loop."""
    g = Grid()
    # Phases: 0 contact, 1 pass, 2 extend, 3 flight, 4 land, 5 gather
    bob = [0, 1, -1, -3, -1, 1][frame]
    stretch = [0.2, -0.4, 0.8, 1.4, 0.6, -0.2][frame]
    bx, by = 15.0, 16.5 + bob

    # Streaming tail (behind / left)
    tip = [1, -1, -3, -4, -2, 0][frame]
    g.thick(bx - 6.5, by + 0.5, bx - 10.5, by - 1 + tip * 0.35, 1.35)
    g.thick(bx - 10.5, by - 1 + tip * 0.35, bx - 13.5, by - 5 + tip, 1.15)
    g.disk(bx - 13.5, by - 5 + tip, 1.4)

    # Body + neck
    g.ellipse(bx, by, 6.6 + stretch * 0.45, 4.4 - abs(stretch) * 0.15)
    g.ellipse(bx + 5.0 + stretch * 0.25, by - 0.8, 3.6, 3.2)

    # Head + pointed ears
    hx = bx + 8.6 + stretch * 0.3
    hy = by - 3.6 + bob * 0.2
    g.ellipse(hx, hy, 4.0, 3.6)
    g.tri((hx - 0.5, hy - 2.2), (hx - 2.8, hy - 7.0), (hx + 1.2, hy - 2.8))
    g.tri((hx + 1.2, hy - 2.0), (hx + 3.8, hy - 6.6), (hx + 3.0, hy - 1.2))
    g.disk(hx + 1.2, hy - 0.3, 1.2, 0)
    g.on(hx + 1.2, hy - 0.3)
    g.on(hx + 3.2, hy + 0.5)

    # Two primary legs (near) + thinner far pair for depth — keep gaps so silhouette reads
    # foot: (hip_dx, foot_x, foot_y, thickness)
    def leg(hip_dx, fx, fy, thick=1.05):
        hipx, hipy = bx + hip_dx, by + 2.8
        mx = hipx * 0.45 + fx * 0.55
        my = hipy * 0.55 + fy * 0.45
        g.thick(hipx, hipy, mx, my, thick)
        g.thick(mx, my, fx, fy, thick * 0.92)
        g.disk(fx, fy, 1.35 if thick > 0.9 else 1.05)

    # Near rear / near front (thick), far rear / far front (thin, phase-shifted)
    near_rear = [
        (-3.0, bx - 6.0, 29),
        (-2.6, bx - 1.5, 28),
        (-2.2, bx + 2.5, 26),
        (-2.0, bx + 5.0, 23),
        (-2.5, bx + 0.5, 25),
        (-2.8, bx - 4.0, 28),
    ][frame]
    near_front = [
        (3.8, bx + 7.0, 28),
        (3.4, bx + 9.5, 25),
        (3.0, bx + 7.5, 23),
        (2.8, bx + 3.0, 25),
        (3.2, bx + 0.5, 28),
        (3.6, bx + 4.0, 29),
    ][frame]
    far_rear = [
        (-4.0, bx - 8.0, 28),
        (-3.6, bx - 4.5, 29),
        (-3.2, bx - 0.5, 28),
        (-2.8, bx + 2.5, 24),
        (-3.4, bx - 2.5, 26),
        (-3.8, bx - 6.5, 27),
    ][frame]
    far_front = [
        (4.6, bx + 9.0, 29),
        (4.2, bx + 10.5, 27),
        (3.8, bx + 8.5, 24),
        (3.4, bx + 5.0, 23),
        (3.8, bx + 2.0, 27),
        (4.2, bx + 5.5, 29),
    ][frame]

    # Draw far legs first so near legs overlap cleanly
    leg(*far_rear, 0.75)
    leg(*far_front, 0.75)
    leg(*near_rear, 1.15)
    leg(*near_front, 1.15)

    # Motion whiskers on extend / flight
    if frame in (2, 3, 4):
        g.on(hx + 4.2, hy)
        g.on(hx + 4.8, hy + 1)
    return g


def octopus(frame: int) -> Grid:
    g = Grid()
    g.ellipse(16, 10, 8.6, 7.2)
    g.disk(12, 9.5, 1.7, 0)
    g.disk(20, 9.5, 1.7, 0)
    g.on(12, 9.5)
    g.on(20, 9.5)
    phase = frame * math.tau / FRAMES
    # Six distinct arms; wave travels left → right.
    arms = [8.0, 11.0, 13.5, 18.5, 21.0, 24.0]
    for i, ox in enumerate(arms):
        spread = -1.2 if i < 3 else 1.2
        for t in range(16):
            wave = math.sin(t * 0.42 + phase + i * 0.85) * (1.4 + t * 0.16)
            x = ox + spread * t * 0.18 + wave
            y = 16.5 + t * 0.9
            g.disk(x, y, 1.15 if t < 12 else 0.95)
    return g


def otter(frame: int) -> Grid:
    g = Grid()
    # Upright sit, pebble-clap, tail flicks.
    bob = [0, -1, -1, 0, 1, 0][frame]
    g.ellipse(16, 10 + bob, 6.4, 5.6)
    g.disk(12, 5 + bob, 1.6)
    g.disk(20, 5 + bob, 1.6)
    g.ellipse(10, 12 + bob, 4.8, 2.6)
    g.disk(13, 9 + bob, 1.35, 0)
    g.on(13, 9 + bob)
    g.ellipse(16, 21 + bob, 7.6, 8.2)
    tw = [0, 1, 2, 1, 0, -1][frame]
    g.thick(22, 22 + bob, 27, 17 + bob + tw, 1.8)
    g.thick(27, 17 + bob + tw, 29, 23 + bob, 1.6)
    g.thick(29, 23 + bob, 26, 26 + bob, 1.4)
    clap = [0, 1, 3, 1, 0, 0][frame]
    g.disk(11 + clap, 17 + bob, 2.15)
    g.disk(21 - clap, 17 + bob, 2.15)
    g.rect(11, 28 + bob, 14, 31 + bob)
    g.rect(18, 28 + bob, 21, 31 + bob)
    return g


def owl(frame: int) -> Grid:
    g = Grid()
    turn = [0, -2, -3, 0, 2, 3][frame]
    blink = frame in (1, 4)
    hx = 16 + turn
    g.ellipse(hx, 13, 9.2, 7.4)
    g.tri((hx - 8, 9), (hx - 7, 1), (hx - 2, 8))
    g.tri((hx + 8, 9), (hx + 7, 1), (hx + 2, 8))
    g.ellipse(16, 22, 8.6, 8.0)
    g.tri((12, 29), (16, 31), (20, 29))
    if blink:
        g.rect(hx - 6, 12, hx - 2, 13, 0)
        g.rect(hx + 2, 12, hx + 6, 13, 0)
    else:
        g.disk(hx - 4, 12.5, 2.7, 0)
        g.disk(hx + 4, 12.5, 2.7, 0)
        g.on(hx - 4, 12.5)
        g.on(hx + 4, 12.5)
    g.tri((hx - 1, 15), (hx + 1, 15), (hx, 18))
    return g


def chameleon(frame: int) -> Grid:
    g = Grid()
    sway = [0, 1, 1, 0, -1, -1][frame]
    # Casque + turret eye + spiral tail + tongue flick.
    g.ellipse(10 + sway, 7, 4.2, 3.6)
    g.ellipse(9 + sway, 14, 6.2, 5.2)
    g.ellipse(17 + sway, 17, 8.4, 6.0)
    g.disk(7 + sway, 13, 1.8, 0)
    g.on(7 + sway, 13)
    g.rect(11 + sway, 22, 14 + sway, 28)
    g.rect(19 + sway, 22, 22 + sway, 28)
    g.disk(12 + sway, 28, 1.5)
    g.disk(20 + sway, 28, 1.5)
    g.thick(24 + sway, 16, 28 + sway, 12, 1.7)
    g.thick(28 + sway, 12, 30 + sway, 17, 1.55)
    g.thick(30 + sway, 17, 27 + sway, 22, 1.45)
    g.thick(27 + sway, 22, 24 + sway, 19, 1.3)
    g.disk(25 + sway, 18, 1.2)
    if frame == 2:
        g.thick(3 + sway, 15, 0, 14, 0.95)
    elif frame == 3:
        g.thick(3 + sway, 15, 0, 13, 0.95)
        g.disk(0, 13, 1.35)
    return g


ANIMALS = {
    "cat": cat,
    "octopus": octopus,
    "otter": otter,
    "owl": owl,
    "chameleon": chameleon,
}

# Photo-real cat run cycle lives as hand-authored cat-0..7.png — do not clobber.
SKIP_RENDER = {"cat"}


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    renderable = [(n, fn) for n, fn in ANIMALS.items() if n not in SKIP_RENDER]
    sheet_w = CANVAS * FRAMES
    sheet_h = CANVAS * len(renderable)
    sheet = Image.new("RGBA", (sheet_w, sheet_h), (12, 16, 28, 255))
    for ay, (name, fn) in enumerate(renderable):
        for i in range(FRAMES):
            im = render(fn(i))
            path = OUT / f"{name}-{i}.png"
            im.save(path, "PNG")
            if i == 0:
                im.save(OUT / f"{name}.png", "PNG")
            sheet.paste(im, (i * CANVAS, ay * CANVAS), im)
    preview = OUT.parent / "scripts" / "mascot-sheet.png"
    sheet.save(preview, "PNG")
    print(f"wrote {len(renderable) * FRAMES} frames + sheet {preview} (skipped {sorted(SKIP_RENDER)})")


if __name__ == "__main__":
    main()
