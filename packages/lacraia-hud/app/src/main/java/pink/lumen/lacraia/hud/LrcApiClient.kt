package pink.lumen.lacraia.hud

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.HttpUrl.Companion.toHttpUrlOrNull
import okhttp3.OkHttpClient
import okhttp3.Request
import org.json.JSONObject
import java.io.IOException
import java.util.concurrent.TimeUnit

data class LrcResult(
    val track: String,
    val artist: String,
    val syncedLyrics: String?,
    val plainLyrics: String?,
    val isInstrumental: Boolean,
    val lines: List<LyricLine>,
    val fromCache: Boolean = false
)

class LrcApiClient(
    private val cache: LyricsCache? = null,
    private val client: OkHttpClient = OkHttpClient.Builder()
        .connectTimeout(5, TimeUnit.SECONDS)
        .readTimeout(10, TimeUnit.SECONDS)
        .build(),
    private val baseUrl: String = "https://lrclib.net/api/get"
) {

    suspend fun getLyrics(
        track: String,
        artist: String,
        album: String? = null,
        durationSeconds: Int? = null
    ): Result<LrcResult> = withContext(Dispatchers.IO) {
        // 1. Check local cache first for offline driving resilience
        cache?.getLyrics(track, artist)?.let { cached ->
            val parsedLines = cached.syncedLyrics?.let { LrcParser.parse(it) } ?: emptyList()
            return@withContext Result.success(
                LrcResult(
                    track = cached.track,
                    artist = cached.artist,
                    syncedLyrics = cached.syncedLyrics,
                    plainLyrics = cached.plainLyrics,
                    isInstrumental = cached.isInstrumental,
                    lines = parsedLines,
                    fromCache = true
                )
            )
        }

        // 2. Query LRCLIB API
        val urlBuilder = baseUrl.toHttpUrlOrNull()?.newBuilder()
            ?: return@withContext Result.failure(IllegalArgumentException("Invalid base URL: $baseUrl"))

        urlBuilder.addQueryParameter("track_name", track)
        urlBuilder.addQueryParameter("artist_name", artist)
        album?.let { urlBuilder.addQueryParameter("album_name", it) }
        durationSeconds?.let { urlBuilder.addQueryParameter("duration", it.toString()) }

        val request = Request.Builder()
            .url(urlBuilder.build())
            .header("User-Agent", "LaCraia-HUD/0.1.0 (https://codeberg.org/lumenpink/lacraia)")
            .build()

        try {
            client.newCall(request).execute().use { response ->
                if (!response.isSuccessful) {
                    return@withContext Result.failure(
                        IOException("LRCLIB returned HTTP status code ${response.code}")
                    )
                }

                val bodyStr = response.body?.string()
                    ?: return@withContext Result.failure(IOException("Empty response body from LRCLIB"))

                val json = JSONObject(bodyStr)
                val synced = if (json.isNull("syncedLyrics")) null else json.optString("syncedLyrics")
                val plain = if (json.isNull("plainLyrics")) null else json.optString("plainLyrics")
                val instrumental = json.optBoolean("instrumental", false)

                val parsedLines = synced?.let { LrcParser.parse(it) } ?: emptyList()

                val result = LrcResult(
                    track = track,
                    artist = artist,
                    syncedLyrics = synced,
                    plainLyrics = plain,
                    isInstrumental = instrumental,
                    lines = parsedLines,
                    fromCache = false
                )

                // 3. Save to local SQLite cache
                cache?.saveLyrics(
                    CachedLyrics(
                        track = track,
                        artist = artist,
                        syncedLyrics = synced,
                        plainLyrics = plain,
                        isInstrumental = instrumental
                    )
                )

                Result.success(result)
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
}
