# Technical Specification: La Craia Ecosystem

**Project:** La Craia (Autonomous Soundwall, Acoustic Affinity Radio & Symbiotic Lyric Ecosystem)  
**Version:** 0.2.0  
**Target Environment:** Linux Host Server (Debian 13 / Ubuntu), Web/PWA & Android Mobile  
**Repository:** `ssh://git@codeberg.org/lumenpink/lacraia.git`  
**Documentation Site:** `lacraia.2lp.in` (Cloudflare Workers Static Assets)  
**Normative Standard:** GOST `1.0.0-draft` / SPEC-01 (`SPEC.md`)

> 📐 **[ PADRÃO TÉCNICO GOST ]**  
> Este documento é mantido em paridade normativa estrita com a especificação canônica [**SPEC-01 (`SPEC.md`)**](../SPEC.md), em conformidade com a norma técnica **GOST `1.0.0-draft`** e o **Perfil Lumen** (`profiles/lumen.md`).

---

## 1. Project Overview & Mission

**La Craia** is an autonomous digital soundwall (*paredão*), continuous radio powered by acoustic affinity graphs, and real-time synchronized heads-up display (HUD) lyrics and chords designed for high-power sound systems, street parties, home listening, desktop Linux, and vehicular environments.

The ecosystem eliminates ad-saturated commercial streaming interfaces, paywalled algorithmic traps, and cognitive sensory overload, prioritizing acoustic continuity, decentralized control, and collective immersion across any audio setup.

---

## 2. Module Architecture

The repository is structured as a monorepo with Rust crates and an Android Kotlin micro-app:

```
[ External Data Sources ]
  ├── Last.fm API (Acoustic similarity graph)
  ├── YouTube Audio Stream (Sanitized extraction via yt-dlp / ffmpeg)
  └── LRCLIB API (Synchronized .lrc lyrics metadata)
            │
            ▼
┌────────────────────────────────────────────────────────┐
│  packages/lacraia-core (Linux Host Server / Debian 13) │
│  Runtime: Rust (Tokio + Axum)                          │
│                                                        │
│  ├── /api/radio: Traverses similarity graphs           │
│  ├── /api/resolve: Audio stream extraction & filter    │
│  ├── /cache: Local static Opus store with HTTP Range   │
│  └── /ws: Telemetry hub & command dispatcher           │
└────────────────────────────────────────────────────────┘
       ▲                               ▲
       │ (REST / WebSocket)            │ (MediaSession / WebSocket)
       ▼                               ▼
┌───────────────────────────┐   ┌───────────────────────────────┐
│  packages/lacraia-app     │   │  packages/lacraia-android     │
│  Stack: Rust / Dioxus     │   │  Modules: :app & :simbiotyka  │
│  Target: Linux & WASM/PWA │   │  Language: Kotlin (Android)   │
│  ├── Multiplatform Engine │   │                               │
│  ├── MediaSession Handler │   │  ├── :app (Cockpit & Jukebox) │
│  └── Cockpit Dashboard    │   │  └── :simbiotyka (HUD Overlay)│
└───────────────────────────┘   └──────────────┬────────────────┘
                                               │ Projects over
                                               ▼
                                  ┌─────────────────────────────┐
                                  │ GPS Navigation Apps (Waze,  │
                                  │ Google Maps, Organic Maps)  │
                                  └─────────────────────────────┘
```

### Module Roles

* **`packages/lacraia-shared` (Rust Crate):** Shared domain models, `serde` serialization, WebSocket protocol packets (`StateUpdate`, `Command`), in-memory `.lrc` lyrics parser, and duration/title fuzzy sanitizer.
* **`packages/lacraia-core` (Rust Crate):** Backend orchestrator running on the Linux host server. Handles track resolution, disk caching in Opus, SQLite state storage, and WebSocket broadcasts.
* **`packages/lacraia-app` (Rust / Dioxus Crate):** Soundwall console player supporting both native Linux desktop binaries (via Wry/WebKitGTK) and progressive web app targets (WASM/PWA). Acts as the track library, playlist controller, and audio engine.
* **`packages/lacraia-android` (Android Kotlin Multi-Module Workspace):**
  * **`:app` (`in.twolp.lacraia.app`):** Native Android Console & Jukebox application (`lacraia-app.apk`) with high-contrast tactile immersion, MediaSession background playback, cabin onboarding and crew interaction.
  * **`:simbiotyka` (`in.twolp.lacraia.simbiotyka`):** Floating HUD companion (`simbiotyka.apk`) running as an Android Foreground Service. Intercepts system media notifications, fetches synced `.lrc` lyrics from LRCLIB, and renders a floating pill HUD over any third-party app or navigation interface via `WindowManager`.

---

## 3. SQLite Storage & Local Similarity Graph

`lacraia-core` maintains an internal SQLite database (`lacraia.sqlite`) powering the continuous playback engine and offline resilience:

* **`tracks`:** Known track metadata (`id`, `artist`, `title`, `duration_sec`, `cached_path`, `genres`, `last_played_at`).
* **`similarity_edges`:** Directed weighted adjacency graph storing transition affinities:
  * `source_track_id`, `target_track_id` (foreign keys referencing `tracks`).
  * `weight`: Float from `0.0` to `1.0` representing similarity score from Last.fm (`match`) or MetaBrainz/ListenBrainz.
  * `provider`: Provenance tag (`lastfm`, `listenbrainz`, `composite`).
* **`queue`:** Ordered playback buffer (`id`, `track_id`, `position`, `state`: `pending` | `cached` | `playing` | `played`).

### Offline Weighted Random Walk

When network connectivity is unavailable on the road, `lacraia-core` falls back to querying cached targets from the local graph:

```sql
SELECT t.id, t.artist, t.title, t.cached_path, e.weight
FROM similarity_edges e
JOIN tracks t ON t.id = e.target_track_id
WHERE e.source_track_id = ?1 AND t.cached_path IS NOT NULL
ORDER BY e.weight DESC
LIMIT ?2;
```

---

## 4. Multi-Provider Similarity Engine (`similarity.rs`)

`lacraia-core` coordinates similarity queries through a composite engine with automatic fallback and local SQLite auto-indexing:

1. **`LastFmProvider`:** Queries the Last.fm `track.getsimilar` API using the configured `LASTFM_API_KEY`. Extracts track names, artist names, `match` floating-point scores (0.0 - 1.0), and canonical `mbid` values.
2. **`MetaBrainzProvider`:** Queries the open MusicBrainz API (`https://musicbrainz.org/ws/2/recording`) without requiring proprietary keys, identifying recording IDs and relevance scores.
3. **`CompositeSimilarityEngine`:**
   * Tries Last.fm first when credentials are provided.
   * Seamlessly falls back to MetaBrainz/ListenBrainz when Last.fm is unavailable or rate-limited.
    * **Auto-indexing:** Every candidate returned from online sources is immediately upserted into SQLite (`tracks`) and wired into `similarity_edges` with its weight.
    * **100% Offline Mode:** When the host server has no external internet connectivity, the engine queries the local SQLite similarity graph directly.

> 📡 **[ TRANSMISSÃO DE CABINE ]**  
> A auto-indexação contínua transforma cada consulta online em um novo nó permanente no banco local. Com o tempo, a máquina aprende a discografia do motorista e opera em total autonomia fora do alcance de antenas 4G/5G.

---

## 5. Media Resolution & Local Opus Cache (`resolver.rs`)

`lacraia-core` implements a resilient multi-tier cascaded architecture to resolve raw audio streams and extract clean Opus tracks for offline delivery:

1. **Tier 1 — Instant SQLite & Disk Cache Verification:**
   Before firing external network queries, the resolver inspects `tracks.cached_path` in SQLite. If the `.opus` file exists on disk, it returns immediately with zero network latency.
2. **Tier 2 — Native Rust Innertube Extractor (`rustypipe`):**
   Executes direct, asynchronous queries against YouTube Music and YouTube via the pure Rust crate `rustypipe`. Streams are verified against fuzzy duration ($\Delta t \le 15\text{s}$) and studio title filters, then fetched and converted to Opus via `ffmpeg`.
3. **Tier 3 — CLI Fallback (`yt-dlp` + `ffmpeg` with Android Client):**
   If native extraction encounters upstream signature changes, SABR experiments, rate limits, or botguard challenges, the resolver automatically falls back to `yt-dlp` with `--extractor-args "youtube:player_client=android"` to bypass visionOS/web CDN 403 Forbidden blocks.
4. **Search & Fuzzy Sanitization:**
   * **Operator-Safe Search Queries:** Queries sanitize literal hyphens and dots (`format!("{} {} audio")`) avoiding Google/YouTube search engine negation (`-`) tokens that previously suppressed results.
   * **Title Lexical Matching:** Verifies token overlap (`is_title_matching`) with diacritic stripping (`strip_accents`) to eliminate false-positive candidates with unrelated titles.
   * **Duration Discrepancy Filter:** Rejects results where $\Delta t = |t_{\text{obtained}} - t_{\text{expected}}| > 15\text{s}$.
   * **Title Noise Filter:** Drops items containing `live`, `ao vivo`, `cover`, `tribute`, `10 hours`, or unrequested `remix`.
5. **Static Range Delivery:**
   Exposed over `/cache/{track_id}.opus` with native HTTP Range Request support (`206 Partial Content`) for immediate seeking in the frontend player.

> 🦂 **[ VENENO NA CARNE ]**  
> Áudios com discrepância temporal, operadores de exclusão acidentais ou títulos adulterados são descartados sem piedade. O filtro de pureza léxica garante a versão de estúdio original.

---

## 6. Lookahead Queue Daemon & Multi-Cabin Pre-Caching (`queue_daemon.rs`)

`lacraia-core` spawns an asynchronous background worker (`QueueDaemon`) tasked with maintaining continuous audio on the road:

1. **Active Multi-Cabin Playhead Tracking:**
   Continuously monitors queue states (`playing`, `cached`, `pending`, `failed`) across all registered cabins (`default`, `van-furacao`, etc.) in SQLite.
2. **Five-Track Autopilot Lookahead Buffer:**
   Ensures that for any active cabin, there are at least **5 upcoming tracks** ahead in the queue (`pending` or `cached`). If the queue falls below 5 tracks during active playback, the daemon queries `CompositeSimilarityEngine` using the seed track (current song or last queued item) to discover similar tracks from Last.fm, MetaBrainz, local SQLite similarity edges, and genre affinity fallbacks, automatically enqueuing them with `requested_by = "AUTOPILOT"`.
3. **Failure Containment & Loop Prevention:**
   If media resolution fails for an unavailable or invalid track, `QueueDaemon` immediately transitions the entry to `failed` state via `db.mark_failed(track_id)`. This prevents infinite pre-caching retry loops while preserving cabin playback flow.
4. **Proactive Background Caching:**
   Triggers `MediaResolver::resolve_and_cache` before the current track finishes. Once the `.opus` file is saved to disk, the database updates the queue state from `pending` to `cached`.
5. **Continuous Infinity Playback & Network Resilience:**
   By maintaining a 5-track lookahead window with pre-cached Opus streams, playback never halts. When a song finishes naturally (`audio.ended`) or the pilot clicks Next, the next song plays with 0ms latency, and the autopilot immediately replenishes the 5th upcoming slot, keeping the buffer perpetually full.

> ⚡ **[ FIO DESENCAPADO ]**  
> Caso a conexão celular caia abruptamente no meio de um túnel ou serra, o QueueDaemon recua para o acervo local em cache e sementes do grafo offline, garantindo zero segundos de silêncio na cabine.

---

## 7. Audio Engine & Multiplatform Media Integration (`site/app/index.html` & `lacraia-app`)

`lacraia-app` and the soundwall web console deliver unified, high-power sound across all endpoints:

1. **HTML5 Audio Engine (`audio-engine`):**
   Connects dynamically to `/cache/{track_id}.opus` streams, executing real audio playback (`audio.play()`, `audio.pause()`) in full synchronization with WebSocket telemetries and gain slider controls (clamped from 0 to 11). On-demand stream resolution (`/api/resolve`) runs automatically if a song is promoted before pre-caching finishes.
2. **MediaSession API Synchronization:**
   Registers active song metadata (`title`, `artist`, `album`, and 512x512 artwork) directly on `navigator.mediaSession` when running in browser / PWA mode.
3. **OS Lockscreen & Keybinding Handlers:**
   Registers system action handlers for `play`, `pause`, `nexttrack`, `previoustrack`, and `seekto`, enabling hardware transport buttons, steering wheels, and lockscreen controls to operate seamlessly.
4. **Real-time WebAudio Spectrum Visualizer & 32 Metameric Legs:**
   Employs `AudioContext.createAnalyser()` connected directly to the `<audio id="audio-engine">` media element source. Bins the frequency spectrum into 32 animated neon bars with biomechanical inertia falloff, simulating the rhythmic scurrying of the centipede's legs during heavy bass impacts. Includes a tactile mechanical switch (`[ ⚡ PATAS: ON / OFF ]`) to cancel `requestAnimationFrame` loops on demand, conserving battery and GPU cycles on mobile and embedded devices, and automatically honors `prefers-reduced-motion: reduce`.

---

## 8. Soundwall Console UI, Queue Controller & PWA Capabilities (`lacraia-app`)

The frontend console is optimized for tactile precision, large touch targets, and outdoor/nighttime visibility:

1. **Tactile Console UI (`ui.rs` & `main.rs`):**
   * High-contrast dark theme (`#121212` background, vibrant accent `#ff007f`) avoiding glare and ensuring readability across phones, tablets, laptops, and consoles.
   * Large touch targets (buttons $\ge 48\text{dp}$) for previous, play/pause, next track, and volume slider up to 11.
   * Prominent elapsed time and duration readout with millisecond formatting (`format_time`).
2. **Station & Queue Controller:**
   * Station seeds with instant switching (e.g. MPB & Samba, Synthwave Drive, Rock Clássico, Lo-Fi Road Trip).
   * Up-next queue list with active track indicator and direct track selection.
3. **PWA Manifest & Offline Shell Caching (`manifest.json` & `sw.js`):**
   * Configured as standalone web application (`display: "standalone"`) with media & music categories.
   * Service worker caches core application shell (`/`, `/manifest.json`, `/index.html`) while streaming requests (`/stream/*`) and WebSocket traffic pass through dynamically.

---

## 10. Cabin Security, Multi-Cabin Tenants & 3-Tier Access Hierarchy

Lacraia features a multi-cabin security layer designed for autonomous spaces shared by an audio pilot (or host) and crew members:

1. **3-Tier Role-Based Security Hierarchy:**
   * **Master (`MASTER` / `LACRAIA_MASTER_KEY`):** Universal administrative authority across all cabins. Can create new cabins (`POST /api/cabin/create`), authenticate into any cabin, rotate passenger keys, and command master transport across the entire fleet.
   * **Pilot (`PILOT` / `pilot_token`):** Scoped driver authority exclusively over their specific cabin. Commands `PLAY`, `PAUSE`, `NEXT`, `PREVIOUS`, `SEEK`, `LOCK_CABIN`, `EJECT_PASSENGER`, `EJECT_ALL`, `QUEUE_REMOVE`, and `rotate-key`.
   * **Passenger (`PASSENGER` / `passenger_key`):** Crew member with shared jukebox access. Permitted exclusively to enqueue songs via `QUEUE_APPEND`. Requests to alter transport, skip tracks, or manipulate cabin state are rejected with authorization errors.
2. **High-Entropy Cryptographic Keys & Instant Key Rotation:**
   * Rather than weak, easily brute-forced numerical PINs, cabin passenger access relies on high-entropy cryptographic keys (`key_<entropy>`).
   * **Key Rotation (`POST /api/cabin/rotate-key`):** The Pilot or Master can rotate the cabin's passenger key at any moment. Previous invite links and QR codes are instantly invalidated without renaming the cabin or clearing the queue.
3. **Door Lock Mechanism (`is_locked`):**
   * The cockpit dashboard features a rocker switch (`[ 🔓 PORTAS ABERTAS / 🔒 TRANCADAS ]`).
   * When locked, the HTTP onboarding endpoint (`POST /api/cabin/join`) and WebSocket handshake reject new passengers, preserving cabin privacy and preventing boarding even if a passenger key is known.
4. **Multi-Cabin Isolation with Shared Opus Cache:**
   * Each cabin operates an independent queue and crew roster in SQLite (`cabins`, `passengers`, `queue`).
   * Audio streams resolve into a unified, shared disk cache (`LACRAIA_CACHE_DIR`). If Cabin A and Cabin B request the same song, the second cabin benefits immediately from zero-network local cache retrieval.
5. **Passenger Jukebox & Requester Attribution:**
   * When passengers enqueue tracks, their name/device handle is tagged via `requested_by`.
   * The cockpit queue displays requester badges (e.g. `[ por: @Mari ]` or `[ PILOTO ]`), giving the driver complete provenance of who queued each track.
6. **Crew Manifesto & Ejection Lever:**
   * Real-time roster broadcast (`CREW_UPDATE`) keeps the cockpit aware of all connected passengers.
   * The pilot can kick disruptive passengers individually (`[ 👢 CHUTAR ]`) or clear the entire vehicle (`[ 💥 EJETAR GERAL ]`).
   * Ejected clients receive an immediate `EJECTED` frame, terminate their WebSocket session, and display a brutalist ejection lockout screen.

> 🦂 **[ VENENO NA CARNE ]**  
> Passageiro engraçadinho tentando pular faixa ou pausar a cantiga alheia toma bloqueio instantâneo no WebSocket. No paredão da La Craia, a agulha obedece unicamente ao piloto.

---

## 11. Daemon Execution & Deployment Configuration (`lacraia-core`)

`lacraia-core` is managed as an asynchronous daemon service on the host:

### Environment Variables

| Variable | Description | Default |
|---|---|---|
| `PORT` | HTTP & WebSocket server port | `8080` |
| `LACRAIA_CACHE_DIR` | Directory for resolved Opus audio files | `./cache` |
| `LACRAIA_DB_PATH` | Path to the SQLite storage file | `./lacraia.sqlite` |
| `LACRAIA_MASTER_KEY` | Master administrative secret for universal cockpit access and cabin provisioning | `lacraia_master_2011` |
| `LASTFM_API_KEY` | Optional Last.fm API credential for similarity graph lookups | *None (falls back to MetaBrainz)* |

### Service Startup

```bash
# Standard launch
cargo run -p lacraia-core

# Custom port and production cache paths
PORT=8081 LACRAIA_CACHE_DIR=/var/cache/la-craia LACRAIA_DB_PATH=./lacraia.sqlite cargo run -p lacraia-core
```

> ✨ **[ O PULO DA TRAVESTI ]**  
> Para orquestrar o backend com banco populado, fila antecipada engatada e a bancada web do paredão autônomo aberta no navegador em `/app` com uma única instrução, basta disparar `./start.sh --open`.

---

## 12. End-to-End Browser Testing & Audio Parity (`tests/e2e/`)

To validate real-world cockpit operation and browser-daemon interoperability without touching user browser profiles, `lacraia` includes an automated headless Playwright test suite:

### Test Suite Architecture (`tests/e2e/run_tests.js`)

1. **Isolation & Headless Guarantees:**
   * Launches system Chromium/Chrome (`/usr/bin/google-chrome`) strictly in **headless** mode with isolated arguments (`--no-sandbox`, `--disable-setuid-sandbox`, `--autoplay-policy=no-user-gesture-required`).
   * Every test runs within an ephemeral incognito context (`browser.newContext()`) with designated viewport dimensions, destroying cookies, cache, and storage between suites.
2. **Coverage Areas:**
   * **Desktop Cockpit Layout:** Validates header telemetry, active cabin badges, master pilot status, and three responsive panels (`#deck-01` HUD & cassette, `#deck-02` queue, `#deck-03` cabin & crew).
   * **Jukebox & Queue Management:** Simulates live user input in `#enqueue-input`, confirms debounced autocomplete suggestions in `#search-dropdown`, and asserts WebSocket queue synchronization into `#queue-list-container`.
   * **Audio Engine & Volume Clamping:** Modulates `#gain-slider` across the 0..11 scale, verifies audio element volume clamping (`audio.volume = 1.0`), and triggers audio resolution on transport play (`#btn-play-pause`).
   * **Mobile Viewport (< 860px):** Emulates mobile devices (375x667), verifying sticky bottom navigation tabs (`#mobile-tabs`) and persistent synchronized miniplayers.
3. **Audio Stream URL Parity:**
   * Normalizes camelCase (`streamUrl`) and snake_case (`stream_url`) payload serialization across Rust backend models and frontend JavaScript consumers.

---

## 13. Sliding Lookahead Window & Multi-Seed Non-Stop Autopilot

To guarantee continuous playback and zero dead-air buffer starvation across any environment (soundwalls, vehicles, parties, and severe highway coverage blackouts), `QueueDaemon` and `CompositeSimilarityEngine` operate a dynamic cumulative temporal lookahead window:

1. **Cumulative Temporal Window Thresholds (3600s / 1 Hour):**
   * **Local Cache Horizon ($\sum \text{duration\_sec} \ge 3600\text{ s}$):** Ensures at least 1 hour (3600 seconds) of upcoming audio ahead of active playback is fully resolved and stored on disk as Opus files (`cached_path.is_some()`), consuming ~57 MB in local storage and shielding playback across severe cellular coverage blackouts ($P_{99}$).
   * **Remote Queue Horizon ($\sum \text{duration\_sec} \ge 3600\text{ s}$):** Ensures at least 1 hour of upcoming tracks ahead of active playback are populated in the SQLite queue buffer.
2. **Multi-Seed Recommendation Graph:**
   * Instead of relying on a single seed track, `maintain_autopilot_queue_sliding` collects up to 5 multi-seeds from recent playback history and currently playing tracks.
   * `find_similar_multi` and `get_recommendations_for_multi_seeds` traverse similarity graph edges, accumulating affinity weights and granting multi-seed connection bonuses while stripping diacritics (`strip_accents`) to avoid duplicate recommendations.
3. **Non-Stop Mode & Cabin Control:**
   * Each cabin maintains an independent `non_stop: bool` state, toggleable via WebSocket action (`TOGGLE_NON_STOP` / `SET_NON_STOP`).
   * The cockpit UI provides a high-contrast neon switch (`#btn-toggle-nonstop`) displaying `🔁 NON-STOP: ON` or `⏸ NON-STOP: OFF`.
   * When enabled, the daemon automatically replenishes upcoming slots as playback advances. When disabled, playback ceases once user-enqueued tracks exhaust.

---

## 14. Cache Inventory & Granular Resolution Attempt Telemetry

To prevent blind spots in offline performance and audit track download provenance across multiple fallback tiers, `lacraia-core` maintains structured telemetry tables in SQLite:

1. **`cache_entries` (Physical Cache Inventory):**
   * Stores physical artifact details: `track_id`, `file_path`, `file_size_bytes`, `duration_sec`, `audio_codec` (`opus`), `hit_count`, `last_served_at`, and `created_at`.
   * Automatically upserted on local cache hits and newly completed downloads, incrementing `hit_count` and serving timestamps.
2. **`cache_resolution_attempts` (Execution Attempt Log):**
   * Captures every resolution attempt (successes and failures alike) across all tiers:
     * `tier_0_local`: Local filesystem or database cache hits.
     * `tier_1_direct`: Direct native YouTube Music queries via RustyPipe.
     * `tier_2_ytdlp_web`: Web-client fallback queries.
     * `tier_3_ytdlp_android`: Android client extraction fallback via yt-dlp.
   * Records exact latency (`duration_ms`), exit codes, error classifications (`RUSTYPIPE_FAILED`, `YTDLP_FAILED`), captured command output/stderr, bytes received, and validation status.
3. **Analytical Statistical Queries:**
   * **Success by Attempt Number (`get_attempts_success_distribution`):** Aggregates how many tracks succeed on the 1st attempt vs fallback attempts.
   * **Tier Success Rates (`get_tier_success_rates`):** Computes total attempts, successes, failures, success percentage, and average latency per resolution tier.
   * **Error Diagnostics (`get_top_resolution_errors`):** Groups error occurrences by tier and error code with sample diagnostic output for post-mortem troubleshooting.

---

## 15. Fast Stream Remuxing & Zero-Transcode Audio Ingestion

To support burst prefetching during transient connectivity windows without saturating CPU threads or heating vehicle host hardware:

1. **Native Opus Detection:**
   * `MediaResolver` inspects stream audio codec metadata (`AudioCodec::Opus`) delivered by RustyPipe or yt-dlp (`ba[acodec=opus]/ba`).
2. **Zero-Transcode Stream Copy:**
   * When streams are natively Opus, FFmpeg executes with `-c:a copy` rather than `-c:a libopus -b:a 128k`.
   * Remuxing into local `.opus` container finishes in under 300 milliseconds (up to 600x realtime), slashing CPU usage by over 90% and preserving battery life.
3. **Graceful Fallback Transcode:**
   * Non-Opus streams fall back to software encoding (`-c:a libopus -b:a 128k`) ensuring universal playback compatibility.

---

## 16. Dynamic System Configuration & 2-Hour Queue Horizon

To accommodate varying journey lengths, device memory limits, and user preferences, `lacraia-core` enables real-time tuning of cache and queue horizons:

1. **Default 2-Hour Autonomous Queue (`target_queue_duration_sec = 7200`):**
   * By default, the Autopilot expands the remote queue up to 7,200 seconds (2 full hours) of continuous music, guaranteeing deep blackout resilience ($P_{99+}$).
   * The default local lookahead cache remains set to 3,600 seconds (1 hour, ~57 MB).
2. **Persistent System Settings (`system_settings` Table):**
   * Key-value persistence in SQLite stores `target_queue_duration_sec`, `target_cache_duration_sec`, and `max_disk_cache_mb`.
3. **RESTful Management (`/api/config`):**
   * `GET /api/config`: Inquires current temporal horizons and cache capacity limits.
   * `POST /api/config`: Validates and writes updated parameters, dynamically updating `QueueDaemon` in-flight without interrupting playback.
4. **Cockpit Settings Modal:**
   * Interactive dialog providing ergonomic unit switches (`[Hours, Minutes]` and `[MB, GB]`) with defensive bounds checking.

---

## 17. Multi-Tiered Testing Pyramid & E2E Execution Modes

To guarantee high velocity in development while maintaining complete systemic resilience, testing is strictly decoupled into distinct operational tiers:

1. **Tier 1 — Fast Gate (Rust Workspace, Commit & PR Mandatory):**
   * Commands: `cargo test --workspace` & `cargo clippy --workspace`.
   * Scope: In-memory SQLite state, audio math, protocol serde, sliding window calculation, and mock daemon lifecycle.
   * Latency: Sub-15 seconds. Runs on every commit and pre-push hook without graphical browser or container overhead.

2. **Tier 2 — Smoke E2E (`node run_tests.js --smoke`):**
   * Scope: Fast sanity verification (< 5s). Tests portal loading, cockpit navigation telemetry, and audio engine initialization.
   * Execution: Rapid local check before feature testing.

3. **Tier 3 — Canonical Regression E2E (`node run_tests.js`):**
   * Scope: 19 core end-to-end scenarios covering security/roles, WebSocket protocols, station dials, cabin locks, and sliding lookahead.
   * Execution: Local integration validation during GOST Phase 8 Quality Gate.

4. **Tier 4 — Exhaustive Deep Suite (`node run_tests.js --exhaustive`):**
   * Scope: Heavy automated stress testing:
     * **Jukebox Input Fuzzing:** Automated injection of extreme Unicode, XSS/SQL escape attempts, and long payloads.
     * **Concurrent Multi-Cabin Stress:** Simultaneous multi-passenger contention across isolated cabins (`cab-alpha`, `cab-beta`, `cab-gamma`).
     * **Autopilot Endurance / Soak Simulation:** Long-running simulated playback across dozens of consecutive tracks verifying sliding window lookahead memory stability.
   * Execution: On-demand via `.forgejo/workflows/e2e-exhaustive.yml` (`workflow_dispatch`), nightly schedules, or release tags (`v*`). Decoupled from fast push pipelines.
