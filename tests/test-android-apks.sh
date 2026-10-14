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

echo "================================================================="
if [ $FAILURES -eq 0 ]; then
    echo "🎉 TODOS OS TESTES ANDROID PASSARAM!"
    exit 0
else
    echo "💥 $FAILURES TESTE(S) FALHARAM."
    exit 1
fi
