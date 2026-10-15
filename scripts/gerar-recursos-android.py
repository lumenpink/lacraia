#!/usr/bin/env python3
"""
Gera os recursos visuais oficiais de ícones Android (mipmap) e site para os módulos
in.twolp.lacraia.app e in.twolp.lacraia.simbiotyka com supersampling 2x (1024x1024 -> 512x512).
"""
import sys
from pathlib import Path
from PIL import Image, ImageDraw

REPO_ROOT = Path(__file__).resolve().parent.parent
ANDROID_DIR = REPO_ROOT / "packages" / "lacraia-android"
SITE_ASSETS = REPO_ROOT / "site" / "assets"

DENSITIES = {
    "mdpi": 48,
    "hdpi": 72,
    "xhdpi": 96,
    "xxhdpi": 144,
    "xxxhdpi": 192,
}

MODULES = {
    "app": {
        "label": "La Craia",
        "description": "Cockpit Brutalista de Som & Jukebox",
        "with_kamina": False,
    },
    "simbiotyka": {
        "label": "Simbiótyka",
        "description": "HUD de Letras Sincronizadas & Cifras",
        "with_kamina": True,
    },
}

def draw_bezier_curve(draw, p0, p1, p2, p3, color, width, steps=100):
    points = []
    for i in range(steps + 1):
        t = i / steps
        u = 1 - t
        tt = t * t
        uu = u * u
        uuu = uu * u
        ttt = tt * t
        x = uuu * p0[0] + 3 * uu * t * p1[0] + 3 * u * tt * p2[0] + ttt * p3[0]
        y = uuu * p0[1] + 3 * uu * t * p1[1] + 3 * u * tt * p2[1] + ttt * p3[1]
        points.append((x, y))
    for i in range(len(points) - 1):
        draw.line([points[i], points[i + 1]], fill=color, width=width)
        draw.ellipse([points[i][0] - width/2, points[i][1] - width/2,
                      points[i][0] + width/2, points[i][1] + width/2], fill=color)

def draw_chassis(draw, size=1024):
    pad = 48
    rad = 192
    draw.rounded_rectangle([pad, pad, size - pad, size - pad], radius=rad, fill=(21, 22, 24, 255), outline=(66, 70, 78, 255), width=18)
    dash_pad = 84
    dash_rad = 160
    draw.rounded_rectangle([dash_pad, dash_pad, size - dash_pad, size - dash_pad], radius=dash_rad, outline=(43, 29, 20, 255), width=6)

def draw_anime_star_gleam(draw, cx, cy, radius=110):
    rx = radius * 1.3
    ry = radius * 0.9
    draw.polygon([
        (cx - rx, cy),
        (cx - rx * 0.1, cy - 8),
        (cx, cy - 8),
        (cx + rx, cy),
        (cx + rx * 0.1, cy + 8),
        (cx, cy + 8)
    ], fill=(255, 255, 255, 255))
    draw.polygon([
        (cx, cy - ry),
        (cx + 8, cy - ry * 0.1),
        (cx + 8, cy),
        (cx, cy + ry),
        (cx - 8, cy + ry * 0.1),
        (cx - 8, cy)
    ], fill=(255, 255, 255, 255))
    d_len = radius * 0.4
    draw.polygon([(cx - d_len, cy - d_len), (cx, cy), (cx + d_len, cy + d_len)], fill=(255, 255, 255, 220))
    draw.polygon([(cx + d_len, cy - d_len), (cx, cy), (cx - d_len, cy + d_len)], fill=(255, 255, 255, 220))
    draw.ellipse([cx - 18, cy - 18, cx + 18, cy + 18], fill=(255, 255, 255, 255))

def render_lacraia(with_kamina=False) -> Image.Image:
    size = 1024
    im = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(im)

    # 1. Base Shield
    draw_chassis(draw, size)

    # 2. Antenas articuladas com respiro
    draw_bezier_curve(draw, (448, 512), (390, 360), (280, 260), (180, 185), color=(212, 255, 0, 255), width=32)
    draw.ellipse([180 - 28, 185 - 28, 180 + 28, 185 + 28], fill=(212, 255, 0, 255))
    draw.ellipse([180 - 14, 185 - 14, 180 + 14, 185 + 14], fill=(255, 255, 255, 255))

    draw_bezier_curve(draw, (576, 512), (634, 360), (744, 260), (844, 185), color=(212, 255, 0, 255), width=32)
    draw.ellipse([844 - 28, 185 - 28, 844 + 28, 185 + 28], fill=(212, 255, 0, 255))
    draw.ellipse([844 - 14, 185 - 14, 844 + 14, 185 + 14], fill=(255, 255, 255, 255))

    # 3. Pinças peçonhentas
    draw_bezier_curve(draw, (352, 736), (288, 832), (352, 912), (432, 880), color=(255, 0, 85, 255), width=28)
    draw.ellipse([432 - 14, 880 - 14, 432 + 14, 880 + 14], fill=(255, 0, 85, 255))

    draw_bezier_curve(draw, (672, 736), (736, 832), (672, 912), (592, 880), color=(255, 0, 85, 255), width=28)
    draw.ellipse([592 - 14, 880 - 14, 592 + 14, 880 + 14], fill=(255, 0, 85, 255))

    # 4. Placa Cefálica
    head_box = [320, 448, 704, 768]
    draw.rounded_rectangle([head_box[0] + 10, head_box[1] + 10, head_box[2] + 10, head_box[3] + 10], radius=96, fill=(10, 11, 13, 200))
    draw.rounded_rectangle(head_box, radius=96, fill=(31, 33, 38, 255), outline=(255, 0, 85, 255), width=24)

    # 5. Fenestra Central
    for y in range(head_box[1] + 48, head_box[3] - 48, 36):
        draw.line([(512, y), (512, y + 20)], fill=(66, 70, 78, 255), width=16)

    # 6. Ocelos Sensoriais
    eye_y = 576
    eye_l_x = 416
    eye_r_x = 608
    eye_r = 38

    draw.ellipse([eye_l_x - eye_r - 8, eye_y - eye_r - 8, eye_l_x + eye_r + 8, eye_y + eye_r + 8], fill=(212, 255, 0, 120))
    draw.ellipse([eye_l_x - eye_r, eye_y - eye_r, eye_l_x + eye_r, eye_y + eye_r], fill=(212, 255, 0, 255))
    draw.ellipse([eye_l_x - 14, eye_y - 14, eye_l_x + 14, eye_y + 14], fill=(255, 255, 255, 255))

    draw.ellipse([eye_r_x - eye_r - 8, eye_y - eye_r - 8, eye_r_x + eye_r + 8, eye_y + eye_r + 8], fill=(212, 255, 0, 120))
    draw.ellipse([eye_r_x - eye_r, eye_y - eye_r, eye_r_x + eye_r, eye_y + eye_r], fill=(212, 255, 0, 255))
    draw.ellipse([eye_r_x - 14, eye_y - 14, eye_r_x + 14, eye_y + 14], fill=(255, 255, 255, 255))

    # 7. Óculos do Kamina
    if with_kamina:
        left_lens = [(512, 530), (160, 430), (320, 680)]
        right_lens = [(512, 530), (864, 430), (704, 680)]

        shades_shadow = Image.new("RGBA", (size, size), (0, 0, 0, 0))
        d_sh = ImageDraw.Draw(shades_shadow)
        d_sh.polygon([(p[0], p[1] + 14) for p in left_lens], fill=(0, 0, 0, 200))
        d_sh.polygon([(p[0], p[1] + 14) for p in right_lens], fill=(0, 0, 0, 200))
        im = Image.alpha_composite(im, shades_shadow)

        shades_layer = Image.new("RGBA", (size, size), (0, 0, 0, 0))
        d_sl = ImageDraw.Draw(shades_layer)

        d_sl.polygon(left_lens, fill=(245, 15, 38, 255))
        d_sl.polygon(right_lens, fill=(245, 15, 38, 255))

        draw_anime_star_gleam(d_sl, cx=740, cy=475, radius=110)

        im = Image.alpha_composite(im, shades_layer)

    return im.resize((512, 512), resample=Image.Resampling.LANCZOS)

def create_circular_icon(img: Image.Image) -> Image.Image:
    """Cria uma variante com recorte circular anti-aliasing."""
    size = img.size
    scale = 4
    mask_size = (size[0] * scale, size[1] * scale)
    mask = Image.new("L", mask_size, 0)
    draw = ImageDraw.Draw(mask)
    draw.ellipse((0, 0, mask_size[0], mask_size[1]), fill=255)
    mask = mask.resize(size, Image.Resampling.LANCZOS)
    
    output = img.copy()
    if output.mode != "RGBA":
        output = output.convert("RGBA")
    output.putalpha(mask)
    return output

def generate_resources():
    SITE_ASSETS.mkdir(parents=True, exist_ok=True)
    
    for mod_name, config in MODULES.items():
        base_512 = render_lacraia(with_kamina=config["with_kamina"])
        
        # Salvar no site/assets
        if mod_name == "app":
            base_512.save(SITE_ASSETS / "icon.png", format="PNG", optimize=True)
        else:
            base_512.save(SITE_ASSETS / f"{mod_name}.png", format="PNG", optimize=True)
        
        res_dir = ANDROID_DIR / mod_name / "src" / "main" / "res"
        
        # Gerar mipmaps
        for density, size in DENSITIES.items():
            density_dir = res_dir / f"mipmap-{density}"
            density_dir.mkdir(parents=True, exist_ok=True)
            
            # 1. Ícone padrão
            resized = base_512.resize((size, size), Image.Resampling.LANCZOS)
            resized.save(density_dir / "ic_launcher.png", format="PNG", optimize=True)
            
            # 2. Ícone circular
            round_icon = create_circular_icon(resized)
            round_icon.save(density_dir / "ic_launcher_round.png", format="PNG", optimize=True)
            
            print(f"[{mod_name}] Gerado mipmap-{density} ({size}x{size})")
        
        # Gerar values/strings.xml
        values_dir = res_dir / "values"
        values_dir.mkdir(parents=True, exist_ok=True)
        strings_xml = values_dir / "strings.xml"
        strings_xml.write_text(f"""<?xml version="1.0" encoding="utf-8"?>
<resources>
    <string name="app_name">{config["label"]}</string>
    <string name="app_description">{config["description"]}</string>
</resources>
""", encoding="utf-8")
        print(f"[{mod_name}] Criado {strings_xml}")

if __name__ == "__main__":
    generate_resources()
    print("✅ Todos os recursos Android gerados com sucesso!")
