package in._2lp.lacraia.simbiotyka

import kotlinx.coroutines.runBlocking
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

class LrcApiClientTest {

    private lateinit var server: MockWebServer

    class MemoryLyricsCache : LyricsCache {
        val storage = mutableMapOf<String, CachedLyrics>()

        override fun getLyrics(track: String, artist: String): CachedLyrics? {
            return storage["${track.lowercase().trim()}:${artist.lowercase().trim()}"]
        }

        override fun saveLyrics(lyrics: CachedLyrics) {
            storage["${lyrics.track.lowercase().trim()}:${lyrics.artist.lowercase().trim()}"] = lyrics
        }
    }

    @Before
    fun setUp() {
        server = MockWebServer()
        server.start()
    }

    @After
    fun tearDown() {
        server.shutdown()
    }

    @Test
    fun testCacheHitBypassesNetwork() = runBlocking {
        val cache = MemoryLyricsCache()
        cache.saveLyrics(
            CachedLyrics(
                track = "Construção",
                artist = "Chico Buarque",
                syncedLyrics = "[00:15.20] Amou daquela vez como se fosse a última",
                plainLyrics = "Amou daquela vez como se fosse a última",
                isInstrumental = false
            )
        )

        // Point to server but don't enqueue anything; if network was touched, test would fail or block
        val client = LrcApiClient(
            cache = cache,
            baseUrl = server.url("/api/get").toString()
        )

        val result = client.getLyrics("Construção", "Chico Buarque")
        assertTrue(result.isSuccess)

        val data = result.getOrNull()
        assertNotNull(data)
        assertTrue(data!!.fromCache)
        assertEquals(1, data.lines.size)
        assertEquals(15200L, data.lines[0].timestampMs)
        assertEquals("Amou daquela vez como se fosse a última", data.lines[0].text)
        assertEquals(0, server.requestCount)
    }

    @Test
    fun testCacheMissFetchesParsesAndCaches() = runBlocking {
        val cache = MemoryLyricsCache()
        val mockJson = """
            {
                "id": 42,
                "trackName": "Águas de Março",
                "artistName": "Tom Jobim",
                "albumName": "Matita Perê",
                "duration": 212,
                "instrumental": false,
                "plainLyrics": "É pau, é pedra, é o fim do caminho",
                "syncedLyrics": "[00:10.50] É pau, é pedra, é o fim do caminho\n[00:18.00] É um resto de toco, é um pouco sozinho"
            }
        """.trimIndent()

        server.enqueue(
            MockResponse()
                .setResponseCode(200)
                .setHeader("Content-Type", "application/json")
                .setBody(mockJson)
        )

        val client = LrcApiClient(
            cache = cache,
            baseUrl = server.url("/api/get").toString()
        )

        val result = client.getLyrics("Águas de Março", "Tom Jobim")
        assertTrue(result.isSuccess)

        val data = result.getOrNull()
        assertNotNull(data)
        assertFalse(data!!.fromCache)
        assertEquals(2, data.lines.size)
        assertEquals(10500L, data.lines[0].timestampMs)
        assertEquals(18000L, data.lines[1].timestampMs)

        // Verify stored in cache
        val cached = cache.getLyrics("Águas de Março", "Tom Jobim")
        assertNotNull(cached)
        assertEquals(data.syncedLyrics, cached!!.syncedLyrics)
    }
}
