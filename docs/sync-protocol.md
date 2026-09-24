# lacraia-sync: WebSocket Synchronization Protocol

When audio is playing on a fixed server (e.g. Linux Debian host via MPV or headless audio) and a mobile phone or dashboard acts as remote control and HUD, `lacraia-core` broadcasts and accepts JSON messages over `/ws`.

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
      "coverUrl": "https://lacraia.local/media/covers/trk_98765.jpg"
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

> 📡 **[ TRANSMISSÃO DE CABINE ]**  
> O payload `STATE_UPDATE` transmite os batimentos cardíacos do paredão a cada 1000ms. Cada tela conectada na van, moto ou cockpit recebe a mesma fração de segundo da reprodução sem atraso perceptível.

---

## 2. Cabin Connection & Onboarding Handshake

Clients connect to the WebSocket endpoint supplying cabin identifier and authentication token:

```
ws://<server>:8080/ws?cabin=<cabin_id>&token=<optional_token>
```

* **Master Key (`token=<LACRAIA_MASTER_KEY>`):** Authenticates as `Pilot` (`role: "PILOT"`), granting universal authority across all cabins without cabin record lookup.
* **Pilot Token (`token=<pilot_token>`):** Validated against the cabin's record in SQLite. Authenticates as `Pilot` (`role: "PILOT"`) strictly for the matching cabin.
* **Passenger Session Token (`token=<passenger_token>`):** Obtained via `POST /api/cabin/join` with `{ cabinId: string, name: string, device?: string, key?: string }`. Requires a valid `key` matching the cabin's active `passenger_key`. Authenticates as `Passenger` (`role: "PASSENGER"`).
* **Anonymous / No Token:** If the cabin is unlocked, grants non-privileged passenger rights (`role: "PASSENGER"`).
* **Locked Cabin:** If `is_locked` is true, connections and joins without verified pilot or master authorization are immediately rejected (`403 Forbidden` / handshake termination).

---

## 3. Cabin & Crew Telemetry Broadcasts

### `CREW_UPDATE` (Crew Manifesto)

Broadcast to cabin participants whenever a passenger connects, disconnects, or is ejected:

```json
{
  "type": "CREW_UPDATE",
  "payload": {
    "cabinId": "default",
    "passengers": [
      {
        "id": "p_101",
        "name": "Mari",
        "device": "Pixel 8"
      },
      {
        "id": "p_102",
        "name": "Giba",
        "device": "iPhone 15"
      }
    ]
  }
}
```

### `CABIN_LOCKED` (Door State Change)

Broadcast when the pilot engages or disengages the cabin door lock:

```json
{
  "type": "CABIN_LOCKED",
  "payload": {
    "cabinId": "default",
    "locked": true
  }
}
```

### `EJECTED` (Passenger Ejection Frame)

Sent directly to a passenger when kicked by the pilot (or to all passengers during emergency clear):

```json
{
  "type": "EJECTED",
  "payload": {
    "reason": "Aqui no paredão quem manda na agulha é o piloto. Comporte-se melhor na próxima curva."
  }
}
```

---

## 4. Client Remote Commands (`COMMAND`)

```json
{
  "type": "COMMAND",
  "payload": {
    "action": "NEXT"
  }
}
```

### Supported Actions & Role Authorization:

| Action | Allowed Roles | Payload Attributes | Description |
|---|---|---|---|
| `PLAY` | `Pilot` | — | Resume playback |
| `PAUSE` | `Pilot` | — | Pause playback |
| `NEXT` | `Pilot` | — | Skip to next track in queue |
| `PREVIOUS` | `Pilot` | — | Restart current track or previous track |
| `SEEK` | `Pilot` | `{ positionMs: number }` | Move playhead to millisecond position |
| `QUEUE_APPEND` | `Pilot`, `Passenger` | `{ query: string, requestedBy?: string }` | Enqueue track with requester credit badge |
| `LOCK_CABIN` | `Pilot` | `{ locked: bool }` | Lock or unlock cabin door for new passengers |
| `EJECT_PASSENGER` | `Pilot` | `{ passengerId: string, reason?: string }` | Kick specific passenger and revoke token |
| `EJECT_ALL` | `Pilot` | `{ reason?: string }` | Clear all passengers from cabin immediately |

> ⚡ **[ FIO DESENCAPADO ]**  
> Comandos remotos desferidos por conexões instáveis sofrem reconciliação imediata: se um `SEEK` chegar defasado, o servidor estipula o carimbo de tempo autoritativo e alinha todos os nós da cabine.

---

## 5. Server PlaybackHub & Ticker Lifecycle (`ws.rs`)

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
   * `QUEUE_APPEND` inserts a new item into the SQLite `queue` preserving `requested_by`.
   * `LOCK_CABIN`, `EJECT_PASSENGER`, `EJECT_ALL` invoke cabin control routines and dispatch targeted `CREW_UPDATE` and `EJECTED` notifications.

> 🦂 **[ VENENO NA CARNE ]**  
> Comandos maliciosos ou payloads truncados enviados ao endpoint `/ws` são abortados antes de tocarem na fila do SQLite, blindando a continuidade sonora contra interferências ou panes de barramento.

> ✨ **[ O PULO DA TRAVESTI ]**  
> Ao plugar um novo tablet ou celular na rede interna da van, o `PlaybackHub` cospe de imediato o snapshot instantâneo do estado atual no primeiro frame, sem esperar o próximo ciclo do ticker.

