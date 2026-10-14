#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
INDEX_HTML="$REPO_ROOT/site/app/index.html"

echo "================================================================="
echo "🧪 [TESTE] Validação de Isolamento de Toque e Aba do Player (#36)"
echo "================================================================="

FAILURES=0

# 1. Verificar arquivo site/app/index.html
echo -n "-> 1. Verificando presença de site/app/index.html... "
if [ -f "$INDEX_HTML" ]; then
    echo "✅ OK"
else
    echo "❌ FALHOU"
    exit 1
fi

# 2. Verificar regras de ancoragem do #deck-01 para mobile (overscroll-behavior e overflow-x)
echo -n "-> 2. Verificando contenção e ancoragem do #deck-01... "
if grep -q "overscroll-behavior: contain" "$INDEX_HTML" && \
   grep -q "overflow-x: hidden" "$INDEX_HTML"; then
    echo "✅ OK (deck-01 ancorado com overscroll contain e overflow-x hidden)"
else
    echo "❌ FALHOU (ausência de overscroll-behavior ou overflow-x contain no player)"
    FAILURES=$((FAILURES + 1))
fi

# 3. Verificar touch-action: manipulation e user-select nos botões de controle
echo -n "-> 3. Verificando blindagem de toque nos controles de reprodução... "
if grep -q "touch-action: manipulation" "$INDEX_HTML" && \
   grep -q "user-select: none" "$INDEX_HTML" && \
   grep -q -- "-webkit-tap-highlight-color: transparent" "$INDEX_HTML"; then
    echo "✅ OK (touch-action manipulation e user-select configurados)"
else
    echo "❌ FALHOU (controles suscetíveis a seleção acidental ou delay de toque)"
    FAILURES=$((FAILURES + 1))
fi

# 4. Verificar isolamento da seek bar e gain slider (touch-action dedicado)
echo -n "-> 4. Verificando touch-action na barra de progresso e slider de ganho... "
if grep -A 10 "\.progress-track" "$INDEX_HTML" | grep -q "touch-action: none" && \
   grep -A 15 "\.gain-slider" "$INDEX_HTML" | grep -q "touch-action: pan-x"; then
    echo "✅ OK (seek bar e gain slider com touch-action dedicado)"
else
    echo "❌ FALHOU (touch-action ausente em .progress-track ou .gain-slider)"
    FAILURES=$((FAILURES + 1))
fi

# 5. Verificar que botões de transporte têm altura mínima confortável no mobile (>= 48px)
echo -n "-> 5. Verificando ergonomia e target size no mobile (>= 48px)... "
if grep -q "min-height: 48px" "$INDEX_HTML"; then
    echo "✅ OK (hit targets ergonomicos para dedos no mobile)"
else
    echo "❌ FALHOU (ausência de min-height: 48px nos controles móveis)"
    FAILURES=$((FAILURES + 1))
fi

# 6. Verificar ausência de listeners globais de swipe em deck-01
echo -n "-> 6. Verificando ausência de listeners de swipe conflitantes no player... "
if ! grep -q "deck-01.*swipe" "$INDEX_HTML"; then
    echo "✅ OK (nenhum listener de swipe forçado sobre o player)"
else
    echo "❌ FALHOU (encontrado listener de swipe no player)"
    FAILURES=$((FAILURES + 1))
fi

echo "================================================================="
if [ $FAILURES -eq 0 ]; then
    echo "🎉 TODOS OS TESTES DE TOQUE DO PLAYER PASSARAM!"
    exit 0
else
    echo "💥 $FAILURES TESTE(S) FALHARAM."
    exit 1
fi
