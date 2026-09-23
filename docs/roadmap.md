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
* **Package:** `packages/lacraia-web`
* **Goal:** Real-time WebAudio frequency analyzer displaying a high-contrast soundwave canvas optimized for night-time driving.
* **Acceptance Criteria:**
  - Connect `AudioContext.createAnalyser()` to the HTML5 audio element.
  - Render an animated 32-bar visualizer using the `#ff007f` neon accent on `#121212` background.
  - Touch-toggle to enable/disable visualizer to conserve rendering cycles on low-spec car tablets.

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
* **Package:** `packages/lacraia-hud`
* **Goal:** Reduce CPU and display power consumption when vehicle battery is low or device is unplugged.
* **Acceptance Criteria:**
  - Battery broadcast receiver monitoring charging state and level (< 20%).
  - Drops lyric ticker interval from 200ms to 1000ms.
  - Disables TextSwitcher slide animations in favor of static text updates.

---

### Ticket #19: Upgrade Workspace to Rust Edition 2024 and Dioxus 0.7
* **Package:** Workspace-wide (`lacraia-shared`, `lacraia-core`, `lacraia-web`)
* **Goal:** Migrate monorepo crates to Rust Edition 2024 and upgrade Dioxus framework to version 0.7 once published / stabilized.
* **Acceptance Criteria:**
  - Update `[workspace.package] edition = "2024"` in `Cargo.toml`.
  - Bump Dioxus dependency to `0.7` in `packages/lacraia-web/Cargo.toml` and adapt UI signals/lifecycle hooks.
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
