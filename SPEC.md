# La Craia Technical Specification (SPEC-01)
**Normative Standard:** GOST `1.0.0-draft` (General Open Source Standard)  
**Implementation Profile:** `profiles/lumen.md` (Lumen Ecosystem)  
**Specification Version:** `1.0.0`  
**Status:** Official / Standard  
**License:** GNU General Public License v3.0 (GPL-3.0) — Strategic Copyleft Safeguard  
**Author:** Lumen Pink <hi@lumen.pink>  
**Repository:** `ssh://git@codeberg.org/lumenpink/lacraia.git`  
**Documentation Portal:** `https://lacraia.2lp.in`

---

## 1. Scope and Anti-Corporate Audio Philosophy

This specification defines the system architecture, API contracts, data models, audio resolution pipelines, telemetry protocols, access control mechanisms, and quality standards of **La Craia**.

**La Craia** is an autonomous digital soundwall (*paredão digital*), continuous acoustic affinity radio, and symbiotic heads-up display (HUD) lyric companion engineered for high-power sound systems, street parties, domestic listening, Linux workstations, and mobile or vehicular environments.

### Core Tenets
1. **Acoustic Sovereignty:** Total rejection of ad-saturated commercial streaming platforms, closed algorithmic recommendation traps, and telemetry surveillance. The audio pipeline operates autonomously using local caches, peer stations, and sanitized public endpoints.
2. **Continuous Flow (Zero Dead Air):** An automated lookahead queue daemon continuously resolves, pre-caches, and buffers forthcoming tracks in high-fidelity Opus format, eliminating gap delays between songs.
3. **Tactile & Brutalist Visibility:** High-contrast mechanical interfaces inspired by Latin American street sound systems (*paredões de som*), workshop machinery, and the subversive stage performances of funk icon Lacraia. Interfaces prioritize oversized controls, direct responsiveness, and zero corporate aesthetic blandness.
4. **Symbiotic Multiplatform Integration:** Unified backend core serving native Linux desktops, mobile web PWAs, and an Android floating companion (*Simbiótyka*) that renders line-by-line synchronized lyrics and guitar chords over third-party applications without stealing window focus.

---

## 2. Repository Anatomy and GOST Conformance

The repository conforms strictly to Sections 2, 3, and 8 of the GOST standard:

```
.
├── LICENSE                     # Strong Copyleft (GNU General Public License v3.0)
├── VERSION                     # Semantic versioning flat string (e.g., 0.2.0-dev.63)
├── CHANGELOG.md                # Human-readable change history (Keep a Changelog)
├── README.md                   # Executive overview, installation, and biographical tribute
├── SPEC.md                     # This normative technical specification (SPEC-01)
├── .cccprc                     # CCCP governance, hooks, and commit compliance configuration
├── Cargo.toml                  # Cargo root workspace manifest (Rust Edition 2021)
├── packages/
│   ├── lacraia-shared/         # Domain structs, LRC parser, fuzzy search & WebSocket protocol
│   ├── lacraia-core/           # Tokio/Axum server, SQLite store, QueueDaemon & resolver
│   ├── lacraia-app/            # Multiplatform console player (Dioxus Linux desktop & WASM/PWA)
│   └── lacraia-android/        # Android Gradle workspace (:app & :simbiotyka modules)
├── site/                       # Public documentation showcase and web console
│   ├── index.html              # Main portal, manifesto, download hub & antennae kinematics
│   ├── _headers                # Strict HTTP security headers (nosniff, DENY, strict-origin)
│   ├── assets/                 # Favicon SVG, brand logos, and 1200x630 social preview card
│   └── app/                    # Standalone PWA soundwall console (/app)
├── tests/
│   └── e2e/                    # Headless Playwright automated browser test suite
├── docs/                       # Architectural diagrams, design system & release roadmaps
└── scripts/                    # Helper scripts (dev.sh, start.sh, demo.sh, og_image generation)
```

### 2.1 Licensing Exception Justification
In accordance with Section 2.2 of the GOST standard, La Craia exercises the strategic copyleft exception by adopting the **GNU General Public License v3.0 (GPL-3.0)** rather than GPL-3.0-or-later. This copyleft protection ensures that the software, custom audio playback algorithms, and hardware integration interfaces remain permanently open-source, preventing proprietary forks, paywalled repackaging, or enclosed commercialization by streaming cartels.

---

## 3. Workspace Architecture

The workspace is organized into modular Rust crates and an Android Kotlin multi-module hierarchy:

### 3.1 `packages/lacraia-shared` (Rust Crate)
- **Role:** Pure data contracts, protocol serialization, and shared domain algorithms.
- **Components:**
  - `protocol`: WebSocket message envelopes (`WsMessage`, `Command`, `StateUpdate`, `CrewUpdate`, `EjectedNotification`).
  - `lrc`: Timestamped lyrics parser supporting synchronized line-by-line karaoke rendering.
  - `fuzzy`: Diacritic stripping, fuzzy title matching, and duration delta tolerance filters.

### 3.2 `packages/lacraia-core` (Rust Crate)
- **Role:** Central daemon and audio streaming engine running on Linux host machines.
- **Components:**
  - `db`: SQLite persistence (`lacraia.sqlite`) managing known tracks, adjacency similarity graphs, cabins, passengers, and lookahead queues.
  - `resolver`: Three-tier audio extraction engine pairing Last.fm metadata, LRCLIB lyrics, and yt-dlp media extraction with Android client spoofing.
  - `queue_daemon`: Autonomous background lookahead worker maintaining a constant 5-track cache buffer.
  - `ws`: Real-time WebSocket telemetry hub dispatching playback state and RBAC commands.
  - `api`: Axum HTTP REST routing (`/api/radio`, `/api/resolve`, `/api/search`, `/api/cabin/*`, `/cache/*`).

### 3.3 `packages/lacraia-app` (Rust / Dioxus Crate)
- **Role:** Soundwall console player supporting native Linux desktop binaries (via Wry/WebKitGTK) and progressive web app targets.
- **Components:**
  - `audio`: Audio engine abstraction managing local media element sources, volume normalization, and MediaSession metadata.
  - `ui`: Tactile brutalist interface components, gain potentiometers (0 to 11), queue decks, and `VisualizerState`.

### 3.4 `packages/lacraia-android` (Kotlin Android Multi-Module)
- **Role:** Native Android applications derived from the `lacraia.2lp.in` domain.
- **Modules:**
  - `:app` (`in._2lp.lacraia.app`): Full-screen tactile console and crew jukebox (`lacraia-app.apk`).
  - `:simbiotyka` (`in._2lp.lacraia.simbiotyka`): Floating lyric HUD overlay companion running as a Foreground Service (`simbiotyka.apk`).

---

## 4. SQLite Storage and Acoustic Similarity Graph

`lacraia-core` maintains an internal SQLite database (`lacraia.sqlite`) governing offline resilience, catalog indexing, and track affinity traversal:

* **`tracks`:** Catalog metadata (`id`, `artist`, `title`, `duration_sec`, `cached_path`, `genres`, `last_played_at`).
* **`similarity_edges`:** Directed weighted adjacency graph storing transition affinities (`source_track_id`, `target_track_id`, `weight`).
* **`stations`:** Seed definitions and acoustic profiles (e.g., MPB & Samba, Synthwave Drive, Rock Clássico, Lo-Fi Road Trip, Paredão Autônomo).
* **`cabins`:** Multi-tenant cabin namespaces with security tokens (`id`, `name`, `master_key`, `pilot_token`, `passenger_key`, `is_locked`).
* **`passengers`:** Connected passenger registry with session identifiers, nicknames, and device signatures.
* **`queue`:** Active cabin queues storing track references, requester attribution (`requested_by`), cache state (`Pending`, `Cached`, `Playing`, `Failed`), and playback order.

---

## 5. Audio Pipeline, Hybrid Resolver & Autopilot Lookahead

Audio streaming employs a resilient, three-tier hybrid resolution architecture:

```
[ Incoming Track Request / Autopilot Seed ]
                   │
                   ▼
       ┌───────────────────────┐
       │ Tier 1: Local Cache   │ ──(HIT: file exists)──► Immediate HTTP 206 Serve
       └───────────────────────┘
                   │ (MISS)
                   ▼
       ┌───────────────────────┐
       │ Tier 2: In-Flight Deduplication │ ──(In Progress)──► Await Broadcast Event
       └───────────────────────┘
                   │ (New Download)
                   ▼
       ┌───────────────────────┐
       │ Tier 3: Remote Resolver│ ──► yt-dlp (android client) + ffmpeg transcode
       └───────────────────────┘
                   │
                   ▼
       Opus Audio Store (/cache/<id>.opus) + SQLite Track Indexing
```

1. **Tier 1 (Instant Local Cache):** If `<id>.opus` exists in `LACRAIA_CACHE_DIR`, audio is served immediately via RFC 7233 HTTP Range requests with zero network latency.
2. **Tier 2 (In-Flight Mutex Deduplication):** Concurrent requests for the same track ID share an active Tokio `broadcast` channel, avoiding duplicate downloads.
3. **Tier 3 (Sanitized Media Extraction):** yt-dlp extracts raw audio streams using `--extractor-args "youtube:player_client=android"` (mitigating HTTP 403 Forbidden restrictions) and transcodes to 48kHz Opus via `ffmpeg`.
4. **Autonomous Lookahead Daemon (`QueueDaemon`):**
   - Polls every 3 seconds to inspect active cabin queues.
   - Guarantees an active lookahead horizon of 5 cached tracks ahead of the currently playing song.
   - Automatically skips and marks unresolvable items as `Failed`, preventing infinite download loops.

---

## 6. WebSocket Protocol and 3-Tier Access Hierarchy

Real-time telemetry and cabin synchronization operate over bidirectional WebSocket connections (`/ws`).

### 6.1 Role-Based Access Control (RBAC)
* **Master (`MASTER` / `LACRAIA_MASTER_KEY`):** Universal administrative authority across all cabins. Can provision cabins, rotate passenger keys, and command playback fleet-wide.
* **Pilot (`PILOT` / `pilot_token`):** Exclusive captaincy over a specific cabin. Authorized to command `PLAY`, `PAUSE`, `NEXT`, `PREVIOUS`, `SEEK`, `LOCK_CABIN`, `ROTATE_KEY`, `EJECT_PASSENGER`, `EJECT_ALL`, and `QUEUE_REMOVE`.
* **Passenger (`PASSENGER` / `passenger_key`):** Crew member with shared jukebox access. Permitted exclusively to enqueue songs via `QUEUE_APPEND`. Unauthorized command attempts are rejected with cryptographic error frames.

### 6.2 Door Lock Mechanism and Immediate Ejection
* **Door Rocker Switch (`is_locked`):** When locked, onboarding endpoints (`POST /api/cabin/join`) and WebSocket handshakes reject new connections even if the passenger key is known.
* **Instant Key Rotation (`POST /api/cabin/rotate-key`):** The pilot can invalidate existing passenger invite links and QR codes instantaneously without interrupting playback.
* **Crew Manifesto & Ejection Lever:** Real-time roster broadcasts (`CREW_UPDATE`) keep the console informed of all connected devices. The pilot can eject individual passengers (`EJECT_PASSENGER`) or clear the cabin (`EJECT_ALL`), immediately severing their session and displaying a brutalist lockout notification.

---

## 7. Web Audio API and 32-Metameric Leg Spectrum Visualizer

The soundwall web console (`site/app/index.html`) and desktop frontend integrate a real-time frequency visualizer:

1. **AudioContext Analyser Graph:** Connects `createMediaElementSource(audioEngine)` to an `AnalyserNode` (`fftSize: 128`, 64 frequency bins, smoothing 0.8) and routes to `destination`.
2. **Lazy Initialization:** `AudioContext` initializes or resumes on the user's first interactive gesture (click, tap, or playback command), complying with browser autoplay restrictions.
3. **32 Metameric Legs Mapping:**
   - *Legs 0..10 (Front Claws):* Sub-bass and low frequencies (spikes trigger white highlights and biomechanical claw pulses).
   - *Legs 11..21 (Thorax):* Vocal and mid-range frequencies.
   - *Legs 22..31 (Rear Forcípulae):* High frequencies and cymbals.
4. **Inertia Falloff:** Amplitudes employ a smooth decay algorithm (`val = val * 0.88 + target * 0.12`), preventing jitter and simulating organic muscle movement.
5. **Tactile Power Switch (`[ ⚡ PATAS: ON / OFF ]`):** Cancels the `requestAnimationFrame` render loop on demand to preserve CPU/GPU battery on mobile devices.
6. **Accessibility:** Automatically disables continuous rendering when `prefers-reduced-motion: reduce` is detected.

---

## 8. Test Quality Strategy and Testing Pyramid

In accordance with Section 6 of the GOST standard, La Craia enforces a multi-tiered testing strategy:

```
                  ┌────────────────────────┐
                  │ E2E Playwright Tests   │  7 Scenarios (Headless Chrome)
                  ├────────────────────────┤
                  │ Integration Tests      │  5 Tests (Axum API + SQLite + yt-dlp)
                  ├────────────────────────┤
                  │ Unit Tests (Rust)      │  32 Tests (Shared, Core, App)
                  └────────────────────────┘
```

1. **Unit Tests (32 tests):**
   - `lacraia-shared` (9 tests): LRC parser edge cases, diacritic stripping, fuzzy title matching, and protocol serialization.
   - `lacraia-core` (14 tests): SQLite schema migrations, similarity graph queries, RBAC dispatch, and lookahead window selection.
   - `lacraia-app` (9 tests): Audio engine lifecycle, gain slider normalization to 11, time formatting, and `VisualizerState` lifecycle.
2. **Integration Tests (5 tests):**
   - End-to-end Axum API resolution and static Opus cache delivery (`/cache/`).
   - Search query sanitization and operator safety.
   - Autopilot infinite lookahead queue maintenance.
   - Queue daemon failure isolation (`QueueState::Failed`).
   - Real resolver download with Android client parameters.
3. **Automated End-to-End Playwright Suite (`tests/e2e/run_tests.js`):**
   - Executed against headless Google Chrome in an ephemeral incognito context.
   - **Test 1:** Desktop Console load, title validation, and 3-deck architecture layout.
   - **Test 2:** Jukebox catalog search, debounced autocomplete dropdown, and track queuing.
   - **Test 3:** Audio engine transport controls, gain slider scaling to 11, and time displays.
   - **Test 4:** Mobile viewport responsiveness (375×667), sticky miniplayer, and bottom navigation tabs.
   - **Test 5:** Autopilot lookahead buffer validation (5 tracks maintained continuously).
   - **Test 6:** Vector favicon branding and biomechanical antennae kinematics.
   - **Test 7:** WebAudio spectrum visualizer canvas, 32 metameric legs, and ON/OFF toggle switch.

---

## 9. Static Analysis and Zero Warnings

In compliance with Section 7 of the GOST standard:
* Rust compilation and linter suites execute with strict warning enforcement (`cargo clippy --all-targets --all-features -- -D warnings` and `cargo fmt -- --check`).
* Zero compiler warnings are permitted on the `main` branch.
* External dependencies are pruned and audited for security vulnerabilities via `cargo audit`.

---

## 10. Public Web Showcase and HTTP Security Headers

The public documentation portal and web application reside under `./site`:
1. **Pure Web Technologies:** HTML5, CSS3, vanilla JavaScript, and SVG vector graphics without mandatory bundler dependencies.
2. **HTTP Security Headers (`site/_headers`):**
   ```http
   /*
     X-Content-Type-Options: nosniff
     X-Frame-Options: DENY
     Referrer-Policy: strict-origin-when-cross-origin
   ```
3. **PWA Capabilities:** Standalone web application manifest (`site/app/manifest.json`) and service worker shell caching (`site/app/sw.js`).

---

## 11. Continuous Integration and Dual Deployment (CI/CD)

The pipeline (`.forgejo/workflows/ci.yml`) enforces the GOST separation of concerns:
* **Job `test`:** Runs on all branches and pull requests:
  - Formatter check (`cargo fmt -- --check`)
  - Static analysis (`cargo clippy --all-targets --all-features -- -D warnings`)
  - Workspace test suite execution (`cargo test --workspace`)
  - Playwright E2E browser test execution (`node tests/e2e/run_tests.js`)
* **Job `deploy-site`:** Conditioned on `needs: [test]` and `github.ref == 'refs/heads/main'`:
  - Primary Edge: Cloudflare Workers deployment to `https://lacraia.2lp.in` via Wrangler.
  - Fallback Redundancy: Automated publication to Codeberg Pages (`https://lumenpink.codeberg.page/lacraia/`).

---

## 12. Social Preview and Safe Zone 4:3 (KokoroSim Standard)

To prevent mobile chat truncation and aggressive 1:1 cropping across WhatsApp, Telegram, Discord, and Twitter/X:
* **Dimensions:** Exactly 1200 × 630 pixels (`site/assets/og_preview.png`).
* **Safe Zone 4:3:** All critical typographic elements, logotypes, and badges are centered within a protected 720px × 500px area.
* **Lateral Bleeds:** 240px lateral borders contain solely background textures (asphalt/rockplate and diagonal hazard hatching).
* **Metadata Length Bounds:**
  - `<meta property="og:title">`: 50 to 65 characters (maximum 70).
  - `<meta property="og:description">`: 120 to 155 characters (maximum 160).
