package pink.lumen.lacraia.hud

import android.content.ContentValues
import android.content.Context
import android.database.sqlite.SQLiteDatabase
import android.database.sqlite.SQLiteOpenHelper

data class CachedLyrics(
    val track: String,
    val artist: String,
    val syncedLyrics: String?,
    val plainLyrics: String?,
    val isInstrumental: Boolean,
    val cachedAt: Long = System.currentTimeMillis()
)

interface LyricsCache {
    fun getLyrics(track: String, artist: String): CachedLyrics?
    fun saveLyrics(lyrics: CachedLyrics)
}

class LrcSqliteCacheHelper(context: Context) :
    SQLiteOpenHelper(context, DATABASE_NAME, null, DATABASE_VERSION), LyricsCache {

    companion object {
        const val DATABASE_NAME = "lacraia_lrc_cache.db"
        const val DATABASE_VERSION = 1

        const val TABLE_LYRICS = "cached_lyrics"
        const val COL_TRACK = "track"
        const val COL_ARTIST = "artist"
        const val COL_SYNCED = "synced_lyrics"
        const val COL_PLAIN = "plain_lyrics"
        const val COL_INSTRUMENTAL = "is_instrumental"
        const val COL_CACHED_AT = "cached_at"
    }

    override fun onCreate(db: SQLiteDatabase) {
        val createSql = """
            CREATE TABLE $TABLE_LYRICS (
                $COL_TRACK TEXT NOT NULL,
                $COL_ARTIST TEXT NOT NULL,
                $COL_SYNCED TEXT,
                $COL_PLAIN TEXT,
                $COL_INSTRUMENTAL INTEGER NOT NULL DEFAULT 0,
                $COL_CACHED_AT INTEGER NOT NULL,
                PRIMARY KEY ($COL_TRACK, $COL_ARTIST)
            )
        """.trimIndent()
        db.execSQL(createSql)
    }

    override fun onUpgrade(db: SQLiteDatabase, oldVersion: Int, newVersion: Int) {
        db.execSQL("DROP TABLE IF EXISTS $TABLE_LYRICS")
        onCreate(db)
    }

    override fun getLyrics(track: String, artist: String): CachedLyrics? {
        val db = readableDatabase
        val cursor = db.query(
            TABLE_LYRICS,
            arrayOf(COL_TRACK, COL_ARTIST, COL_SYNCED, COL_PLAIN, COL_INSTRUMENTAL, COL_CACHED_AT),
            "$COL_TRACK = ? AND $COL_ARTIST = ?",
            arrayOf(track.lowercase().trim(), artist.lowercase().trim()),
            null,
            null,
            null
        )

        return cursor.use {
            if (it.moveToFirst()) {
                CachedLyrics(
                    track = it.getString(it.getColumnIndexOrThrow(COL_TRACK)),
                    artist = it.getString(it.getColumnIndexOrThrow(COL_ARTIST)),
                    syncedLyrics = it.getString(it.getColumnIndexOrThrow(COL_SYNCED)),
                    plainLyrics = it.getString(it.getColumnIndexOrThrow(COL_PLAIN)),
                    isInstrumental = it.getInt(it.getColumnIndexOrThrow(COL_INSTRUMENTAL)) == 1,
                    cachedAt = it.getLong(it.getColumnIndexOrThrow(COL_CACHED_AT))
                )
            } else {
                null
            }
        }
    }

    override fun saveLyrics(lyrics: CachedLyrics) {
        val db = writableDatabase
        val values = ContentValues().apply {
            put(COL_TRACK, lyrics.track.lowercase().trim())
            put(COL_ARTIST, lyrics.artist.lowercase().trim())
            put(COL_SYNCED, lyrics.syncedLyrics)
            put(COL_PLAIN, lyrics.plainLyrics)
            put(COL_INSTRUMENTAL, if (lyrics.isInstrumental) 1 else 0)
            put(COL_CACHED_AT, lyrics.cachedAt)
        }
        db.insertWithOnConflict(TABLE_LYRICS, null, values, SQLiteDatabase.CONFLICT_REPLACE)
    }
}
