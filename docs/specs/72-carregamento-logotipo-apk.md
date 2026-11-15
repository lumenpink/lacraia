# Especificação Técnica - Ticket #72: Corrigir Carregamento do Logotipo no APK

## 1. Contexto e Problema
Na versão compilada para Android (APK), o logotipo da aplicação localizado no cabeçalho superior esquerdo (`.brand-section`) não era renderizado.
A causa raiz do problema foi o uso de caminho com barra inicial absoluta (`src="/assets/icon.svg"`). Na WebView do Android (`file:///android_asset/...`), URLs com barra absoluta resolvem para a raiz do dispositivo (`file:///assets/icon.svg`), onde os arquivos não existem, resultando em ícone quebrado.

## 2. Árvore de Decisões (Contexto para Natasha)
Para que Natasha entenda por que esta solução foi adotada:
1. **Alternativa Descartada 1 - Usar caminho relativo `../assets/icon.svg` ou `assets/icon.svg`:**
   - *Por que foi descartada:* Depende da estrutura de pastas em tempo de build do Gradle (`assets.srcDirs`) e pode divergir quando o app roda na web com URL rewrites.
2. **Alternativa Descartada 2 - Converter o ícone para PNG bitmap e injetar como Base64:**
   - *Por que foi descartada:* Perde a nitidez vetorial do exoesqueleto da Lacraia em telas de alta densidade (OLED/Retina) e aumenta o payload.
3. **Decisão Adotada - Embarcar o SVG Vetorial Diretamente no DOM (`inline SVG`):**
   - O SVG original de `site/assets/icon.svg` possui apenas 1.4 KB e já tem estilo previsto em `.brand-section svg, .brand-logo-img`.
   - Inserir o SVG inline diretamente no elemento `.brand-section` garante hermeticidade total:
     - Renderização instantânea sem requisições HTTP adicionais.
     - Zero dependência de caminhos de arquivos relativos ou absolutos na WebView Android.
     - Preservação dos filtros luminosos neon (`glow-veneno` e `glow-fucsia`) e animações de hover.
   - Ajustar o link do logotipo para `href="#"` (ou `href="javascript:void(0)"`) evitando navegação indesejada para fora do Cockpit no Android.

## 3. Critérios de Aceite e Contrato
1. O elemento `.brand-section` em `site/app/index.html` deve renderizar o logotipo vetorial da Lacraia sem referências a caminhos de arquivos quebrados (`/assets/icon.svg`).
2. O logotipo deve possuir a classe `brand-logo-img` e manter dimensões de 28x28 pixels com viewBox `0 0 64 64`.
3. Os filtros glow e o traçado biomecânico devem permanecer idênticos ao asset canônico.
4. Testes automatizados em Rust devem assegurar a presença do SVG inline e a eliminação do caminho absoluto quebrado.
