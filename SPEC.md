# La Craia Technical Specification (SPEC-01)
**Normative Standard:** GOST `2.0.0` (General Open Source Standard)  
**Implementation Profile:** `profiles/lilya.md` (Lilya Ecosystem)  
**Specification Version:** `1.1.0`  
**Status:** Official / Standard  
**License:** GNU General Public License v3.0 (GPL-3.0) — Strategic Copyleft Safeguard  
**Author:** Lumen Pink <hi@lumen.pink>  
**Repository:** `https://gambi.2lp.in/lumen/lacraia`  
**Documentation Portal:** `https://lacraia.2lp.in`

---

## 1. Escopo e Filosofia de Áudio Anticorporativa

Esta especificação define a arquitetura de sistema, contratos de API, modelos de dados, pipelines de resolução de áudio, topologia de operação híbrida (Standalone e Federada), protocolos de telemetria, mecanismos de controle de acesso e padrões de qualidade da **La Craia**.

**A La Craia** é um paredão digital autônomo, rádio contínua por afinidade acústica e HUD simbiótico de letras sincronizadas projetada para sistemas de som de alta potência, festas de rua, escuta doméstica, estações de trabalho Linux e ambientes móveis ou veiculares.

### Princípios Fundamentais
1. **Autonomia Acústica:** Rejeição total a plataformas comerciais de streaming saturadas de anúncios, armadilhas de recomendação algorítmica fechada e vigilância por telemetria. O pipeline de áudio opera de forma autônoma utilizando caches locais, servidores domésticos federados (*Paredão Matriz*) e endpoints públicos sanitizados.
2. **Fluxo Contínuo (Store-and-Forward / Zero Dead Air):** Diferente de players baseados em streaming *just-in-time* que engasgam diante de oscilações de rede ou bloqueios de CDN, um daemon autônomo (`QueueDaemon`) resolve e pré-armazena faixas futuras em formato Opus/WebM ao longo de uma janela temporal cumulativa de **3.600 segundos (1 hora)** de cache local e **7.200 segundos (2 horas)** de fila remota, blindando a reprodução contra zonas de sombra rodoviárias severas (P99).
3. **Operação Autônoma Zero-Fricção e Simbiose com Servidor:** A aplicação móvel e desktop opera em modo 100% standalone sem exigir infraestrutura prévia para pessoas leigas, mas permite acoplamento transparente via internet a um servidor doméstico (`lacraia-server`) para hidratação a partir de acervos pessoais massivos.
4. **Visibilidade Tátil e Brutalista:** Interfaces mecânicas de alto contraste inspiradas nos paredões de som latino-americanos, maquinário de oficina e nas performances subversivas da ícone do funk Lacraia.
5. **Integração Multiplataforma Simbiótica:** Motor unificado (`lacraia-core`) servindo binários nativos de desktop Linux, aplicativos Android autônomos, PWAs web e o companion flutuante Android (*Simbiótyka*) que projeta letras sincronizadas linha a linha sobre aplicações de terceiros sem roubar foco de janela.

---

## 2. Anatomia do Repositório e Conformidade GOST

O repositório segue estritamente as Seções 2, 3 e 8 do padrão GOST:

    .
    ├── LICENSE                     # Copyleft Forte (GNU General Public License v3.0)
    ├── VERSION                     # String plana de versionamento semântico
    ├── CHANGELOG.md                # Histórico de alterações legível por humanos
    ├── README.md                   # Visão executiva, instalação e tributo biográfico (GPL-3.0)
    ├── SPEC.md                     # Esta especificação técnica normativa (SPEC-01)
    ├── .cccprc                     # Governança CCCP, hooks e diretivas de gênero feminino ("a La Craia")
    ├── Cargo.toml                  # Manifesto raiz do workspace Cargo (Rust Edition 2024)
    ├── packages/
    │   ├── lacraia-shared/         # Structs de domínio, parser LRC, busca fuzzy e protocolo WS (WASM-safe)
    │   ├── lacraia-core/           # Motor compartilhado (rlib + cdylib): Axum, SQLite, QueueDaemon, Resolver e JNI
    │   ├── lacraia-server/         # Binário CLI de servidor dedicado com feature `cli-fallback` (yt-dlp + ffmpeg)
    │   ├── lacraia-app/            # Console player multiplataforma (Dioxus Linux desktop c/ core & WASM/PWA)
    │   └── lacraia-android/        # Workspace Gradle Android (módulos :app c/ liblacraia_core.so & :simbiotyka)
    ├── site/                       # Portal de documentação pública, manifesto dinâmico e console web
    │   ├── index.html              # Portal principal, manifesto, hub de downloads e cinemática das antenas
    │   ├── extractor-manifest.json # Configuração remota quente de perfis Innertube e instâncias de fallback
    │   ├── _headers                # Cabeçalhos HTTP estritos (nosniff, DENY, strict-origin, CORS e MIME APK)
    │   ├── assets/                 # Favicon SVG, logotipos, arte halftone e card de preview social 1200x630
    │   └── app/                    # Console PWA standalone do paredão (/app)
    ├── tests/
    │   └── e2e/                    # Suíte automatizada Playwright (--smoke, regressão canônica e --exhaustive)
    ├── docs/                       # Diagramas arquiteturais, design system e roadmaps
    └── scripts/                    # Scripts auxiliares (dev.sh, start.sh, demo.sh, generate_og_image.py)

### 2.1 Justificativa de Exceção de Licenciamento e Paridade Documental
Em conformidade com a Seção 2.2 do padrão GOST, a La Craia adota a **GNU General Public License v3.0 (GPL-3.0)** em todos os artefatos e documentações (incluindo o rodapé do `README.md`, eliminando menções residuais à licença MIT). Esta proteção garante que o software, os algoritmos de afinidade acústica e as interfaces permaneçam permanentemente livres, impedindo fechamento proprietário.

---

## 3. Arquitetura do Workspace e Divisão de Crates

O workspace desacopla a lógica do motor das cascas de execução para permitir que o mesmo código rode em servidores dedicados e embarcado dentro do aplicativo de usuárias finais:

### 3.1 `packages/lacraia-shared` (Rust Crate — `rlib`)
- **Papel:** Contratos de dados puros, serialização de protocolo e algoritmos de domínio sem dependências de I/O de sistema operacional (100% compatível com `wasm32-unknown-unknown`).
- **Componentes:**
  - `protocol`: Envelopes WebSocket (`WsMessage`, `Command`, `StateUpdate`, `CrewUpdate`, `EjectedNotification`, `CacheAlertInfo`).
  - `lrc`: Parser de letras sincronizadas com precisão de milissegundos.
  - `fuzzy`: Remoção de diacríticos (`strip_accents`), correspondência léxica de títulos (`is_title_matching`) e filtros de tolerância de duração (delta máximo de 15s).
  - `models`: Definições de entidades, estado de reprodução, inventário de cache e métricas de resolução.

### 3.2 `packages/lacraia-core` (Rust Crate — `rlib` & `cdylib`)
- **Papel:** Biblioteca central contendo todo o motor de streaming, persistência e sincronização. Compila como biblioteca Rust (`rlib`) para o servidor/desktop e como biblioteca dinâmica C-ABI (`liblacraia_core.so`) via `cargo-ndk` para embarque nativo no Android via JNI.
- **Cargo Features:**
  - `default = []`: Extração 100% nativa em Rust (zero dependência de Python ou binários externos).
  - `cli-fallback`: Habilita a invocação de subprocessos `yt-dlp` e `ffmpeg` como último recurso de extração.
- **Componentes:**
  - `db`: Persistência SQLite (`lacraia.sqlite`), grafo de similaridade, cabines, histórico, inventário de cache e algoritmo de expurgo LRFU.
  - `resolver`: Motor de resolução em cascata (Cache Local -> Paredão Matriz Remoto -> Rust Innertube/Piped -> CLI Fallback opcional).
  - `queue_daemon`: Worker assíncrono de lookahead temporal e garbage collection de disco.
  - `ws` & `api`: Hub de telemetria WebSocket e roteamento Axum REST/Range.
  - `ffi`: Interface JNI (`Java_in_twolp_lacraia_app_CoreBridge_startDaemon`) para execução em `ForegroundService` no Android.

### 3.3 `packages/lacraia-server` (Rust Crate — `bin`)
- **Papel:** Casca executável leve para servidores dedicados Linux/Debian, containers e mini-PCs veiculares.
- **Comportamento:** Compila a `lacraia-core` com a feature `cli-fallback` ativa, lê variáveis de ambiente (`PORT`, `LACRAIA_CACHE_DIR`, `LACRAIA_DB_PATH`, `LACRAIA_MASTER_KEY`), configura cota de disco expandida (padrão 50 GB ou ilimitado) e faz o bind do servidor Axum em `0.0.0.0:8080`.

### 3.4 `packages/lacraia-app` (Rust / Dioxus Crate — `bin` & `wasm`)
- **Papel:** Console player do paredão para Linux Desktop (via Wry/WebKitGTK) e Web/PWA.
- **Isolamento Condicional de Target:**
  - **Alvo Nativo (`cfg(not(target_arch = "wasm32"))`):** Embute a `lacraia-core` diretamente no binário, iniciando o daemon em `127.0.0.1` automaticamente ao abrir a janela (detectando `yt-dlp` no `PATH` do sistema caso instalado).
  - **Alvo Web/WASM (`cfg(target_arch = "wasm32")`):** Compila exclusivamente a UI e o cliente de áudio/WebSocket (dependendo apenas da `lacraia-shared`), conectando-se ao servidor que serviu a página (`/app`).

### 3.5 `packages/lacraia-android` (Kotlin Android Multi-Module)
- **Papel:** Aplicativos Android nativos sob o namespace canônico `in.twolp.*`.
- **Módulos:**
  - `:app` (`in.twolp.lacraia.app`): Carrega `liblacraia_core.so` em um `ForegroundService` local, oferecendo operação autônoma de bolso, console tátil e jukebox (`lacraia.apk` / `lacraia-app.apk`).
  - `:simbiotyka` (`in.twolp.lacraia.simbiotyka`): Companion HUD flutuante de letras sincronizadas e *audio ducking* para navegação GPS (`simbiotyka.apk`).

---

## 4. Topologia de Operação em 3 Modos

Ao compartilhar a `lacraia-core` entre o servidor e o aplicativo, a La Craia opera em três topologias intercambiáveis em tempo de execução:

1. **Modo 1 — Autônomo Solo (Loopback `127.0.0.1`):**
   - Padrão imediato após instalar o APK ou abrir o app Desktop.
   - O daemon Rust roda localmente no aparelho em `127.0.0.1:8080`.
   - Zero configuração de servidor necessária: o grafo SQLite e o cache de 1 hora (teto de 1 GB) são gerenciados no próprio dispositivo.
2. **Modo 2 — Paredão de Bolso / Cabine Ad-Hoc (`0.0.0.0`):**
   - Ao acionar `[ 📡 ABRIR CABINE LOCAL ]` no app móvel/desktop, o listener Axum reabre em `0.0.0.0:8080` na rede Wi-Fi ou Hotspot portátil do celular.
   - O próprio celular serve a interface web `/app` e o QR Code de embarque para passageiros próximos enfileirarem cantigas sem precisarem instalar aplicativo nem possuir servidor dedicado.
3. **Modo 3 — Federado com Servidor Doméstico (*Paredão Matriz* via Internet):**
   - Usuárias que possuem um `lacraia-server` rodando em casa configuram a `upstream_server_url` e o token de acesso em `system_settings`.
   - Na rua (4G/5G), o app continua operando localmente sem engasgos, mas utiliza o servidor doméstico pela internet como fonte primária de arquivos `.opus` já cacheados e como delegador de extração `yt-dlp`.

---

## 5. Armazenamento SQLite, Grafo de Afinidade e Expurgo LRFU de 1 GB

A `lacraia-core` mantém o banco relacional `lacraia.sqlite` com as seguintes estruturas normativas:

* **`tracks`:** Metadados do catálogo (`id`, `artist`, `title`, `duration_sec`, `cached_path`, `genres`, `last_played_at`).
* **`similarity_edges`:** Grafo direcionado ponderado (`source_track_id`, `target_track_id`, `weight`, `provider`) alimentado por Last.fm e MetaBrainz/ListenBrainz.
* **`cabins`, `passengers`, `queue`, `cabin_playback_state`, `playback_history`:** Gestão multi-cabine, controle de acesso RBAC, fila ordenada e histórico de reprodução.
* **`cache_entries`:** Inventário físico dos artefatos de áudio (`track_id`, `file_path`, `file_size_bytes`, `duration_sec`, `audio_codec`, `hit_count`, `last_served_at`, `created_at`).
* **`cache_resolution_attempts`:** Log granular de diagnósticos por tier de resolução (`tier`, `method`, `success`, `exit_code`, `error_code`, `duration_ms`, `bytes_received`).
* **`system_settings`:** Parâmetros dinâmicos persistidos (`target_queue_duration_sec`, `target_cache_duration_sec`, `max_disk_cache_mb`, `upstream_server_url`, `upstream_token`, `metered_network_mode`).

### 5.1 Política de Cota de Disco e Algoritmo de Expurgo Ponderado (LRFU)
Para evitar esgotamento de armazenamento em dispositivos móveis mantendo as músicas favoritas e as recém-baixadas protegidas:
1. **Cotas Padrão (`max_disk_cache_mb`):**
   - **Mobile / App Standalone:** `1024 MB` (**1 GB**, equivalente a ~17 horas de áudio Opus a 128 kbps).
   - **Servidor Dedicado (`lacraia-server`):** `51200 MB` (50 GB) ou ilimitado (`0`), configurável via interface.
2. **Fórmula de Relevância Híbrida (Frequência + Recência):**
   Sempre que `SUM(file_size_bytes)` em `cache_entries` ultrapassar `max_disk_cache_mb`, o `QueueDaemon` avalia os arquivos elegíveis (excluindo faixas que estão com estado `playing` ou `cached` na fila ativa `queue`) ordenando pelo menor score de retenção:

       Score(t) = (hit_count * 1.0) + (10.0 / (dias_desde_ultimo_uso + 1.0))

   Onde `dias_desde_ultimo_uso = (agora - last_served_at) / 86400`.
3. **Imunidade de Recém-Chegadas e Favoritas:**
   - Uma faixa que acabou de ser baixada ou tocada (`dias_desde_ultimo_uso ~ 0`, `hit_count = 1`) recebe `Score ~ 11.0`, ficando imune ao expurgo imediato frente a faixas antigas que tocaram poucas vezes (`dias_desde_ultimo_uso = 15` resulta em `Score ~ 1.6`).
   - Uma faixa clássica do gosto da usuária (`hit_count >= 12`) mantém `Score > 12.0` mesmo após semanas sem tocar.
4. **Preservação do Grafo:** O expurgo remove estritamente o arquivo físico em `/cache/` e o registro em `cache_entries`, limpando `tracks.cached_path = NULL`, mas **jamais apaga** a faixa de `tracks` ou `similarity_edges`.

---

## 6. Pipeline de Áudio, Resolver Federado e Manifesto Quente

A resolução de áudio opera em modo *Store-and-Forward* (pré-download integral em blocos de HTTP Range antes da reprodução) através de 5 tiers em cascata:

    [ Solicitação de Faixa / Semente do Autopilot ]
                       │
                       ▼
           ┌───────────────────────────┐
           │ Tier 0: Cache Local Disco │ ──(HIT)──► Serve Imediato (HTTP 206 Range)
           └───────────────────────────┘
                       │ (MISS)
                       ▼
           ┌───────────────────────────┐
           │ Tier 0.5: Paredão Matriz  │ ──(Configurado?)──► Puxa .opus do Server de Casa
           └───────────────────────────┘                     ou delega /api/resolve
                       │ (Offline / Sem Server)
                       ▼
           ┌───────────────────────────┐
           │ Tier 1: Rust Innertube    │ ──► RustyPipe + Perfis de extractor-manifest.json
           └───────────────────────────┘
                       │ (Falha / Rate Limit 429)
                       ▼
           ┌───────────────────────────┐
           │ Tier 2: Rust Piped API    │ ──► Streams via Instâncias Públicas do Manifesto
           └───────────────────────────┘
                       │ (Falha & feature `cli-fallback` ativa)
                       ▼
           ┌───────────────────────────┐
           │ Tier 3: CLI yt-dlp/ffmpeg │ ──► yt-dlp (android) + remux copy (-c:a copy)
           └───────────────────────────┘

### 6.1 Sobrevivência Single-Dev: Manifesto Remoto Quente (`extractor-manifest.json`)
Para evitar que alterações nos desafios anti-bot do YouTube exijam recompilação do APK Android ou do binário Rust:
- Ao iniciar (e a cada 12 horas), a `lacraia-core` sincroniza de forma assíncrona o arquivo `https://lacraia.2lp.in/extractor-manifest.json` (mantendo cópia em cache no SQLite para operação offline).
- O manifesto define dinamicamente os clientes da API `youtubei/v1/player` que não exigem decifração de JS (ex: `ANDROID_VR`, `IOS`, `ANDROID_TESTSUITE`), seus respectivos `clientVersion`, `clientId`, strings de `User-Agent` e a lista atualizada de endpoints Piped/Invidious de fallback.

### 6.2 Ingestão Zero-Transcode e Compatibilidade Nativa de Containers
- **No Mobile e Desktop Standalone:** Os fluxos de áudio extraídos no Tier 1 e Tier 2 (`itag 251` Opus em container WebM ou `itag 140` AAC/M4A) são baixados via requisições de janela (`Range: bytes=0-10485759`) com concorrência limitada a no máximo 2 downloads simultâneos com jitter, sendo persistidos diretamente em disco sem invocação do `ffmpeg`. Os motores de áudio modernos (HTML5 `<audio>`, Android `Media3`/`ExoPlayer` e WebKit) decodificam WebM/Opus nativamente.
- **No Servidor (`cli-fallback`):** Quando o `ffmpeg` está presente, fluxos nativos em Opus utilizam cópia direta de stream (`-c:a copy`), concluindo o remux em menos de 300 ms sem re-encoding por software.

---

## 7. Dimensionamento Estatístico de Desconexão e `QueueDaemon` Temporal

A fila de lookahead é dimensionada por **tempo acumulado de reprodução** em vez de número fixo de arquivos, baseando-se na distribuição estatística de zonas de sombra celular em rodovias (60 a 80 km/h):

| Percentil (P) | Extensão sem Sinal | Duração da Queda | Cobertura Garantida pela La Craia |
| :--- | :--- | :--- | :--- |
| **P50 (Mediana)** | 2 a 4 km | 3 a 5 min | Coberto com folga |
| **P80** | 12 a 16 km | 12 a 18 min | Coberto pelo Modo Econômico Mobile (20 min) |
| **P90** | 25 a 30 km | 25 a 35 min | Coberto pela Janela Padrão (60 min) |
| **P95 a P99** | 40 a 60 km | 40 a 60 min | **Coberto pela Janela Padrão de Cache (`3600s` / ~57 MB)** |

### 7.1 Regras Operacionais do `QueueDaemon`
1. **Horizonte de Fila Remota (`target_queue_duration_sec = 7200`):** Quando o modo `non_stop` está ativo, o Autopilot consulta o grafo de similaridade multi-semente (`find_similar_multi` usando até 5 sementes recentes) para manter sempre **2 horas (7.200s)** de faixas enfileiradas à frente da música atual.
2. **Horizonte de Cache Local (`target_cache_duration_sec = 3600`):** Pré-carrega os arquivos de áudio em disco até acumular **1 hora (3.600s)** de áudio físico pronto para tocar. Em redes móveis medidas (4G/5G sem Paredão Matriz), se `metered_network_mode` estiver ativo, reduz o horizonte de pré-download para `1200s` (20 minutos, cobrindo acima de P80) e restaura para `3600s` ao detectar Wi-Fi.
3. **Prioridade de Metadados e Letras (LRCLIB):** No instante em que uma faixa entra na fila, o arquivo `.lrc` do LRCLIB é resolvido e salvo no SQLite local antes mesmo do término do download do áudio, garantindo que o HUD *Simbiótyka* nunca fique sem letra sincronizada na serra.
4. **Fallback Offline de Afinidade:** Caso o aparelho entre em zona de sombra prolongada (acima de 1 hora) e esgote os itens `cached` da fila antes do sinal voltar, o `QueueDaemon` executa um *Weighted Random Walk* restrito aos arquivos já presentes no acervo de 1 GB do aparelho (`WHERE t.cached_path IS NOT NULL`), garantindo zero segundos de silêncio.

---

## 8. Protocolo WebSocket, Controle de Cabines (RBAC) e Visualizador de 32 Patas

As seções de sincronização em tempo real, segurança de cabines e interface visual seguem os contratos normativos estabelecidos:
- **Hierarquia RBAC de 3 Níveis (`ws.rs`):** `MASTER` (`LACRAIA_MASTER_KEY`), `PILOT` (`pilot_token`) e `PASSENGER` (`passenger_key` rotativa de alta entropia).
- **Tranca de Porta e Alavanca de Ejeção:** Controle `is_locked`, manifesto em tempo real via `CREW_UPDATE` e ejeção atômica individual (`EJECT_PASSENGER`) ou geral (`EJECT_ALL`) com tela de bloqueio temática.
- **HUD de Estrangulamento (`CACHE_ALERT`):** Alerta visual distinguindo `local_cache_missing` e `remote_buffer_starved`.
- **Analisador de Espectro de 32 Patas Metaméricas:** Conectado via Web Audio API (`fftSize: 128`, decaimento inercial `0.88`), chave tátil `[ ⚡ PATAS: ON / OFF ]` e respeito a `prefers-reduced-motion: reduce`.
- **Isolamento de Gestos e Contenção Horizontal Mobile:** Painéis ancorados com `max-width: 100vw; overflow-x: hidden; min-width: 0;`, controles com `touch-action: manipulation;` e navegação estrita por abas (`#mobile-tabs`) sem conflitos de swipe.

---

## 9. Pirâmide de Testes, CI/CD e Distribuição Própria (A Pratilera)

1. **Pirâmide de Qualidade (4 Tiers):**
   - **Tier 1 (Fast Gate Rust):** `cargo fmt -- --check`, `cargo clippy -- -D warnings` e `cargo test --workspace` (incluindo testes unitários do cálculo temporal de 3600s/7200s e do score de expurgo LRFU).
   - **Tier 2 (Smoke E2E):** `node tests/e2e/run_tests.js --smoke` (< 5s).
   - **Tier 3 (Canonical Regression E2E):** 19 cenários em Headless Chrome cobrindo cabines, áudio, jukebox e janela deslizante.
   - **Tier 4 (Exhaustive Stress Suite):** `node tests/e2e/run_tests.js --exhaustive` (fuzzing, estresse multi-cabine e soak test) acionado via `.forgejo/workflows/e2e-exhaustive.yml`.
2. **Distribuição Própria Android e Web:**
   - Compilação cruzada da `liblacraia_core.so` via NDK e assinatura de produção dos pacotes `in.twolp.lacraia.app` e `in.twolp.lacraia.simbiotyka` com a chave própria `in_twolp` (APK Signature Scheme v2) e ícones vetoriais dedicados (8-bit RGBA).
   - Publicação automática dos binários no bucket Cloudflare R2 (`twolp-bucket/lacraia.apk` e `twolp-bucket/simbiotyka.apk`) e disparo de webhook para atualização do índice F-Droid/Droid-ify na loja oficial **A Pratilera** (`https://banca.2lp.in/pratilera`).
   - Deploy dual do portal e do `extractor-manifest.json` no Cloudflare Workers (`https://lacraia.2lp.in`) e Codeberg Pages.

---
