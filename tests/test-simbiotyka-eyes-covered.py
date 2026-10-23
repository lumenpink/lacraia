#!/usr/bin/env python3
"""
Validação automatizada de cobertura total dos olhos pelos óculos Kamina
no ícone do Simbiótyka (Ticket #51).
"""
import sys
from pathlib import Path
from PIL import Image

REPO_ROOT = Path(__file__).resolve().parent.parent
SIMBIOTYKA_IMG = REPO_ROOT / "site" / "assets" / "simbiotyka.png"

if not SIMBIOTYKA_IMG.exists():
    print(f"ERRO: {SIMBIOTYKA_IMG} não encontrado.")
    sys.exit(1)

im = Image.open(SIMBIOTYKA_IMG)
assert im.size == (512, 512), f"Tamanho incorreto: {im.size}"

# Verificar olho esquerdo (x=208 +/- 10) e olho direito (x=304 +/- 10)
# A lente deve cobrir integralmente a região vertical dos olhos (y=265 a y=318)
uncovered_left = []
uncovered_right = []

for x in range(198, 218):
    for y in range(265, 318):
        p = im.getpixel((x, y))
        # Se contiver tom esverdeado/amarelado do ocelo (G > 30 e R < 200)
        if p[1] > 30 and p[0] < 200:
            uncovered_left.append((x, y, p))

for x in range(294, 314):
    for y in range(265, 318):
        p = im.getpixel((x, y))
        if p[1] > 30 and p[0] < 200:
            uncovered_right.append((x, y, p))

total_uncovered = len(uncovered_left) + len(uncovered_right)
if total_uncovered > 0:
    print(f"❌ FALHA: Ocelos sensoriais descobertos sob os óculos! ({total_uncovered} pixels com vazamento)")
    sys.exit(1)

print("✅ SUCESSO: Os óculos Kamina cobrem 100% dos olhos da mascote no ícone do Simbiótyka.")
sys.exit(0)
