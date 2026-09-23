# Android Micro-App: lacraia-hud

* **Language:** Kotlin
* **Compatibility:** Min SDK 26 (Android 8.0 Oreo) | Target SDK 34 (Android 14)
* **Architecture:** Decoupled background service (`Foreground Service`), no persistent fullscreen activity.

---

## 1. System Permissions (`AndroidManifest.xml`)

```xml
<uses-permission android:name="android.permission.SYSTEM_ALERT_WINDOW" />
<uses-permission android:name="android.permission.FOREGROUND_SERVICE" />
<uses-permission android:name="android.permission.FOREGROUND_SERVICE_MEDIA_PLAYBACK" />
<uses-permission android:name="android.permission.INTERNET" />
```

---

## 2. Universal Media Capture

`lacraia-hud` does not need to know which media player is currently producing sound:
1. Registers a `NotificationListenerService`.
2. Extracts the active playback `MediaSession.Token`.
3. Instantiates an active `MediaController`:
   * `onPlaybackStateChanged(state)`: Monitors `STATE_PLAYING`, millisecond position, and seek updates.
   * `onMetadataChanged(metadata)`: Extracts artist, title, album, and track duration.

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

---

## 4. Floating Pill Overlay over Waze (`HudViewController.kt`)

Injected directly into Android's `WindowManager` via `LacraiaHudService`:

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

### Automotive UI Guidelines & Architecture

* **Dimensions & Geometry:** Fixed height of 56dp, horizontal padding of 8dp, rounded corners with a 28dp radius (`GradientDrawable`).
* **Palette & Contrast:** Translucent black backdrop (`#CC121212`) paired with high-contrast white text (`#FFFFFF`, sans-serif bold, 15sp) avoiding driver distraction at night.
* **Touch Targets ($\ge 48\text{dp}$):** Prominent Play/Pause and Next Track buttons exceeding the Android accessibility guidelines (48x48dp minimum hit box) with transparent backgrounds.
* **Minimalist Karaoke (`TextSwitcher`):** Displays strictly the current active lyric line with a 150ms vertical slide transition. Automatically falls back to marquee-scrolling `{Artist} - {Title}` for instrumental or unindexed songs.
* **Vertical Drag Gesture:** Supports smooth vertical touch dragging (`ACTION_DOWN` / `ACTION_MOVE` updating `LayoutParams.y`) to avoid obscuring navigation alerts, speed limits, or turn arrows.

