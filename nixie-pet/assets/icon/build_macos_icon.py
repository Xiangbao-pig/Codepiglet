#!/usr/bin/env python3
"""从源 PNG 生成 macOS AppIcon.iconset（需 Pillow；再运行 iconutil 得 AppIcon.icns）。"""
from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

from PIL import Image


def load_trim_pig(src: Path) -> Image.Image:
    im = Image.open(src).convert("RGBA")
    px = im.load()
    w, h = im.size
    for y in range(h):
        for x in range(w):
            r, g, b, a = px[x, y]
            # 白底 → 透明；浅灰阴影（与背景接近）渐隐
            if r >= 248 and g >= 248 and b >= 248:
                px[x, y] = (0, 0, 0, 0)
            elif r >= 230 and g >= 230 and b >= 230 and max(r, g, b) - min(r, g, b) < 12:
                # 软阴影：按亮度降 alpha
                lum = (r + g + b) / 3
                alpha = max(0, int(255 * (248 - lum) / 20))
                px[x, y] = (r, g, b, alpha)
    bbox = im.getbbox()
    if not bbox:
        raise SystemExit("empty image after keying")
    im = im.crop(bbox)
    pad = 2
    pw, ph = im.size
    padded = Image.new("RGBA", (pw + pad * 2, ph + pad * 2), (0, 0, 0, 0))
    padded.paste(im, (pad, pad))
    return padded


def center_on_square(im: Image.Image, side: int) -> Image.Image:
    """像素画：最近邻放大，居中置于透明方图。"""
    pw, ph = im.size
    margin = int(side * 0.08)
    max_inner = side - 2 * margin
    scale = min(max_inner / pw, max_inner / ph)
    nw, nh = max(1, int(round(pw * scale))), max(1, int(round(ph * scale)))
    scaled = im.resize((nw, nh), Image.Resampling.NEAREST)
    canvas = Image.new("RGBA", (side, side), (0, 0, 0, 0))
    canvas.paste(scaled, ((side - nw) // 2, (side - nh) // 2))
    return canvas


def write_iconset(master: Image.Image, iconset: Path) -> None:
    iconset.mkdir(parents=True, exist_ok=True)
    # iconutil 要求的命名与尺寸
    spec = [
        ("icon_16x16.png", 16),
        ("icon_16x16@2x.png", 32),
        ("icon_32x32.png", 32),
        ("icon_32x32@2x.png", 64),
        ("icon_128x128.png", 128),
        ("icon_128x128@2x.png", 256),
        ("icon_256x256.png", 256),
        ("icon_256x256@2x.png", 512),
        ("icon_512x512.png", 512),
        ("icon_512x512@2x.png", 1024),
    ]
    for name, dim in spec:
        out = master.resize((dim, dim), Image.Resampling.LANCZOS)
        out.save(iconset / name, format="PNG")


def main() -> None:
    root = Path(__file__).resolve().parent
    src = root / "nixie-pig-source.png"
    if not src.exists():
        print("缺少 nixie-pig-source.png，请将小猪原图放到 assets/icon/", file=sys.stderr)
        sys.exit(1)
    pig = load_trim_pig(src)
    master = center_on_square(pig, 1024)
    iconset = root / "AppIcon.iconset"
    write_iconset(master, iconset)
    icns = root / "AppIcon.icns"
    subprocess.run(
        ["iconutil", "-c", "icns", str(iconset), "-o", str(icns)],
        check=True,
    )
    shutil.rmtree(iconset, ignore_errors=True)
    print(f"Wrote {icns}")


if __name__ == "__main__":
    main()
