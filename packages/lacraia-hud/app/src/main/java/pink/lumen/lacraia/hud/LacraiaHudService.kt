package pink.lumen.lacraia.hud

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Context
import android.content.Intent
import android.os.Build
import android.os.IBinder
import android.view.WindowManager
import androidx.core.app.NotificationCompat

class LacraiaHudService : Service() {

    private var windowManager: WindowManager? = null
    private var hudViewController: HudViewController? = null

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onCreate() {
        super.onCreate()
        startForegroundService()
        initOverlay()
    }

    private fun startForegroundService() {
        val channelId = "lacraia_hud_channel"
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                channelId,
                "La Craia Simbiótyka Overlay",
                NotificationManager.IMPORTANCE_LOW
            )
            val manager = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
            manager.createNotificationChannel(channel)
        }

        val notification: Notification = NotificationCompat.Builder(this, channelId)
            .setContentTitle("La Craia Simbiótyka HUD Active")
            .setContentText("Projecting synchronized lyrics over GPS navigation")
            .setSmallIcon(android.R.drawable.ic_media_play)
            .build()

        startForeground(101, notification)
    }

    private fun initOverlay() {
        val wm = getSystemService(Context.WINDOW_SERVICE) as WindowManager
        windowManager = wm

        val overlayType = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY
        } else {
            @Suppress("DEPRECATION")
            WindowManager.LayoutParams.TYPE_PHONE
        }

        hudViewController = HudViewController(
            context = this,
            windowManager = wm,
            onPlayPauseClick = {
                sendBroadcast(Intent(MediaCaptureListener.ACTION_TOGGLE_PLAYBACK))
            },
            onNextClick = {
                sendBroadcast(Intent(MediaCaptureListener.ACTION_NEXT_TRACK))
            }
        ).apply {
            val view = createFloatingPill(overlayType)
            wm.addView(view, layoutParams)
        }

        val filter = android.content.IntentFilter(MediaCaptureListener.ACTION_UPDATE_HUD)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            registerReceiver(hudUpdateReceiver, filter, Context.RECEIVER_NOT_EXPORTED)
        } else {
            registerReceiver(hudUpdateReceiver, filter)
        }
    }

    private val hudUpdateReceiver = object : android.content.BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: Intent?) {
            if (intent?.action == MediaCaptureListener.ACTION_UPDATE_HUD) {
                val text = intent.getStringExtra(MediaCaptureListener.EXTRA_DISPLAY_TEXT) ?: return
                val isMarquee = intent.getBooleanExtra(MediaCaptureListener.EXTRA_IS_MARQUEE, false)
                val isPlaying = intent.getBooleanExtra(MediaCaptureListener.EXTRA_IS_PLAYING, false)
                updateState(HudViewState(displayText = text, isMarquee = isMarquee, isPlaying = isPlaying))
            }
        }
    }

    fun updateState(state: HudViewState) {
        hudViewController?.renderState(state)
    }

    override fun onDestroy() {
        super.onDestroy()
        try {
            unregisterReceiver(hudUpdateReceiver)
        } catch (_: Exception) {}
        hudViewController?.floatingView?.let { windowManager?.removeView(it) }
    }
}

