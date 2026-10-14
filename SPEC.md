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
2. **Continuous Flow (Zero Dead Air):** An automated lookahead queue daemon continuously resolves, pre-caches, and buffers forthcoming tracks in high-fidelity Opus format across a 3600-second (1-hour) cumulative temporal window, eliminating gap delays between songs and shielding playback across severe highway coverage blackouts ($P_{99}$).
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
  - `protocol`: WebSocket message envelopes (`WsMessage`, `Command`, `StateUpdate`, `CrewUpdate`, `EjectedNotification`, `CacheAlertInfo`).
  - `lrc`: Timestamped lyrics parser supporting synchronized line-by-line karaoke rendering.
  - `fuzzy`: Diacritic stripping, fuzzy title matching, and duration delta tolerance filters.
  - `models`: Shared entity definitions, playback state, cache inventory, and resolution attempt metrics.

### 3.2 `packages/lacraia-core` (Rust Crate)
- **Role:** Central daemon and audio streaming engine running on Linux host machines.
- **Components:**
  - `db`: SQLite persistence (`lacraia.sqlite`) managing known tracks, adjacency similarity graphs, cabins, passengers, playback history, state persistence, cache inventory, and attempt telemetry.
  - `resolver`: Multi-tier audio extraction engine pairing Last.fm metadata, LRCLIB lyrics, native RustyPipe streaming, and yt-dlp media extraction with Android client spoofing.
  - `queue_daemon`: Autonomous background lookahead worker maintaining a continuous 3600-second (1-hour) cumulative cache and queue horizon.
  - `ws`: Real-time WebSocket telemetry hub dispatching playback state, starvation alerts, and RBAC commands.
  - `api`: Axum HTTP REST routing (`/api/radio`, `/api/resolve`, `/api/search`, `/api/history`, `/api/cabin/*`, `/cache/*`).

### 3.3 `packages/lacraia-app` (Rust / Dioxus Crate)
- **Role:** Soundwall console player supporting native Linux desktop binaries (via Wry/WebKitGTK) and progressive web app targets.
- **Components:**
  - `audio`: Audio engine abstraction managing local media element sources, volume normalization, and MediaSession metadata.
  - `ui`: Tactile brutalist interface components, gain potentiometers (0 to 11), queue decks, and `VisualizerState`.

### 3.4 `packages/lacraia-android` (Kotlin Android Multi-Module)
- **Role:** Native Android applications derived from the `lacraia.2lp.in` domain.
- **Modules:**
  - `:app` (`in.twolp.lacraia.app`): Full-screen tactile console and crew jukebox (`lacraia-app.apk`).
  - `:simbiotyka` (`in.twolp.lacraia.simbiotyka`): Floating lyric HUD overlay companion running as a Foreground Service (`simbiotyka.apk`).

### 3.4.1 Canonical Package Architecture & Build Pipeline
- **Unified Identifier:** All Android components are canonically anchored under `in.twolp.*` (`in.twolp.lacraia.app` and `in.twolp.lacraia.simbiotyka`). Legacy references to `in._2lp.*` are formally deprecated and eliminated.
- **Service Binding:** The companion HUD service is canonically declared and bound as `in.twolp.lacraia.simbiotyka.SimbiotykaService`.
- **Packaging & Signing:** Automated via `./gradlew assembleRelease` signed with the sovereign keystore `~/.android/in_twolp_release.keystore` (alias `in_twolp`) via APK Signature Scheme v2.

---

## 4. SQLite Storage and Acoustic Similarity Graph

`lacraia-core` maintains an internal SQLite database (`lacraia.sqlite`) governing offline resilience, catalog indexing, and track affinity traversal:

* **`tracks`:** Catalog metadata (`id`, `artist`, `title`, `duration_sec`, `cached_path`, `genres`, `last_played_at`).
* **`similarity_edges`:** Directed weighted adjacency graph storing transition affinities (`source_track_id`, `target_track_id`, `weight`, `provider`).
* **`stations`:** Seed definitions and acoustic profiles (e.g., MPB & Samba, Synthwave Drive, Rock Clássico, Lo-Fi Road Trip, Paredão Autônomo).
* **`cabins`:** Multi-tenant cabin namespaces with security tokens (`id`, `name`, `master_key`, `pilot_token`, `passenger_key`, `is_locked`).
* **`passengers`:** Connected passenger registry with session identifiers, nicknames, and device signatures.
* **`queue`:** Active cabin queues storing track references, requester attribution (`requested_by`), cache state (`pending`, `cached`, `playing`, `played`, `failed`), and playback order.
* **`cabin_playback_state`:** Persistent cabin playback coordinate (`cabin_id`, `track_id`, `position_ms`, `playback_state`, `updated_at`).
* **`playback_history`:** Historical playback records with cabin and passenger attribution (`id`, `cabin_id`, `track_id`, `passenger_name`, `played_at`, `duration_sec`).
* **`cache_entries`:** Physical cache artifact inventory (`track_id`, `file_path`, `file_size_bytes`, `duration_sec`, `audio_codec`, `hit_count`, `last_served_at`, `created_at`).
* **`cache_resolution_attempts`:** Granular tier attempt and diagnostic logs (`track_id`, `attempt_number`, `tier`, `method`, `command_line`, `success`, `exit_code`, `error_code`, `error_output`, `duration_ms`, `bytes_received`, `validation_status`, `cabin_id`, `created_at`).

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
       │ Tier 3: Remote Resolver│ ──► RustyPipe native / yt-dlp android + remux copy
       └───────────────────────┘
                   │
                   ▼
       Opus Audio Store (/cache/<id>.opus) + SQLite Track Indexing
```

1. **Tier 1 (Instant Local Cache):** If `<id>.opus` exists in `LACRAIA_CACHE_DIR`, audio is served immediately via RFC 7233 HTTP Range requests with zero network latency.
2. **Tier 2 (In-Flight Mutex Deduplication):** Concurrent requests for the same track ID share an active Tokio `broadcast` channel, avoiding duplicate downloads.
3. **Tier 3 (Sanitized Media Extraction):** Primary resolution queries native YouTube Music streams via RustyPipe with zero transcode overhead; fallback triggers yt-dlp with `--extractor-args "youtube:player_client=android"` and fast stream copy remuxing (`-c:a copy`), preserving battery and minimizing latency.
4. **Autonomous Lookahead Daemon (`QueueDaemon`) & 3600s (1h) Temporal Horizon:**
   - Evaluates active cabin queues against a cumulative temporal horizon: $\sum \text{duration\_sec(cache)} \ge 3600\text{ s}$ (1 hour) and $\sum \text{duration\_sec(queue)} \ge 3600\text{ s}$ (1 hour).
   - Ingests candidates via multi-seed affinity graph traversal (`find_similar_multi`) whenever upcoming playtime falls below 3600s, guaranteeing uncompressed offline playback headroom covering severe highway coverage blackouts ($P_{99}$), consuming only ~57 MB of disk.
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
                  │ E2E Playwright Tests   │  18 Scenarios (Headless Chrome)
                  ├────────────────────────┤
                  │ Integration Tests      │  5 Tests (Axum API + SQLite + yt-dlp)
                  ├────────────────────────┤
                  │ Unit Tests (Rust)      │  47 Tests (Shared, Core, App)
                  └────────────────────────┘
```

1. **Unit Tests (47 tests):**
   - `lacraia-shared` (12 tests): LRC parser, diacritic stripping, fuzzy title matching, command deserialization, non-stop mode serialization, starvation alert emit, and cache inventory models.
   - `lacraia-core` (26 tests): SQLite schema migrations, similarity graph queries, multi-seed graph traversal, RBAC dispatch, cabin persistence, playback history, starvation evaluation, cache inventory & attempt diagnostics, and lookahead daemon.
   - `lacraia-app` (9 tests): Audio engine lifecycle, gain slider normalization to 11, time formatting, and `VisualizerState` lifecycle.
2. **Integration Tests (5 tests):**
   - End-to-end Axum API resolution and static Opus cache delivery (`/cache/`).
   - Search query sanitization and operator safety.
   - Autopilot infinite lookahead queue maintenance.
   - Queue daemon failure isolation (`QueueState::Failed`).
   - Real resolver download with Android client parameters.
3. **Automated End-to-End Playwright Suite (`tests/e2e/run_tests.js` - 18 Scenarios):**
   - Executed against headless Google Chrome in an ephemeral incognito context.
   - **Test 1:** Portal (`/`): Favicon SVG, OG metadata, kinematics, and cockpit link.
   - **Test 2:** Cockpit Web (`/app`): Desktop 3-deck architecture and telemetry layout.
   - **Test 3:** Mobile Viewport (375×667): Bottom navigation tabs and sticky miniplayer.
   - **Test 4:** Security & Roles: Passenger restrictions, auth modal triggers, and master key login.
   - **Test 5:** Audio Engine & Sound Output: Playback, currentTime advancement, volume to 11, and visualizer.
   - **Test 6:** Station Dials: Graph dialect switching across all 5 preset stations.
   - **Test 7:** Jukebox & Search: Debounced autocomplete, candidate selection, and form submission.
   - **Test 8:** Autopilot Lookahead: Adding a track triggers automatic population of upcoming slots.
   - **Test 9:** Queue Management: Direct play from queue and individual item removal.
   - **Test 10:** Cabin Lock & Doors: Toggle lock and verify passenger enqueuing is strictly blocked.
   - **Test 11:** Crew & Ejection: Verify crew list, mass ejection signal, and glitch lockout overlay.
   - **Test 12:** Modals System: QR code canvas, key rotation, ESC closing, and cabin creation.
   - **Test 13:** Shortcuts & Sticky Miniplayer: Space, Arrows, transport buttons, and input isolation.
   - **Test 14:** Clean-Slate Lifecycle: Zerada -> Enqueue 1 -> Play -> Pause -> Enqueue 2 -> Jump -> Enqueue 3 -> Next.
   - **Test 15:** Cabin State Persistence: Reloading cockpit resumes last track & position across sessions.
   - **Test 16:** Playback History: Track plays record to `/api/history` and render with replay action.
   - **Test 17:** Buffer Starvation HUD: Displays alert banner overlay distinguishing local vs remote starvation.
   - **Test 18:** Lookahead Sliding Window & Non-Stop Mode: 3600s temporal headroom with non-stop toggle.

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

---

## 13. Sovereign Android Distribution via A Pratilera (F-Droid / Droid-ify)

To ensure autonomous, self-updating delivery across smartphones and vehicular soundwalls without gatekeeping by Google Play Store:
1. **Canonical Package Identifiers:**
   - Console & Jukebox: `in.twolp.lacraia.app` (`lacraia-app.apk`)
   - Symbiotic Lyric HUD: `in.twolp.lacraia.simbiotyka` (`simbiotyka.apk`)
2. **Sovereign Signing:**
   - Production and continuous APKs are signed using the official key (`in_twolp`) backed by `~/.android/in_twolp_release.keystore` and verified via `apksigner`.
3. **Central Store Integration (A Pratilera na Banca):**
   - Official Store URL: `https://banca.2lp.in/pratilera`
   - Canonical HTTPS Entry: Droid-ify and Neo Store repository form input requires `https://banca.2lp.in/pratilera` (avoiding input scheme rejection).
   - One-touch clipboard copy button provided in web interface for instant addition.
   - Friendly aliases with automatic 301 redirect: `/patilera`, `/prateleira`, `/repo`.
4. **Autonomous Distribution Endpoint & Headers (`https://lacraia.2lp.in`):**
   - Direct APK downloads remain hosted as fallback targets: `https://lacraia.2lp.in/lacraia-app.apk` and `https://lacraia.2lp.in/simbiotyka.apk`.
   - MIME type headers configured via `site/_headers` (`application/vnd.android.package-archive` and `Access-Control-Allow-Origin: *`).
   - Version manifests (`site/version.json` and `site/version-nightly.json`) preserve compatibility with Obtainium and standalone JSON updaters.

---

## 14. Android Visual Identity and Launcher Metadata (Ticket #34)

To eliminate generic default Android icons and ensure native, professional presentation across Android Launchers, App Drawers, and sovereign stores like Droid-ify / A Pratilera:
1. **Dedicated Resource Hierarchy:**
   - Both `packages/lacraia-android/app` (`in.twolp.lacraia.app`) and `packages/lacraia-android/simbiotyka` (`in.twolp.lacraia.simbiotyka`) must provide dedicated mipmap density assets under `src/main/res/`:
     - `mipmap-mdpi`: 48 × 48 px (`ic_launcher.png`, `ic_launcher_round.png`)
     - `mipmap-hdpi`: 72 × 72 px (`ic_launcher.png`, `ic_launcher_round.png`)
     - `mipmap-xhdpi`: 96 × 96 px (`ic_launcher.png`, `ic_launcher_round.png`)
     - `mipmap-xxhdpi`: 144 × 144 px (`ic_launcher.png`, `ic_launcher_round.png`)
     - `mipmap-xxxhdpi`: 192 × 192 px (`ic_launcher.png`, `ic_launcher_round.png`)
2. **Round Icon Masking:**
   - Circular variants (`ic_launcher_round.png`) are cropped with an anti-aliased circular alpha mask to conform with Android 7.1+ launcher standards.
3. **Manifest Integration:**
   - `AndroidManifest.xml` in each module must declare:
     ```xml
     android:icon="@mipmap/ic_launcher"
     android:roundIcon="@mipmap/ic_launcher_round"
     ```
   - Any reference to system placeholder `@android:drawable/sym_def_app_icon` is strictly prohibited.
4. **Binary Verification:**
   - Final signed APKs in `dist/` must contain valid entries for `res/mipmap-*/ic_launcher.png` and `res/mipmap-*/ic_launcher_round.png`.

