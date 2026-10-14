package `in`.twolp.lacraia.simbiotyka

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class HudViewStateTest {

    @Test
    fun testComputeWithActiveLyric() {
        val state = HudViewState.compute(
            currentLyric = "Amou daquela vez como se fosse a última",
            artist = "Chico Buarque",
            title = "Construção",
            isPlaying = true
        )

        assertEquals("Amou daquela vez como se fosse a última", state.displayText)
        assertFalse(state.isMarquee)
        assertTrue(state.isPlaying)
    }

    @Test
    fun testComputeFallbackTrackTitleMarquee() {
        val state = HudViewState.compute(
            currentLyric = null,
            artist = "Elis Regina",
            title = "Como Nossos Pais",
            isPlaying = true
        )

        assertEquals("Elis Regina - Como Nossos Pais", state.displayText)
        assertTrue(state.isMarquee)
        assertTrue(state.isPlaying)
    }

    @Test
    fun testComputeInstrumentalFallback() {
        val state = HudViewState.compute(
            currentLyric = null,
            artist = "Hermeto Pascoal",
            title = "Bebê",
            isPlaying = false,
            isInstrumental = true
        )

        assertEquals("🎵 Hermeto Pascoal - Bebê (Instrumental)", state.displayText)
        assertTrue(state.isMarquee)
        assertFalse(state.isPlaying)
    }

    @Test
    fun testComputeIdleState() {
        val state = HudViewState.compute(
            currentLyric = null,
            artist = null,
            title = null,
            isPlaying = false
        )

        assertEquals("La Craia HUD • Aguardando áudio...", state.displayText)
        assertFalse(state.isMarquee)
        assertFalse(state.isPlaying)
    }
}
