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
