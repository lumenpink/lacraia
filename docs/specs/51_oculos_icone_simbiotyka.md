# Spec: Reposicionamento dos Óculos no Ícone do Simbiótyka (#51)

## Contexto
No aplicativo Android e na vitrine da Lacraia, o ícone do módulo Simbiótyka (`in.twolp.lacraia.simbiotyka`) apresenta a ilustração da mascote Lacraia utilizando óculos escuros estilo Kamina com brilho de estrela.
Entretanto, a posição vertical dos óculos estava configurada com vértices superiores muito altos ($y=430$ e $y=530$), permitindo que parte dos ocelos sensoriais (olhos localizados em $y=576$) ficasse visível acima e abaixo da lente.

## Objetivo
Descer ligeiramente as lentes dos óculos de modo que cubram integralmente a área dos ocelos sensoriais ($y=576 \pm 46$), mantendo a inclinação característica e o brilho estelar.

## Critérios de Aceitação
1. A caixa delimitadora dos óculos (triângulos poligonais) deve abranger verticalmente os ocelos (de $y \ge 520$ até $y \ge 640$ na altura dos centros dos olhos em $x=416$ e $x=608$).
2. Execução do script gerador `python3 scripts/gerar-recursos-android.py` sem erros, atualizando:
   - `site/assets/simbiotyka.png`
   - `packages/lacraia-android/simbiotyka/src/main/res/mipmap-*/ic_launcher.png`
   - `packages/lacraia-android/simbiotyka/src/main/res/mipmap-*/ic_launcher_round.png`
3. Teste automatizado validando a cobertura dos olhos pelos óculos nos pixels do ícone gerado.
