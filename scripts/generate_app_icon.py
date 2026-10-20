#!/usr/bin/env python3
"""Regenerate the Runalytics application icon (1024px master).

Design: flat and 2026-oriented, in the ClipStack idiom. One squircle tile
carrying a single soft diagonal fade through emerald, and a lone readiness
arc — the app's score ring promoted to a solid glyph whose tail fades from
full white to barely-there. No sheen, no rim light, no drop shadow, no
sparkline, no glow — the depth comes from the fades alone. No text.

    python3 scripts/generate_app_icon.py

Writes docs/app-icon.png (the master). Feed that to `npx tauri icon
docs/app-icon.png --output src-tauri/icons` to produce every bundle format
(icns, ico, png set).
"""

import math
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

SIZE = 1024
SS = 4  # supersample factor
CANVAS = SIZE * SS

# Tile: one flat diagonal fade through emerald. The top-left keeps enough
# light to read as green rather than mud, and the bottom-right goes near-black
# so the glyph carries the contrast it has on the dashboard.
TILE_A = (16, 148, 104, 255)  # top-left, lit emerald
TILE_B = (4, 38, 28, 255)  # bottom-right, near-black forest

# The glyph: a readiness arc, open at the bottom like the score rings.
# Drawn as segments whose alpha ramps from a barely-there tail to absolute
# white at the head — the fade is the whole idea.
ARC_CX, ARC_CY = 512.0, 512.0
ARC_R = 250.0
ARC_W = 96.0
ARC_START = 120.0  # degrees, clockwise from 3 o'clock (bottom-left)
ARC_SWEEP = 300.0  # leaves the sporty gap at the bottom-left
SEGMENTS = 240
TAIL_ALPHA = 60  # alpha at the tail end of the arc
HEAD_ALPHA = 255  # alpha at the head (the "current readiness" point)


def u(x):
    """Icon-grid units (0..1024) to supersampled pixels."""
    return x * SS


def squircle_path(cx, cy, half, steps=64):
    """Continuous-curve squircle as a polygon (superellipse, n=5)."""
    n = 5.0
    pts = []
    for i in range(steps * 4):
        t = 2 * math.pi * i / (steps * 4)
        c, s = math.cos(t), math.sin(t)
        x = cx + half * math.copysign(abs(c) ** (2 / n), c)
        y = cy + half * math.copysign(abs(s) ** (2 / n), s)
        pts.append((u(x), u(y)))
    return pts


def diagonal_fade(size, a, b):
    """Linear gradient running corner to corner, top-left to bottom-right.

    A one-pixel-tall strip holding the ramp is sampled at u = x + y with an
    affine transform, which is exact and avoids a per-pixel Python loop at
    supersampled sizes.
    """
    strip = Image.new("RGB", (2 * size, 1))
    px = strip.load()
    span = 2 * size - 1
    for x in range(2 * size):
        t = x / span
        px[x, 0] = tuple(int(i + (j - i) * t) for i, j in zip(a[:3], b[:3]))
    out = strip.transform(
        (size, size), Image.AFFINE, (1, 1, 0, 0, 0, 0), resample=Image.BILINEAR
    )
    return out.convert("RGBA")


def draw_tile():
    """Flat squircle carrying one diagonal fade and nothing else."""
    tile = diagonal_fade(CANVAS, TILE_A, TILE_B)

    mask = Image.new("L", (CANVAS, CANVAS), 0)
    ImageDraw.Draw(mask).polygon(squircle_path(512, 512, 448), fill=255)
    # Soften the superellipse corners toward the Apple continuous look.
    mask = mask.filter(ImageFilter.GaussianBlur(2 * SS))

    img = Image.new("RGBA", (CANVAS, CANVAS), (0, 0, 0, 0))
    img.paste(tile, (0, 0), mask)
    return img


def draw_arc():
    """The readiness arc: a thick stroke whose alpha fades tail -> head.

    Drawn as short overlapping line segments with round caps, each at its
    own alpha. Overdraw at the seams is invisible because neighbouring
    segments share nearly the same alpha.
    """
    layer = Image.new("RGBA", (CANVAS, CANVAS), (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)
    cx, cy, r, w = u(ARC_CX), u(ARC_CY), u(ARC_R), u(ARC_W)
    step = ARC_SWEEP / SEGMENTS
    for i in range(SEGMENTS):
        t0 = i / SEGMENTS
        t1 = (i + 1) / SEGMENTS
        alpha = round(TAIL_ALPHA + (HEAD_ALPHA - TAIL_ALPHA) * ((t0 + t1) / 2))
        a0 = math.radians(ARC_START + step * i)
        a1 = math.radians(ARC_START + step * (i + 1))
        # Thick line segment along the circle, round caps at both ends.
        x0, y0 = cx + r * math.cos(a0), cy + r * math.sin(a0)
        x1, y1 = cx + r * math.cos(a1), cy + r * math.sin(a1)
        d.line([(x0, y0), (x1, y1)], fill=(255, 255, 255, alpha), width=int(w))
        rr = w / 2
        for x, y in ((x0, y0), (x1, y1)):
            d.ellipse([x - rr, y - rr, x + rr, y + rr], fill=(255, 255, 255, alpha))
    return layer


def main():
    img = draw_tile()
    img.alpha_composite(draw_arc())

    img = img.resize((SIZE, SIZE), Image.LANCZOS)
    out = Path(__file__).resolve().parent.parent / "docs" / "app-icon.png"
    out.parent.mkdir(exist_ok=True)
    img.save(out)
    print(f"wrote {out} ({SIZE}x{SIZE})")


if __name__ == "__main__":
    main()
