# Spec: Sincronização do Ícone do Site com o Aplicativo Android - Lacraia (#52)

## Contexto
O site web da Lacraia utilizava uma ilustração vetorial simplificada em `site/assets/icon.svg` e `site/favicon.svg` que divergia visualmente do ícone oficial gerado para o aplicativo Android (`in.twolp.lacraia.app`), o qual conta com o chassi industrial de ferro bate-pedra, antenas em curva de Bézier com glow fluorescente, ocelos sensoriais e parafusos de cantoneira.

## Objetivo
Unificar a identidade visual do portal web `lacraia.2lp.in` com o aplicativo oficial Android, garantindo que o cabeçalho da bancada e as tags de ícone do documento HTML utilizem diretamente o artefato oficial `assets/icon.png` gerado com base no módulo Android.

## Critérios de Aceitação
1. O elemento `<img>` do cabeçalho da bancada em `site/index.html` deve referenciar `assets/icon.png` (ou variante alinhada ao app Android).
2. As tags `<link rel="icon">` e `<link rel="apple-touch-icon">` devem apontar para o ícone oficial em `assets/icon.png`.
3. Teste automatizado validando que o site web referencia o ícone oficial sincronizado com o app Android.
