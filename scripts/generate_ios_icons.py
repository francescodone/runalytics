#!/usr/bin/env python3
"""Regenerate the iOS app icons from the master so the phone matches macOS.

The iPhone shows whatever sits in
src-tauri/gen/apple/Assets.xcassets/AppIcon.appiconset/, and `tauri icon`
never writes there — it only fills src-tauri/icons/. The appiconset is a
separate, git-tracked copy that Xcode reads directly, so it goes stale
silently (it kept showing the old cyan/yellow logo long after the brand
went purple). This script rebuilds it from docs/app-icon.png, the same
master `npx tauri icon` feeds the macOS icns, which is what makes the two
platforms agree.

Two iOS-specific traps handled here:

  * iOS masks the WHOLE canvas into a squircle, unlike macOS, which composites
    the art as-is. The master keeps a transparent margin around the squircle
    art, so a naive resize would show transparent (i.e. white-flattened)
    gutters inside the rounded shape. Cropping to the art's alpha bbox and
    scaling it to fill the canvas fixes that: the squircle then overflows the
    canvas slightly and iOS's mask trims it back — the standard full-bleed
    treatment.
  * App Store icons must not carry alpha. Every output is composited onto
    white and saved opaque.

    python3 scripts/generate_ios_icons.py

Needs Pillow. Reads docs/app-icon.png and the sizes named in the appiconset's
Contents.json, so adding a slot there is enough for this script to fill it.
"""

import io
import json
from pathlib import Path

from PIL import Image, ImageChops

ROOT = Path(__file__).resolve().parent.parent
MASTER = ROOT / "docs" / "app-icon.png"
ICONSET = ROOT / "src-tauri" / "gen" / "apple" / "Assets.xcassets" / "AppIcon.appiconset"
# `tauri icon` writes the same filenames here; mirror the set so the two
# trees cannot drift to different logos again.
MIRROR = ROOT / "src-tauri" / "icons" / "ios"


def full_bleed_master():
    """The master cropped to its artwork and scaled to fill a 1024 canvas."""
    img = Image.open(MASTER).convert("RGBA")
    bbox = ImageChops.invert(img.getchannel("A")).getbbox()
    art = img.crop(bbox)
    side = max(art.size)
    if art.size != (side, side):  # pad the shorter axis to stay square
        padded = Image.new("RGBA", (side, side), (0, 0, 0, 0))
        off = ((side - art.width) // 2, (side - art.height) // 2)
        padded.paste(art, off)
        art = padded
    art = art.resize((1024, 1024), Image.LANCZOS)
    opaque = Image.new("RGBA", (1024, 1024), (255, 255, 255, 255))
    opaque.alpha_composite(art)
    return opaque.convert("RGB")


def target_sizes():
    """(filename, pixel side) for every slot in Contents.json."""
    manifest = json.loads((ICONSET / "Contents.json").read_text())
    slots = []
    for entry in manifest["images"]:
        w = float(entry["size"].split("x")[0])
        mult = float(entry["scale"].rstrip("x"))
        side = int(round(w * mult))
        slots.append((entry["filename"], side))
    return slots


def write_png(image, path):
    """Save by replacing the file outright.

    Overwriting an existing PNG in place can hit EPERM on macOS when another
    process (Spotlight, a sync agent, Xcode) holds the file open; unlinking
    first sidesteps it, and the bytes land identically.
    """
    buf = io.BytesIO()
    image.save(buf, format="PNG")
    if path.exists():
        path.unlink()
    path.write_bytes(buf.getvalue())


def main():
    if not MASTER.exists():
        raise SystemExit(f"{MASTER} missing - run scripts/generate_app_icon.py first")
    master = full_bleed_master()
    slots = target_sizes()
    for directory in (ICONSET, MIRROR):
        directory.mkdir(parents=True, exist_ok=True)
        for filename, side in slots:
            write_png(master.resize((side, side), Image.LANCZOS), directory / filename)
    print(
        f"wrote {len(slots)} icons to {ICONSET.relative_to(ROOT)} "
        f"(mirrored into {MIRROR.relative_to(ROOT)})"
    )


if __name__ == "__main__":
    main()
