package pink.lumen.lacraia.hud

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class LyricSyncCoordinatorTest {

    private val sampleLyrics = listOf(
        LyricLine(10000L, "Primeiro verso"),
        LyricLine(25000L, "Segundo verso"),
        LyricLine(40000L, "Terceiro verso")
    )

    @Test
    fun testFindActiveLyricBeforeFirstLine() {
        val result = LyricSyncCoordinator.findActiveLyric(sampleLyrics, 5000L)
        assertNull(result)
    }

    @Test
    fun testFindActiveLyricExactMatch() {
        val result = LyricSyncCoordinator.findActiveLyric(sampleLyrics, 10000L)
        assertEquals("Primeiro verso", result?.text)
        assertEquals(10000L, result?.timestampMs)
    }

    @Test
    fun testFindActiveLyricBetweenLines() {
        val result = LyricSyncCoordinator.findActiveLyric(sampleLyrics, 32000L)
        assertEquals("Segundo verso", result?.text)
        assertEquals(25000L, result?.timestampMs)
    }

    @Test
    fun testFindActiveLyricAfterLastLine() {
        val result = LyricSyncCoordinator.findActiveLyric(sampleLyrics, 60000L)
        assertEquals("Terceiro verso", result?.text)
        assertEquals(40000L, result?.timestampMs)
    }

    @Test
    fun testFindActiveLyricEmptyList() {
        val result = LyricSyncCoordinator.findActiveLyric(emptyList(), 15000L)
        assertNull(result)
    }

    @Test
    fun testAudioDuckingVolumeCalculation() {
        // Unducked base volumes
        assertEquals(1.0f, LyricSyncCoordinator.calculateVolume(1.0f, isDucked = false), 0.001f)
        assertEquals(0.8f, LyricSyncCoordinator.calculateVolume(0.8f, isDucked = false), 0.001f)

        // Ducked volumes (scaled to 20%)
        assertEquals(0.20f, LyricSyncCoordinator.calculateVolume(1.0f, isDucked = true), 0.001f)
        assertEquals(0.16f, LyricSyncCoordinator.calculateVolume(0.8f, isDucked = true), 0.001f)
        assertEquals(0.10f, LyricSyncCoordinator.calculateVolume(0.5f, isDucked = true), 0.001f)
    }

    @Test
    fun testEstimatePositionInterpolation() {
        // Playing: delta is added
        val posPlaying = LyricSyncCoordinator.estimatePosition(
            lastReportedPosMs = 15000L,
            lastUpdateTimeMs = 1000L,
            currentTimeMs = 1500L,
            isPlaying = true
        )
        assertEquals(15500L, posPlaying)

        // Paused: position remains frozen
        val posPaused = LyricSyncCoordinator.estimatePosition(
            lastReportedPosMs = 15000L,
            lastUpdateTimeMs = 1000L,
            currentTimeMs = 1500L,
            isPlaying = false
        )
        assertEquals(15000L, posPaused)
    }
}
