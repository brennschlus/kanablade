#!/usr/bin/env python3
"""Renders the app icon in every format the three platforms want.

    python3 packaging/icons/render.py      # needs Pillow

The icon is the app itself in miniature: 禅 lit gold with bloom on the night
background, inside a faint brush ring. Output is committed, so CI never runs this.
"""
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageFilter, ImageFont

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
FONT = ROOT / "assets/fonts/ZenOldMincho-Regular.ttf"

BG_TOP = (22, 24, 31)
BG_BOTTOM = (9, 10, 13)
GOLD = (255, 214, 156)
GLOW = (255, 160, 70)


def render(px: int) -> Image.Image:
    """One icon at `px`, drawn at 4x and downsampled. Small sizes drop detail."""
    s = px * 4
    small = px <= 32

    # Apple's grid: the tile is 824/1024 of the canvas, the rest is shadow room.
    # Tiny sizes use the whole canvas, there is no room to waste.
    inset = 0.0 if small else s * 100 / 1024
    radius = (s - 2 * inset) * 0.225
    box = (inset, inset, s - inset, s - inset)

    mask = Image.new("L", (s, s), 0)
    ImageDraw.Draw(mask).rounded_rectangle(box, radius, fill=255)

    grad = Image.linear_gradient("L").resize((s, s))
    bg = Image.composite(Image.new("RGB", (s, s), BG_BOTTOM), Image.new("RGB", (s, s), BG_TOP), grad)
    tile = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    tile.paste(bg, mask=mask)

    cx = cy = s / 2
    body = s - 2 * inset

    # Brush ring, a little open at the top right like an ensō.
    if not small:
        ring = Image.new("L", (s, s), 0)
        r = body * 0.36
        w = max(2, int(body * 0.018))
        ImageDraw.Draw(ring).arc((cx - r, cy - r, cx + r, cy + r), start=-50, end=275, fill=110, width=w)
        tile.paste(Image.new("RGBA", (s, s), GOLD + (255,)), mask=ring)

    # The glyph: a blurred copy for bloom, then the sharp one on top.
    glyph = Image.new("L", (s, s), 0)
    size = body * (0.78 if small else 0.50)
    font = ImageFont.truetype(str(FONT), int(size))
    ImageDraw.Draw(glyph).text((cx, cy), "禅", font=font, anchor="mm", fill=255)

    bloom = glyph.filter(ImageFilter.GaussianBlur(body * 0.045)).point(lambda v: int(v * 0.9))
    bloom = ImageChops.multiply(bloom, mask)
    tile = Image.alpha_composite(tile, solid(s, GLOW, bloom))
    tile = Image.alpha_composite(tile, solid(s, GOLD, glyph))

    return tile.resize((px, px), Image.LANCZOS)


def solid(s, rgb, alpha):
    layer = Image.new("RGBA", (s, s), rgb + (0,))
    layer.putalpha(alpha)
    return layer


def main():
    sizes = [16, 24, 32, 48, 64, 128, 256, 512, 1024]
    icons = {px: render(px) for px in sizes}

    # Linux: hicolor theme sizes. The 256 one is also embedded for the X11 window icon.
    for px in [16, 24, 32, 48, 64, 128, 256, 512]:
        out = HERE / "hicolor" / f"{px}x{px}" / "apps"
        out.mkdir(parents=True, exist_ok=True)
        icons[px].save(out / "io.github.brennschlus.kanablade.png", optimize=True)

    # Windows: one .ico with every size Explorer and the taskbar ask for.
    ico = [16, 24, 32, 48, 64, 128, 256]
    icons[256].save(HERE / "kanablade.ico", sizes=[(p, p) for p in ico], append_images=[icons[p] for p in ico])

    # macOS: .icns straight from Pillow, every slot from its own rendering.
    icons[1024].save(HERE / "kanablade.icns", append_images=[icons[p] for p in [16, 32, 64, 128, 256, 512]])

    # Preview for the README.
    icons[1024].save(HERE / "icon-1024.png", optimize=True)


if __name__ == "__main__":
    main()
