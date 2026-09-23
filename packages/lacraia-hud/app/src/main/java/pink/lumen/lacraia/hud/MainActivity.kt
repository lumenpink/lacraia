package pink.lumen.lacraia.hud

import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.provider.Settings
import android.widget.Button
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity

class MainActivity : AppCompatActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val layout = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(48, 48, 48, 48)
        }

        val title = TextView(this).apply {
            text = "La Craia HUD Setup"
            textSize = 24f
            setPadding(0, 0, 0, 32)
        }
        layout.addView(title)

        val overlayBtn = Button(this).apply {
            text = "1. Permitir Sobreposição de Tela (Overlay)"
            setOnClickListener {
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M && !Settings.canDrawOverlays(this@MainActivity)) {
                    val intent = Intent(
                        Settings.ACTION_MANAGE_OVERLAY_PERMISSION,
                        Uri.parse("package:$packageName")
                    )
                    startActivity(intent)
                }
            }
        }
        layout.addView(overlayBtn)

        val notifBtn = Button(this).apply {
            text = "2. Permitir Acesso a Notificações de Mídia"
            setOnClickListener {
                startActivity(Intent("android.settings.ACTION_NOTIFICATION_LISTENER_SETTINGS"))
            }
        }
        layout.addView(notifBtn)

        val startBtn = Button(this).apply {
            text = "3. Iniciar Serviço HUD"
            setOnClickListener {
                val serviceIntent = Intent(this@MainActivity, LacraiaHudService::class.java)
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                    startForegroundService(serviceIntent)
                } else {
                    startService(serviceIntent)
                }
            }
        }
        layout.addView(startBtn)

        setContentView(layout)
    }
}
