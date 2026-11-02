package `in`.twolp.lacraia.app

import android.annotation.SuppressLint
import android.os.Bundle
import android.view.View
import android.webkit.WebChromeClient
import android.webkit.WebSettings
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.FrameLayout
import android.widget.EditText
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity

class MainActivity : AppCompatActivity() {

    private lateinit var webView: WebView

    @SuppressLint("SetJavaScriptEnabled")
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val root = FrameLayout(this).apply {
            setBackgroundColor(0xFF151618.toInt()) // --ferro-bate-pedra
        }

        webView = WebView(this).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            )
            setBackgroundColor(0xFF151618.toInt())

            settings.apply {
                javaScriptEnabled = true
                domStorageEnabled = true
                databaseEnabled = true
                allowFileAccess = true
                allowContentAccess = true
                mediaPlaybackRequiresUserGesture = false
                mixedContentMode = WebSettings.MIXED_CONTENT_ALWAYS_ALLOW
                cacheMode = WebSettings.LOAD_DEFAULT
                useWideViewPort = true
                loadWithOverviewMode = true
                userAgentString = "${userAgentString} LacraiaApp/0.2.0"
            }

            webChromeClient = WebChromeClient()
            val localAssetUrl = "file:///android_asset/index.html"
            webViewClient = object : WebViewClient() {
                override fun shouldOverrideUrlLoading(view: WebView?, url: String?): Boolean {
                    return false
                }

                override fun onPageStarted(view: WebView?, url: String?, favicon: android.graphics.Bitmap?) {
                    super.onPageStarted(view, url, favicon)
                    view?.evaluateJavascript("window.LACRAIA_NATIVE = true;", null)
                }

                override fun onReceivedError(view: WebView?, errorCode: Int, description: String?, failingUrl: String?) {
                    super.onReceivedError(view, errorCode, description, failingUrl)
                    if (failingUrl != localAssetUrl) {
                        view?.loadUrl(localAssetUrl)
                    }
                }

                override fun onReceivedError(
                    view: WebView?,
                    request: android.webkit.WebResourceRequest?,
                    error: android.webkit.WebResourceError?
                ) {
                    super.onReceivedError(view, request, error)
                    if (request?.isForMainFrame == true) {
                        val failing = request.url?.toString()
                        if (failing != null && failing != localAssetUrl) {
                            view?.loadUrl(localAssetUrl)
                        }
                    }
                }
            }
        }

        root.addView(webView)
        setContentView(root)

        // Automotive immersive mode: hide system bars for full screen wall-of-sound experience
        window.decorView.systemUiVisibility = (
            View.SYSTEM_UI_FLAG_LAYOUT_STABLE
                or View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
                or View.SYSTEM_UI_FLAG_FULLSCREEN
                or View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY
        )

        val prefs = getSharedPreferences("lacraia_prefs", MODE_PRIVATE)
        val defaultEndpoint = "http://10.0.2.2:2011"
        val savedEndpoint = prefs.getString("server_endpoint", null)
        val localAssetUrl = "file:///android_asset/index.html"
        val targetUrl = intent?.getStringExtra("TARGET_URL") ?: savedEndpoint ?: localAssetUrl

        // Plug-and-play: carrega o Cockpit local de imediato sem popup bloqueante
        webView.loadUrl(targetUrl)
    }

    fun showServerConfigDialog(defaultEndpoint: String) {
        val input = EditText(this).apply {
            setText(defaultEndpoint)
            setSelection(defaultEndpoint.length)
        }

        AlertDialog.Builder(this)
            .setTitle("La Craia :: Endereço do Core")
            .setMessage("Informe o endpoint HTTP do servidor lacraia-core para carregar o Cockpit:")
            .setView(input)
            .setCancelable(false)
            .setPositiveButton("Conectar") { _, _ ->
                var url = input.text.toString().trim()
                if (url.isEmpty()) {
                    url = defaultEndpoint
                }
                if (!url.startsWith("http://") && !url.startsWith("https://")) {
                    url = "http://$url"
                }
                val prefs = getSharedPreferences("lacraia_prefs", MODE_PRIVATE)
                prefs.edit().putString("server_endpoint", url).apply()
                webView.loadUrl(url)
            }
            .setNegativeButton("Modo Local Padrão") { _, _ ->
                val prefs = getSharedPreferences("lacraia_prefs", MODE_PRIVATE)
                prefs.edit().putString("server_endpoint", defaultEndpoint).apply()
                webView.loadUrl(defaultEndpoint)
            }
            .show()
    }

    override fun onBackPressed() {
        if (webView.canGoBack()) {
            webView.goBack()
        } else {
            super.onBackPressed()
        }
    }
}
