package pink.lumen.lacraia.hud

import android.content.ComponentName
import android.content.Context
import android.media.MediaMetadata
import android.media.session.MediaController
import android.media.session.MediaSessionManager
import android.media.session.PlaybackState
import android.service.notification.NotificationListenerService
import android.util.Log

class MediaCaptureListener : NotificationListenerService() {

    private val TAG = "MediaCaptureListener"
    private var activeController: MediaController? = null

    private val callback = object : MediaController.Callback() {
        override fun onPlaybackStateChanged(state: PlaybackState?) {
            super.onPlaybackStateChanged(state)
            state?.let {
                val isPlaying = it.state == PlaybackState.STATE_PLAYING
                val positionMs = it.position
                Log.d(TAG, "Playback state changed: isPlaying=$isPlaying, pos=$positionMs")
            }
        }

        override fun onMetadataChanged(metadata: MediaMetadata?) {
            super.onMetadataChanged(metadata)
            metadata?.let {
                val title = it.getString(MediaMetadata.METADATA_KEY_TITLE)
                val artist = it.getString(MediaMetadata.METADATA_KEY_ARTIST)
                val duration = it.getLong(MediaMetadata.METADATA_KEY_DURATION)
                Log.d(TAG, "Metadata changed: $title by $artist (duration: ${duration}ms)")
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
            activeController?.unregisterCallback(callback)
            activeController = controllers[0]
            activeController?.registerCallback(callback)
            Log.d(TAG, "Attached to active media session: ${activeController?.packageName}")
        }
    }
}
