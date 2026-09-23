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
