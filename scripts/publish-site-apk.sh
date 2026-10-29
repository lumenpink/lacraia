#!/usr/bin/env bash
# ==============================================================================
# La Craia — Script de Publicação de APKs no Site (lacraia.2lp.in)
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SITE_DIR="$REPO_ROOT/site"

echo "================================================================="
echo "La Craia — Publicação Oficial de APKs & Manifesto Obtainium"
echo "================================================================="

cd "$REPO_ROOT"

VERSION="$(head -n 1 "$REPO_ROOT/VERSION" | tr -d ' \r\n')"
GIT_HASH="$(git rev-parse --short HEAD 2>/dev/null || echo 'main')"
PUBLISHED_AT="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

echo "-> Versão:       $VERSION"
echo "-> Commit:       $GIT_HASH"
echo "-> Publicado em: $PUBLISHED_AT"

# 1. Compilar APKs via Gradle
echo "-> Compilando APKs de produção..."
cd "$REPO_ROOT/packages/lacraia-android"
if command -v gradle >/dev/null 2>&1; then
    gradle assembleRelease || gradle assembleDebug
elif [ -f "./gradlew" ]; then
    ./gradlew assembleRelease || ./gradlew assembleDebug
else
    echo "AVISO: Gradle não encontrado no PATH, buscando APKs já gerados em dist/..."
fi

mkdir -p "$REPO_ROOT/dist"

# Copiar lacraia.apk
if [ -f "$REPO_ROOT/packages/lacraia-android/app/build/outputs/apk/release/app-release.apk" ]; then
    cp "$REPO_ROOT/packages/lacraia-android/app/build/outputs/apk/release/app-release.apk" "$REPO_ROOT/dist/lacraia.apk"
    cp "$REPO_ROOT/dist/lacraia.apk" "$REPO_ROOT/dist/lacraia-app.apk"
elif [ -f "$REPO_ROOT/packages/lacraia-android/app/build/outputs/apk/debug/app-debug.apk" ]; then
    cp "$REPO_ROOT/packages/lacraia-android/app/build/outputs/apk/debug/app-debug.apk" "$REPO_ROOT/dist/lacraia.apk"
    cp "$REPO_ROOT/dist/lacraia.apk" "$REPO_ROOT/dist/lacraia-app.apk"
fi

# Copiar simbiotyka.apk
if [ -f "$REPO_ROOT/packages/lacraia-android/simbiotyka/build/outputs/apk/release/simbiotyka-release.apk" ]; then
    cp "$REPO_ROOT/packages/lacraia-android/simbiotyka/build/outputs/apk/release/simbiotyka-release.apk" "$REPO_ROOT/dist/simbiotyka.apk"
elif [ -f "$REPO_ROOT/packages/lacraia-android/simbiotyka/build/outputs/apk/debug/simbiotyka-debug.apk" ]; then
    cp "$REPO_ROOT/packages/lacraia-android/simbiotyka/build/outputs/apk/debug/simbiotyka-debug.apk" "$REPO_ROOT/dist/simbiotyka.apk"
fi

# 2. Assinar com chave oficial se apksigner e chave in_twolp estiverem disponíveis
if [ -x "$REPO_ROOT/scripts/sign-release.sh" ] && [ -f "$HOME/.android/in_twolp_release.keystore" ]; then
    echo "-> Assinando APKs com a chave in_twolp..."
    if [ -f "$REPO_ROOT/dist/lacraia.apk" ]; then
        bash "$REPO_ROOT/scripts/sign-release.sh" "$REPO_ROOT/dist/lacraia.apk" || true
        cp "$REPO_ROOT/dist/lacraia.apk" "$REPO_ROOT/dist/lacraia-app.apk"
    fi
    if [ -f "$REPO_ROOT/dist/simbiotyka.apk" ]; then
        bash "$REPO_ROOT/scripts/sign-release.sh" "$REPO_ROOT/dist/simbiotyka.apk" || true
    fi
fi

# 3. Integrar APKs ao site/
if [ -f "$REPO_ROOT/dist/lacraia.apk" ]; then
    cp "$REPO_ROOT/dist/lacraia.apk" "$SITE_DIR/lacraia.apk"
    cp "$REPO_ROOT/dist/lacraia.apk" "$SITE_DIR/lacraia-app.apk"
    echo "✅ lacraia.apk copiado para site/"
fi
if [ -f "$REPO_ROOT/dist/simbiotyka.apk" ]; then
    cp "$REPO_ROOT/dist/simbiotyka.apk" "$SITE_DIR/simbiotyka.apk"
    echo "✅ simbiotyka.apk copiado para site/"
fi

# 4. Atualizar site/version.json
cat <<EOF > "$SITE_DIR/version.json"
{
  "version": "${VERSION}",
  "min_compatible_version": "0.1.0",
  "package_id": "in.twolp.lacraia.app",
  "apk_url": "https://lacraia.2lp.in/lacraia.apk",
  "release_notes": "Compilação automatizada e contínua do ecossistema La Craia para Android.",
  "published_at": "${PUBLISHED_AT}",
  "apps": {
    "lacraia-app": {
      "package_id": "in.twolp.lacraia.app",
      "apk_url": "https://lacraia.2lp.in/lacraia.apk",
      "name": "La Craia App (Cockpit & Jukebox)"
    },
    "simbiotyka": {
      "package_id": "in.twolp.lacraia.simbiotyka",
      "apk_url": "https://lacraia.2lp.in/simbiotyka.apk",
      "name": "Simbiótyka (HUD de Letras Sincronizadas)"
    }
  }
}
EOF

# 5. Sincronizar com A Pratilera na Banca se o repositório irmão banca existir
BANCA_DIR="$(cd "$REPO_ROOT/../banca" 2>/dev/null && pwd || true)"
if [ -n "$BANCA_DIR" ] && [ -d "$BANCA_DIR/site/pratilera" ]; then
    echo "-> Sincronizando APKs com A Pratilera na Banca ($BANCA_DIR)..."
    [ -f "$REPO_ROOT/dist/lacraia.apk" ] && cp "$REPO_ROOT/dist/lacraia.apk" "$BANCA_DIR/site/pratilera/lacraia.apk"
    [ -f "$REPO_ROOT/dist/simbiotyka.apk" ] && cp "$REPO_ROOT/dist/simbiotyka.apk" "$BANCA_DIR/site/pratilera/simbiotyka.apk"
    if [ -f "$BANCA_DIR/scripts/gerar-pratilera.py" ]; then
        python3 "$BANCA_DIR/scripts/gerar-pratilera.py" || true
    fi
fi

# 6. Publicar via Wrangler se solicitado ou configurado
if [ "${1:-}" = "--deploy" ]; then
    echo "-> Publicando no Cloudflare Workers..."
    cd "$REPO_ROOT"
    npx wrangler deploy
fi

echo ""
echo "================================================================="
echo "🎉 Publicação pronta em $SITE_DIR!"
echo "   Package ID App:   in.twolp.lacraia.app"
echo "   Package ID HUD:   in.twolp.lacraia.simbiotyka"
echo "   Manifesto:        https://lacraia.2lp.in/version.json"
echo "================================================================="
