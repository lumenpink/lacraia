package in._2lp.lacraia.simbiotyka

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class LrcParserTest {

    @Test
    fun testParseStandardLrc() {
        val lrc = """
            [00:15.20] Amou daquela vez como se fosse a última
            [00:22.50] Beijou sua mulher como se fosse a última
            [00:30.80] E cada filho seu como se fosse o único
        """.trimIndent()

        val lines = LrcParser.parse(lrc)
        assertEquals(3, lines.size)

        assertEquals(15200L, lines[0].timestampMs)
        assertEquals("Amou daquela vez como se fosse a última", lines[0].text)

        assertEquals(22500L, lines[1].timestampMs)
        assertEquals("Beijou sua mulher como se fosse a última", lines[1].text)

        assertEquals(30800L, lines[2].timestampMs)
        assertEquals("E cada filho seu como se fosse o único", lines[2].text)
    }

    @Test
    fun testParseThreeDigitMilliseconds() {
        val lrc = """
            [01:05.123] Linha com três casas decimais
            [02:10.050] Outra linha
        """.trimIndent()

        val lines = LrcParser.parse(lrc)
        assertEquals(2, lines.size)
        assertEquals(65123L, lines[0].timestampMs)
        assertEquals(130050L, lines[1].timestampMs)
    }

    @Test
    fun testParseOutOfOrderSorting() {
        val lrc = """
            [01:00.00] Segundo verso
            [00:10.00] Primeiro verso
            [02:00.00] Terceiro verso
        """.trimIndent()

        val lines = LrcParser.parse(lrc)
        assertEquals(3, lines.size)
        assertEquals("Primeiro verso", lines[0].text)
        assertEquals("Segundo verso", lines[1].text)
        assertEquals("Terceiro verso", lines[2].text)
    }

    @Test
    fun testIgnoresMetadataTags() {
        val lrc = """
            [ti:Construção]
            [ar:Chico Buarque]
            [al:Construção]
            [length:04:18]
            [00:15.00] Letra real
        """.trimIndent()

        val lines = LrcParser.parse(lrc)
        assertEquals(1, lines.size)
        assertEquals("Letra real", lines[0].text)
        assertEquals(15000L, lines[0].timestampMs)
    }

    @Test
    fun testEmptyInput() {
        val lines = LrcParser.parse("")
        assertTrue(lines.isEmpty())
    }
}
