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

## 4. Floating Pill Overlay over Waze

Injected directly into Android's `WindowManager`:

```kotlin
val params = WindowManager.LayoutParams(
    WindowManager.LayoutParams.MATCH_PARENT,
    WindowManager.LayoutParams.WRAP_CONTENT,
    WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY,
    WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or
    WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN,
    PixelFormat.TRANSLUCENT
).apply {
    gravity = Gravity.BOTTOM or Gravity.CENTER_HORIZONTAL
    y = 120 // Vertical margin away from navigation bar buttons
}
windowManager.addView(floatingView, params)
```

### Automotive UI Guidelines

* **Dimensions:** Fixed height of 56dp, horizontal margins of 12dp, corner radius of 28dp.
* **Palette & Contrast:** Translucent black backdrop (`#CC121212`) with pure white bold text (`#FFFFFF`, sans-serif bold, 15sp).
* **Physical Touch Targets:** Two large touch targets (minimum 48x48dp): Play/Pause and Next Track.
* **Minimalist Karaoke:** Displays strictly the current active lyric line. Fast vertical transition (150ms) via `TextSwitcher`. Falls back to marquee `{Artist} - {Title}` if instrumental or missing `.lrc`.
* **Vertical Drag:** Supports vertical touch dragging to prevent obscuring critical GPS speed or turn warnings.
