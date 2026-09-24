# Technical Specification: La Craia Ecosystem

**Project:** La Craia (Decentralized Automotive Audio & Heads-Up Display)  
**Version:** 0.1.0  
**Target Environment:** Linux Host Server (Debian 13 / Ubuntu) & Android Mobile  
**Repository:** `ssh://git@codeberg.org/lumenpink/lacraia.git`  
**Documentation Site:** `lacraia.2lp.in` (Cloudflare Workers Static Assets)

---

## 1. Project Overview & Mission

**La Craia** is an integrated automotive audio streaming suite, continuous radio powered by acoustic similarity graphs, and real-time synchronized heads-up display (HUD) lyrics projected line-by-line over navigation apps (Waze).

The ecosystem eliminates ad-saturated commercial streaming interfaces, text-based recommendation traps, and cognitive sensory overload, prioritizing acoustic continuity, local control, and driver focus on road traffic.

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
* **`packages/lacraia-app` (Rust / Dioxus Crate):** Automotive cockpit player supporting both native Linux desktop binaries (via Wry/WebKitGTK) and progressive web app targets (WASM/PWA). Acts as the track library, playlist controller, and audio engine.
* **`packages/lacraia-android` (Android Kotlin Multi-Module Workspace):**
  * **`:app` (`in._2lp.lacraia.app`):** Native Android Cockpit & Jukebox application (`lacraia-app.apk`) with full-screen automotive immersion, MediaSession background playback, cabin onboarding and crew interaction.
  * **`:simbiotyka` (`in._2lp.lacraia.simbiotyka`):** Floating HUD companion (`simbiotyka.apk`) running as an Android Foreground Service. Intercepts system media notifications, fetches synced `.lrc` lyrics from LRCLIB, and renders a floating pill HUD over GPS navigation apps via `WindowManager`.

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

`lacraia-core` resolves raw audio streams and extracts clean Opus tracks for offline delivery:

1. **Instant SQLite Cache Verification:**
   Before firing external search queries, the resolver inspects `tracks.cached_path` in SQLite. If the `.opus` file exists on disk, it returns immediately with zero network latency.
2. **Search & Fuzzy Sanitization:**
   Executes `yt-dlp` / YouTube search with JSON output parsing:
   * **Duration Discrepancy Filter:** Rejects results where $\Delta t = |t_{\text{obtained}} - t_{\text{expected}}| > 15\text{s}$.
   * **Title Noise Filter:** Drops items containing `live`, `ao vivo`, `cover`, `tribute`, `10 hours`, or unrequested `remix`.
3. **Extraction & Conversion:**
   Downloads and converts audio directly into `/var/cache/la-craia/{track_id}.opus` via `yt-dlp` and `ffmpeg`.
4. **Static Range Delivery:**
   Exposed over `/cache/{track_id}.opus` with native HTTP Range Request support (`206 Partial Content`) for immediate seeking in the frontend player.

> 🦂 **[ VENENO NA CARNE ]**  
> Áudios com discrepância temporal ou títulos adulterados por gravações piratas de celular em shows são descartados sem piedade. O filtro de pureza garante a versão de estúdio original.

---

## 6. Lookahead Queue Daemon & Gapless Playback (`queue_daemon.rs`)

`lacraia-core` spawns an asynchronous background worker (`QueueDaemon`) tasked with maintaining continuous audio on the road:

1. **Active Playhead Tracking:**
   Continuously monitors queue states (`playing`, `cached`, `pending`) in SQLite.
2. **Two-Track Lookahead Window:**
   Identifies the next 2 tracks immediately subsequent to the currently playing song that are still in `pending` state.
3. **Proactive Background Caching:**
   Triggers `MediaResolver::resolve_and_cache` before the current track finishes. Once the `.opus` file is saved to disk, the database updates the queue state from `pending` to `cached`.
4. **Resilience to Network Drops:**
   By maintaining two fully resolved tracks cached in advance, the driver experiences zero playback interruptions even when passing through cellular dead zones.

> ⚡ **[ FIO DESENCAPADO ]**  
> Caso a conexão celular caia abruptamente no meio de um túnel ou serra, o QueueDaemon recua para o acervo local em cache, garantindo zero segundos de silêncio na cabine.

---

## 7. Audio Engine & Multiplatform Media Integration (`lacraia-app`)

`lacraia-app` acts as an automotive cockpit application supporting native Linux desktop and PWA targets:

1. **HTML5 & Native Audio Engine (`audio.rs`):**
   Manages playback with support for volume clamping, precise millisecond seeking, and error recovery across native desktop (Wry) and web targets.
2. **MediaSession API Synchronization:**
   Registers active song metadata (`title`, `artist`, `album`, and 512x512 artwork) directly on `navigator.mediaSession` when running in browser / PWA mode.
3. **OS Lockscreen & Keybinding Handlers:**
   Registers system action handlers for `play`, `pause`, `nexttrack`, `previoustrack`, and `seekto`, enabling hardware vehicle steering wheel buttons and lockscreen controls to operate seamlessly.

---

## 8. Cockpit Player UI, Queue Controller & PWA Capabilities (`lacraia-app`)

The automotive frontend is optimized for driver touch targets and cockpit display visibility:

1. **Automotive Cockpit UI (`ui.rs` & `main.rs`):**
   * High-contrast dark theme (`#121212` background, vibrant accent `#ff007f`) avoiding glare while driving at night.
   * Large touch targets (buttons $\ge 48\text{dp}$) for previous, play/pause, next track, and volume slider.
   * Prominent elapsed time and duration readout with millisecond formatting (`format_time`).
2. **Station & Queue Controller:**
   * Station seeds with instant switching (e.g. MPB & Samba, Synthwave Drive, Rock Clássico, Lo-Fi Road Trip).
   * Up-next queue list with active track indicator and direct track selection.
3. **PWA Manifest & Offline Shell Caching (`manifest.json` & `sw.js`):**
   * Configured as standalone web application (`display: "standalone"`) with automotive categories.
   * Service worker caches core application shell (`/`, `/manifest.json`, `/index.html`) while streaming requests (`/stream/*`) and WebSocket traffic pass through dynamically.

---

## 10. Cabin Security, Multi-Cabin Tenants & 3-Tier Access Hierarchy

Lacraia features an automotive multi-cabin security layer designed for vehicles carrying the driver, delegated pilots, and passengers:

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
> Para orquestrar o backend com banco populado, fila antecipada engatada e o reprodutor web do cockpit automotivo aberto no navegador em `/app` com uma única instrução, basta disparar `./start.sh --open`.




