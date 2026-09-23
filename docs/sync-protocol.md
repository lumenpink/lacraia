# lacraia-sync: WebSocket Synchronization Protocol

When audio is playing on a fixed server (e.g. Baquara Debian host via MPV or headless audio) and a mobile phone or dashboard acts as remote control and HUD, `lacraia-core` broadcasts and accepts JSON messages over `/ws`.

---

## 1. Server Telemetry Broadcast (`STATE_UPDATE`)

```json
{
  "type": "STATE_UPDATE",
  "payload": {
    "track": {
      "id": "trk_98765",
      "title": "Rebel Rebel",
      "artist": "David Bowie",
      "durationMs": 274000,
      "coverUrl": "https://baquara.lan/media/covers/trk_98765.jpg"
    },
    "playback": {
      "state": "PLAYING",
      "positionMs": 35200,
      "updatedAt": 1727047200000
    },
    "hasLrc": true
  }
}
```

---

## 2. Client Remote Commands (`COMMAND`)

```json
{
  "type": "COMMAND",
  "payload": {
    "action": "NEXT"
  }
}
```

### Supported Actions:
* `PLAY`: Resume playback
* `PAUSE`: Pause playback
* `NEXT`: Skip to next track in queue
* `PREVIOUS`: Restart current track or move to previous track
* `SEEK`: Move playhead to `{ positionMs: number }`
* `QUEUE_APPEND`: Append track ID or search query to radio queue

---

## 3. Server PlaybackHub & Ticker Lifecycle (`ws.rs`)

The WebSocket endpoint `/ws` is backed by `PlaybackHub`:

1. **Immediate Initial Snapshot:**
   Whenever a client (Web Player or Android HUD) connects to `/ws`, the server immediately transmits the latest `STATE_UPDATE` snapshot so the UI syncs instantly without waiting for a ticker pulse.
2. **1Hz Heartbeat & Playhead Advance:**
   An internal Tokio timer ticks every 1000ms:
   * When state is `PLAYING`, increments `position_ms` by 1000ms.
   * Compares `position_ms` against current track `duration_ms`. When the song ends, dispatches an automatic `NEXT` command to smoothly transition tracks.
3. **Reactive Command Dispatching:**
   When a client sends a `COMMAND`:
   * `PLAY` / `PAUSE` updates playback state.
   * `SEEK` jumps playhead position.
   * `NEXT` calls `db.advance_queue()`, updates the active track, resets position to 0, and notifies all connected clients via broadcast.
   * `QUEUE_APPEND` inserts a new item into the SQLite `queue`.
