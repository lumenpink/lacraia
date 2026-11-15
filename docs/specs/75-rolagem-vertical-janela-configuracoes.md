# Especificação Técnica - Ticket #75: Rolagem Vertical na Janela de Configurações

## 1. Contexto e Problema
A janela modal de configurações (`#settings-modal`) do Cockpit (`site/app/index.html`) reúne diversos parâmetros de operação: origem do servidor (autônomo vs homelab remoto), tamanho do lookahead de pré-cache, cota de disco, configurações do Modo Van e credenciais de chave mestra.
Em dispositivos móveis (smartphones com telas compactas ou teclados virtuais abertos) e monitores com altura reduzida, o conteúdo total da modal ultrapassa a altura da viewport. Como a classe `.modal-card` possuía apenas dimensões fixas ou expansão automática sem contenção vertical (`max-height`) e sem declaração de rolagem (`overflow-y: auto`), os controles inferiores e botões de ação (como "Salvar Configurações" e "Fechar") ficavam inacessíveis para fora do viewport.

## 2. Árvore de Decisões (Contexto para Natasha)
Para que Natasha entenda por que adotamos esta solução e não outras alternativas descartadas:
1. **Alternativa Descartada 1 - Paginação por abas na modal:** Dividir a janela de configurações em múltiplas abas ou steps horizontais (ex.: "Cache", "Servidor", "Modo Van").
   - *Por que foi descartada:* Aumenta drasticamente a complexidade do JavaScript e do DOM para um formulário enxuto. O Cockpit preza pela operação rápida durante discotecagens ou ajustes de paredão, onde o operador deve ver todos os campos rolando rapidamente em uma única visão contínua.
2. **Alternativa Descartada 2 - Overflow no `.modal-overlay`:** Deixar o card estourar e rolar a página ou o overlay de fundo inteiro.
   - *Por que foi descartada:* Causa problemas visuais graves em navegadores móveis (rubber-banding do Safari/Chrome mobile, desalinhamento de centralização e corte de sombras do modal-card).
3. **Decisão Adotada - Contenção no `.modal-card` com limite de viewport (`max-height: 85vh; overflow-y: auto;`):**
   - Aplica-se no contêiner de cartão `.modal-card` (e especificamente reforçado para `#settings-modal .modal-card`), garantindo margem de respiro superior/inferior para o overlay backdrop.
   - Adiciona estilização coerente de scrollbar com as variáveis do tema paredão/industrial (`--fucsia-furacao` e `--asfalto-frio`), com suporte a `scrollbar-width: thin` e `::-webkit-scrollbar` para WebViews Android e navegadores desktop.
   - Garante que botões de ação permaneçam roláveis e totalmente acionáveis.

## 3. Contrato de UI / CSS e Critérios de Aceite
1. O seletor `.modal-card` (ou especificamente `#settings-modal .modal-card`) deve possuir:
   - `max-height: 85vh;` (ou equivalente entre 80vh e 90vh para contenção segura em viewports móveis).
   - `overflow-y: auto;` para habilitar a rolagem vertical sempre que o conteúdo exceder o limite.
   - Estilização de barra de rolagem (WebKit e padrão CSS `scrollbar-width`).
2. Todos os campos, inputs e botões de ação (`#btn-save-settings`, `#btn-close-settings`) devem permanecer dentro do container rolável sem serem cortados.
3. Testes automatizados (Rust e/ou scripts de teste de UI) devem validar estaticamente e dinamicamente a presença das diretivas de scroll e contenção.
