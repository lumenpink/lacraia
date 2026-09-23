package pink.lumen.lacraia.hud

import android.content.Context
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.text.TextUtils
import android.view.Gravity
import android.view.MotionEvent
import android.view.View
import android.view.WindowManager
import android.view.animation.Animation
import android.view.animation.TranslateAnimation
import android.widget.Button
import android.widget.LinearLayout
import android.widget.TextSwitcher
import android.widget.TextView

data class HudViewState(
    val displayText: String,
    val isMarquee: Boolean,
    val isPlaying: Boolean
) {
    companion object {
        fun compute(
            currentLyric: String?,
            artist: String?,
            title: String?,
            isPlaying: Boolean,
            isInstrumental: Boolean = false
        ): HudViewState {
            return when {
                !currentLyric.isNullOrBlank() -> HudViewState(
                    displayText = currentLyric.trim(),
                    isMarquee = false,
                    isPlaying = isPlaying
                )
                isInstrumental && !title.isNullOrBlank() -> HudViewState(
                    displayText = "🎵 ${artist.orEmpty()} - $title (Instrumental)".trim(),
                    isMarquee = true,
                    isPlaying = isPlaying
                )
                !title.isNullOrBlank() -> HudViewState(
                    displayText = "${artist.orEmpty()} - $title".trim().removePrefix("- "),
                    isMarquee = true,
                    isPlaying = isPlaying
                )
                else -> HudViewState(
                    displayText = "La Craia HUD • Aguardando áudio...",
                    isMarquee = false,
                    isPlaying = isPlaying
                )
            }
        }
    }
}

class HudViewController(
    private val context: Context,
    private val windowManager: WindowManager,
    private val onPlayPauseClick: () -> Unit,
    private val onNextClick: () -> Unit
) {

    var floatingView: View? = null
        private set
    var layoutParams: WindowManager.LayoutParams? = null
        private set

    private var textSwitcher: TextSwitcher? = null
    private var playPauseButton: Button? = null
    private var nextButton: Button? = null
    private var lastRenderedText: String = ""

    fun createFloatingPill(overlayType: Int): View {
        val density = context.resources.displayMetrics.density
        fun dp(value: Int): Int = (value * density).toInt()

        layoutParams = WindowManager.LayoutParams(
            WindowManager.LayoutParams.MATCH_PARENT,
            dp(56),
            overlayType,
            WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or
                    WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN,
            android.graphics.PixelFormat.TRANSLUCENT
        ).apply {
            gravity = Gravity.BOTTOM or Gravity.CENTER_HORIZONTAL
            y = dp(120)
        }

        // Pill outer container with 28dp rounded corners and translucent dark background
        val pill = LinearLayout(context).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(8), 0, dp(8), 0)

            val backgroundDrawable = GradientDrawable().apply {
                setColor(Color.parseColor("#CC121212"))
                cornerRadius = dp(28).toFloat()
            }
            background = backgroundDrawable
        }

        // Play/Pause button (touch target >= 48dp)
        playPauseButton = Button(context).apply {
            text = "▶"
            textSize = 18f
            setTextColor(Color.WHITE)
            setBackgroundColor(Color.TRANSPARENT)
            minWidth = dp(48)
            minHeight = dp(48)
            layoutParams = LinearLayout.LayoutParams(dp(48), dp(48))
            setOnClickListener { onPlayPauseClick() }
        }

        // TextSwitcher with 150ms vertical transition
        textSwitcher = TextSwitcher(context).apply {
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1.0f).apply {
                gravity = Gravity.CENTER_VERTICAL
                setMargins(dp(8), 0, dp(8), 0)
            }

            setFactory {
                TextView(context).apply {
                    textSize = 15f
                    setTextColor(Color.WHITE)
                    typeface = Typeface.create(Typeface.DEFAULT, Typeface.BOLD)
                    gravity = Gravity.CENTER
                    maxLines = 1
                    ellipsize = TextUtils.TruncateAt.MARQUEE
                    isSelected = true
                    marqueeRepeatLimit = -1
                }
            }

            // 150ms vertical slide in / out
            inAnimation = TranslateAnimation(
                Animation.RELATIVE_TO_PARENT, 0f, Animation.RELATIVE_TO_PARENT, 0f,
                Animation.RELATIVE_TO_PARENT, 1.0f, Animation.RELATIVE_TO_PARENT, 0f
            ).apply { duration = 150 }

            outAnimation = TranslateAnimation(
                Animation.RELATIVE_TO_PARENT, 0f, Animation.RELATIVE_TO_PARENT, 0f,
                Animation.RELATIVE_TO_PARENT, 0f, Animation.RELATIVE_TO_PARENT, -1.0f
            ).apply { duration = 150 }
        }

        // Next Track button (touch target >= 48dp)
        nextButton = Button(context).apply {
            text = "⏭"
            textSize = 18f
            setTextColor(Color.WHITE)
            setBackgroundColor(Color.TRANSPARENT)
            minWidth = dp(48)
            minHeight = dp(48)
            layoutParams = LinearLayout.LayoutParams(dp(48), dp(48))
            setOnClickListener { onNextClick() }
        }

        pill.addView(playPauseButton)
        pill.addView(textSwitcher)
        pill.addView(nextButton)

        setupDragGesture(pill)

        // Set initial state
        renderState(HudViewState.compute(null, null, null, false))

        floatingView = pill
        return pill
    }

    private fun setupDragGesture(view: View) {
        var initialY = 0
        var initialTouchY = 0f

        view.setOnTouchListener { _, event ->
            when (event.action) {
                MotionEvent.ACTION_DOWN -> {
                    initialY = layoutParams?.y ?: 0
                    initialTouchY = event.rawY
                    true
                }
                MotionEvent.ACTION_MOVE -> {
                    layoutParams?.let { params ->
                        params.y = initialY - (event.rawY - initialTouchY).toInt()
                        windowManager.updateViewLayout(floatingView, params)
                    }
                    true
                }
                else -> false
            }
        }
    }

    fun renderState(state: HudViewState) {
        playPauseButton?.text = if (state.isPlaying) "⏸" else "▶"

        if (state.displayText != lastRenderedText) {
            lastRenderedText = state.displayText
            textSwitcher?.setText(state.displayText)
        }
    }
}
