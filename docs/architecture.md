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
