package pink.lumen.lacraia.hud

import android.content.BroadcastReceiver
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.media.AudioAttributes
import android.media.AudioFocusRequest
import android.media.AudioManager
import android.media.MediaMetadata
import android.media.session.MediaController
import android.media.session.MediaSessionManager
import android.media.session.PlaybackState
import android.os.Build
import android.os.SystemClock
import android.service.notification.NotificationListenerService
import android.util.Log
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch

class MediaCaptureListener : NotificationListenerService() {

    companion object {
        const val TAG = "MediaCaptureListener"
        const val ACTION_TOGGLE_PLAYBACK = "pink.lumen.lacraia.hud.ACTION_TOGGLE_PLAYBACK"
        const val ACTION_NEXT_TRACK = "pink.lumen.lacraia.hud.ACTION_NEXT_TRACK"
        const val ACTION_UPDATE_HUD = "pink.lumen.lacraia.hud.ACTION_UPDATE_HUD"
        const val EXTRA_DISPLAY_TEXT = "extra_display_text"
        const val EXTRA_IS_MARQUEE = "extra_is_marquee"
        const val EXTRA_IS_PLAYING = "extra_is_playing"
    }

    private val serviceScope = CoroutineScope(Dispatchers.Main + Job())
    private var tickerJob: Job? = null

    private var activeController: MediaController? = null
    private var lrcApiClient: LrcApiClient? = null
    private var currentLyrics: List<LyricLine> = emptyList()
    private var currentArtist: String = ""
    private var currentTitle: String = ""
    private var isInstrumental: Boolean = false

    private var lastKnownPositionMs: Long = 0L
    private var lastPositionUpdateTimeMs: Long = 0L
    private var isPlayingState: Boolean = false

    private var audioManager: AudioManager? = null
    private var originalVolumeLevel: Int = -1
    private var isDucked: Boolean = false

    private val controlReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: Intent?) {
            when (intent?.action) {
                ACTION_TOGGLE_PLAYBACK -> {
                    activeController?.let { controller ->
                        val state = controller.playbackState?.state
                        if (state == PlaybackState.STATE_PLAYING) {
                            controller.transportControls.pause()
                        } else {
                            controller.transportControls.play()
                        }
                    }
                }
                ACTION_NEXT_TRACK -> {
                    activeController?.transportControls?.skipToNext()
                }
            }
        }
    }

    private val audioFocusChangeListener = AudioManager.OnAudioFocusChangeListener { focusChange ->
        when (focusChange) {
            AudioManager.AUDIOFOCUS_LOSS_TRANSIENT_CAN_DUCK -> {
                // Duck volume down to 20% during Waze turn-by-turn prompts
                duckVolume()
            }
            AudioManager.AUDIOFOCUS_GAIN -> {
                // Restore volume back to pre-ducking level
                restoreVolume()
            }
            AudioManager.AUDIOFOCUS_LOSS -> {
                activeController?.transportControls?.pause()
            }
        }
    }

    override fun onCreate() {
        super.onCreate()
        lrcApiClient = LrcApiClient(cache = LrcSqliteCacheHelper(this))
        audioManager = getSystemService(Context.AUDIO_SERVICE) as AudioManager

        val filter = IntentFilter().apply {
            addAction(ACTION_TOGGLE_PLAYBACK)
            addAction(ACTION_NEXT_TRACK)
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            registerReceiver(controlReceiver, filter, Context.RECEIVER_NOT_EXPORTED)
        } else {
            registerReceiver(controlReceiver, filter)
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        tickerJob?.cancel()
        unregisterReceiver(controlReceiver)
        activeController?.unregisterCallback(mediaCallback)
    }

    private val mediaCallback = object : MediaController.Callback() {
        override fun onPlaybackStateChanged(state: PlaybackState?) {
            super.onPlaybackStateChanged(state)
            state?.let {
                isPlayingState = (it.state == PlaybackState.STATE_PLAYING)
                lastKnownPositionMs = it.position
                lastPositionUpdateTimeMs = SystemClock.elapsedRealtime()

                if (isPlayingState) {
                    startTicker()
                } else {
                    stopTicker()
                    updateHudDisplay(lastKnownPositionMs)
                }
            }
        }

        override fun onMetadataChanged(metadata: MediaMetadata?) {
            super.onMetadataChanged(metadata)
            metadata?.let {
                val title = it.getString(MediaMetadata.METADATA_KEY_TITLE) ?: ""
                val artist = it.getString(MediaMetadata.METADATA_KEY_ARTIST) ?: ""
                val durationMs = it.getLong(MediaMetadata.METADATA_KEY_DURATION)

                if (title != currentTitle || artist != currentArtist) {
                    currentTitle = title
                    currentArtist = artist
                    currentLyrics = emptyList()
                    isInstrumental = false

                    fetchLyrics(title, artist, (durationMs / 1000).toInt())
                }
            }
        }
    }

    private fun fetchLyrics(title: String, artist: String, durationSec: Int) {
        if (title.isBlank()) return

        serviceScope.launch {
            val result = lrcApiClient?.getLyrics(
                track = title,
                artist = artist,
                durationSeconds = if (durationSec > 0) durationSec else null
            )

            result?.onSuccess { lrcResult ->
                currentLyrics = lrcResult.lines
                isInstrumental = lrcResult.isInstrumental
                updateHudDisplay(lastKnownPositionMs)
            }?.onFailure { err ->
                Log.w(TAG, "Failed to load lyrics: ${err.message}")
                updateHudDisplay(lastKnownPositionMs)
            }
        }
    }

    private fun startTicker() {
        tickerJob?.cancel()
        tickerJob = serviceScope.launch {
            while (isActive && isPlayingState) {
                val currentPos = LyricSyncCoordinator.estimatePosition(
                    lastReportedPosMs = lastKnownPositionMs,
                    lastUpdateTimeMs = lastPositionUpdateTimeMs,
                    currentTimeMs = SystemClock.elapsedRealtime(),
                    isPlaying = true
                )
                updateHudDisplay(currentPos)
                delay(200)
            }
        }
    }

    private fun stopTicker() {
        tickerJob?.cancel()
        tickerJob = null
    }

    private fun updateHudDisplay(positionMs: Long) {
        val activeLine = LyricSyncCoordinator.findActiveLyric(currentLyrics, positionMs)
        val viewState = HudViewState.compute(
            currentLyric = activeLine?.text,
            artist = currentArtist,
            title = currentTitle,
            isPlaying = isPlayingState,
            isInstrumental = isInstrumental
        )

        val intent = Intent(ACTION_UPDATE_HUD).apply {
            putExtra(EXTRA_DISPLAY_TEXT, viewState.displayText)
            putExtra(EXTRA_IS_MARQUEE, viewState.isMarquee)
            putExtra(EXTRA_IS_PLAYING, viewState.isPlaying)
        }
        sendBroadcast(intent)
    }

    private fun duckVolume() {
        audioManager?.let { am ->
            if (!isDucked) {
                originalVolumeLevel = am.getStreamVolume(AudioManager.STREAM_MUSIC)
                val maxVolume = am.getStreamMaxVolume(AudioManager.STREAM_MUSIC)
                val duckedLevel = (originalVolumeLevel * LyricSyncCoordinator.DUCK_RATIO).toInt().coerceAtLeast(1)
                am.setStreamVolume(AudioManager.STREAM_MUSIC, duckedLevel, 0)
                isDucked = true
                Log.d(TAG, "Ducked volume from $originalVolumeLevel to $duckedLevel (max $maxVolume)")
            }
        }
    }

    private fun restoreVolume() {
        audioManager?.let { am ->
            if (isDucked && originalVolumeLevel >= 0) {
                am.setStreamVolume(AudioManager.STREAM_MUSIC, originalVolumeLevel, 0)
                isDucked = false
                Log.d(TAG, "Restored volume back to $originalVolumeLevel")
            }
        }
    }

    override fun onListenerConnected() {
        super.onListenerConnected()
        registerActiveSession()
    }

    private fun registerActiveSession() {
        val sessionManager = getSystemService(Context.MEDIA_SESSION_SERVICE) as MediaSessionManager
        val component = ComponentName(this, MediaCaptureListener::class.java)
        val controllers = sessionManager.getActiveSessions(component)

        if (controllers.isNotEmpty()) {
            activeController?.unregisterCallback(mediaCallback)
            activeController = controllers[0]
            activeController?.registerCallback(mediaCallback)
            Log.d(TAG, "Attached to active media session: ${activeController?.packageName}")
        }
    }
}
