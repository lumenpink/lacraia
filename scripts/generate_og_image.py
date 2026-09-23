#!/usr/bin/env python3
"""
Generate the Open Graph / Twitter Card banner (1200x630 pixels) for La Craia.

Layout follows the same safe-zone strategy used in kokorosim/scripts/generate_og_image.py:
  - Safe zone centered at 4:3 ratio to survive WhatsApp / mobile network cropping.
  - Badge strip at top with project identity.
  - Hero brand in the upper-center area.
  - Two-line title block centered in the safe-zone body.
  - Footer bar with URL left / license+tag right, full width 1200px minus margins.
  - Top-edge accent bar in --fucsia-furacao; bottom-edge in --veneno-eletrico.

Color tokens from docs/design-system.md:
  --ferro-bate-pedra:  #151618  (deep chassis background)
  --ferro-painel:      #1f2126  (mid-tone surface)
  --fucsia-furacao:    #ff0055  (primary accent, top border, active elements)
  --veneno-eletrico:   #d4ff00  (secondary accent, legs, bottom border)
  --alta-voltagem:     #ffb700  (subtitle / warning)
  --papel-mimeografo:  #e8e6df  (body text)
  --grafite-mono:      #8a909a  (muted / secondary text)
  --metal-cantoneira:  #42464e  (borders, grid lines)
"""

import os
from PIL import Image, ImageDraw, ImageFont, ImageFilter


# ---------------------------------------------------------------------------
# Color palette (from docs/design-system.md)
# ---------------------------------------------------------------------------
C_BG_DEEP    = (21, 22, 24, 255)    # --ferro-bate-pedra
C_BG_MID     = (31, 33, 38, 255)    # --ferro-painel
C_GRID       = (42, 46, 50, 80)     # --metal-cantoneira (dim for grid)
C_FUCSIA     = (255, 0, 85, 255)    # --fucsia-furacao
C_VENENO     = (212, 255, 0, 255)   # --veneno-eletrico
C_VOLTAGEM   = (255, 183, 0, 255)   # --alta-voltagem
C_PAPEL      = (232, 230, 223, 255) # --papel-mimeografo
C_GRAFITE    = (138, 144, 154, 220) # --grafite-mono
C_MAGENTA    = (216, 0, 100, 255)   # --cetim-magenta (shadow)
C_GLITTER    = (128, 0, 255, 120)   # --glitter-roxo (glow fill)

FONT_BOLD   = "/usr/share/fonts/truetype/firacode/FiraCode-Bold.ttf"
FONT_MEDIUM = "/usr/share/fonts/truetype/firacode/FiraCode-Medium.ttf"
FONT_MONO   = "/usr/share/fonts/truetype/liberation/LiberationMono-Bold.ttf"

W, H = 1200, 630
MARGIN = 72  # Horizontal margin for safe zone


def load_font(path: str, size: int, fallback_path: str = FONT_MONO) -> ImageFont.FreeTypeFont:
    try:
        return ImageFont.truetype(path, size)
    except Exception:
        try:
            return ImageFont.truetype(fallback_path, size)
        except Exception:
            return ImageFont.load_default()


def draw_centered_text(draw: ImageDraw.ImageDraw, y: int, text: str,
                        font: ImageFont.FreeTypeFont, fill: tuple,
                        shadow: tuple | None = None, shadow_offset: int = 3) -> None:
    """Draw text horizontally centered on the 1200px canvas."""
    bbox = draw.textbbox((0, 0), text, font=font)
    tw = bbox[2] - bbox[0]
    x = (W - tw) // 2
    if shadow:
        draw.text((x + shadow_offset, y + shadow_offset), text, font=font, fill=shadow)
    draw.text((x, y), text, font=font, fill=fill)


def create_og_image(output_path: str = "site/assets/og_preview.png") -> None:
    # ------------------------------------------------------------------
    # 1. BASE CANVAS: deep chassis background + diagonal grid texture
    # ------------------------------------------------------------------
    base = Image.new("RGBA", (W, H), C_BG_DEEP)
    draw = ImageDraw.Draw(base)

    # Diagonal hatching (like the CSS fita-isolante / bate-pedra texture)
    step = 28
    for i in range(-(H), W + H, step):
        draw.line([(i, 0), (i + H, H)], fill=C_GRID, width=1)

    # Vertical fine grid
    for x in range(0, W, 48):
        draw.line([(x, 0), (x, H)], fill=(42, 46, 50, 40), width=1)

    # ------------------------------------------------------------------
    # 2. AMBIENT GLOW: fúcsia + veneno sobre o chassi escuro
    # ------------------------------------------------------------------
    glow = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    gd = ImageDraw.Draw(glow)
    gd.ellipse([300, 80, 900, 480], fill=(255, 0, 85, 38))     # fucsia center
    gd.ellipse([500, 200, 820, 440], fill=(128, 0, 255, 28))   # glitter inner
    glow = glow.filter(ImageFilter.GaussianBlur(90))
    base = Image.alpha_composite(base, glow)

    # Veneno panel glow (lower-right)
    glow2 = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    gd2 = ImageDraw.Draw(glow2)
    gd2.ellipse([750, 300, 1200, 700], fill=(212, 255, 0, 22))
    glow2 = glow2.filter(ImageFilter.GaussianBlur(70))
    base = Image.alpha_composite(base, glow2)

    draw = ImageDraw.Draw(base)

    # ------------------------------------------------------------------
    # 3. FONTS
    # ------------------------------------------------------------------
    f_badge   = load_font(FONT_BOLD,   15)   # badge strip uppercase label
    f_brand   = load_font(FONT_BOLD,   90)   # "LA CRAIA" hero
    f_title   = load_font(FONT_BOLD,   34)   # primary title line
    f_sub     = load_font(FONT_MEDIUM, 26)   # subtitle / tagline
    f_url     = load_font(FONT_BOLD,   20)   # footer URL
    f_meta    = load_font(FONT_MEDIUM, 18)   # footer right credits

    # ------------------------------------------------------------------
    # 4. TOP ACCENT BAR (fucsia) + BADGE STRIP
    # ------------------------------------------------------------------
    draw.line([(0, 0), (W, 0)], fill=C_FUCSIA, width=4)
    draw.line([(0, 3), (W, 3)], fill=C_MAGENTA, width=2)

    # Badge panel
    badge_text = "  SUÍTE AUTOMOTIVA DISSIDENTE  //  GPLv3+  //  RUST + WASM  "
    b_bbox = draw.textbbox((0, 0), badge_text, font=f_badge)
    bw = b_bbox[2] - b_bbox[0]
    bh = b_bbox[3] - b_bbox[1]
    bx = (W - bw) // 2
    by = 22
    draw.rectangle([bx - 16, by - 7, bx + bw + 16, by + bh + 7],
                    fill=(10, 10, 12, 220), outline=C_VENENO, width=1)
    draw.text((bx, by), badge_text, font=f_badge, fill=C_VENENO)

    # ------------------------------------------------------------------
    # 5. HERO BRAND: "LA CRAIA" with fucsia drop shadow + veneno accent
    # ------------------------------------------------------------------
    brand = "LA CRAIA"
    br_bbox = draw.textbbox((0, 0), brand, font=f_brand)
    brw = br_bbox[2] - br_bbox[0]
    bry = 95

    # Shadow layers (offset stacking for brutalist depth)
    for ox, oy, alpha in [(5, 5, 180), (3, 3, 120), (1, 1, 70)]:
        shadow_color = (216, 0, 100, alpha)
        draw.text(((W - brw) // 2 + ox, bry + oy), brand, font=f_brand, fill=shadow_color)

    # Main brand text in white
    draw.text(((W - brw) // 2, bry), brand, font=f_brand, fill=(255, 255, 255, 255))

    # Decorative underline bar beneath brand (fita-isolante zebra strip)
    bar_y = bry + (br_bbox[3] - br_bbox[1]) + 10
    stripe_w = 20
    for sx in range(MARGIN, W - MARGIN, stripe_w * 2):
        draw.rectangle([sx, bar_y, sx + stripe_w, bar_y + 6],
                        fill=C_VENENO)
        draw.rectangle([sx + stripe_w, bar_y, sx + stripe_w * 2, bar_y + 6],
                        fill=(0, 0, 0, 255))

    # ------------------------------------------------------------------
    # 6. TITLE BLOCK (safe zone body): two lines centered
    # ------------------------------------------------------------------
    t1 = "Streaming por Grafo de Afinidade Acústica"
    t2 = "HUD Simbiótyka de Letras Sincronizadas · Paredão Autônomo"

    t1_y = bar_y + 28
    draw_centered_text(draw, t1_y, t1, f_title, fill=C_PAPEL,
                        shadow=(20, 0, 40, 160), shadow_offset=2)

    t2_y = t1_y + 52
    draw_centered_text(draw, t2_y, t2, f_sub, fill=C_VOLTAGEM)

    # ------------------------------------------------------------------
    # 7. METAMERY INDICATOR: 4 segment tags as visual decoration
    # ------------------------------------------------------------------
    tags = ["lacraia-shared", "lacraia-core", "lacraia-app", "Simbiótyka"]
    tag_total_w = sum(
        draw.textbbox((0, 0), f"[ {t} ]", font=f_badge)[2] for t in tags
    ) + (len(tags) - 1) * 20
    tx = (W - tag_total_w) // 2
    ty = t2_y + 56

    for t in tags:
        label = f"[ {t} ]"
        lbox = draw.textbbox((0, 0), label, font=f_badge)
        lw = lbox[2] - lbox[0]
        lh = lbox[3] - lbox[1]
        draw.rectangle([tx - 6, ty - 4, tx + lw + 6, ty + lh + 4],
                        fill=(31, 33, 38, 220), outline=(66, 70, 78, 200), width=1)
        draw.text((tx, ty), label, font=f_badge, fill=C_GRAFITE)
        tx += lw + 20

    # ------------------------------------------------------------------
    # 8. FOOTER BAR: separator + URL left / credits right
    # ------------------------------------------------------------------
    foot_y = H - 78
    draw.line([(MARGIN, foot_y - 14), (W - MARGIN, foot_y - 14)],
               fill=(66, 70, 78, 180), width=1)

    # LED indicator (veneno circle)
    led_x, led_y = MARGIN, foot_y + 4
    draw.ellipse([led_x, led_y, led_x + 14, led_y + 14], fill=C_VENENO)
    draw.ellipse([led_x + 2, led_y + 2, led_x + 12, led_y + 12],
                  fill=(180, 220, 0, 255))

    # URL text
    url_text = "lacraia.2lp.in"
    draw.text((led_x + 24, foot_y), url_text, font=f_url, fill=C_VENENO)

    # Credits right-aligned
    cred_text = "Software Livre  ·  Feito com ódio & glitter"
    cred_bbox = draw.textbbox((0, 0), cred_text, font=f_meta)
    draw.text((W - MARGIN - (cred_bbox[2] - cred_bbox[0]), foot_y + 2),
               cred_text, font=f_meta, fill=C_GRAFITE)

    # ------------------------------------------------------------------
    # 9. BOTTOM ACCENT BAR (veneno-eletrico)
    # ------------------------------------------------------------------
    draw.line([(0, H - 3), (W, H - 3)], fill=C_VENENO, width=3)
    draw.line([(0, H - 1), (W, H - 1)], fill=C_FUCSIA, width=1)

    # ------------------------------------------------------------------
    # 10. SAVE
    # ------------------------------------------------------------------
    os.makedirs(os.path.dirname(output_path), exist_ok=True)
    final = base.convert("RGB")
    final.save(output_path, format="PNG", optimize=True)
    kb = os.path.getsize(output_path) / 1024
    print(f"✅ OG banner generated: {output_path} ({W}x{H}, {kb:.1f} KB)")


if __name__ == "__main__":
    create_og_image()
