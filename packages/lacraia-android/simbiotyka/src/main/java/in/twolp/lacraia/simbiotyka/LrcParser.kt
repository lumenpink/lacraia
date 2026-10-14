package `in`.twolp.lacraia.simbiotyka

data class LyricLine(
    val timestampMs: Long,
    val text: String
)

object LrcParser {
    private val regex = Regex("""\[(\d{2,}):(\d{2})(?:\.(\d{2,3}))?\](.*)""")

    fun parse(rawLrc: String): List<LyricLine> {
        return rawLrc.lines().mapNotNull { line ->
            regex.find(line.trim())?.let { match ->
                val (minStr, secStr) = match.destructured
                val subStr = match.groups[3]?.value ?: "0"
                val text = match.groups[4]?.value?.trim() ?: ""

                val min = minStr.toLongOrNull() ?: 0L
                val sec = secStr.toLongOrNull() ?: 0L
                val ms = if (subStr.length == 2) {
                    (subStr.toLongOrNull() ?: 0L) * 10
                } else {
                    subStr.toLongOrNull() ?: 0L
                }

                val totalMs = (min * 60 * 1000) + (sec * 1000) + ms
                LyricLine(totalMs, text)
            }
        }.sortedBy { it.timestampMs }
    }
}
