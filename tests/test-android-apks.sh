#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
ANDROID_DIR="$REPO_ROOT/packages/lacraia-android"
DIST_DIR="$REPO_ROOT/dist"

echo "================================================================="
echo "🧪 [TESTE] Validação dos APKs Android e Padronização in.twolp"
echo "================================================================="

FAILURES=0

# 1. Verificar se existem resíduos de in._2lp ou in.2lp no código Android
echo -n "-> 1. Verificando ausência de in._2lp / in.2lp no módulo Android... "
LEGACY_COUNT=$(grep -rnE "(in\._2lp|in\.2lp)" "$ANDROID_DIR/app/src" "$ANDROID_DIR/simbiotyka/src" 2>/dev/null | wc -l || true)
if [ "$LEGACY_COUNT" -eq 0 ]; then
    echo "✅ OK (0 ocorrências legadas)"
else
    echo "❌ FALHOU ($LEGACY_COUNT ocorrências de in._2lp/in.2lp encontradas)"
    FAILURES=$((FAILURES + 1))
fi

# 2. Verificar namespaces nos build.gradle.kts
echo -n "-> 2. Verificando namespaces nos build.gradle.kts... "
APP_NS=$(grep "namespace = \"in.twolp.lacraia.app\"" "$ANDROID_DIR/app/build.gradle.kts" || true)
SIM_NS=$(grep "namespace = \"in.twolp.lacraia.simbiotyka\"" "$ANDROID_DIR/simbiotyka/build.gradle.kts" || true)
if [ -n "$APP_NS" ] && [ -n "$SIM_NS" ]; then
    echo "✅ OK (namespaces padronizados com in.twolp.*)"
else
    echo "❌ FALHOU (namespaces divergentes em build.gradle.kts)"
    FAILURES=$((FAILURES + 1))
fi

# 3. Verificar existência de dist/lacraia-app.apk e dist/simbiotyka.apk com tamanho > 1MB
echo -n "-> 3. Verificando APKs em dist/ (> 1MB)... "
if [ -f "$DIST_DIR/lacraia-app.apk" ] && [ -f "$DIST_DIR/simbiotyka.apk" ]; then
    SZ1=$(stat -c%s "$DIST_DIR/lacraia-app.apk")
    SZ2=$(stat -c%s "$DIST_DIR/simbiotyka.apk")
    if [ "$SZ1" -gt 1048576 ] && [ "$SZ2" -gt 1048576 ]; then
        echo "✅ OK (APKs presentes e com tamanho real)"
    else
        echo "❌ FALHOU (tamanho insuficiente dos APKs)"
        FAILURES=$((FAILURES + 1))
    fi
else
    echo "❌ FALHOU (arquivos ausentes em dist/)"
    FAILURES=$((FAILURES + 1))
fi

# 4. Verificar assinatura criptográfica via apksigner
echo -n "-> 4. Verificando assinatura oficial dos APKs... "
if [ -f "$DIST_DIR/lacraia-app.apk" ] && [ -f "$DIST_DIR/simbiotyka.apk" ]; then
    if /usr/bin/apksigner verify "$DIST_DIR/lacraia-app.apk" >/dev/null 2>&1 && \
       /usr/bin/apksigner verify "$DIST_DIR/simbiotyka.apk" >/dev/null 2>&1; then
        echo "✅ OK (ambos os APKs verificados com sucesso)"
    else
        echo "❌ FALHOU (assinatura inválida)"
        FAILURES=$((FAILURES + 1))
    fi
else
    echo "❌ FALHOU (APKs não encontrados para verificação)"
    FAILURES=$((FAILURES + 1))
fi

# 5. Verificar recursos mipmap em todas as densidades para app e simbiotyka
echo -n "-> 5. Verificando estrutura de mipmaps (mdpi, hdpi, xhdpi, xxhdpi, xxxhdpi)... "
MIPMAP_FAIL=0
for mod in "app" "simbiotyka"; do
    for density in "mdpi" "hdpi" "xhdpi" "xxhdpi" "xxxhdpi"; do
        for icon in "ic_launcher.png" "ic_launcher_round.png"; do
            target="$ANDROID_DIR/$mod/src/main/res/mipmap-$density/$icon"
            if [ ! -f "$target" ] || [ ! -s "$target" ]; then
                MIPMAP_FAIL=1
                break 3
            fi
        done
    done
done

if [ "$MIPMAP_FAIL" -eq 0 ]; then
    echo "✅ OK (todos os mipmaps presentes e não vazios)"
else
    echo "❌ FALHOU (mipmaps ausentes em app ou simbiotyka)"
    FAILURES=$((FAILURES + 1))
fi

# 6. Verificar que AndroidManifest.xml referencia @mipmap/ic_launcher e não o genérico
echo -n "-> 6. Verificando AndroidManifest.xml dos módulos app e simbiotyka... "
MANIFEST_FAIL=0
for mod in "app" "simbiotyka"; do
    manifest="$ANDROID_DIR/$mod/src/main/AndroidManifest.xml"
    if grep -q "@android:drawable/sym_def_app_icon" "$manifest"; then
        MANIFEST_FAIL=1
    fi
    if ! grep -q 'android:icon="@mipmap/ic_launcher"' "$manifest"; then
        MANIFEST_FAIL=1
    fi
    if ! grep -q 'android:roundIcon="@mipmap/ic_launcher_round"' "$manifest"; then
        MANIFEST_FAIL=1
    fi
done

if [ "$MANIFEST_FAIL" -eq 0 ]; then
    echo "✅ OK (manifestos utilizam @mipmap/ic_launcher e @mipmap/ic_launcher_round)"
else
    echo "❌ FALHOU (manifestos ainda contêm ícones genéricos ou faltam declarações @mipmap)"
    FAILURES=$((FAILURES + 1))
fi

# 7. Verificar que os APKs compilados contêm os recursos de ícone dedicados
echo -n "-> 7. Verificando presença de ícones nos APKs compilados... "
APK_ICONS_FAIL=0
AAPT_BIN="/usr/lib/android-sdk/build-tools/34.0.0/aapt"
if [ ! -x "$AAPT_BIN" ]; then
    AAPT_BIN=$(command -v aapt || true)
fi

if [ -f "$DIST_DIR/lacraia-app.apk" ] && [ -f "$DIST_DIR/simbiotyka.apk" ]; then
    if [ -n "$AAPT_BIN" ] && [ -x "$AAPT_BIN" ]; then
        APP_ICONS=$("$AAPT_BIN" dump badging "$DIST_DIR/lacraia-app.apk" | grep -c "application-icon-" || true)
        SIM_ICONS=$("$AAPT_BIN" dump badging "$DIST_DIR/simbiotyka.apk" | grep -c "application-icon-" || true)
        if [ "$APP_ICONS" -eq 0 ] || [ "$SIM_ICONS" -eq 0 ]; then
            APK_ICONS_FAIL=1
        fi
    else
        APP_PNGS=$(unzip -l "$DIST_DIR/lacraia-app.apk" | grep -cE "res/.*\.png" || true)
        SIM_PNGS=$(unzip -l "$DIST_DIR/simbiotyka.apk" | grep -cE "res/.*\.png" || true)
        if [ "$APP_PNGS" -eq 0 ] || [ "$SIM_PNGS" -eq 0 ]; then
            APK_ICONS_FAIL=1
        fi
    fi
else
    APK_ICONS_FAIL=1
fi

if [ "$APK_ICONS_FAIL" -eq 0 ]; then
    echo "✅ OK (recursos de ícone embutidos nos binários APK)"
else
    echo "❌ FALHOU (recursos de ícone ausentes no pacote APK)"
    FAILURES=$((FAILURES + 1))
fi

# 8. Verificar integridade e especificações visuais de alta definição dos ícones (Ticket #44)
echo -n "-> 8. Verificando especificações visuais de alta definição e 8-bit RGBA (Ticket #44)... "
PYTHON_ICON_TEST=$(python3 -c '
import sys
from PIL import Image
from pathlib import Path

repo_root = Path("'"$REPO_ROOT"'")
android_dir = repo_root / "packages" / "lacraia-android"

densities = {
    "mdpi": (48, 48),
    "hdpi": (72, 72),
    "xhdpi": (96, 96),
    "xxhdpi": (144, 144),
    "xxxhdpi": (192, 192)
}

errors = []

for mod in ["app", "simbiotyka"]:
    for dens, size in densities.items():
        for icon_name in ["ic_launcher.png", "ic_launcher_round.png"]:
            path = android_dir / mod / "src" / "main" / "res" / f"mipmap-{dens}" / icon_name
            if not path.exists():
                errors.append(f"Missing: {path}")
                continue
            with Image.open(path) as im:
                if im.size != size:
                    errors.append(f"Bad size for {path}: expected {size}, got {im.size}")
                if im.mode != "RGBA":
                    errors.append(f"Bad mode for {path}: expected RGBA, got {im.mode}")

# Verificar que Simbiótyka possui os óculos do Kamina em vermelho escarlate
sim_path = android_dir / "simbiotyka" / "src" / "main" / "res" / "mipmap-xxhdpi" / "ic_launcher.png"
if sim_path.exists():
    with Image.open(sim_path) as im:
        pixels = list(im.getdata())
        # Procurar pixels vermelhos escarlate do Kamina (R > 220, G < 50, B < 60)
        has_kamina_red = any(r > 220 and g < 50 and b < 60 and a > 200 for (r, g, b, a) in pixels)
        if not has_kamina_red:
            errors.append("Simbiotyka icon does not contain Kamina red lenses!")

# Verificar que Lacraia app não é o mock minúsculo
app_path = android_dir / "app" / "src" / "main" / "res" / "mipmap-xxhdpi" / "ic_launcher.png"
if app_path.exists():
    with Image.open(app_path) as im:
        pixels = list(im.getdata())
        # Verificar presença de amarelo veneno elétrico (#d4ff00)
        has_venom = any(r > 190 and g > 230 and b < 40 and a > 200 for (r, g, b, a) in pixels)
        # Verificar presença de fúcsia (#ff0055)
        has_fucsia = any(r > 230 and g < 30 and b > 70 and a > 200 for (r, g, b, a) in pixels)
        if not (has_venom and has_fucsia):
            errors.append("Lacraia app icon missing venom yellow or fuchsia chitin elements!")

if errors:
    print("; ".join(errors[:3]))
    sys.exit(1)
else:
    print("OK")
    sys.exit(0)
' || true)

if [ "$PYTHON_ICON_TEST" = "OK" ]; then
    echo "✅ OK (resoluções, 8-bit RGBA e paletas do Kamina e Lacraia conformes)"
else
    echo "❌ FALHOU ($PYTHON_ICON_TEST)"
    FAILURES=$((FAILURES + 1))
fi

# 10. Verificar automação de publicação no R2 e dispatch da Banca no CI
echo -n "-> 10. Verificando R2 (twolp-bucket) e dispatch da Banca no CI... "
CI_WORKFLOW="$REPO_ROOT/.forgejo/workflows/ci.yml"
if [ -f "$CI_WORKFLOW" ]; then
    if grep -q "twolp-bucket" "$CI_WORKFLOW" && \
       (grep -q "https://gambi.2lp.in/api/v1/repos/lumen/banca/actions/workflows/ci.yml/dispatches" "$CI_WORKFLOW" || grep -q "https://codeberg.org/api/v1/repos/lumenpink/banca/actions/workflows/ci.yml/dispatches" "$CI_WORKFLOW"); then
        echo "✅ OK (R2 e dispatch configurados no CI)"
    else
        echo "❌ FALHOU (R2 twolp-bucket ou dispatch da banca ausentes no CI)"
        FAILURES=$((FAILURES + 1))
    fi
else
    echo "❌ FALHOU (.forgejo/workflows/ci.yml ausente)"
    FAILURES=$((FAILURES + 1))
fi

# 11. Verificar versionamento canônico dinâmico nos build.gradle.kts (Ticket #61)
echo -n "-> 11. Verificando versionamento canônico dinâmico do VERSION em app e simbiotyka (Ticket #61)... "
GRADLE_VERSION_FAIL=0
for mod in "app" "simbiotyka"; do
    gradle_file="$ANDROID_DIR/$mod/build.gradle.kts"
    if grep -q 'versionName = "0.2.0"' "$gradle_file"; then
        GRADLE_VERSION_FAIL=1
        break
    fi
    if ! grep -q 'VERSION' "$gradle_file"; then
        GRADLE_VERSION_FAIL=1
        break
    fi
done

if [ "$GRADLE_VERSION_FAIL" -eq 0 ]; then
    echo "✅ OK (versionName dinâmico extraído de VERSION nos módulos Android)"
else
    echo "❌ FALHOU (versionName estático '0.2.0' ainda presente ou ausência de leitura de VERSION)"
    FAILURES=$((FAILURES + 1))
fi

# 12. Verificar unificação do binário lacraia.apk em dist/ e CI (Ticket #61)
echo -n "-> 12. Verificando unificação do binário oficial dist/lacraia.apk (Ticket #61)... "
if [ -f "$DIST_DIR/lacraia.apk" ] && [ -f "$DIST_DIR/simbiotyka.apk" ]; then
    SZ_LAC=$(stat -c%s "$DIST_DIR/lacraia.apk")
    if [ "$SZ_LAC" -gt 1048576 ]; then
        echo "✅ OK (lacraia.apk e simbiotyka.apk presentes com tamanho válido)"
    else
        echo "❌ FALHOU (tamanho insuficiente de dist/lacraia.apk)"
        FAILURES=$((FAILURES + 1))
    fi
else
    echo "❌ FALHOU (dist/lacraia.apk ausente)"
    FAILURES=$((FAILURES + 1))
fi

# 13. Verificar job deploy-apk no CI com gost-ci-android e gradlew assembleRelease (Ticket #61)
echo -n "-> 13. Verificando job deploy-apk no CI com gost-ci-android (Ticket #61)... "
if [ -f "$CI_WORKFLOW" ]; then
    if grep -q "deploy-apk" "$CI_WORKFLOW" && \
       grep -q "gost-ci-android" "$CI_WORKFLOW" && \
       grep -q "gradlew assembleRelease" "$CI_WORKFLOW" && \
       grep -q "twolp-bucket/lacraia.apk" "$CI_WORKFLOW" && \
       grep -q "twolp-bucket/simbiotyka.apk" "$CI_WORKFLOW"; then
        echo "✅ OK (job deploy-apk configurado no CI com compilação e publicação R2)"
    else
        echo "❌ FALHOU (job deploy-apk ausente ou incompleto no CI)"
        FAILURES=$((FAILURES + 1))
    fi
else
    echo "❌ FALHOU (.forgejo/workflows/ci.yml ausente)"
    FAILURES=$((FAILURES + 1))
fi

echo "================================================================="
if [ $FAILURES -eq 0 ]; then
    echo "🎉 TODOS OS TESTES ANDROID PASSARAM!"
    exit 0
else
    echo "💥 $FAILURES TESTE(S) FALHARAM."
    exit 1
fi

