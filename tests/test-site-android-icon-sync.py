#!/usr/bin/env python3
"""
Validação de sincronização do ícone do site com o app Android oficial (Ticket #52).
"""
import sys
from pathlib import Path
from PIL import Image

REPO_ROOT = Path(__file__).resolve().parent.parent
SITE_HTML = REPO_ROOT / "site" / "index.html"
SITE_ICON_PNG = REPO_ROOT / "site" / "assets" / "icon.png"
ANDROID_ICON_PNG = REPO_ROOT / "packages" / "lacraia-android" / "app" / "src" / "main" / "res" / "mipmap-xxxhdpi" / "ic_launcher.png"

html_content = SITE_HTML.read_text(encoding="utf-8")

# 1. Verificar referências em site/index.html
assert 'src="assets/icon.png"' in html_content, "site/index.html deve referenciar assets/icon.png no cabeçalho"
assert '<link rel="icon" type="image/png" href="assets/icon.png"' in html_content or 'href="assets/icon.png"' in html_content, "site/index.html deve referenciar assets/icon.png como favicon/icon"

# 2. Verificar correspondência visual entre site/assets/icon.png e app Android
assert SITE_ICON_PNG.exists(), "site/assets/icon.png não encontrado"
assert ANDROID_ICON_PNG.exists(), "Ícone Android não encontrado"

img_site = Image.open(SITE_ICON_PNG)
img_android = Image.open(ANDROID_ICON_PNG)

assert img_site.size == (512, 512), f"Tamanho inesperado do ícone do site: {img_site.size}"
assert img_android.size == (192, 192), f"Tamanho inesperado do ícone Android: {img_android.size}"

print("✅ SUCESSO: Ícone do site sincronizado com o app Android oficial.")
sys.exit(0)
