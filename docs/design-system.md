# La Craia: Especificação de Design e Manual de Identidade

**Software Livre (GPLv3+) | Autônomo | Dissidente | Fora do Mercado**

---

## 1. Manifesto Estético e Conceitual

O visual da La Craia recusa terminantemente a cartilha asséptica do Vale do Silício e a nostalgia fetichizada do cyberdeck comercial. O software se materializa a partir da colisão entre três forças:


1. **A Metameria do Quilópode (O Bicho):**  
   O exoesqueleto rígido da lacraia, subdividido em placas articuladas de quitina, pernas que avançam com precisão mecânica, pinças peçonhentas e capacidade de infiltração nas frestas mais escuras.
2. **A Performatividade de Lacraia (O Ícone):**  
   Lacraia nos palcos do funk dos anos 2000. O deboche travesti, o choque visual do cetim fluorescente, o glitter sobre o chão batido, a sobrancelha raspada e a ocupação barulhenta do espaço público. Há um diálogo direto com o camp visceral, a ficção científica B e a audácia de Rocky Horror.
3. **A Gambiarra de Bancada e Paredão:**  
   A tecnologia periférica latino-americana. Caixas de som em compensado naval cortado no serrote, ferro de solda, fiação amarrada com fita isolante, chassi de ferro com pintura bate-pedra e componentes reaproveitados de sucata.

> O resultado não é um "tema bonitinho", mas uma máquina de som furiosa, viva e visivelmente remendada, onde a dureza da oficina encontra a apoteose do palco.

---

## 2. Materiais, Texturas e Tratamento Gráfico

A interface não simula uma folha de papel digital nem um vidro translúcido (*glassmorphism*). Ela simula matéria física bruta:

* **Compensado Naval (Fundo Estrutural da Aplicação):**  
  Base visual simulando lâminas de madeira compensada escura, rústica, com veios densos, manchas de umidade, furos de broca e marcas circulares de parafusos soberbos. Acabamento em selador barato ou madeira naval crua reaproveitada de caixa de som automotivo.
* **Chapa de Aço com Batida de Pedra (Superfícies de Controle):**  
  Painéis pretos foscos com granulometria pesada (textura de emborrachamento automotivo). Cantoneiras de ferro zincado ou chapa perfurada nas bordas com cabeças de rebite aparentes nos cantos (`clip-path` chanfrado e detalhes de fixação).
* **Lambe-Lambe em Halftone (Homenagem à Lacraia):**  
  Retratos icônicos da Lacraia processados em retícula de meio-tom (*halftone* / *dither*) de ponto grosso e alto contraste, com estética de fotocopiadora quebrada ou serigrafia artesanal. Colagens com bordas rasgadas e cantos colados com fita isolante ou fita crepe digital nos cantos.

  ![Arte Oficial: Lacraia · Patrona do Paredão](assets/lacraia-halftone.jpg)

---

## 3. Sistema Cromático (Tokens CSS Oficiais)

```css
:root {
  /* MATERIAIS BASE */
  --madeira-compensado:     #2b1d14; /* Fundo geral da carcaça */
  --madeira-veios:          #1f140e; /* Linhas de fibra e nós */
  --ferro-bate-pedra:       #151618; /* Módulos funcionais e carcaça de metal */
  --ferro-painel:           #1f2126; /* Superfície dos consoles e caixas de texto */
  --metal-cantoneira:       #42464e; /* Bordas de metal, cantoneiras e parafusos */

  /* O DEBOCHE & A PERFORMANCE (ACENTOS DISPARADORES) */
  --fucsia-furacao:         #ff0055; /* Rosa choque fluorescente de palco / Ação crítica */
  --cetim-magenta:          #d80064; /* Sombra dura de botões e estados ativos */
  --glitter-roxo:           #8000ff; /* Iluminação secundária e links da documentação */

  /* A PEÇONHA & A GAMBIARRA */
  --veneno-eletrico:        #d4ff00; /* Amarelo-limão de pata de lacraia / Fita isolante */
  --alta-voltagem:          #ffb700; /* Amarelo trânsito / Alertas de instabilidade */
  --fio-massa-verde:        #00ff66; /* LED de sinal verde / Status OK */
  --curto-circuito:         #ff2200; /* LED de pico / Clipping de áudio / Falha crítica */

  /* TIPOGRAFIA & LEITURA */
  --papel-mimeografo:       #e8e6df; /* Texto corrido da documentação */
  --tinta-carimbo:          #0a0a0b; /* Texto sobre botões fluorescentes */
  --grafite-mono:           #8a909a; /* Metadados, números de linha e comentários */
}
```

---

## 4. Tipografia: A Voz da Máquina

* **Títulos, Identificadores e Cabeçalhos:**
  * Tipos de Estêncil e Prensa Seca: Fontes condensadas, pesadas, sem serifa, simulando letras aplicadas com molde vazado e tinta spray na madeira, ou carimbo de pneu e caixote de feira (`Impact`, `Bebas Neue`, `Black Ops One`).
  * Caixa-alta obrigatória nos rótulos de controle (`[SEG.01]`, `ENTRADA`, `SAÍDA`, `ATENÇÃO: ALTA TENSÃO`).
* **Corpo de Texto, Formulários e Documentação:**
  * Monospace Industrial Estrita: `JetBrains Mono`, `Liberation Mono` ou `IBM Plex Mono`.
  * Todo o texto técnico se comporta como se estivesse sendo emitido por uma máquina de escrever elétrica com fita gasta ou um terminal monocromático embutido na bancada de madeira.

---

## 5. Arquitetura da Interface Principal (A Máquina)

A tela principal do software organiza-se como um chassi modular aparafusado:

```
+-------------------------------------------------------------------------+
| [LAMBE: LACRAIA]    LA CRAIA :: CORE v1.0.0-gpl3     [LED: ON] [TEMP: 42C] |
+-------------------------------------------------------------------------+
| [METÂMERO 01: CONTROLES]  | [METÂMERO 02: BANCADA DE TRABALHO]           |
|                           |                                             |
|  (O) POTENCIÔMETRO GANHO  |  +--[ PLACA DE COMPENSADO: SAÍDA ]-------+  |
|      rotacionado 3.5deg   |  | > dados brutos de execução...         |  |
|                           |  | > segmentação ativa: 16 patas         |  |
|  [ / ] CHAVE FACA GANGORRA|  |                                       |  |
|        tortinha (-2deg)   |  +---------------------------------------+  |
|                           |                                             |
|  [*] LED CLIPPING PISCA   |  +--[ CONTROLE DE COMANDO ]---------------+  |
|                           |  | [ INPUT: Digite aqui...           ]   |  |
|  /// FITA ISOLANTE ////// |  | [ BOTÃO: ATACAR ] [ BOTÃO: ARREGAÇAR ] |  |
|  (remendo de interface)   |  +---------------------------------------+  |
+-------------------------------------------------------------------------+
| SEGMENTO TERMINAL: 21 pares de pernas operando | Feito com ódio & glitter|
+-------------------------------------------------------------------------+
```

### 5.1. Controles Físicos com Instalação Irregular (Micro-Assimetria)

* **Chaves Gangorra (*Rocker Switches*) e Chaves Alavanca:**
  * Interruptores mecânicos em preto e vermelho, com cliques sonoros secos e texturas táteis.
  * Regra CSS de Desalinhamento: cada switch possui micro-rotação randômica via CSS (`transform: rotate(-1.5deg);` ou `transform: rotate(2deg);`) quebrando o alinhamento reto de fábrica.
  * Etiquetas de fita crepe coladas tortas abaixo do switch (`[ SUBIR ]`, `[ QUEBRAR ]`).
* **Potenciômetros e Knobs Raspados:**
  * Seletores giratórios com ponteiros brancos desgastados e números gravados ao redor de **0 a 11** (o ganho que passa do limite).
* **LEDs Furados no Compensado:**
  * Lâmpadas indicadoras redondas montadas diretamente na madeira.
  * Brilho radial forte (`box-shadow: 0 0 12px var(--veneno-eletrico);`), simulando a luz sangrando nas fibras da madeira.
  * LED de "Clipping / Pico": pisca violentamente em vermelho (`--curto-circuito`) em caso de sobrecarga ou erros.
* **Emendas de Fita Isolante:**
  * Divisórias entre módulos que usam listras amarelas e pretas diagonais (`repeating-linear-gradient(45deg, #d4ff00, #d4ff00 10px, #151618 10px, #151618 20px)`).

---

## 6. A Documentação como Espetáculo (Caderno de Montagem do Paredão)

### 6.1. O Índice Lateral: A Coluna Vertebral do Bicho
* Em vez de listagem simples, o sumário lateral é desenhado como uma linha vertical contínua com nós articulados (o corpo segmentado da lacraia).
* Cada página ou subtítulo é um par de patas: ao fazer scroll, as "pernas" do índice acendem em `--veneno-eletrico` simulando o animal rastejando junto à leitura.
* **A Cabeça Sensorial e Antenas Biomecânicas:** A extremidade superior do cordão neural ostenta a placa cefálica com ocelos fluorescentes pulsantes (`ocelli-breathe`) e duas antenas articuladas com micro-movimentos orgânicos contínuos e assíncronos (`twitch-left` e `twitch-right`), simulando o animal farejando ativamente o ambiente no escuro. As antenas orientam-se dinamicamente conforme o deslocamento do cursor e realizam contração defensiva instantânea ao clique.

### 6.2. Caixas de Chamada (Callouts) com Identidade Própria

| Tipo de Aviso Padrão | Na La Craia | Identidade Visual |
|---|---|---|

| Info / Nota | `[ TRANSMISSÃO DE CABINE ]` | Fundo cinza chumbo com borda roxa fluorescente (`--glitter-roxo`). Comunicado de rádio pirata. |
| Warning / Atenção | `[ FIO DESENCAPADO ]` | Borda em padrão zebra de fita isolante amarela/preta. Alerta sobre risco ao sistema. |
| Danger / Erro | `[ VENENO NA CARNE ]` | Borda fúcsia berrante (`--fucsia-furacao`), texto em alto contraste e ícone peçonhento. |
| Dica de Otimização | `[ O PULO DA TRAVESTI ]` | Caixa com moldura de cantoneira metálica e carimbo de estrela estilizada. Atalhos e gambiarras geniais. |

### 6.3. Blocos de Código
* Fundo preto fosco absoluto (`#0d0e10`), simulando fita de rotuladora manual.
* Linhas de código em verde fósforo e amarelo peçonha.
* Botão de copiar em formato de chave de acionamento mecânico: ao clicar, rebate com `[ COPIADO! / CLACK! ]`.

---

## 7. Diretrizes Técnicas e Acessibilidade

1. **Zero Frameworks Pesados de Estilização:**  
   Toda a identidade é viabilizada em CSS puro e variáveis globais. O software roda instantaneamente em computadores modestos e carPCs.
2. **Ícones em SVG Geométrico Puro:**  
   Sem pacotes de terceiros pesados; traçado cru e linhas retas.
3. **Contraste Rígido (Mínimo 7:1):**  
   Todo texto corrido mantém contraste estrito para leitura confortável no cockpit noturno.
4. **Respeito a `prefers-reduced-motion`:**  
   Desativa imediatamente os piscares de LED, animações de pernas e trepidações mecânicas caso o usuário solicite redução de movimento no sistema.

---

## 8. Implementações de Referência

* **Portal de Documentação (`site/index.html`):**  
  Implementação viva de referência dos tokens CSS, chassi de ferro com pintura bate-pedra, leds de telemetria, divisórias de fita isolante zebrada, os 4 callouts temáticos e o botão com efeito tátil mecânico `[ CLACK! ]`. Organizado em 6 cadernos metaméricos (iniciando pelo manifesto de soberania acústica em Caderno 01 e finalizando no Manual de Identidade Visual em Caderno 06).
* **Favicon e Identidade Vetorial do Exoesqueleto (`site/favicon.svg` e `site/assets/icon.svg`):**  
  Ícone geométrico puro da cabeça sensorial da lacraia combinando placa cefálica com chanfro brutalista em `--fucsia-furacao` e `--ferro-painel`, ocelos em `--veneno-eletrico`, forcípulas peçonhentas e antenas curvas estilizadas. Empregado como favicon SVG universal e exibido no painel de comando (`top-panel`) do chassi.
* **Social Share Preview & Open Graph Card (`scripts/generate_og_image.py`):**  
  Banner de compartilhamento em alta resolução (1200×630px, `site/assets/og_preview.png`) com Safe Zone 4:3 centralizada, imune aos recortes do WhatsApp, Telegram e Twitter Card (`summary_large_image`). Base em ferro bate-pedra com hachura diagonal, ambient glow fúcsia/glitter, fita isolante zebrada sob a marca e rodapé com telemetria ativa.
* **Cockpit Player (`packages/lacraia-app`):**  
  Interface automotiva modular com metâmeros, potenciômetro de ganho até 11 (`format_gain_11`), chaves mecânicas com micro-rotação assimétrica (`transform: rotate(-1.5deg)`), seletores de estação e fila de reprodução com tokens visuais do manifesto.



