# Technical Specification: La Craia Ecosystem

**Project:** La Craia (Decentralized Automotive Audio & Heads-Up Display)  
**Version:** 0.1.0  
**Target Environment:** Baquara Server (Debian 13) & Android Mobile  
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
│  packages/lacraia-core (Baquara Server / Debian 13)   │
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
│  packages/lacraia-web     │   │  packages/lacraia-hud         │
│  Stack: Rust / Dioxus Web │   │  Language: Kotlin (Android)   │
│  Target: WASM (PWA)       │   │                               │
│  ├── HTML5 Audio Engine   │   │  ├── NotificationListener     │
│  ├── MediaSession Handler │   │  ├── LRCLIB Client & Parser   │
│  └── Queue Management     │   │  └── WindowManager Overlay UI │
└───────────────────────────┘   └──────────────┬────────────────┘
                                               │ Projects over
                                               ▼
                                  ┌─────────────────────────────┐
                                  │ WAZE (Active GPS Navigation)│
                                  └─────────────────────────────┘
```

### Module Roles

* **`packages/lacraia-shared` (Rust Crate):** Shared domain models, `serde` serialization, WebSocket protocol packets (`StateUpdate`, `Command`), in-memory `.lrc` lyrics parser, and duration/title fuzzy sanitizer.
* **`packages/lacraia-core` (Rust Crate):** Backend orchestrator running on the Baquara server. Handles track resolution, disk caching in Opus, SQLite state storage, and WebSocket broadcasts.
* **`packages/lacraia-web` (Rust / Dioxus Crate):** Lightweight WebAssembly PWA frontend. Acts as the track library, playlist controller, and audio engine registered with the browser `MediaSession API`.
* **`packages/lacraia-hud` (Android Kotlin App):** Native Android background service (`Foreground Service`). Intercepts system media notifications, fetches synced `.lrc` lyrics from LRCLIB, and renders a floating pill HUD directly over Waze via `WindowManager`.
