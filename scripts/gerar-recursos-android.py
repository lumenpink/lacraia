#!/usr/bin/env python3
"""
Gera os recursos visuais de ícones Android (mipmap) para os módulos
in.twolp.lacraia.app e in.twolp.lacraia.simbiotyka.
"""
import sys
from pathlib import Path
from PIL import Image, ImageDraw

REPO_ROOT = Path(__file__).resolve().parent.parent
BANCA_ICONS = REPO_ROOT.parent / "banca" / "site" / "pratilera" / "icons"
ANDROID_DIR = REPO_ROOT / "packages" / "lacraia-android"

DENSITIES = {
    "mdpi": 48,
    "hdpi": 72,
    "xhdpi": 96,
    "xxhdpi": 144,
    "xxxhdpi": 192,
}

MODULES = {
    "app": {
        "source": BANCA_ICONS / "in.twolp.lacraia.app.png",
        "label": "La Craia",
        "description": "Cockpit Brutalista de Som &amp; Jukebox",
    },
    "simbiotyka": {
        "source": BANCA_ICONS / "in.twolp.lacraia.simbiotyka.png",
        "label": "Simbiótyka",
        "description": "HUD de Letras Sincronizadas &amp; Cifras",
    },
}

def create_circular_icon(img: Image.Image) -> Image.Image:
    """Cria uma variante com recorte circular anti-aliasing."""
    size = img.size
    # Cria máscara 4x maior para anti-aliasing de alta qualidade
    scale = 4
    mask_size = (size[0] * scale, size[1] * scale)
    mask = Image.new("L", mask_size, 0)
    draw = ImageDraw.Draw(mask)
    draw.ellipse((0, 0, mask_size[0], mask_size[1]), fill=255)
    mask = mask.resize(size, Image.Resampling.LANCZOS)
    
    # Aplica máscara circular no canal alpha
    output = img.copy()
    if output.mode != "RGBA":
        output = output.convert("RGBA")
    output.putalpha(mask)
    return output

def generate_resources():
    for mod_name, config in MODULES.items():
        src_path = config["source"]
        if not src_path.exists():
            print(f"❌ Imagem fonte não encontrada: {src_path}", file=sys.stderr)
            sys.exit(1)
        
        base_img = Image.open(src_path).convert("RGBA")
        res_dir = ANDROID_DIR / mod_name / "src" / "main" / "res"
        
        # Gerar mipmaps
        for density, size in DENSITIES.items():
            density_dir = res_dir / f"mipmap-{density}"
            density_dir.mkdir(parents=True, exist_ok=True)
            
            # 1. Ícone padrão
            resized = base_img.resize((size, size), Image.Resampling.LANCZOS)
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
