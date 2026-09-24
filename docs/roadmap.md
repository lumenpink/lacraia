# Roadmap v0.2.0 – La Craia

## Sprint v0.2.0: Automotive Cockpit & Connectivity

This sprint focuses on steering wheel hardware controls, automotive smart caching, and responsive cockpit visuals.

---

### Ticket #14: Bluetooth AVRCP Steering Wheel Hardware Integration
* **Package:** `packages/lacraia-hud`
* **Goal:** Enable vehicle steering wheel media buttons to control track playback via native Android AVRCP and MediaSession delegation.
* **Acceptance Criteria:**
  - Map hardware `KEYCODE_MEDIA_PLAY`, `KEYCODE_MEDIA_PAUSE`, `KEYCODE_MEDIA_NEXT`, and `KEYCODE_MEDIA_PREVIOUS` events.
  - Forward events to `MediaCaptureListener` active controller without losing focus.
  - Unit tests validating key event translation to transport controls.

---

### Ticket #15: Garage Wi-Fi Station Pre-caching Daemon
* **Package:** `packages/lacraia-core`
* **Goal:** Proactively download and pre-cache favorite station tracks when the vehicle connects to a configured high-speed garage Wi-Fi network.
* **Acceptance Criteria:**
  - API endpoint `/api/sync/garage` triggered on home network connection.
  - Background worker pre-caches up to 20 lookahead tracks per active station seed into `/var/cache/la-craia/`.
  - SQLite table `garage_sync_log` recording timestamp, track count, and cached bytes.

---

### Ticket #16: Cockpit Audio Waveform Visualizer
* **Package:** `packages/lacraia-app`
* **Goal:** Real-time WebAudio frequency analyzer displaying a high-contrast soundwave canvas optimized for night-time driving.
* **Acceptance Criteria:**
  - Connect `AudioContext.createAnalyser()` to the HTML5 audio element.
  - Render an animated 32-bar visualizer using the `--fucsia-furacao` (`#ff0055`) neon accent on `--ferro-bate-pedra` (`#151618`) background.
  - Touch-toggle to enable/disable visualizer to conserve rendering cycles on low-spec car tablets.

> 📡 **[ TRANSMISSÃO DE CABINE ]**  
> Os 32 segmentos do analisador de espectro pulsam simulando as patas laterais do quilópode em plena correria durante os picos de grave.


---

### Ticket #17: Smart Silence Detection & Transition Smoother
* **Package:** `packages/lacraia-shared` & `packages/lacraia-core`
* **Goal:** Detect leading/trailing silence in resolved audio files and automatically trim or crossfade.
* **Acceptance Criteria:**
  - FFmpeg silence detection integration (`silencedetect=noise=-50dB:d=0.5`).
  - Metadata stores detected start/end audio trims.
  - Continuous gapless playback without dead air between tracks.

---

### Ticket #18: Low-Power Automotive Eco Mode
* **Package:** `packages/lacraia-hud` ("La Craia Simbiótyka")
* **Goal:** Reduce CPU and display power consumption when vehicle battery is low or device is unplugged.
* **Acceptance Criteria:**
  - Battery broadcast receiver monitoring charging state and level (< 20%).
  - Drops lyric ticker interval from 200ms to 1000ms.
  - Disables TextSwitcher slide animations in favor of static text updates.

---

### Ticket #19: Upgrade Workspace to Rust Edition 2024 and Dioxus 0.7
* **Package:** Workspace-wide (`lacraia-shared`, `lacraia-core`, `lacraia-app`)
* **Goal:** Migrate monorepo crates to Rust Edition 2024 and upgrade Dioxus framework to version 0.7 once published / stabilized.
* **Acceptance Criteria:**
  - Update `[workspace.package] edition = "2024"` in `Cargo.toml`.
  - Bump Dioxus dependency to `0.7` in `packages/lacraia-app/Cargo.toml` and adapt UI signals/lifecycle hooks.
  - Ensure `cargo test --workspace` and `cargo check --workspace` pass cleanly without deprecations.

---

### Ticket #20: Automated Cockpit Demo Launcher Script (`demo.sh`)
* **Package:** Root repository tooling (`demo.sh`)
* **Goal:** Provide an automated one-command orchestration script to spin up the `lacraia-core` daemon, initialize the SQLite database, serve the web player preview, and perform health probes.
* **Acceptance Criteria:**
  - Create executable `demo.sh` supporting `-p / --port`, `-o / --open`, and `--seed`.
  - Auto-probe port availability and start `lacraia-core` with lookahead queue daemon.
  - Handle cleanup trap (`SIGINT` / `SIGTERM`) to kill spawned background processes cleanly.
  - Print cockpit dashboard URLs and live WebSocket status.

---

### Ticket #21: Rebrand HUD to La Craia Simbiótyka and Migrate to Multi-Platform `lacraia-app`
* **Package:** `packages/lacraia-app` & `packages/lacraia-hud`
* **Goal:** Rebrand the Android HUD companion to "La Craia Simbiótyka" and transition `lacraia-web` into a multi-platform `lacraia-app` supporting both native Linux Desktop (WebKitGTK/Wry) and Web/PWA targets.
* **Acceptance Criteria:**
  - Rename `packages/lacraia-web` to `packages/lacraia-app`.
  - Add native Linux `desktop` feature and `web` feature in `Cargo.toml`.
  - Update Android notification and channel strings to "La Craia Simbiótyka".
  - Compile and test cleanly on both Linux desktop and wasm32 targets.

---

### Ticket #22: Generalização de Infraestrutura e Desacoplamento de 'Baquara'
* **Package:** `site` & `docs`
* **Goal:** Remover todas as referências residuais ao host específico "Baquara" / "baquara.lan", garantindo que a documentação e o portal reflitam um servidor Linux/Debian auto-hospedável de uso universal (`lacraia-server`, `localhost` / `lacraia.local`).
* **Acceptance Criteria:**
  - Substituir referências em [`docs/architecture.md`](file:///home/lumen/src/lacraia/docs/architecture.md), [`docs/sync-protocol.md`](file:///home/lumen/src/lacraia/docs/sync-protocol.md) e [`site/index.html`](file:///home/lumen/src/lacraia/site/index.html).
  - Exemplos de URLs de covers e WebSockets padronizados para hosts locais ou genéricos.
  - Rótulos dos nós metaméricos renomeados para terminologia arquitetural universal.

---

### Ticket #23: Favicon e Identidade Vetorial do Exoesqueleto (`site/favicon.svg`)
* **Package:** `site`
* **Goal:** Adotar o ícone da cabeça sensorial da lacraia (placa cefálica, ocelos fluorescentes e antenas) como o favicon vetorial oficial do portal, com suporte a navegadores modernos e integração visual no chassi.
* **Acceptance Criteria:**
  - Criar `site/assets/icon.svg` e `site/favicon.svg` com dimensões escaláveis e traçado puro em `--fucsia-furacao` e `--veneno-eletrico`.
  - Inserir tags `<link rel="icon" type="image/svg+xml" href="assets/icon.svg">` no `<head>` de [`site/index.html`](file:///home/lumen/src/lacraia/site/index.html).
  - Integrar o ícone em miniatura no painel superior (`top-panel`) do chassi ao lado de `LA CRAIA :: BANCADA`.

---

### Ticket #24: Mobilidade Biomecânica das Antenas Sensoriais
* **Package:** `site`
* **Goal:** Implementar mobilidade fluida e orgânica nas antenas da cabeça do quilópode no site principal, simulando o farejar contínuo e a exploração sensorial do animal.
* **Acceptance Criteria:**
  - Implementar animações CSS cíclicas de micro-contrações e ondulação orgânica nas antenas (`@keyframes twitch-antenna-left` e `@keyframes twitch-antenna-right`).
  - Modular a resposta dinâmica ao cursor do mouse: as antenas flexionam e apontam suavemente na direção do ponteiro ou da leitura do scroll.
  - Desativação imediata via `prefers-reduced-motion: reduce` mantendo a geometria estática segura.

---

### Ticket #25: Sistema de Cabines, Controle de Portas, Jukebox de Passageiros e Ejeção
* **Package:** Workspace-wide (`lacraia-shared`, `lacraia-core`, `lacraia-app`)
* **Goal:** Implementar o sistema de cabines com permissões Piloto/Passageiro, tranca de portas da cabine, exibição dinâmica/toggleável de QR Code, identificação do vivente que enfileirou cada cantiga e alavanca de ejeção de tripulantes.
* **Acceptance Criteria:**
  - **Tranca de Cabine (`is_locked: bool`):** Switch gangorra no cockpit do motorista (`[ 🔓 PORTAS ABERTAS / 🔒 TRANCADAS ]`). Quando trancada, o endpoint de embarque recusa novas conexões mesmo com o QR Code visível.
  - **QR Code na Interface com Alternância:** Botão `[ 📲 EMBARQUE ]` no cockpit para exibir ou ocultar o QR Code na tela (útil quando não se tem o adesivo físico ou para passageiros no banco traseiro).
  - **Identificação do Vivente na Fila (`requested_by`):** Campo `requested_by: Option<String>` nos itens de fila. A interface renderiza o badge com o nome do solicitante (ex: `[ por: @Mari ]` ou `[ PILOTO ]`).
  - **Painel de Tripulação Conectada:** Lista em tempo real no cockpit com os passageiros embarcados, seus aparelhos e as músicas solicitadas.
  - **Alavanca de Ejeção (`EJECT_PASSENGER` e `EJECT_ALL`):**
    - Chute individual (`[ 👢 ]`) revogando o token daquele vivente.
    - Ejeção total (`[ 💥 EJETAR GERAL ]`) limpando todos os passageiros da cabine.
    - Tela de bloqueio imediato no celular do passageiro com mensagem temática: `[ 🦂 VOCÊ FOI EJETADO DA CABINE PELO MOTORISTA! ]`.

---

### Ticket #26: Automação CI/CD no Forgejo, Build de APKs e Hub de Downloads
* **Package:** CI & Site (`.forgejo`, `site`, `packages/lacraia-hud`)
* **Goal:** Implementar o pipeline completo de integração contínua e distribuição no Forgejo Actions, incluindo compilação de binários nativos Rust, build de APK Android HUD via Gradle, deploy dual da documentação (Cloudflare + Codeberg Pages), canal contínuo Nightly e botões de download direto no site.
* **Acceptance Criteria:**
  - **Job Rust Workspace:** Validação de `cargo check`, testes unitários e compilação em `--release` dos binários `lacraia-core` e `lacraia-app` com upload de artefatos.
  - **Job Android HUD:** Setup de JDK 17, Android SDK e Gradle 8.7 para executar testes unitários e compilar `lacraia-hud-debug.apk`.
  - **Job Deploy Dual:** Publicação do portal estático via Wrangler no Cloudflare Workers (`lacraia.2lp.in`) com fallback automatizado para o branch `pages` no Codeberg Pages.
  - **Job Release & Nightly:** Publicação de releases oficiais com tags `v*` (com notas extraídas de `CHANGELOG.md`) e canal rolling `nightly` atualizado a cada push na `main` com links diretos para APK e executáveis Linux.
  - **Bancada de Embarque (Hub de Downloads no Site):** Painel estético brutalista na página principal com botões de download direto para APK Android, Desktop Linux, Daemon Core e releases do Codeberg.

---

### Ticket #27: Workspace Android Multi-Módulo, APK de Produção La Craia App e Simbiótyka
* **Package:** Android & CI (`packages/lacraia-android`, `.forgejo`, `site`, `docs`)
* **Goal:** Migrar o projeto Android para uma estrutura multi-módulo oficial com pacotes derivados do domínio `lacraia.2lp.in` (`in._2lp.lacraia.app` e `in._2lp.lacraia.simbiotyka`), gerando APKs de produção (`lacraia-app.apk` e `simbiotyka.apk`), e atualizar o nome oficial do HUD para Simbiótyka com múltiplos cenários de uso na documentação.
* **Acceptance Criteria:**
  - **Módulo `:app` (`in._2lp.lacraia.app`):** Aplicativo nativo completo para motorista/passageiro, com fullscreen imersivo, integração MediaSession e cockpit automotivo. Gera `lacraia-app.apk`.
  - **Módulo `:simbiotyka` (`in._2lp.lacraia.simbiotyka`):** Serviço de HUD flutuante de letras sincronizadas sobre navegação GPS, renomeado para Simbiótyka (`SimbiotykaService`). Gera `simbiotyka.apk`.
  - **APKs de Produção:** Configuração de release signing no Gradle e pipeline do Forgejo (`ci.yml`) para compilar sem sufixo `.debug`.
  - **Hub de Downloads do Site:** Botões dedicados para `[ BAIXAR LA CRAIA APP ]` e `[ BAIXAR SIMBIÓTYKA ]`.
  - **Documentação de Casos de Uso:** Especificação expandida em `docs/hud.md` detalhando navegação em trânsito, viagens em van/caronas, karaokê offline de serra e captura agnóstica de reprodutores de áudio.

---

### Ticket #6: Homologação e Correção dos Cards Open Graph & Twitter
* **Package:** `site`, `scripts`
* **Goal:** Sanear os metadados Open Graph e Twitter Cards do portal `lacraia.2lp.in` para exibição consistente em mensageiros e redes sociais (WhatsApp, Telegram, Twitter/X, Discord, LinkedIn).
* **Acceptance Criteria:**
  - **Gerador de Banner (`scripts/generate_og_image.py`):** Atualizado para a terminologia oficial Simbiótyka, gerando banner 1200x630px com branding oficial.
  - **Meta Tags de Cabeçalho (`site/index.html`):** Adicionado prefixo `og: https://ogp.me/ns#`, fallbacks Schema.org/WhatsApp (`itemprop="image"`, `link rel="image_src"`), `twitter:domain` e remoção de menções restritivas a apps específicos.
  - **Headers de Distribuição (`site/_headers`):** Charset UTF-8 explícito em HTML e liberação de CORS (`Access-Control-Allow-Origin: *`) para assets estáticos.

---

### Ticket #7: Cockpit Web em `/app` e Script Orquestrador de Execução
* **Package:** `site`, `packages/lacraia-core`, Workspace scripts
* **Goal:** Publicar a versão Web/PWA da La Craia no endereço `/app` do site e fornecer script unificado de inicialização para o daemon core e a interface web.

* **Acceptance Criteria:**
  - **Cockpit Web em `/app` (`site/app/index.html`):** Interface completa de reprodução, cockpit, cabines, alavanca de ejeção e jukebox acessível diretamente via navegador em `https://lacraia.2lp.in/app` com suporte PWA (`manifest.json`), visualizador neon de 32 barras e integração com MediaSession API.
  - **Fallback Estático no Core (`packages/lacraia-core`):** Roteador Axum configurado com `fallback_service` para servir arquivos do diretório `site/` (incluindo `/app/`), permitindo operação integrada em uma única porta.
  - **Script Orquestrador (`start.sh`):** Inicialização combinada de `lacraia-core` e servidor web local com auto-seed, migração de colunas da fila, verificação de portas e opção `--open` para navegador.
  - **Hub de Downloads e Painel Superior (`site/index.html`):** Adicionado botão de acesso instantâneo `[ ABRIR COCKPIT WEB ]` na bancada de embarque e atalho no painel superior.

---

### Ticket #8: Painel Visual da Fila de Reprodução e Sincronia WebSocket
* **Package:** `site`, `packages/lacraia-shared`, `packages/lacraia-core`, `packages/lacraia-app`
* **Goal:** Implementar o deck visual da fila de reprodução (lookahead queue) no cockpit automotivo e sanear a comunicação do protocolo WebSocket.
* **Acceptance Criteria:**
  - **Lookahead Queue no Protocolo:** Adicionado `queue: Vec<QueueItem>` na estrutura `StatePayload` e endpoint `GET /api/queue` para consulta direta.
  - **Saneamento do Barramento WebSocket:** Padronização das tags de mensagem em caixa alta (`STATE_UPDATE`, `CREW_UPDATE`, `EJECTED`, `CABIN_LOCKED`) e comandos (`PLAY`, `PAUSE`, `NEXT`, `PREVIOUS`, `SEEK`, `LOCK_CABIN`, `QUEUE_APPEND`, `QUEUE_REMOVE`).
  - **Deck da Fila no Cockpit Web (`site/app/index.html`):** Painel `#03` exibindo posições ordenadas, metadados, solicitantes (`requested_by`), badges de status (`CACHED`, `PLAYING`, `PENDING`) e botão de remoção para o piloto.
  - **Robustez de Conexão:** Detecção de ambiente HTTPS com fallback informativo e reconexão automática.

---

### Ticket #9: Endpoint de Busca no Catálogo e Jukebox com Autocomplete Interativo
* **Package:** `packages/lacraia-core`, `site/app/index.html`, `docs`
* **Goal:** Implementar endpoint `/api/search` com ranqueamento diacrítico e interface de autocomplete em tempo real para busca e enfileiramento de faixas no cockpit automotivo.
* **Acceptance Criteria:**
  - **Endpoint de Busca (`GET /api/search`):** Consulta ranqueada insensível a maiúsculas e diacríticos (`strip_accents`) no SQLite local cobrindo título, artista e gêneros, com sementes padrão automotivas (MPB, bossa nova, samba, synthwave, rock).
  - **Suporte a Faixas Ad-hoc:** Geração transparente de candidatos para termos formatados como `"Artista - Faixa"` permitindo enfileirar cantigas que ainda não constam no banco local.
  - **Autocomplete no Cockpit Web (`site/app/index.html`):** Componente debounced (250ms) com dropdown estilizado no tema cyberpunk automotivo, badges de status (`CACHED` vs `NUVEM`) e botão de ação direta `[ ➕ FILA ]`.
  - **Despacho WebSocket com Identificação:** Transmissão de `QUEUE_APPEND` com identificação do solicitante (`PILOTO` ou `@Passageiro`), notificação toast temporária e atualização otimista da fila.

---

### Ticket #5: Diretiva de Gênero Feminino para La Craia na Configuração CCCP
* **Package:** Repository & CCCP (`.cccprc`, `.antigravity/AGENTS.md`, `docs`)
* **Goal:** Incluir na configuração do CCCP e nas diretrizes dos agentes a regra obrigatória de tratar a aplicação "La Craia" sempre no gênero feminino ("a La Craia", "da La Craia", "na La Craia").
* **Acceptance Criteria:**
  - **Configuração no `.cccprc`:** Adicionadas chaves `agent_var_extras`, `agent_var_additional_directives` e `extras` determinando explicitamente o tratamento feminino.
  - **Diretrizes em `AGENTS.md`:** Seção de princípios operacionais estipula a política de concordância de gênero para todos os agentes autônomos.

---

### Ticket #4: Padronização das Referências à Aplicação La Craia no Gênero Feminino
* **Package:** `docs`, `site/app/index.html`
* **Goal:** Padronizar todas as menções textuais à aplicação La Craia em toda a documentação e interfaces do repositório para o gênero feminino ("a La Craia", "da La Craia", "na La Craia").
* **Acceptance Criteria:**
  - **Saneamento Textual:** Substituição de referências masculinas residuais ("o La Craia", "do La Craia", "no La Craia", "do Lacraia") por formas femininas em `docs/design-system.md`, `docs/architecture.md`, `docs/roadmap.md` e `site/app/index.html`.
  - **Conformidade com Diretrizes Operacionais:** Alinhamento integral com a política de concordância de gênero estabelecida no Ticket #5.

---

### Ticket #2: Seção Biográfica e Homenagem Cultural à Lacraia
* **Package:** `README.md`, `docs`
* **Goal:** Adicionar informações biográficas e contexto cultural sobre a artista Lacraia na documentação oficial do repositório.
* **Acceptance Criteria:**
  - **Diretrizes de Identidade:** Tratamento estritamente no feminino (*ela*, *a artista*, *a patrona*), reconhecimento e respeito explícito à sua identidade como travesti negra e periférica, e total ausência do nome morto.
  - **Contextualização Histórica:** Destaque de sua trajetória revolucionária nos anos 2000 ao lado de MC Serginho, presença arrebatadora na TV e palcos nacionais, e atuação posterior como DJ de música eletrônica.
  - **Manifesto de Princípios do Software:** Paralelo entre a autonomia, coragem cênica, velocidade e alegria radical de Lacraia com a arquitetura descentralizada, independente e livre da La Craia.

---

### Ticket #3: Arte Halftone em Estilo Lambe-Lambe da Lacraia
* **Package:** `docs/assets/`, `site/assets/`, `README.md`, `docs/design-system.md`, `site/index.html`
* **Goal:** Adicionar ilustração oficial da artista Lacraia aplicando retícula halftone de alto contraste com estética de arte urbana e lambe-lambe.
* **Acceptance Criteria:**
  - **Asset Gráfico Oficial:** Geração e armazenamento de `docs/assets/lacraia-halftone.jpg` e `site/assets/lacraia-halftone.jpg` em alta resolução (896x1200) com textura de retícula meio-tom, detalhes em fita adesiva nos cantos, destaques na cor `--fucsia-furacao` e tipografia stencil "LACRAIA · PATRONA DO PAREDÃO".
  - **Integração na Documentação:** Arte incorporada com destaque no `README.md`, no Caderno 02 do `docs/design-system.md` e no Caderno 06 da página oficial (`site/index.html`).

---

### Ticket #10: Suporte Mobile, Reordenação de Decks e Miniplayer Persistente
* **Package:** `site/app/index.html`, `docs/roadmap.md`
* **Goal:** Otimizar o cockpit web e a jukebox para dispositivos móveis (360px–480px), reorganizar os decks lógicos e fornecer controle contínuo via miniplayer persistente.
* **Acceptance Criteria:**
  - **Reordenação Lógica dos Decks:**
    - Deck #01: Master Playback & Paredão (fita, visualizador, seek, transport, ganho, estações).
    - Deck #02: Fila do Paredão & Jukebox (busca integrada no topo com autocomplete debounced e lista lookahead ordenada).
    - Deck #03: Cabine & Tripulação (ações de porta, QR Code de embarque, tripulantes e controles do piloto no final).
  - **Miniplayer Persistente nos Decks 02 e 03:** Mini-dock com Play/Pause, Avançar, Título/Artista, tempo e mini seekline neon sincronizado em tempo real com o motor de áudio.
  - **Sticky Bottom Miniplayer Mobile:** Barra flutuante de controle na base da tela ativada ao navegar pelas abas da Fila e da Cabine.
  - **Seletor de Abas Mobile (< 860px):** Navegação tátil por abas (`PLAYER`, `FILA`, `CABINE`) com badges de contagem dinâmica e áreas de toque mínimas de 44px.

---

### Ticket #11: Hierarquia de Acesso Master-Piloto e Rotação de Chaves de Passageiros
* **Package:** `packages/lacraia-core`, `site/app/index.html`, `docs/architecture.md`, `docs/roadmap.md`
* **Goal:** Implementar o controle de acesso de três níveis (Master, Piloto, Passageiro) com rotação dinâmica de senhas e autenticação por cabine.
* **Acceptance Criteria:**
  - **Autenticação Master (`LACRAIA_MASTER_KEY`):** Chave mestra imutável via variável de ambiente concedendo soberania administrativa sobre qualquer cabine do sistema.
  - **Chaves de Embarque Rotativas de Passageiros:** Geração e rotação instantânea de PINs alfanuméricos de 6 caracteres via `/api/cabin/rotate-key` invalidando passageiros anteriores.
  - **Persistência Segura em SQLite:** Tabelas `cabins` e `passengers` armazenando tokens e metadados de acesso.
  - **Ejeção Atômica de Tripulantes:** Comandos de ejeção individual e coletiva com encerramento de sessão em tempo real via WebSocket.

---

### Ticket #12: Resolução Híbrida (rustypipe + yt-dlp), Pre-cache Multi-Cabine e Áudio Real no Cockpit
* **Package:** `packages/lacraia-core`, `site/app/index.html`, `docs/architecture.md`, `docs/roadmap.md`
* **Goal:** Conectar o downloader nativo em Rust (`rustypipe`) com fallback em cascata para `yt-dlp`, estender o pre-cache do `QueueDaemon` para cobrir todas as cabines ativas e ligar o motor de áudio HTML5 no cockpit web para saída real de som nos alto-falantes.
* **Acceptance Criteria:**
  - **MediaResolver Híbrido em Cascata (`resolver.rs`):**
    - Verificação primária de cache local em disco (`./cache/*.opus`) com zero rede.
    - Tentativa primária de busca e extração de áudio via crate Rust nativa (`rustypipe`).
    - Fallback secundário transparente via CLI (`yt-dlp` + `ffmpeg`) em caso de falha ou incompatibilidade de assinatura na crate.
    - Gravação atômica do arquivo Opus e catalogação no SQLite.
  - **Pre-cache Multi-Cabine (`queue_daemon.rs`):**
    - O daemon inspeciona e pre-carrega faixas pendentes da janela de lookahead de todas as cabines ativas e da cabine padrão.
  - **Execução Real de Áudio no Cockpit (`site/app/index.html`):**
    - Elemento `<audio id="audio-engine">` conectado ao `stream_url` da faixa em reprodução.
    - Disparo de reprodução (`audio.play()`) ao iniciar ou receber estado `PLAYING`.
    - Pausa (`audio.pause()`) ao receber estado `PAUSED`.
    - Resolução sob demanda (`/api/resolve`) para faixas não cacheadas com feedback visual no player.
    - Sincronização do seekline e do controle de ganho até 11.










