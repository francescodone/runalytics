#!/usr/bin/env python3
"""Regenerate the Runalytics application icon (1024px master).

Design: the dashboard logo, promoted to an app icon. The sidebar mark in
app.css (#nav .brand::before) is a white rounded tile carrying an italic,
weight-800 Geist "R" in --brand-ink, sitting on the nav's 155deg brand
gradient. The icon reproduces exactly that: the brand gradient fills the
squircle, the white tile floats on it with the logo's 12/32 corner radius,
and the R is drawn from the real vendored Geist outline (src/styles/fonts/
geist-latin.woff2) instantiated at wght 800 and sheared the way a browser
synthesises `font-style: italic` for a font with no italic cut.

Because the glyph comes from the shipped font, the icon can never drift from
the logo again — regenerate it whenever the brand tokens change.

    python3 scripts/generate_app_icon.py

Needs fontTools + brotli (to read the woff2) and Pillow. Writes
docs/app-icon.png (the master). Feed that to `npx tauri icon
docs/app-icon.png --output src-tauri/icons` to produce every bundle format
(icns, ico, png set).
"""

import math
from pathlib import Path

import numpy as np
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer
from PIL import Image, ImageChops, ImageDraw, ImageFilter

ROOT = Path(__file__).resolve().parent.parent
FONT_PATH = ROOT / "src" / "styles" / "fonts" / "geist-latin.woff2"

SIZE = 1024
SS = 4  # supersample factor
CANVAS = SIZE * SS

# Brand tokens, dark scheme (app.css :root). The nav paints
# linear-gradient(155deg, --brand-start, --brand-mid 52%, --brand-end).
BRAND_START = (0x55, 0x44, 0xE9)
BRAND_MID = (0x69, 0x36, 0xE9)
BRAND_MID_AT = 0.52
BRAND_END = (0x53, 0x24, 0xCD)
GRADIENT_DEG = 155.0  # CSS angle: 0 = to top, 90 = to right

TILE_HALF = 448.0  # squircle half-width in icon units, as before
TILE_BLUR = 2.0  # softens the superellipse corners toward Apple's look

# The mark itself: --on-brand tile, --brand-ink glyph.
TILE_FILL = (255, 255, 255, 255)
GLYPH_COLOR = (0x56, 0x34, 0xD9, 255)  # --brand-ink
TILE_SIDE = 640.0  # white tile side in icon units
TILE_RADIUS_RATIO = 12.0 / 32.0  # --radius-control over the 32px mark
# The logo sets the R at --text-2xl (20px) on a 32px tile, and Geist's cap
# height is 710/1000 em, so the glyph is 14.2/32 = 0.444 of the tile.
GLYPH_FILL = (20.0 * 0.710) / 32.0

# The logo's box-shadow: 0 6px 16px brand-end at 20%. Scaled by 20 (the logo's
# 32px tile -> the 640u tile here): 120u down, and a CSS blur of 16px is a
# Gaussian sigma of 8px = 160u. Clipped to the squircle below, so the wide
# falloff cannot bleed past the tile edge into the transparent margin.
SHADOW_COLOR = (0x53, 0x24, 0xCD)
SHADOW_ALPHA = 51  # 20% of 255
SHADOW_DY = 120.0
SHADOW_BLUR = 160.0

# Geist ships no italic cut (italicAngle 0), so `font-style: italic` makes the
# browser synthesise an oblique: a shear of 0.25, i.e. atan(0.25) = 14.04deg.
# Fitted by rasterising this outline at several shears and maximising IoU
# against a real Chrome render of the logo's declarations at 200px — 14deg
# scores 0.994, 11deg only 0.928.
ITALIC_SHEAR_DEG = math.degrees(math.atan(0.25))

# The logo centres the glyph by its ADVANCE box, which leaves an italic R
# sitting ~6% right of the tile's centre. That is a side effect of centring
# inline text, not a decision, and on a standalone icon it reads as a
# misregistration — so the icon centres the ink instead.
GLYPH_NAME = "R"
QUAD_STEPS = 24  # samples per quadratic segment


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


def gradient_direction(deg):
    """Unit vector of a CSS gradient line, in screen coords (y grows down)."""
    rad = math.radians(deg)
    return math.sin(rad), -math.cos(rad)


def linear_fade(size, stops, deg):
    """Multi-stop linear gradient across the box along a CSS angle.

    A one-pixel-tall strip holds the colour ramp; an affine transform samples
    it at the projected coordinate u = dot(p, dir), which is exact and avoids
    a per-pixel Python loop at supersampled sizes.
    """
    dx, dy = gradient_direction(deg)
    # Projection range over the box corners.
    corners = [(0.0, 0.0), (size, 0.0), (0.0, size), (size, size)]
    dots = [x * dx + y * dy for x, y in corners]
    lo, hi = min(dots), max(dots)
    span = hi - lo

    strip_len = 512
    strip = Image.new("RGB", (strip_len, 1))
    px = strip.load()
    for i in range(strip_len):
        t = i / (strip_len - 1)
        # Walk the stop list (position, rgb) and lerp inside its segment.
        marks = [(0.0, stops[0][1])]
        for pos, color in stops[1:-1]:
            marks.append((pos, color))
        marks.append((1.0, stops[-1][1]))
        for (p0, c0), (p1, c1) in zip(marks, marks[1:]):
            if t <= p1 or (p0, c0) == marks[-2]:
                f = 0.0 if p1 == p0 else (t - p0) / (p1 - p0)
                f = max(0.0, min(1.0, f))
                px[i, 0] = tuple(int(a + (b - a) * f) for a, b in zip(c0, c1))
                break

    scale = strip_len - 1
    a = dx * scale / span
    b = dy * scale / span
    c = -lo * scale / span
    out = strip.transform(
        (size, size), Image.AFFINE, (a, b, c, 0, 0, 0), resample=Image.BILINEAR
    )
    return out.convert("RGBA")


def draw_tile():
    """Flat squircle carrying the brand gradient and nothing else."""
    tile = linear_fade(
        CANVAS,
        [(0.0, BRAND_START), (BRAND_MID_AT, BRAND_MID), (1.0, BRAND_END)],
        GRADIENT_DEG,
    )

    mask = Image.new("L", (CANVAS, CANVAS), 0)
    ImageDraw.Draw(mask).polygon(squircle_path(512, 512, TILE_HALF), fill=255)
    mask = mask.filter(ImageFilter.GaussianBlur(TILE_BLUR * SS))

    img = Image.new("RGBA", (CANVAS, CANVAS), (0, 0, 0, 0))
    img.paste(tile, (0, 0), mask)
    return img


def rounded_tile_mask(side, radius, cx=512.0, cy=512.0, dy=0.0):
    """Solid rounded-square mask in icon units, optionally offset."""
    mask = Image.new("L", (CANVAS, CANVAS), 0)
    half = side / 2.0
    box = [
        u(cx - half),
        u(cy - half + dy),
        u(cx + half),
        u(cy + half + dy),
    ]
    ImageDraw.Draw(mask).rounded_rectangle(box, radius=u(radius), fill=255)
    return mask


def draw_tile_shadow():
    """The logo's soft drop shadow, in brand colour rather than black.

    Multiplied against the squircle so the shadow's wide falloff stops at the
    tile edge instead of bleeding into the transparent margin, which macOS
    would otherwise bake into the icon as a dark halo.
    """
    mask = rounded_tile_mask(TILE_SIDE, TILE_SIDE * TILE_RADIUS_RATIO, dy=SHADOW_DY)
    mask = mask.filter(ImageFilter.GaussianBlur(SHADOW_BLUR * SS))
    clip = Image.new("L", (CANVAS, CANVAS), 0)
    ImageDraw.Draw(clip).polygon(squircle_path(512, 512, TILE_HALF), fill=255)
    mask = ImageChops.multiply(mask, clip)
    layer = Image.new("RGBA", (CANVAS, CANVAS), (0, 0, 0, 0))
    solid = Image.new("RGBA", (CANVAS, CANVAS), SHADOW_COLOR + (SHADOW_ALPHA,))
    layer.paste(solid, (0, 0), mask)
    return layer


def glyph_contours():
    """Outline contours of the mark's glyph, in font units, y-up.

    Reads the shipped Geist woff2, pins it to weight 800 (the logo's
    font-weight), and shears each point the way a browser synthesises italic.
    """
    font = TTFont(FONT_PATH)
    weight = 800
    if "fvar" in font:
        font = instancer.instantiateVariableFont(
            font, {"wght": weight}, inplace=False
        )
    glyph = font.getGlyphSet()[GLYPH_NAME]

    shear = math.tan(math.radians(ITALIC_SHEAR_DEG))

    contours = []
    current = []

    def emit(x, y):
        current.append((x + shear * y, y))

    def close():
        if current:
            contours.append(list(current))
            current.clear()

    def draw_pen():
        state = {"x": 0.0, "y": 0.0}

        class Pen:
            def moveTo(self, pt):
                close()
                state["x"], state["y"] = pt
                emit(*pt)

            def lineTo(self, pt):
                state["x"], state["y"] = pt
                emit(*pt)

            def curveTo(self, *points):
                # Cubic: sample with de Casteljau.
                pts = [state["x"], state["y"], *sum(points, ())]
                for i in range(1, QUAD_STEPS + 1):
                    t = i / QUAD_STEPS
                    x, y = _cubic_at(pts, t)
                    emit(x, y)
                state["x"], state["y"] = points[-1]

            def qCurveTo(self, *points):
                # TrueType-style run: all but the last are off-curve controls,
                # implicit on-curve points sit at midpoints of the controls.
                controls = list(points[:-1])
                end = points[-1]
                start = (state["x"], state["y"])
                for i, ctrl in enumerate(controls):
                    nxt = (
                        ((ctrl[0] + controls[i + 1][0]) / 2.0,
                         (ctrl[1] + controls[i + 1][1]) / 2.0)
                        if i < len(controls) - 1
                        else end
                    )
                    for k in range(1, QUAD_STEPS + 1):
                        t = k / QUAD_STEPS
                        emit(*_quad_at(start, ctrl, nxt, t))
                    start = nxt
                state["x"], state["y"] = end

            def closePath(self):
                close()

            def endPath(self):
                close()

            def addComponent(self, *args, **kwargs):
                pass

        glyph.draw(Pen())

    draw_pen()
    close()
    return contours


def _quad_at(p0, p1, p2, t):
    mt = 1.0 - t
    return (
        mt * mt * p0[0] + 2 * mt * t * p1[0] + t * t * p2[0],
        mt * mt * p0[1] + 2 * mt * t * p1[1] + t * t * p2[1],
    )


def _cubic_at(v, t):
    mt = 1.0 - t
    x = (
        mt**3 * v[0]
        + 3 * mt * mt * t * v[2]
        + 3 * mt * t * t * v[4]
        + t**3 * v[6]
    )
    y = (
        mt**3 * v[1]
        + 3 * mt * mt * t * v[3]
        + 3 * mt * t * t * v[5]
        + t**3 * v[7]
    )
    return x, y


def winding_fill(polygons):
    """Nonzero-winding scanline fill of closed polygons (supersampled pixels).

    Hand-rolled because PIL's polygon fill mis-renders the R's outer contour
    where it doubles back at the bowl/leg junction. Nonzero winding rather than
    even-odd for the same reason fonts use it: the outer contour and its
    counter are oppositely wound, so the counter cancels to zero, and the
    near-horizontal run at the junction only changes the count without ever
    dropping it to zero — which is what punches the counter out cleanly.
    """
    edges = []
    for poly in polygons:
        n = len(poly)
        for i in range(n):
            x0, y0 = poly[i]
            x1, y1 = poly[(i + 1) % n]
            if y0 != y1:
                edges.append((x0, y0, x1, y1))
    ex = np.array(edges, dtype=float).T
    x0e, y0e, x1e, y1e = ex
    e_min = np.minimum(y0e, y1e)
    e_max = np.maximum(y0e, y1e)
    slope = (x1e - x0e) / (y1e - y0e)
    # +1 for a downward-crossing edge, -1 for upward, in screen coords.
    sign = np.where(y1e > y0e, 1.0, -1.0)

    mask = np.zeros((CANVAS, CANVAS), dtype=np.uint8)
    for row in range(CANVAS):
        y = row + 0.5
        sel = (y >= e_min) & (y < e_max)
        if not sel.any():
            continue
        xs = x0e[sel] + (y - y0e[sel]) * slope[sel]
        order = np.argsort(xs, kind="stable")
        xs, signs = xs[order], sign[sel][order]

        winding = 0
        run_start = None
        for i in range(len(xs)):
            winding += int(signs[i])
            if winding != 0 and run_start is None:
                run_start = xs[i]
            elif winding == 0 and run_start is not None:
                lo = max(int(math.ceil(run_start - 0.5)), 0)
                hi = min(int(math.floor(xs[i] - 0.5)), CANVAS - 1)
                if hi >= lo:
                    mask[row, lo : hi + 1] = 255
                run_start = None
    return Image.fromarray(mask)


def draw_glyph():
    """The italic weight-800 R, centred on the white tile."""
    contours = glyph_contours()
    xs = [p[0] for c in contours for p in c]
    ys = [p[1] for c in contours for p in c]
    x0, x1, y0, y1 = min(xs), max(xs), min(ys), max(ys)

    target_h = GLYPH_FILL * TILE_SIDE
    scale = target_h / (y1 - y0)
    gx, gy = 512.0, 512.0
    cx0, cy0 = (x0 + x1) / 2.0, (y0 + y1) / 2.0

    polygons = [
        [(u(gx + (x - cx0) * scale), u(gy - (y - cy0) * scale)) for x, y in c]
        for c in contours
    ]
    mask = winding_fill(polygons)

    layer = Image.new("RGBA", (CANVAS, CANVAS), (0, 0, 0, 0))
    solid = Image.new("RGBA", (CANVAS, CANVAS), GLYPH_COLOR)
    layer.paste(solid, (0, 0), mask)
    return layer


def main():
    img = draw_tile()
    img.alpha_composite(draw_tile_shadow())

    tile = Image.new("RGBA", (CANVAS, CANVAS), (0, 0, 0, 0))
    tile.paste(
        Image.new("RGBA", (CANVAS, CANVAS), TILE_FILL),
        (0, 0),
        rounded_tile_mask(TILE_SIDE, TILE_SIDE * TILE_RADIUS_RATIO),
    )
    img.alpha_composite(tile)
    img.alpha_composite(draw_glyph())

    img = img.resize((SIZE, SIZE), Image.LANCZOS)
    out = ROOT / "docs" / "app-icon.png"
    out.parent.mkdir(exist_ok=True)
    img.save(out)
    print(f"wrote {out} ({SIZE}x{SIZE})")


if __name__ == "__main__":
    main()
