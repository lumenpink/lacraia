# Especificação Técnica - Ticket #74: Simplificação dos Textos da Janela de Configurações

## 1. Contexto e Problema
A janela de configurações do Cockpit (`#settings-modal`) utilizava termos com alta carga de jargão de infraestrutura e engenharia de telecomunicações (ex.: "Lookahead pré-cache", "CGNAT", "Horizonte temporal de música antecipada", "Cota física de disco", "Padrão Monousuário").
Durante o uso em paredões de som, vans comunitárias ou festas de rua, esses termos causavam hesitação nos operadores e usuários que apenas queriam controlar o tempo de fila ou o espaço em disco.

## 2. Árvore de Decisões (Contexto para Natasha)
Para que Natasha entenda os motivos e as alternativas descartadas na revisão de linguagem:
1. **Alternativa Descartada 1 - Esconder completamente as opções avançadas atrás de uma chave 'Modo Desenvolvedor':**
   - *Por que foi descartada:* Criaria uma barreira de acessibilidade desnecessária. A intenção não é impedir o acesso ao buffer ou ao retransmissor de rede móvel, mas sim tornar a finalidade de cada controle imediatamente óbvia em português simples.
2. **Alternativa Descartada 2 - Substituir labels técnicos por ícones sem texto explicativo:**
   - *Por que foi descartada:* Ícones sozinhos aumentam a ambiguidade cognitiva. Quem opera som sob luz forte ou pressa precisa ler com clareza o que cada número representa.
3. **Decisão Adotada - Tradução Pop e Popular dos Rótulos e Dicas:**
   - "Fila do Autopilot (Horizonte Futuro)" -> "PILOTO AUTOMÁTICO (TEMPO DE MÚSICA NA FILA)"
   - "Lookahead Pré-cache (Cache Local Antecipado)" -> "MÚSICAS PRÉ-BAIXADAS (DOWNLOAD ANTECIPADO)"
   - "Cota Total de Disco (Cache Físico)" -> "LIMITE DE ESPAÇO NO APARELHO"
   - Explicação de 5G sem mencionar "CGNAT", focando na função real: permitir que as pessoas peçam som pelo seu próprio 4G/5G sem precisar conectar no Wi-Fi da van.
   - Preservação intacta de todos os IDs de elementos HTML (`input-queue-size`, `input-cache-size`, `input-disk-quota`, `input-master-key`, `check-van-mode`, `input-relay-url`) para não quebrar a bridge de automação e scripts JS.

## 3. Critérios de Aceite e Contrato
1. Os textos de rótulos e descrições na modal de configurações devem utilizar vocabulário acessível, mantendo o espírito colaborativo e popular.
2. Todos os identificadores de elementos (IDs e seletores CSS) devem permanecer inalterados.
3. Termos jargoados como "CGNAT" e "Lookahead" devem ser removidos dos rótulos principais visíveis.
4. Testes automatizados devem assegurar a presença dos novos textos amigáveis e a ausência do jargão técnico excessivo.
