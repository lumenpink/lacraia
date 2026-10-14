package `in`.twolp.lacraia.simbiotyka

object LyricSyncCoordinator {

    const val DUCK_RATIO = 0.20f

    /**
     * Finds the current active lyric line for the given playback position in milliseconds.
     * Assumes lines are sorted in ascending order of timestampMs.
     */
    fun findActiveLyric(lines: List<LyricLine>, currentPositionMs: Long): LyricLine? {
        if (lines.isEmpty()) return null
        var active: LyricLine? = null
        for (line in lines) {
            if (line.timestampMs <= currentPositionMs) {
                active = line
            } else {
                break
            }
        }
        return active
    }

    /**
     * Calculates the audio ducking volume level during GPS/navigation voice prompts.
     * Scales volume to 20% when ducking is active.
     */
    fun calculateVolume(baseVolume: Float, isDucked: Boolean): Float {
        val clamped = baseVolume.coerceIn(0.0f, 1.0f)
        return if (isDucked) {
            clamped * DUCK_RATIO
        } else {
            clamped
        }
    }

    /**
     * Estimates current playback position using system elapsed real-time delta.
     */
    fun estimatePosition(lastReportedPosMs: Long, lastUpdateTimeMs: Long, currentTimeMs: Long, isPlaying: Boolean): Long {
        if (!isPlaying || currentTimeMs <= lastUpdateTimeMs) {
            return lastReportedPosMs
        }
        val delta = currentTimeMs - lastUpdateTimeMs
        return lastReportedPosMs + delta
    }
}
