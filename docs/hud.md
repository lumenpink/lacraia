# Simbiótyka: Android HUD Micro-App (`packages/lacraia-android/simbiotyka`)

* **Component:** Floating Heads-Up Display (HUD) overlay and GPS audio-ducking companion
* **Package / Namespace:** `in._2lp.lacraia.simbiotyka`
* **Artifact:** `simbiotyka.apk` (Production Release)
* **Language:** Kotlin
* **Compatibility:** Min SDK 26 (Android 8.0 Oreo) | Target SDK 34 (Android 14)
* **Architecture:** Decoupled background service (`Foreground Service`), no persistent fullscreen activity.

---

## 1. Múltiplos Casos de Uso e Cenários de Operação

Simbiótyka não é amarrada a nenhum aplicativo específico; opera de forma agnóstica como uma camada sensorial flutuante sobre o ecossistema Android:

1. **Navegação GPS Ativa em Trânsito:**
   Projeta a pílula de letra flutuante diretamente sobre qualquer aplicativo de mapas e navegação (Waze, Google Maps, Organic Maps, OSMAnd, MapFactor), mantendo a rota e as instruções de direção sempre visíveis no primeiro plano, sem necessidade de alternar janelas.
2. **Viagens em Grupo, Caronas e Vans:**
   Permite que o motorista e os passageiros acompanhem a letra e cantem juntos em tempo real enquanto o tablet do painel ou celular do suporte continua exibindo o trajeto e os alertas de velocidade.
3. **Karaokê de Estrada & Paredão Ambulante:**
   Oferece sincronização precisa milissegundo a milissegundo das estrofes com cache SQLite local (`LrcCacheHelper`). Em trechos sem cobertura de dados (serras, túneis, áreas rurais), as letras previamente cacheadas continuam disponíveis sem internet.
4. **Captura Universal e Agnóstica de Mídia:**
   Intercepta eventos de reprodução via `NotificationListenerService` e `MediaSessionManager`. Funciona tanto com o próprio servidor `lacraia-core` quanto com qualquer player de mídia instalado no dispositivo (Spotify, YouTube Music, VLC, MPV, rádios online).

---

## 2. System Permissions (`AndroidManifest.xml`)

```xml
<uses-permission android:name="android.permission.SYSTEM_ALERT_WINDOW" />
<uses-permission android:name="android.permission.FOREGROUND_SERVICE" />
<uses-permission android:name="android.permission.FOREGROUND_SERVICE_MEDIA_PLAYBACK" />
<uses-permission android:name="android.permission.INTERNET" />
```

---

## 3. Universal Media Capture & Audio Ducking (`MediaCaptureListener.kt`)

Simbiótyka does not need to know which media player is currently producing sound:
1. **Notification & MediaSession Capture:**
   * Registers a `NotificationListenerService`.
   * Queries `MediaSessionManager.getActiveSessions()` to extract active media controllers.
   * Attaches `MediaController.Callback`:
     * `onPlaybackStateChanged(state)`: Monitors `STATE_PLAYING`, millisecond position, and initiates the 200ms lyric sync ticker.
     * `onMetadataChanged(metadata)`: Extracts artist, title, album, and duration, firing `fetchLyrics` via `LrcApiClient`.
2. **Lyric Synchronization Ticker (`LyricSyncCoordinator.kt`):**
   * Uses `estimatePosition()` with system elapsed real-time delta between ticks to maintain smooth, continuous timeline progression without jitter.
   * Queries `findActiveLyric()` to locate the active lyric line for the exact current millisecond.
   * Broadcasts `ACTION_UPDATE_HUD` to dynamically update the floating pill overlay.
3. **Automotive GPS Audio Ducking:**
   * Registers an `AudioManager.OnAudioFocusChangeListener`.
   * Intercepts `AUDIOFOCUS_LOSS_TRANSIENT_CAN_DUCK` when GPS navigation apps emit turn-by-turn spoken guidance.
   * Scales active media stream volume down to **20%** (`LyricSyncCoordinator.DUCK_RATIO`).
   * Automatically restores full pre-ducking volume level on `AUDIOFOCUS_GAIN`.
4. **Transport Controls Delegation:**
   * Receives `ACTION_TOGGLE_PLAYBACK` and `ACTION_NEXT_TRACK` broadcast from the floating pill touch targets and dispatches `transportControls.play()`, `pause()`, or `skipToNext()` directly to the active media session.

> 📡 **[ TRANSMISSÃO DE CABINE ]**  
> Simbiótyka opera como um parasita acústico simbiótico: intercepta o canal de áudio automotivo sem que o aplicativo reprodutor primário saiba da sua existência, extraindo letras e metadados ao vivo diretamente do fluxo do sistema.

> ⚡ **[ FIO DESENCAPADO ]**  
> O evento `AUDIOFOCUS_LOSS_TRANSIENT_CAN_DUCK` ativado por apps de navegação atenua o volume da música para 20%. Se o sistema de som do carro tentar forçar o ganho manualmente nesse intervalo, o limitador digital segura o estouro para evitar distorção destrutiva.


---

## 3. Lyrics Synchronization (LRCLIB Integration & SQLite Caching)

When a track change event triggers, `LrcApiClient` first checks local SQLite storage before making network requests:
1. **Local SQLite Cache (`LrcCacheHelper.kt`):**
   * Stores `(track, artist)` as primary key, along with `synced_lyrics`, `plain_lyrics`, `is_instrumental`, and timestamp `cached_at`.
   * Cache hits resolve instantaneously offline without internet access during road trips.
2. **Network Query (`LrcApiClient.kt`):**
   ```
   GET https://lrclib.net/api/get?artist_name={artist}&track_name={title}&duration={duration_sec}
   ```
   * User-Agent identified as `LaCraia-HUD/0.1.0`.
   * On successful retrieval, parsed JSON is automatically persisted to local SQLite storage.
3. **Millisecond-Accurate LRC Parsing (`LrcParser.kt`):**
   * Parses standard 2-digit centiseconds (`[mm:ss.xx]`) and 3-digit milliseconds (`[mm:ss.xxx]`).
   * Automatically sorts lines out-of-order by timestamp `timestampMs`.
   * Filters out header metadata tags (`[ti:...]`, `[ar:...]`).

> ✨ **[ O PULO DA TRAVESTI ]**  
> Para evitar quedas na serra e zonas mortas sem sinal de celular, cada estrofe sincronizada capturada pelo LRCLIB é imediatamente injetada no SQLite local. Se você já tocou aquela música uma vez na rodovia, a letra nunca mais precisará de internet.

---

## 4. Floating Pill Overlay over Navigation & Cockpit (`HudViewController.kt`)

Injected directly into Android's `WindowManager` via `SimbiotykaService`:

```kotlin
val params = WindowManager.LayoutParams(
    WindowManager.LayoutParams.MATCH_PARENT,
    dp(56),
    overlayType,
    WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or
    WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN,
    PixelFormat.TRANSLUCENT
).apply {
    gravity = Gravity.BOTTOM or Gravity.CENTER_HORIZONTAL
    y = dp(120)
}
windowManager.addView(floatingView, params)
```

> 🦂 **[ VENENO NA CARNE ]**  
> Linhas de metadados corrompidas, tags de sincronização quebradas ou arquivos de karaokê pirateados com atrasos grosseiros são purgados antes de subir para o display. O piloto só enxerga a rima exata na fração do segundo em que ela bate.

### Automotive UI Guidelines & Architecture

* **Dimensions & Geometry:** Fixed height of 56dp, horizontal padding of 8dp, rounded corners with a 28dp radius (`GradientDrawable`).
* **Palette & Contrast:** Translucent black backdrop (`#CC121212`) paired with high-contrast white text (`#FFFFFF`, sans-serif bold, 15sp) avoiding driver distraction at night.
* **Touch Targets ($\ge 48\text{dp}$):** Prominent Play/Pause and Next Track buttons exceeding the Android accessibility guidelines (48x48dp minimum hit box) with transparent backgrounds.
* **Minimalist Karaoke (`TextSwitcher`):** Displays strictly the current active lyric line with a 150ms vertical slide transition. Automatically falls back to marquee-scrolling `{Artist} - {Title}` for instrumental or unindexed songs.
* **Vertical Drag Gesture:** Supports smooth vertical touch dragging (`ACTION_DOWN` / `ACTION_MOVE` updating `LayoutParams.y`) to avoid obscuring navigation alerts, speed limits, or turn arrows.

