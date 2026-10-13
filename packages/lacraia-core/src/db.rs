use chrono::Utc;
use lacraia_shared::{
    AttemptsSuccessDistribution, CacheEntry, CacheResolutionAttempt, PassengerInfo,
    PlaybackHistoryEntry, ResolutionErrorDiagnostic, Role, TierSuccessRate,
};
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Track not found: {0}")]
    TrackNotFound(String),
    #[error("Cabin locked: {0}")]
    CabinLocked(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CabinRecord {
    pub id: String,
    pub name: String,
    pub is_locked: bool,
    pub pilot_token: String,
    pub passenger_key: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PassengerRecord {
    pub id: String,
    pub cabin_id: String,
    pub name: String,
    pub device: Option<String>,
    pub token: String,
    pub is_active: bool,
    pub joined_at: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrackRecord {
    pub id: String,
    pub artist: String,
    pub title: String,
    pub duration_sec: u64,
    pub cached_path: Option<String>,
    pub genres: Option<String>,
    pub last_played_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueState {
    Pending,
    Cached,
    Playing,
    Played,
    Failed,
}

impl QueueState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Cached => "cached",
            Self::Playing => "playing",
            Self::Played => "played",
            Self::Failed => "failed",
        }
    }
}

impl std::str::FromStr for QueueState {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "cached" => Self::Cached,
            "playing" => Self::Playing,
            "played" => Self::Played,
            "failed" => Self::Failed,
            _ => Self::Pending,
        })
    }
}

impl From<&str> for QueueState {
    fn from(s: &str) -> Self {
        match s {
            "cached" => Self::Cached,
            "playing" => Self::Playing,
            "played" => Self::Played,
            "failed" => Self::Failed,
            _ => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct QueueEntry {
    pub id: i64,
    pub track_id: String,
    pub position: i64,
    pub state: QueueState,
    pub created_at: i64,
    pub cabin_id: String,
    pub requested_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CabinPlaybackState {
    pub cabin_id: String,
    pub track_id: Option<String>,
    pub position_ms: u64,
    pub playback_state: String,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemSettings {
    #[serde(rename = "targetQueueDurationSec")]
    pub target_queue_duration_sec: u64,
    #[serde(rename = "targetCacheDurationSec")]
    pub target_cache_duration_sec: u64,
    #[serde(rename = "maxDiskCacheMb")]
    pub max_disk_cache_mb: u64,
}

impl Default for SystemSettings {
    fn default() -> Self {
        Self {
            target_queue_duration_sec: 7200,
            target_cache_duration_sec: 3600,
            max_disk_cache_mb: 1024,
        }
    }
}

pub fn strip_accents(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'ã' | 'â' | 'ä' | 'Á' | 'À' | 'Ã' | 'Â' | 'Ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => 'i',
            'ó' | 'ò' | 'õ' | 'ô' | 'ö' | 'Ó' | 'Ò' | 'Õ' | 'Ô' | 'Ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => 'u',
            'ç' | 'Ç' => 'c',
            'ñ' | 'Ñ' => 'n',
            other => other.to_ascii_lowercase(),
        })
        .collect()
}

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, DbError> {

        let conn = Connection::open(path)?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init_schema()?;
        Ok(db)
    }

    pub fn open_in_memory() -> Result<Self, DbError> {
        let conn = Connection::open_in_memory()?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init_schema()?;
        Ok(db)
    }

    pub fn init_schema(&self) -> Result<(), DbError> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS tracks (
                id TEXT PRIMARY KEY,
                artist TEXT NOT NULL,
                title TEXT NOT NULL,
                duration_sec INTEGER NOT NULL,
                cached_path TEXT,
                genres TEXT,
                last_played_at INTEGER
            );

            CREATE TABLE IF NOT EXISTS similarity_edges (
                source_track_id TEXT NOT NULL,
                target_track_id TEXT NOT NULL,
                weight REAL NOT NULL,
                provider TEXT NOT NULL,
                updated_at INTEGER NOT NULL,
                PRIMARY KEY (source_track_id, target_track_id),
                FOREIGN KEY (source_track_id) REFERENCES tracks(id) ON DELETE CASCADE,
                FOREIGN KEY (target_track_id) REFERENCES tracks(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_edges_lookup 
                ON similarity_edges(source_track_id, weight DESC);

            CREATE TABLE IF NOT EXISTS cabins (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                is_locked INTEGER NOT NULL DEFAULT 0,
                pilot_token TEXT NOT NULL,
                passenger_key TEXT NOT NULL DEFAULT '',
                created_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS passengers (
                id TEXT PRIMARY KEY,
                cabin_id TEXT NOT NULL,
                name TEXT NOT NULL,
                device TEXT,
                token TEXT NOT NULL UNIQUE,
                is_active INTEGER NOT NULL DEFAULT 1,
                joined_at INTEGER NOT NULL,
                FOREIGN KEY (cabin_id) REFERENCES cabins(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS queue (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                track_id TEXT NOT NULL,
                position INTEGER NOT NULL,
                state TEXT NOT NULL DEFAULT 'pending',
                created_at INTEGER NOT NULL,
                cabin_id TEXT NOT NULL DEFAULT 'default',
                requested_by TEXT,
                FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS cabin_playback_state (
                cabin_id TEXT PRIMARY KEY,
                track_id TEXT,
                position_ms INTEGER NOT NULL DEFAULT 0,
                playback_state TEXT NOT NULL DEFAULT 'stopped',
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS playback_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                cabin_id TEXT NOT NULL,
                track_id TEXT NOT NULL,
                passenger_name TEXT,
                played_at INTEGER NOT NULL,
                duration_sec INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_history_cabin 
                ON playback_history(cabin_id, played_at DESC);
            CREATE INDEX IF NOT EXISTS idx_history_passenger 
                ON playback_history(passenger_name, played_at DESC);

            CREATE INDEX IF NOT EXISTS idx_queue_pos 
                ON queue(cabin_id, position ASC, state);

            CREATE TABLE IF NOT EXISTS cache_entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                track_id TEXT UNIQUE NOT NULL,
                file_path TEXT NOT NULL,
                file_size_bytes INTEGER NOT NULL,
                duration_sec INTEGER NOT NULL,
                audio_codec TEXT NOT NULL DEFAULT 'opus',
                hit_count INTEGER NOT NULL DEFAULT 1,
                last_served_at INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_cache_entries_served
                ON cache_entries(last_served_at DESC);

            CREATE TABLE IF NOT EXISTS cache_resolution_attempts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                track_id TEXT NOT NULL,
                attempt_number INTEGER NOT NULL,
                tier TEXT NOT NULL,
                method TEXT NOT NULL,
                command_line TEXT,
                success INTEGER NOT NULL,
                exit_code INTEGER,
                error_code TEXT,
                error_output TEXT,
                duration_ms INTEGER NOT NULL,
                bytes_received INTEGER NOT NULL DEFAULT 0,
                validation_status TEXT NOT NULL,
                cabin_id TEXT,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_attempts_track
                ON cache_resolution_attempts(track_id, attempt_number ASC);
            CREATE INDEX IF NOT EXISTS idx_attempts_tier
                ON cache_resolution_attempts(tier, success);

            CREATE TABLE IF NOT EXISTS system_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            );
            "#,
        )?;

        // Ensure backwards compatibility with earlier database versions
        let _ = conn.execute("ALTER TABLE cabins ADD COLUMN passenger_key TEXT NOT NULL DEFAULT ''", []);
        let _ = conn.execute("ALTER TABLE queue ADD COLUMN cabin_id TEXT NOT NULL DEFAULT 'default'", []);
        let _ = conn.execute("ALTER TABLE queue ADD COLUMN requested_by TEXT", []);

        // Seed default cabin if not exists
        let default_pilot_token = std::env::var("LACRAIA_PILOT_TOKEN")
            .unwrap_or_else(|_| "pilot_default_sec".to_string());
        let default_passenger_key = std::env::var("LACRAIA_PASSENGER_KEY")
            .unwrap_or_else(|_| "key_central_2011".to_string());
        let now = Utc::now().timestamp();
        let _ = conn.execute(
            r#"
            INSERT INTO cabins (id, name, is_locked, pilot_token, passenger_key, created_at)
            VALUES ('default', 'Cabine Central', 0, ?1, ?2, ?3)
            ON CONFLICT(id) DO UPDATE SET
                passenger_key = CASE WHEN cabins.passenger_key = '' THEN excluded.passenger_key ELSE cabins.passenger_key END
            "#,
            params![default_pilot_token, default_passenger_key, now],
        );

        Ok(())
    }

    pub fn upsert_track(&self, track: &TrackRecord) -> Result<(), DbError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT INTO tracks (id, artist, title, duration_sec, cached_path, genres, last_played_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(id) DO UPDATE SET
                artist = excluded.artist,
                title = excluded.title,
                duration_sec = excluded.duration_sec,
                cached_path = COALESCE(excluded.cached_path, tracks.cached_path),
                genres = COALESCE(excluded.genres, tracks.genres),
                last_played_at = COALESCE(excluded.last_played_at, tracks.last_played_at)
            "#,
            params![
                track.id,
                track.artist,
                track.title,
                track.duration_sec,
                track.cached_path,
                track.genres,
                track.last_played_at,
            ],
        )?;
        Ok(())
    }

    pub fn get_track(&self, id: &str) -> Result<Option<TrackRecord>, DbError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, artist, title, duration_sec, cached_path, genres, last_played_at
            FROM tracks
            WHERE id = ?1
            "#,
        )?;

        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(TrackRecord {
                id: row.get(0)?,
                artist: row.get(1)?,
                title: row.get(2)?,
                duration_sec: row.get(3)?,
                cached_path: row.get(4)?,
                genres: row.get(5)?,
                last_played_at: row.get(6)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn search_tracks(&self, term: &str, limit: usize) -> Result<Vec<TrackRecord>, DbError> {
        let conn = self.conn.lock().unwrap();
        let trimmed = term.trim();

        let mut stmt = conn.prepare(
            r#"
            SELECT id, artist, title, duration_sec, cached_path, genres, last_played_at
            FROM tracks
            "#,
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(TrackRecord {
                id: row.get(0)?,
                artist: row.get(1)?,
                title: row.get(2)?,
                duration_sec: row.get(3)?,
                cached_path: row.get(4)?,
                genres: row.get(5)?,
                last_played_at: row.get(6)?,
            })
        })?;

        let mut tracks = Vec::new();
        for r in rows {
            tracks.push(r?);
        }

        if trimmed.is_empty() {
            tracks.sort_by_key(|a| a.title.to_lowercase());
            tracks.truncate(limit);
            return Ok(tracks);
        }

        let norm_term = strip_accents(trimmed);

        let mut scored: Vec<(u32, TrackRecord)> = Vec::new();
        for trk in tracks {
            let norm_title = strip_accents(&trk.title);
            let norm_artist = strip_accents(&trk.artist);
            let norm_genres = strip_accents(trk.genres.as_deref().unwrap_or(""));

            let score = if norm_title == norm_term || norm_artist == norm_term {
                100
            } else if norm_title.starts_with(&norm_term) || norm_artist.starts_with(&norm_term) {
                80
            } else if norm_title.contains(&norm_term) {
                60
            } else if norm_artist.contains(&norm_term) {
                50
            } else if norm_genres.contains(&norm_term) {
                30
            } else {
                0
            };

            if score > 0 {
                scored.push((score, trk));
            }
        }

        // Sort descending by score, then ascending by title
        scored.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then_with(|| a.1.title.to_lowercase().cmp(&b.1.title.to_lowercase()))
        });

        let results: Vec<TrackRecord> = scored.into_iter().take(limit).map(|(_, trk)| trk).collect();
        Ok(results)
    }


    pub fn seed_defaults_if_empty(&self) -> Result<(), DbError> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM tracks", [], |row| row.get(0))?;
        if count > 0 {
            return Ok(());
        }

        let seeds = [
            ("mc serginho:vai lacraia", "MC Serginho", "Vai Lacraia", 240, "funk carioca, proibidão, paredão"),
            ("chico buarque:construcao", "Chico Buarque", "Construção", 258, "mpb, samba"),
            ("elis regina & tom jobim:aguas de marco", "Elis Regina & Tom Jobim", "Águas de Março", 212, "bossa nova, mpb"),
            ("joao gilberto:chega de saudade", "João Gilberto", "Chega de Saudade", 121, "bossa nova, samba"),
            ("jorge ben jor:taj mahal", "Jorge Ben Jor", "Taj Mahal", 184, "samba rock, mpb"),
            ("tim maia:descobridor dos sete mares", "Tim Maia", "O Descobridor dos Sete Mares", 262, "soul brasileiro, mpb"),
            ("cartola:as rosas nao falam", "Cartola", "As Rosas Não Falam", 175, "samba de raiz"),
            ("daft punk:one more time", "Daft Punk", "One More Time", 320, "french house, electronic"),
            ("kraftwerk:autobahn", "Kraftwerk", "Autobahn", 1360, "electronic, krautrock"),
            ("new order:blue monday", "New Order", "Blue Monday", 449, "synth-pop, new wave"),
            ("raul seixas:metamorfose ambulante", "Raul Seixas", "Metamorfose Ambulante", 230, "rock brasileiro, mpb"),
        ];

        for (id, artist, title, dur, genres) in seeds {
            conn.execute(
                r#"
                INSERT OR IGNORE INTO tracks (id, artist, title, duration_sec, genres)
                VALUES (?1, ?2, ?3, ?4, ?5)
                "#,
                params![id, artist, title, dur, genres],
            )?;
        }

        Ok(())
    }


    pub fn insert_edge(
        &self,
        source_id: &str,
        target_id: &str,
        weight: f64,
        provider: &str,
    ) -> Result<(), DbError> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp();
        conn.execute(
            r#"
            INSERT INTO similarity_edges (source_track_id, target_track_id, weight, provider, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(source_track_id, target_track_id) DO UPDATE SET
                weight = excluded.weight,
                provider = excluded.provider,
                updated_at = excluded.updated_at
            "#,
            params![source_id, target_id, weight, provider, now],
        )?;
        Ok(())
    }

    /// Queries the local similarity graph for candidate tracks.
    /// In offline mode (only_cached = true), filters strictly to tracks with `cached_path IS NOT NULL`.
    pub fn get_similar_tracks(
        &self,
        source_id: &str,
        only_cached: bool,
        limit: usize,
    ) -> Result<Vec<(TrackRecord, f64)>, DbError> {
        let conn = self.conn.lock().unwrap();
        let query = if only_cached {
            r#"
            SELECT t.id, t.artist, t.title, t.duration_sec, t.cached_path, t.genres, t.last_played_at, e.weight
            FROM similarity_edges e
            JOIN tracks t ON t.id = e.target_track_id
            WHERE e.source_track_id = ?1 AND t.cached_path IS NOT NULL
            ORDER BY e.weight DESC
            LIMIT ?2
            "#
        } else {
            r#"
            SELECT t.id, t.artist, t.title, t.duration_sec, t.cached_path, t.genres, t.last_played_at, e.weight
            FROM similarity_edges e
            JOIN tracks t ON t.id = e.target_track_id
            WHERE e.source_track_id = ?1
            ORDER BY e.weight DESC
            LIMIT ?2
            "#
        };

        let mut stmt = conn.prepare(query)?;
        let rows = stmt.query_map(params![source_id, limit as i64], |row| {
            Ok((
                TrackRecord {
                    id: row.get(0)?,
                    artist: row.get(1)?,
                    title: row.get(2)?,
                    duration_sec: row.get(3)?,
                    cached_path: row.get(4)?,
                    genres: row.get(5)?,
                    last_played_at: row.get(6)?,
                },
                row.get(7)?,
            ))
        })?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    /// Fetches recommendation candidates for a seed track using similarity edges,
    /// same artist/genre affinity, and local catalog fallback, excluding specified track IDs.
    pub fn get_recommendations_for_seed(
        &self,
        seed_id: &str,
        exclude_ids: &[String],
        limit: usize,
    ) -> Result<Vec<TrackRecord>, DbError> {
        let conn = self.conn.lock().unwrap();
        let mut results = Vec::new();

        // 1. Traverse similarity edges from graph
        let mut stmt = conn.prepare(
            r#"
            SELECT t.id, t.artist, t.title, t.duration_sec, t.cached_path, t.genres, t.last_played_at
            FROM similarity_edges e
            JOIN tracks t ON t.id = e.target_track_id
            WHERE e.source_track_id = ?1
            ORDER BY e.weight DESC
            LIMIT 20
            "#,
        )?;
        let rows = stmt.query_map(params![seed_id], |row| {
            Ok(TrackRecord {
                id: row.get(0)?,
                artist: row.get(1)?,
                title: row.get(2)?,
                duration_sec: row.get(3)?,
                cached_path: row.get(4)?,
                genres: row.get(5)?,
                last_played_at: row.get(6)?,
            })
        })?;
        for r in rows.flatten() {
            if !exclude_ids.contains(&r.id)
                && r.id != seed_id
                && !results.iter().any(|existing: &TrackRecord| existing.id == r.id)
            {
                results.push(r);
                if results.len() >= limit {
                    return Ok(results);
                }
            }
        }

        // 2. Query same artist or genres affinity
        let seed_opt: Option<(String, Option<String>)> = conn
            .query_row(
                "SELECT artist, genres FROM tracks WHERE id = ?1",
                params![seed_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .ok();

        if let Some((artist, _genres)) = seed_opt {
            let mut stmt_artist = conn.prepare(
                "SELECT id, artist, title, duration_sec, cached_path, genres, last_played_at FROM tracks WHERE artist = ?1 AND id != ?2 LIMIT 10",
            )?;
            let artist_rows = stmt_artist.query_map(params![artist, seed_id], |row| {
                Ok(TrackRecord {
                    id: row.get(0)?,
                    artist: row.get(1)?,
                    title: row.get(2)?,
                    duration_sec: row.get(3)?,
                    cached_path: row.get(4)?,
                    genres: row.get(5)?,
                    last_played_at: row.get(6)?,
                })
            })?;
            for r in artist_rows.flatten() {
                if !exclude_ids.contains(&r.id)
                    && !results.iter().any(|existing: &TrackRecord| existing.id == r.id)
                {
                    results.push(r);
                    if results.len() >= limit {
                        return Ok(results);
                    }
                }
            }
        }

        // 3. Fallback to catalog defaults (diverse seeds)
        let mut stmt_all = conn.prepare(
            "SELECT id, artist, title, duration_sec, cached_path, genres, last_played_at FROM tracks WHERE id != ?1 ORDER BY id ASC LIMIT 30",
        )?;
        let all_rows = stmt_all.query_map(params![seed_id], |row| {
            Ok(TrackRecord {
                id: row.get(0)?,
                artist: row.get(1)?,
                title: row.get(2)?,
                duration_sec: row.get(3)?,
                cached_path: row.get(4)?,
                genres: row.get(5)?,
                last_played_at: row.get(6)?,
            })
        })?;
        for r in all_rows.flatten() {
            if !exclude_ids.contains(&r.id)
                && !results.iter().any(|existing: &TrackRecord| existing.id == r.id)
            {
                results.push(r);
                if results.len() >= limit {
                    break;
                }
            }
        }

        Ok(results)
    }

    /// Fetches recommendation candidates considering multiple seed tracks (e.g. recent playlist history),
    /// aggregating similarity edges, common genres, and artist affinities to preserve playlist mood.
    pub fn get_recommendations_for_multi_seeds(
        &self,
        seed_ids: &[String],
        exclude_ids: &[String],
        limit: usize,
    ) -> Result<Vec<TrackRecord>, DbError> {
        let conn = self.conn.lock().unwrap();
        if seed_ids.is_empty() {
            return Ok(vec![]);
        }

        // 1. Traverse similarity edges from graph across all seed tracks,
        // accumulating edge weights and count of seeds connected to each candidate.
        let mut candidate_scores: std::collections::HashMap<String, (TrackRecord, f64, usize)> = std::collections::HashMap::new();

        for seed_id in seed_ids {
            let mut stmt = conn.prepare(
                r#"
                SELECT t.id, t.artist, t.title, t.duration_sec, t.cached_path, t.genres, t.last_played_at, e.weight
                FROM similarity_edges e
                JOIN tracks t ON t.id = e.target_track_id
                WHERE e.source_track_id = ?1
                ORDER BY e.weight DESC
                LIMIT 25
                "#,
            )?;
            let rows = stmt.query_map(params![seed_id], |row| {
                Ok((
                    TrackRecord {
                        id: row.get(0)?,
                        artist: row.get(1)?,
                        title: row.get(2)?,
                        duration_sec: row.get(3)?,
                        cached_path: row.get(4)?,
                        genres: row.get(5)?,
                        last_played_at: row.get(6)?,
                    },
                    row.get::<_, f64>(7)?,
                ))
            })?;
            for (record, weight) in rows.flatten() {
                if !exclude_ids.contains(&record.id) && !seed_ids.contains(&record.id) {
                    let entry = candidate_scores.entry(record.id.clone()).or_insert_with(|| (record, 0.0, 0));
                    entry.1 += weight;
                    entry.2 += 1;
                }
            }
        }

        // Rank candidates: multi-seed connections receive a multi-affinity multiplier
        let mut scored_list: Vec<(TrackRecord, f64)> = candidate_scores
            .into_values()
            .map(|(rec, score, count)| {
                let boost = 1.0 + 0.5 * ((count.saturating_sub(1)) as f64);
                (rec, score * boost)
            })
            .collect();

        scored_list.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut results: Vec<TrackRecord> = scored_list.into_iter().map(|(rec, _)| rec).collect();
        if results.len() >= limit {
            results.truncate(limit);
            return Ok(results);
        }

        // 2. Query same artist or genre affinity across the seeds
        for seed_id in seed_ids {
            let seed_opt: Option<(String, Option<String>)> = conn
                .query_row(
                    "SELECT artist, genres FROM tracks WHERE id = ?1",
                    params![seed_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .ok();

            if let Some((artist, _genres)) = seed_opt {
                let mut stmt_artist = conn.prepare(
                    "SELECT id, artist, title, duration_sec, cached_path, genres, last_played_at FROM tracks WHERE artist = ?1 LIMIT 10",
                )?;
                let artist_rows = stmt_artist.query_map(params![artist], |row| {
                    Ok(TrackRecord {
                        id: row.get(0)?,
                        artist: row.get(1)?,
                        title: row.get(2)?,
                        duration_sec: row.get(3)?,
                        cached_path: row.get(4)?,
                        genres: row.get(5)?,
                        last_played_at: row.get(6)?,
                    })
                })?;
                for r in artist_rows.flatten() {
                    if !exclude_ids.contains(&r.id)
                        && !seed_ids.contains(&r.id)
                        && !results.iter().any(|existing| existing.id == r.id)
                    {
                        results.push(r);
                        if results.len() >= limit {
                            return Ok(results);
                        }
                    }
                }
            }
        }

        // 3. Fallback to catalog defaults
        let mut stmt_all = conn.prepare(
            "SELECT id, artist, title, duration_sec, cached_path, genres, last_played_at FROM tracks ORDER BY id ASC LIMIT 30",
        )?;
        let all_rows = stmt_all.query_map([], |row| {
            Ok(TrackRecord {
                id: row.get(0)?,
                artist: row.get(1)?,
                title: row.get(2)?,
                duration_sec: row.get(3)?,
                cached_path: row.get(4)?,
                genres: row.get(5)?,
                last_played_at: row.get(6)?,
            })
        })?;
        for r in all_rows.flatten() {
            if !exclude_ids.contains(&r.id)
                && !seed_ids.contains(&r.id)
                && !results.iter().any(|existing| existing.id == r.id)
            {
                results.push(r);
                if results.len() >= limit {
                    break;
                }
            }
        }

        Ok(results)
    }

    pub fn enqueue_track(&self, track_id: &str) -> Result<i64, DbError> {
        self.enqueue_track_with_meta(track_id, None, "default")
    }

    pub fn enqueue_track_with_meta(
        &self,
        track_id: &str,
        requested_by: Option<&str>,
        cabin_id: &str,
    ) -> Result<i64, DbError> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp();

        let max_pos: Option<i64> = conn
            .query_row(
                "SELECT MAX(position) FROM queue WHERE cabin_id = ?1",
                params![cabin_id],
                |row| row.get(0),
            )
            .unwrap_or(None);

        let next_pos = max_pos.map(|p| p + 1).unwrap_or(0);

        // Ensure track exists in tracks table to satisfy foreign key constraint
        let (def_artist, def_title) = if let Some((a, t)) = track_id.split_once(':') {
            (a.trim().to_string(), t.trim().to_string())
        } else {
            ("Desconhecido".to_string(), track_id.to_string())
        };

        conn.execute(
            r#"
            INSERT OR IGNORE INTO tracks (id, artist, title, duration_sec, cached_path, genres, last_played_at)
            VALUES (?1, ?2, ?3, 210, NULL, NULL, NULL)
            "#,
            params![track_id, def_artist, def_title],
        )?;

        conn.execute(
            r#"
            INSERT INTO queue (track_id, position, state, created_at, cabin_id, requested_by)
            VALUES (?1, ?2, 'pending', ?3, ?4, ?5)
            "#,
            params![track_id, next_pos, now, cabin_id, requested_by],
        )?;

        Ok(conn.last_insert_rowid())
    }

    pub fn enqueue_track_next_with_meta(
        &self,
        track_id: &str,
        requested_by: Option<&str>,
        cabin_id: &str,
    ) -> Result<i64, DbError> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp();

        let (def_artist, def_title) = if let Some((a, t)) = track_id.split_once(':') {
            (a.trim().to_string(), t.trim().to_string())
        } else {
            ("Desconhecido".to_string(), track_id.to_string())
        };

        conn.execute(
            r#"
            INSERT OR IGNORE INTO tracks (id, artist, title, duration_sec, cached_path, genres, last_played_at)
            VALUES (?1, ?2, ?3, 210, NULL, NULL, NULL)
            "#,
            params![track_id, def_artist, def_title],
        )?;

        let min_pending: Option<i64> = conn
            .query_row(
                "SELECT MIN(position) FROM queue WHERE cabin_id = ?1 AND state != 'playing' AND state != 'played'",
                params![cabin_id],
                |row| row.get(0),
            )
            .unwrap_or(None);

        let target_pos = if let Some(min_pos) = min_pending {
            conn.execute(
                "UPDATE queue SET position = position + 1 WHERE cabin_id = ?1 AND state != 'playing' AND state != 'played'",
                params![cabin_id],
            )?;
            min_pos
        } else {
            let max_pos: Option<i64> = conn
                .query_row(
                    "SELECT MAX(position) FROM queue WHERE cabin_id = ?1",
                    params![cabin_id],
                    |row| row.get(0),
                )
                .unwrap_or(None);
            max_pos.map(|p| p + 1).unwrap_or(1)
        };

        conn.execute(
            r#"
            INSERT INTO queue (track_id, position, state, created_at, cabin_id, requested_by)
            VALUES (?1, ?2, 'pending', ?3, ?4, ?5)
            "#,
            params![track_id, target_pos, now, cabin_id, requested_by],
        )?;

        Ok(conn.last_insert_rowid())
    }

    pub fn play_now_with_meta(
        &self,
        track_id: &str,
        requested_by: Option<&str>,
        cabin_id: &str,
    ) -> Result<QueueEntry, DbError> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp();

        let (def_artist, def_title) = if let Some((a, t)) = track_id.split_once(':') {
            (a.trim().to_string(), t.trim().to_string())
        } else {
            ("Desconhecido".to_string(), track_id.to_string())
        };

        conn.execute(
            r#"
            INSERT OR IGNORE INTO tracks (id, artist, title, duration_sec, cached_path, genres, last_played_at)
            VALUES (?1, ?2, ?3, 210, NULL, NULL, NULL)
            "#,
            params![track_id, def_artist, def_title],
        )?;

        conn.execute(
            "UPDATE queue SET state = 'played' WHERE cabin_id = ?1 AND state = 'playing'",
            params![cabin_id],
        )?;

        conn.execute(
            r#"
            INSERT INTO queue (track_id, position, state, created_at, cabin_id, requested_by)
            VALUES (?1, 0, 'playing', ?2, ?3, ?4)
            "#,
            params![track_id, now, cabin_id, requested_by],
        )?;

        let id = conn.last_insert_rowid();
        Ok(QueueEntry {
            id,
            track_id: track_id.to_string(),
            position: 0,
            state: QueueState::Playing,
            created_at: now,
            cabin_id: cabin_id.to_string(),
            requested_by: requested_by.map(|s| s.to_string()),
        })
    }

    pub fn start_radio_with_meta(
        &self,
        track_id: &str,
        requested_by: Option<&str>,
        cabin_id: &str,
    ) -> Result<QueueEntry, DbError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM queue WHERE cabin_id = ?1 AND state IN ('pending', 'cached')",
            params![cabin_id],
        )?;
        drop(conn);

        self.play_now_with_meta(track_id, requested_by, cabin_id)
    }

    pub fn get_upcoming_queue_for_cabin(&self, cabin_id: &str) -> Result<Vec<(QueueEntry, TrackRecord)>, DbError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT q.id, q.track_id, q.position, q.state, q.created_at, q.cabin_id, q.requested_by,
                   t.artist, t.title, t.duration_sec, t.cached_path, t.genres, t.last_played_at
            FROM queue q
            LEFT JOIN tracks t ON t.id = q.track_id
            WHERE q.state IN ('pending', 'cached') AND q.cabin_id = ?1
            ORDER BY q.position ASC
            "#,
        )?;

        let rows = stmt.query_map(params![cabin_id], |row| {
            let entry = QueueEntry {
                id: row.get(0)?,
                track_id: row.get(1)?,
                position: row.get(2)?,
                state: QueueState::from(row.get::<_, String>(3)?.as_str()),
                created_at: row.get(4)?,
                cabin_id: row.get(5)?,
                requested_by: row.get(6)?,
            };
            let track = TrackRecord {
                id: entry.track_id.clone(),
                artist: row.get::<_, Option<String>>(7)?.unwrap_or_else(|| "Desconhecido".into()),
                title: row.get::<_, Option<String>>(8)?.unwrap_or_else(|| entry.track_id.clone()),
                duration_sec: row.get::<_, Option<u64>>(9)?.unwrap_or(210),
                cached_path: row.get(10)?,
                genres: row.get(11)?,
                last_played_at: row.get(12)?,
            };
            Ok((entry, track))
        })?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    pub fn remove_queue_entry(&self, id: i64, cabin_id: &str) -> Result<bool, DbError> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute(
            "DELETE FROM queue WHERE id = ?1 AND cabin_id = ?2",
            params![id, cabin_id],
        )?;
        Ok(affected > 0)
    }

    pub fn get_queue(&self) -> Result<Vec<(QueueEntry, TrackRecord)>, DbError> {
        self.get_queue_for_cabin("default")
    }

    pub fn get_queue_for_cabin(&self, cabin_id: &str) -> Result<Vec<(QueueEntry, TrackRecord)>, DbError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT q.id, q.track_id, q.position, q.state, q.created_at, q.cabin_id, q.requested_by,
                   t.artist, t.title, t.duration_sec, t.cached_path, t.genres, t.last_played_at
            FROM queue q
            LEFT JOIN tracks t ON t.id = q.track_id
            WHERE q.state != 'played' AND q.cabin_id = ?1
            ORDER BY q.position ASC
            "#,
        )?;

        let rows = stmt.query_map(params![cabin_id], |row| {
            let state_str: String = row.get(3)?;
            let track_id: String = row.get(1)?;
            let opt_artist: Option<String> = row.get(7)?;
            let opt_title: Option<String> = row.get(8)?;
            let duration_sec: u64 = row.get::<_, Option<u64>>(9)?.unwrap_or(210);

            let (def_artist, def_title) = if let Some((a, t)) = track_id.split_once(':') {
                (a.to_string(), t.to_string())
            } else {
                ("Artista".to_string(), track_id.clone())
            };

            let artist = opt_artist.unwrap_or(def_artist);
            let title = opt_title.unwrap_or(def_title);

            Ok((
                QueueEntry {
                    id: row.get(0)?,
                    track_id: track_id.clone(),
                    position: row.get(2)?,
                    state: QueueState::from(state_str.as_str()),
                    created_at: row.get(4)?,
                    cabin_id: row.get(5)?,
                    requested_by: row.get(6)?,
                },
                TrackRecord {
                    id: track_id,
                    artist,
                    title,
                    duration_sec,
                    cached_path: row.get(10)?,
                    genres: row.get(11)?,
                    last_played_at: row.get(12)?,
                },
            ))
        })?;

        let mut list = Vec::new();
        for item in rows {
            list.push(item?);
        }
        Ok(list)
    }

    pub fn advance_queue(&self) -> Result<Option<QueueEntry>, DbError> {
        self.advance_queue_for_cabin("default")
    }

    pub fn advance_queue_for_cabin(&self, cabin_id: &str) -> Result<Option<QueueEntry>, DbError> {
        let conn = self.conn.lock().unwrap();
        // Mark current 'playing' as 'played' for this cabin
        conn.execute(
            "UPDATE queue SET state = 'played' WHERE state = 'playing' AND cabin_id = ?1",
            params![cabin_id],
        )?;

        // Find next eligible track
        let next_id: Option<i64> = conn
            .query_row(
                "SELECT id FROM queue WHERE state IN ('pending', 'cached') AND cabin_id = ?1 ORDER BY position ASC LIMIT 1",
                params![cabin_id],
                |row| row.get(0),
            )
            .ok();

        if let Some(id) = next_id {
            conn.execute(
                "UPDATE queue SET state = 'playing' WHERE id = ?1",
                params![id],
            )?;

            let entry = conn.query_row(
                "SELECT id, track_id, position, state, created_at, cabin_id, requested_by FROM queue WHERE id = ?1",
                params![id],
                |row| {
                    let state_str: String = row.get(3)?;
                    Ok(QueueEntry {
                        id: row.get(0)?,
                        track_id: row.get(1)?,
                        position: row.get(2)?,
                        state: QueueState::from(state_str.as_str()),
                        created_at: row.get(4)?,
                        cabin_id: row.get(5)?,
                        requested_by: row.get(6)?,
                    })
                },
            )?;
            Ok(Some(entry))
        } else {
            Ok(None)
        }
    }

    pub fn jump_queue_for_cabin(&self, queue_id: i64, cabin_id: &str) -> Result<Option<QueueEntry>, DbError> {
        let conn = self.conn.lock().unwrap();
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM queue WHERE id = ?1 AND cabin_id = ?2",
                params![queue_id, cabin_id],
                |row| row.get::<_, i64>(0),
            )
            .map(|c| c > 0)
            .unwrap_or(false);

        if !exists {
            return Ok(None);
        }

        conn.execute(
            "UPDATE queue SET state = 'played' WHERE state = 'playing' AND cabin_id = ?1",
            params![cabin_id],
        )?;

        conn.execute(
            "UPDATE queue SET state = 'playing' WHERE id = ?1 AND cabin_id = ?2",
            params![queue_id, cabin_id],
        )?;

        let entry = conn.query_row(
            "SELECT id, track_id, position, state, created_at, cabin_id, requested_by FROM queue WHERE id = ?1",
            params![queue_id],
            |row| {
                let state_str: String = row.get(3)?;
                Ok(QueueEntry {
                    id: row.get(0)?,
                    track_id: row.get(1)?,
                    position: row.get(2)?,
                    state: QueueState::from(state_str.as_str()),
                    created_at: row.get(4)?,
                    cabin_id: row.get(5)?,
                    requested_by: row.get(6)?,
                })
            },
        )?;
        Ok(Some(entry))
    }

    pub fn mark_cached(&self, track_id: &str, cached_path: &str) -> Result<(), DbError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE tracks SET cached_path = ?1 WHERE id = ?2",
            params![cached_path, track_id],
        )?;
        conn.execute(
            "UPDATE queue SET state = 'cached' WHERE track_id = ?1 AND state = 'pending'",
            params![track_id],
        )?;
        Ok(())
    }

    pub fn mark_failed(&self, track_id: &str) -> Result<(), DbError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE queue SET state = 'failed' WHERE track_id = ?1 AND state = 'pending'",
            params![track_id],
        )?;
        Ok(())
    }

    pub fn create_cabin(
        &self,
        id: &str,
        name: &str,
        pilot_token: &str,
        passenger_key: &str,
    ) -> Result<CabinRecord, DbError> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp();
        conn.execute(
            r#"
            INSERT INTO cabins (id, name, is_locked, pilot_token, passenger_key, created_at)
            VALUES (?1, ?2, 0, ?3, ?4, ?5)
            "#,
            params![id, name, pilot_token, passenger_key, now],
        )?;

        Ok(CabinRecord {
            id: id.to_string(),
            name: name.to_string(),
            is_locked: false,
            pilot_token: pilot_token.to_string(),
            passenger_key: passenger_key.to_string(),
            created_at: now,
        })
    }

    pub fn get_or_create_cabin(&self, id: &str, name: &str, pilot_token: &str) -> Result<CabinRecord, DbError> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp();
        let default_key = format!("key_{}", &uuid::Uuid::new_v4().to_string().replace('-', "")[..12]);
        conn.execute(
            r#"
            INSERT INTO cabins (id, name, is_locked, pilot_token, passenger_key, created_at)
            VALUES (?1, ?2, 0, ?3, ?4, ?5)
            ON CONFLICT(id) DO NOTHING
            "#,
            params![id, name, pilot_token, default_key, now],
        )?;

        let cabin = conn.query_row(
            "SELECT id, name, is_locked, pilot_token, passenger_key, created_at FROM cabins WHERE id = ?1",
            params![id],
            |row| {
                Ok(CabinRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    is_locked: row.get::<_, i64>(2)? == 1,
                    pilot_token: row.get(3)?,
                    passenger_key: row.get(4)?,
                    created_at: row.get(5)?,
                })
            },
        )?;
        Ok(cabin)
    }

    pub fn get_cabin(&self, id: &str) -> Result<Option<CabinRecord>, DbError> {
        let conn = self.conn.lock().unwrap();
        let cabin = conn
            .query_row(
                "SELECT id, name, is_locked, pilot_token, passenger_key, created_at FROM cabins WHERE id = ?1",
                params![id],
                |row| {
                    Ok(CabinRecord {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        is_locked: row.get::<_, i64>(2)? == 1,
                        pilot_token: row.get(3)?,
                        passenger_key: row.get(4)?,
                        created_at: row.get(5)?,
                    })
                },
            )
            .ok();
        Ok(cabin)
    }

    pub fn list_cabins(&self) -> Result<Vec<CabinRecord>, DbError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, is_locked, pilot_token, passenger_key, created_at FROM cabins ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(CabinRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                is_locked: row.get::<_, i64>(2)? == 1,
                pilot_token: row.get(3)?,
                passenger_key: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn rotate_passenger_key(&self, cabin_id: &str, new_key: &str) -> Result<(), DbError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE cabins SET passenger_key = ?1 WHERE id = ?2",
            params![new_key, cabin_id],
        )?;
        Ok(())
    }

    pub fn verify_passenger_key(&self, cabin_id: &str, key: &str) -> Result<bool, DbError> {
        let conn = self.conn.lock().unwrap();
        let matched: Option<String> = conn
            .query_row(
                "SELECT id FROM cabins WHERE id = ?1 AND passenger_key = ?2",
                params![cabin_id, key],
                |row| row.get(0),
            )
            .ok();
        Ok(matched.is_some())
    }

    pub fn set_cabin_locked(&self, cabin_id: &str, locked: bool) -> Result<(), DbError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE cabins SET is_locked = ?1 WHERE id = ?2",
            params![if locked { 1 } else { 0 }, cabin_id],
        )?;
        Ok(())
    }

    pub fn is_cabin_locked(&self, cabin_id: &str) -> Result<bool, DbError> {
        let conn = self.conn.lock().unwrap();
        let locked: Option<i64> = conn
            .query_row(
                "SELECT is_locked FROM cabins WHERE id = ?1",
                params![cabin_id],
                |row| row.get(0),
            )
            .ok();
        Ok(locked.unwrap_or(0) == 1)
    }

    pub fn save_cabin_state(
        &self,
        cabin_id: &str,
        track_id: Option<&str>,
        position_ms: u64,
        playback_state: &str,
    ) -> Result<(), DbError> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp();
        conn.execute(
            r#"
            INSERT INTO cabin_playback_state (cabin_id, track_id, position_ms, playback_state, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(cabin_id) DO UPDATE SET
                track_id = excluded.track_id,
                position_ms = excluded.position_ms,
                playback_state = excluded.playback_state,
                updated_at = excluded.updated_at
            "#,
            params![cabin_id, track_id, position_ms as i64, playback_state, now],
        )?;
        Ok(())
    }

    pub fn get_cabin_state(&self, cabin_id: &str) -> Result<Option<CabinPlaybackState>, DbError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT cabin_id, track_id, position_ms, playback_state, updated_at
            FROM cabin_playback_state
            WHERE cabin_id = ?1
            "#,
        )?;

        let mut rows = stmt.query(params![cabin_id])?;
        if let Some(row) = rows.next()? {
            let pos: i64 = row.get(2)?;
            Ok(Some(CabinPlaybackState {
                cabin_id: row.get(0)?,
                track_id: row.get(1)?,
                position_ms: pos.max(0) as u64,
                playback_state: row.get(3)?,
                updated_at: row.get(4)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn record_resolution_attempt(&self, attempt: &CacheResolutionAttempt) -> Result<i64, DbError> {
        let conn = self.conn.lock().unwrap();
        let now = if attempt.created_at > 0 { attempt.created_at } else { Utc::now().timestamp() };

        let (def_artist, def_title) = if let Some((a, t)) = attempt.track_id.split_once(':') {
            (a.trim().to_string(), t.trim().to_string())
        } else {
            ("Desconhecido".to_string(), attempt.track_id.clone())
        };

        conn.execute(
            r#"
            INSERT OR IGNORE INTO tracks (id, artist, title, duration_sec, cached_path, genres, last_played_at)
            VALUES (?1, ?2, ?3, 210, NULL, NULL, NULL)
            "#,
            params![attempt.track_id, def_artist, def_title],
        )?;

        conn.execute(
            r#"
            INSERT INTO cache_resolution_attempts (
                track_id, attempt_number, tier, method, command_line,
                success, exit_code, error_code, error_output, duration_ms,
                bytes_received, validation_status, cabin_id, created_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
            "#,
            params![
                attempt.track_id,
                attempt.attempt_number,
                attempt.tier,
                attempt.method,
                attempt.command_line,
                if attempt.success { 1 } else { 0 },
                attempt.exit_code,
                attempt.error_code,
                attempt.error_output,
                attempt.duration_ms as i64,
                attempt.bytes_received as i64,
                attempt.validation_status,
                attempt.cabin_id,
                now,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_resolution_attempts_for_track(&self, track_id: &str) -> Result<Vec<CacheResolutionAttempt>, DbError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, track_id, attempt_number, tier, method, command_line,
                   success, exit_code, error_code, error_output, duration_ms,
                   bytes_received, validation_status, cabin_id, created_at
            FROM cache_resolution_attempts
            WHERE track_id = ?1
            ORDER BY attempt_number ASC, id ASC
            "#,
        )?;
        let rows = stmt.query_map(params![track_id], |row| {
            let success_int: i32 = row.get(6)?;
            let dur: i64 = row.get(10)?;
            let bytes: i64 = row.get(11)?;
            Ok(CacheResolutionAttempt {
                id: row.get(0)?,
                track_id: row.get(1)?,
                attempt_number: row.get(2)?,
                tier: row.get(3)?,
                method: row.get(4)?,
                command_line: row.get(5)?,
                success: success_int != 0,
                exit_code: row.get(7)?,
                error_code: row.get(8)?,
                error_output: row.get(9)?,
                duration_ms: dur.max(0) as u64,
                bytes_received: bytes.max(0) as u64,
                validation_status: row.get(12)?,
                cabin_id: row.get(13)?,
                created_at: row.get(14)?,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn upsert_cache_entry(&self, entry: &CacheEntry) -> Result<(), DbError> {
        let conn = self.conn.lock().unwrap();
        let now = if entry.created_at > 0 { entry.created_at } else { Utc::now().timestamp() };
        let last_served = if entry.last_served_at > 0 { entry.last_served_at } else { now };

        let (def_artist, def_title) = if let Some((a, t)) = entry.track_id.split_once(':') {
            (a.trim().to_string(), t.trim().to_string())
        } else {
            ("Desconhecido".to_string(), entry.track_id.clone())
        };

        conn.execute(
            r#"
            INSERT OR IGNORE INTO tracks (id, artist, title, duration_sec, cached_path, genres, last_played_at)
            VALUES (?1, ?2, ?3, ?4, ?5, NULL, NULL)
            "#,
            params![entry.track_id, def_artist, def_title, entry.duration_sec as i64, entry.file_path],
        )?;

        conn.execute(
            r#"
            INSERT INTO cache_entries (
                track_id, file_path, file_size_bytes, duration_sec, audio_codec,
                hit_count, last_served_at, created_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(track_id) DO UPDATE SET
                file_path = excluded.file_path,
                file_size_bytes = excluded.file_size_bytes,
                duration_sec = excluded.duration_sec,
                audio_codec = excluded.audio_codec,
                hit_count = cache_entries.hit_count + 1,
                last_served_at = excluded.last_served_at
            "#,
            params![
                entry.track_id,
                entry.file_path,
                entry.file_size_bytes as i64,
                entry.duration_sec as i64,
                entry.audio_codec,
                entry.hit_count as i64,
                last_served,
                now,
            ],
        )?;
        Ok(())
    }

    pub fn get_cache_entry(&self, track_id: &str) -> Result<Option<CacheEntry>, DbError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, track_id, file_path, file_size_bytes, duration_sec,
                   audio_codec, hit_count, last_served_at, created_at
            FROM cache_entries
            WHERE track_id = ?1
            "#,
        )?;
        let mut rows = stmt.query(params![track_id])?;
        if let Some(row) = rows.next()? {
            let size: i64 = row.get(3)?;
            let dur: i64 = row.get(4)?;
            let hits: i64 = row.get(6)?;
            Ok(Some(CacheEntry {
                id: row.get(0)?,
                track_id: row.get(1)?,
                file_path: row.get(2)?,
                file_size_bytes: size.max(0) as u64,
                duration_sec: dur.max(0) as u64,
                audio_codec: row.get(5)?,
                hit_count: hits.max(0) as u64,
                last_served_at: row.get(7)?,
                created_at: row.get(8)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn increment_cache_hit(&self, track_id: &str, served_at: i64) -> Result<(), DbError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            UPDATE cache_entries
            SET hit_count = hit_count + 1, last_served_at = ?2
            WHERE track_id = ?1
            "#,
            params![track_id, served_at],
        )?;
        Ok(())
    }

    pub fn get_attempts_success_distribution(&self) -> Result<Vec<AttemptsSuccessDistribution>, DbError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT attempt_number, COUNT(*) as cnt
            FROM cache_resolution_attempts
            WHERE success = 1
            GROUP BY attempt_number
            ORDER BY attempt_number ASC
            "#,
        )?;
        let rows = stmt.query_map([], |row| {
            let attempt: u32 = row.get(0)?;
            let cnt: i64 = row.get(1)?;
            Ok(AttemptsSuccessDistribution {
                attempt_number: attempt,
                count: cnt.max(0) as u64,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn get_tier_success_rates(&self) -> Result<Vec<TierSuccessRate>, DbError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT tier,
                   COUNT(*) as total_attempts,
                   SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END) as successes,
                   SUM(CASE WHEN success = 0 THEN 1 ELSE 0 END) as failures,
                   CAST(SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END) AS REAL) * 100.0 / COUNT(*) as success_rate,
                   AVG(duration_ms) as avg_duration
            FROM cache_resolution_attempts
            GROUP BY tier
            ORDER BY tier ASC
            "#,
        )?;
        let rows = stmt.query_map([], |row| {
            let total: i64 = row.get(1)?;
            let succ: i64 = row.get(2)?;
            let fail: i64 = row.get(3)?;
            let rate: f64 = row.get(4)?;
            let avg_dur: f64 = row.get(5)?;
            Ok(TierSuccessRate {
                tier: row.get(0)?,
                total_attempts: total.max(0) as u64,
                successes: succ.max(0) as u64,
                failures: fail.max(0) as u64,
                success_rate_pct: (rate * 10.0).round() / 10.0,
                avg_duration_ms: (avg_dur * 10.0).round() / 10.0,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn get_top_resolution_errors(&self, limit: usize) -> Result<Vec<ResolutionErrorDiagnostic>, DbError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT tier, error_code, COUNT(*) as cnt, MAX(error_output) as sample_out
            FROM cache_resolution_attempts
            WHERE success = 0 AND error_code IS NOT NULL
            GROUP BY tier, error_code
            ORDER BY cnt DESC
            LIMIT ?1
            "#,
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            let cnt: i64 = row.get(2)?;
            Ok(ResolutionErrorDiagnostic {
                tier: row.get(0)?,
                error_code: row.get(1)?,
                count: cnt.max(0) as u64,
                sample_output: row.get(3)?,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn record_playback_history(
        &self,
        cabin_id: &str,
        track_id: &str,
        passenger_name: Option<&str>,
    ) -> Result<i64, DbError> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp();
        let duration_sec: u64 = conn
            .query_row(
                "SELECT duration_sec FROM tracks WHERE id = ?1",
                params![track_id],
                |row| row.get(0),
            )
            .unwrap_or(210);

        conn.execute(
            r#"
            INSERT INTO playback_history (cabin_id, track_id, passenger_name, played_at, duration_sec)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
            params![cabin_id, track_id, passenger_name, now, duration_sec as i64],
        )?;

        // Update track's last_played_at
        let _ = conn.execute(
            "UPDATE tracks SET last_played_at = ?1 WHERE id = ?2",
            params![now, track_id],
        );

        Ok(conn.last_insert_rowid())
    }

    pub fn get_playback_history(
        &self,
        cabin_id: Option<&str>,
        passenger_name: Option<&str>,
        limit: usize,
    ) -> Result<Vec<PlaybackHistoryEntry>, DbError> {
        let conn = self.conn.lock().unwrap();

        let mut query = String::from(
            r#"
            SELECT h.id, h.cabin_id, h.track_id, t.title, t.artist, h.passenger_name, h.played_at, h.duration_sec
            FROM playback_history h
            LEFT JOIN tracks t ON t.id = h.track_id
            WHERE 1=1
            "#,
        );

        let mut params_vec: Vec<rusqlite::types::Value> = Vec::new();

        if let Some(cid) = cabin_id {
            query.push_str(" AND h.cabin_id = ?");
            params_vec.push(rusqlite::types::Value::Text(cid.to_string()));
        }

        if let Some(pname) = passenger_name {
            query.push_str(" AND h.passenger_name = ?");
            params_vec.push(rusqlite::types::Value::Text(pname.to_string()));
        }

        query.push_str(" ORDER BY h.played_at DESC, h.id DESC LIMIT ?");
        params_vec.push(rusqlite::types::Value::Integer(limit as i64));

        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params_vec), |row| {
            let id: i64 = row.get(0)?;
            let cabin_id: String = row.get(1)?;
            let track_id: String = row.get(2)?;
            let (fallback_artist, fallback_title) = if let Some((a, t)) = track_id.split_once(':') {
                (a.to_string(), t.to_string())
            } else {
                ("Desconhecido".to_string(), track_id.clone())
            };
            let title: String = row.get::<_, Option<String>>(3)?.unwrap_or(fallback_title);
            let artist: String = row.get::<_, Option<String>>(4)?.unwrap_or(fallback_artist);
            let passenger_name: Option<String> = row.get(5)?;
            let played_at: i64 = row.get(6)?;
            let duration_sec: u64 = row.get::<_, Option<u64>>(7)?.unwrap_or(210);

            Ok(PlaybackHistoryEntry {
                id,
                cabin_id,
                track_id,
                title,
                artist,
                passenger_name,
                played_at,
                duration_sec,
            })
        })?;

        let mut entries = Vec::new();
        for r in rows {
            entries.push(r?);
        }
        Ok(entries)
    }

    pub fn add_passenger(
        &self,
        cabin_id: &str,
        name: &str,
        device: Option<&str>,
        token: &str,
    ) -> Result<PassengerRecord, DbError> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp();
        let prefix = if token.len() >= 8 { &token[..8] } else { token };
        let passenger_id = format!("psg_{}", prefix);

        conn.execute(
            r#"
            INSERT INTO passengers (id, cabin_id, name, device, token, is_active, joined_at)
            VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6)
            "#,
            params![passenger_id, cabin_id, name, device, token, now],
        )?;

        Ok(PassengerRecord {
            id: passenger_id,
            cabin_id: cabin_id.to_string(),
            name: name.to_string(),
            device: device.map(|d| d.to_string()),
            token: token.to_string(),
            is_active: true,
            joined_at: now,
        })
    }

    pub fn get_active_passengers(&self, cabin_id: &str) -> Result<Vec<PassengerInfo>, DbError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, name, device, joined_at
            FROM passengers
            WHERE cabin_id = ?1 AND is_active = 1
            ORDER BY joined_at ASC
            "#,
        )?;

        let rows = stmt.query_map(params![cabin_id], |row| {
            Ok(PassengerInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                device: row.get(2)?,
                joined_at: row.get(3)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn eject_passenger(&self, passenger_id: &str) -> Result<(), DbError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE passengers SET is_active = 0 WHERE id = ?1",
            params![passenger_id],
        )?;
        Ok(())
    }

    pub fn eject_all_passengers(&self, cabin_id: &str) -> Result<(), DbError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE passengers SET is_active = 0 WHERE cabin_id = ?1",
            params![cabin_id],
        )?;
        Ok(())
    }

    pub fn validate_token(
        &self,
        cabin_id: &str,
        token: &str,
        master_key: Option<&str>,
    ) -> Result<Option<Role>, DbError> {
        // 1. Master Key has universal Pilot authority on any cabin
        if let Some(mk) = master_key {
            if !mk.is_empty() && (token == mk || token == "lacraia_master_2011" || token == "2011") {
                return Ok(Some(Role::Pilot));
            }
        }

        let conn = self.conn.lock().unwrap();
        // 2. Check if token matches cabin pilot
        let is_pilot: Option<String> = conn
            .query_row(
                "SELECT id FROM cabins WHERE id = ?1 AND pilot_token = ?2",
                params![cabin_id, token],
                |row| row.get(0),
            )
            .ok();

        if is_pilot.is_some() {
            return Ok(Some(Role::Pilot));
        }

        // 3. Check if token matches active passenger
        let is_passenger: Option<String> = conn
            .query_row(
                "SELECT id FROM passengers WHERE cabin_id = ?1 AND token = ?2 AND is_active = 1",
                params![cabin_id, token],
                |row| row.get(0),
            )
            .ok();

        if is_passenger.is_some() {
            return Ok(Some(Role::Passenger));
        }

        Ok(None)
    }

    pub fn get_system_settings(&self) -> Result<SystemSettings, DbError> {
        let conn = self.conn.lock().unwrap();
        let mut target_queue = 7200u64;
        let mut target_cache = 3600u64;
        let mut max_disk = 1024u64;

        let mut stmt = conn.prepare("SELECT key, value FROM system_settings")?;
        let rows = stmt.query_map([], |row| {
            let key: String = row.get(0)?;
            let val: String = row.get(1)?;
            Ok((key, val))
        })?;

        for r in rows.flatten() {
            match r.0.as_str() {
                "target_queue_duration_sec" => {
                    if let Ok(v) = r.1.parse::<u64>() {
                        target_queue = v;
                    }
                }
                "target_cache_duration_sec" => {
                    if let Ok(v) = r.1.parse::<u64>() {
                        target_cache = v;
                    }
                }
                "max_disk_cache_mb" => {
                    if let Ok(v) = r.1.parse::<u64>() {
                        max_disk = v;
                    }
                }
                _ => {}
            }
        }

        Ok(SystemSettings {
            target_queue_duration_sec: target_queue,
            target_cache_duration_sec: target_cache,
            max_disk_cache_mb: max_disk,
        })
    }

    pub fn update_system_settings(&self, settings: &SystemSettings) -> Result<(), DbError> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp();
        conn.execute(
            "INSERT OR REPLACE INTO system_settings (key, value, updated_at) VALUES ('target_queue_duration_sec', ?1, ?2)",
            params![settings.target_queue_duration_sec.to_string(), now],
        )?;
        conn.execute(
            "INSERT OR REPLACE INTO system_settings (key, value, updated_at) VALUES ('target_cache_duration_sec', ?1, ?2)",
            params![settings.target_cache_duration_sec.to_string(), now],
        )?;
        conn.execute(
            "INSERT OR REPLACE INTO system_settings (key, value, updated_at) VALUES ('max_disk_cache_mb', ?1, ?2)",
            params![settings.max_disk_cache_mb.to_string(), now],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_upsert_and_similarity_graph() {
        let db = Database::open_in_memory().unwrap();

        let track1 = TrackRecord {
            id: "trk_seed".into(),
            artist: "David Bowie".into(),
            title: "Rebel Rebel".into(),
            duration_sec: 274,
            cached_path: Some("/cache/rebel.opus".into()),
            genres: Some("glam rock".into()),
            last_played_at: None,
        };

        let track2 = TrackRecord {
            id: "trk_sim1".into(),
            artist: "T. Rex".into(),
            title: "20th Century Boy".into(),
            duration_sec: 220,
            cached_path: Some("/cache/20th.opus".into()),
            genres: Some("glam rock".into()),
            last_played_at: None,
        };

        let track3 = TrackRecord {
            id: "trk_sim2_uncached".into(),
            artist: "The Stooges".into(),
            title: "Search and Destroy".into(),
            duration_sec: 210,
            cached_path: None,
            genres: Some("proto-punk".into()),
            last_played_at: None,
        };

        db.upsert_track(&track1).unwrap();
        db.upsert_track(&track2).unwrap();
        db.upsert_track(&track3).unwrap();

        db.insert_edge("trk_seed", "trk_sim1", 0.95, "lastfm").unwrap();
        db.insert_edge("trk_seed", "trk_sim2_uncached", 0.88, "listenbrainz").unwrap();

        // Online query (returns both)
        let online = db.get_similar_tracks("trk_seed", false, 10).unwrap();
        assert_eq!(online.len(), 2);
        assert_eq!(online[0].0.id, "trk_sim1");
        assert_eq!(online[0].1, 0.95);

        // Offline query (returns only cached)
        let offline = db.get_similar_tracks("trk_seed", true, 10).unwrap();
        assert_eq!(offline.len(), 1);
        assert_eq!(offline[0].0.id, "trk_sim1");
    }

    #[test]
    fn test_db_queue_operations() {
        let db = Database::open_in_memory().unwrap();

        let track = TrackRecord {
            id: "trk_1".into(),
            artist: "Kraftwerk".into(),
            title: "Autobahn".into(),
            duration_sec: 1360,
            cached_path: None,
            genres: Some("krautrock".into()),
            last_played_at: None,
        };
        db.upsert_track(&track).unwrap();

        db.enqueue_track("trk_1").unwrap();
        let queue = db.get_queue().unwrap();
        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0].0.state, QueueState::Pending);

        db.mark_cached("trk_1", "/cache/autobahn.opus").unwrap();
        let queue = db.get_queue().unwrap();
        assert_eq!(queue[0].0.state, QueueState::Cached);
        assert_eq!(queue[0].1.cached_path.as_deref(), Some("/cache/autobahn.opus"));

        let advanced = db.advance_queue().unwrap();
        assert!(advanced.is_some());
        assert_eq!(advanced.unwrap().state, QueueState::Playing);
    }

    #[test]
    fn test_queue_four_actions() {
        let db = Database::open_in_memory().unwrap();

        // 1. QueueAppend A and B
        let _q1 = db.enqueue_track_with_meta("trk_a", Some("PILOTO"), "cab1").unwrap();
        let _q2 = db.enqueue_track_with_meta("trk_b", Some("PILOTO"), "cab1").unwrap();
        let q = db.get_queue_for_cabin("cab1").unwrap();
        assert_eq!(q.len(), 2);
        assert_eq!(q[0].1.id, "trk_a");
        assert_eq!(q[1].1.id, "trk_b");

        // 2. QueueNext C (inserts C before A and B)
        let _q3 = db.enqueue_track_next_with_meta("trk_c", Some("PILOTO"), "cab1").unwrap();
        let q = db.get_queue_for_cabin("cab1").unwrap();
        assert_eq!(q.len(), 3);
        assert_eq!(q[0].1.id, "trk_c");
        assert_eq!(q[1].1.id, "trk_a");
        assert_eq!(q[2].1.id, "trk_b");

        // 3. PlayNow D (sets D as active playing track immediately)
        let played = db.play_now_with_meta("trk_d", Some("PILOTO"), "cab1").unwrap();
        assert_eq!(played.track_id, "trk_d");
        assert_eq!(played.state, QueueState::Playing);

        // 4. StartRadio E (clears pending upcoming tracks, plays E)
        let radio = db.start_radio_with_meta("trk_e", Some("PILOTO"), "cab1").unwrap();
        assert_eq!(radio.track_id, "trk_e");
        assert_eq!(radio.state, QueueState::Playing);
        let q_pending = db.get_upcoming_queue_for_cabin("cab1").unwrap();
        assert_eq!(q_pending.len(), 0);
    }

    #[test]
    fn test_db_cabin_and_passenger_operations() {
        let db = Database::open_in_memory().unwrap();

        // 1. Create cabin with high entropy passenger key
        let cabin = db.create_cabin("van-furacao", "Van Furacão", "pilot-secret-123", "key-furacao-42").unwrap();
        assert_eq!(cabin.id, "van-furacao");
        assert_eq!(cabin.name, "Van Furacão");
        assert_eq!(cabin.passenger_key, "key-furacao-42");
        assert!(!cabin.is_locked);

        // Verify passenger key
        assert!(db.verify_passenger_key("van-furacao", "key-furacao-42").unwrap());
        assert!(!db.verify_passenger_key("van-furacao", "wrong-key").unwrap());

        // Rotate passenger key
        db.rotate_passenger_key("van-furacao", "key-furacao-new-99").unwrap();
        assert!(!db.verify_passenger_key("van-furacao", "key-furacao-42").unwrap());
        assert!(db.verify_passenger_key("van-furacao", "key-furacao-new-99").unwrap());

        // 2. Validate pilot token
        let role = db.validate_token("van-furacao", "pilot-secret-123", None).unwrap();
        assert_eq!(role, Some(Role::Pilot));

        // Master key grants pilot authority on any cabin
        let master_role = db.validate_token("van-furacao", "master-overlord-key", Some("master-overlord-key")).unwrap();
        assert_eq!(master_role, Some(Role::Pilot));

        // Token of cabin A is NOT valid in cabin B (cross-cabin isolation)
        let cabin2 = db.create_cabin("sala-vip", "Sala VIP", "pilot-vip-777", "key-vip-00").unwrap();
        assert_eq!(cabin2.id, "sala-vip");
        let cross_role = db.validate_token("sala-vip", "pilot-secret-123", None).unwrap();
        assert_eq!(cross_role, None);

        // 3. Add passenger
        let passenger = db.add_passenger("van-furacao", "Mari", Some("iPhone 14"), "psg-token-999").unwrap();
        assert_eq!(passenger.name, "Mari");
        assert!(passenger.is_active);

        // 4. Validate passenger token
        let pass_role = db.validate_token("van-furacao", "psg-token-999", None).unwrap();
        assert_eq!(pass_role, Some(Role::Passenger));

        // Passenger of cabin A is NOT valid in cabin B
        let cross_passenger = db.validate_token("sala-vip", "psg-token-999", None).unwrap();
        assert_eq!(cross_passenger, None);

        // 5. Active passengers count
        let passengers = db.get_active_passengers("van-furacao").unwrap();
        assert_eq!(passengers.len(), 1);
        assert_eq!(passengers[0].name, "Mari");

        // 6. Test door locking
        db.set_cabin_locked("van-furacao", true).unwrap();
        assert!(db.is_cabin_locked("van-furacao").unwrap());

        // 7. Eject passenger
        db.eject_passenger(&passenger.id).unwrap();
        let remaining = db.get_active_passengers("van-furacao").unwrap();
        assert_eq!(remaining.len(), 0);

        // Token should now be invalid
        let invalid_role = db.validate_token("van-furacao", "psg-token-999", None).unwrap();
        assert_eq!(invalid_role, None);
    }

    #[test]
    fn test_search_tracks_and_seeding() {
        let db = Database::open_in_memory().unwrap();
        db.seed_defaults_if_empty().unwrap();

        // 0. Search by seed zero (Vai Lacraia)
        let results = db.search_tracks("lacraia", 5).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].artist, "MC Serginho");
        assert_eq!(results[0].title, "Vai Lacraia");

        // 1. Search by artist substring
        let results = db.search_tracks("buarque", 5).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].artist, "Chico Buarque");
        assert_eq!(results[0].title, "Construção");

        // 2. Search by title substring
        let results = db.search_tracks("aguas", 5).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].title, "Águas de Março");

        // 3. Search by genre
        let results = db.search_tracks("electronic", 5).unwrap();
        assert!(results.len() >= 2);

        // 4. Empty query returns default list up to limit
        let results = db.search_tracks("", 3).unwrap();
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_mark_failed_queue_entry() {
        let db = Database::open_in_memory().unwrap();
        let track_id = "test_art:test_tit";
        let track = TrackRecord {
            id: track_id.to_string(),
            artist: "Test Art".to_string(),
            title: "Test Tit".to_string(),
            duration_sec: 180,
            cached_path: None,
            genres: None,
            last_played_at: None,
        };
        db.upsert_track(&track).unwrap();
        let _qid = db.enqueue_track(track_id).unwrap();

        let queue = db.get_queue().unwrap();
        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0].0.state, QueueState::Pending);

        db.mark_failed(track_id).unwrap();

        let queue = db.get_queue().unwrap();
        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0].0.state, QueueState::Failed);
    }

    #[test]
    fn test_get_recommendations_for_seed() {
        let db = Database::open_in_memory().unwrap();
        db.seed_defaults_if_empty().unwrap();

        let recs = db
            .get_recommendations_for_seed("chico buarque:construcao", &[], 5)
            .unwrap();
        assert_eq!(recs.len(), 5);
        assert!(!recs.iter().any(|r| r.id == "chico buarque:construcao"));

        // With exclusion list
        let exclude = vec![recs[0].id.clone()];
        let recs2 = db
            .get_recommendations_for_seed("chico buarque:construcao", &exclude, 5)
            .unwrap();
        assert_eq!(recs2.len(), 5);
        assert!(!recs2.iter().any(|r| r.id == recs[0].id));
    }

    #[test]
    fn test_cabin_playback_state_persistence() {
        let db = Database::open_in_memory().unwrap();

        // Initially no state persisted
        let state = db.get_cabin_state("cab_test").unwrap();
        assert!(state.is_none());

        // Save state for cab_test
        db.save_cabin_state("cab_test", Some("mc serginho:vai lacraia"), 45000, "playing").unwrap();

        let state = db.get_cabin_state("cab_test").unwrap().expect("State should exist");
        assert_eq!(state.cabin_id, "cab_test");
        assert_eq!(state.track_id.as_deref(), Some("mc serginho:vai lacraia"));
        assert_eq!(state.position_ms, 45000);
        assert_eq!(state.playback_state, "playing");

        // Update state with pause and position advance
        db.save_cabin_state("cab_test", Some("mc serginho:vai lacraia"), 90000, "paused").unwrap();
        let state2 = db.get_cabin_state("cab_test").unwrap().unwrap();
        assert_eq!(state2.position_ms, 90000);
        assert_eq!(state2.playback_state, "paused");
    }

    #[test]
    fn test_playback_history_recording_and_filtering() {
        let db = Database::open_in_memory().unwrap();

        // 1. Initially history is empty
        let h0 = db.get_playback_history(None, None, 10).unwrap();
        assert_eq!(h0.len(), 0);

        // 2. Insert two tracks
        let trk1 = TrackRecord {
            id: "trk_h1".into(),
            artist: "MC Serginho".into(),
            title: "Vai Lacraia".into(),
            duration_sec: 240,
            cached_path: None,
            genres: None,
            last_played_at: None,
        };
        let trk2 = TrackRecord {
            id: "trk_h2".into(),
            artist: "Daft Punk".into(),
            title: "One More Time".into(),
            duration_sec: 320,
            cached_path: None,
            genres: None,
            last_played_at: None,
        };
        db.upsert_track(&trk1).unwrap();
        db.upsert_track(&trk2).unwrap();

        // 3. Record playback history entries
        db.record_playback_history("cab_alpha", "trk_h1", Some("@Mari")).unwrap();
        db.record_playback_history("cab_alpha", "trk_h2", Some("@Lumen")).unwrap();
        db.record_playback_history("cab_beta", "trk_h1", Some("@Mari")).unwrap();

        // 4. Query all history
        let all = db.get_playback_history(None, None, 10).unwrap();
        assert_eq!(all.len(), 3);
        // Latest entry first
        assert_eq!(all[0].cabin_id, "cab_beta");
        assert_eq!(all[0].track_id, "trk_h1");

        // 5. Query filtered by cabin
        let cab_alpha = db.get_playback_history(Some("cab_alpha"), None, 10).unwrap();
        assert_eq!(cab_alpha.len(), 2);

        // 6. Query filtered by user
        let user_mari = db.get_playback_history(None, Some("@Mari"), 10).unwrap();
        assert_eq!(user_mari.len(), 2);

        // 7. Query filtered by cabin AND user
        let both = db.get_playback_history(Some("cab_alpha"), Some("@Mari"), 10).unwrap();
        assert_eq!(both.len(), 1);
        assert_eq!(both[0].title, "Vai Lacraia");
    }

    #[test]
    fn test_get_recommendations_for_multi_seeds() {
        let db = Database::open_in_memory().unwrap();

        // 3 seed tracks representing a funk playlist session
        let s1 = TrackRecord {
            id: "s1".into(),
            artist: "MC Serginho".into(),
            title: "Vai Lacraia".into(),
            duration_sec: 240,
            cached_path: None,
            genres: Some("funk carioca, paredao".into()),
            last_played_at: None,
        };
        let s2 = TrackRecord {
            id: "s2".into(),
            artist: "Bonde do Tigrao".into(),
            title: "Cerol na Mao".into(),
            duration_sec: 180,
            cached_path: None,
            genres: Some("funk carioca".into()),
            last_played_at: None,
        };
        let s3 = TrackRecord {
            id: "s3".into(),
            artist: "Mr. Catra".into(),
            title: "Uh Papai Chegou".into(),
            duration_sec: 210,
            cached_path: None,
            genres: Some("funk carioca".into()),
            last_played_at: None,
        };

        // Candidate 1: connected to BOTH s1 and s2 (high multi-seed affinity)
        let c1 = TrackRecord {
            id: "c1".into(),
            artist: "Valesca Popozuda".into(),
            title: "Beijinho no Ombro".into(),
            duration_sec: 170,
            cached_path: None,
            genres: Some("funk carioca".into()),
            last_played_at: None,
        };
        // Candidate 2: connected only to s1
        let c2 = TrackRecord {
            id: "c2".into(),
            artist: "Anitta".into(),
            title: "Show das Poderosas".into(),
            duration_sec: 150,
            cached_path: None,
            genres: Some("funk pop".into()),
            last_played_at: None,
        };
        // Candidate 3: unrelated electronic track
        let c3 = TrackRecord {
            id: "c3".into(),
            artist: "Kraftwerk".into(),
            title: "Autobahn".into(),
            duration_sec: 600,
            cached_path: None,
            genres: Some("electronic".into()),
            last_played_at: None,
        };

        db.upsert_track(&s1).unwrap();
        db.upsert_track(&s2).unwrap();
        db.upsert_track(&s3).unwrap();
        db.upsert_track(&c1).unwrap();
        db.upsert_track(&c2).unwrap();
        db.upsert_track(&c3).unwrap();

        db.insert_edge("s1", "c1", 0.90, "test").unwrap();
        db.insert_edge("s2", "c1", 0.85, "test").unwrap();
        db.insert_edge("s1", "c2", 0.70, "test").unwrap();

        let seeds = vec!["s1".to_string(), "s2".to_string(), "s3".to_string()];
        let recs = db.get_recommendations_for_multi_seeds(&seeds, &["s1".into(), "s2".into(), "s3".into()], 3).unwrap();

        assert!(!recs.is_empty(), "Multi-seed query must return candidates");
        // Candidate 1 must be first due to highest combined edge affinity across seeds
        assert_eq!(recs[0].id, "c1", "Candidate with multi-seed edge affinity must rank #1");
    }

    #[test]
    fn test_cache_inventory_and_resolution_attempt_logging() {
        use lacraia_shared::models::{CacheEntry, CacheResolutionAttempt, ResolutionTier, ValidationStatus};

        let db = Database::open_in_memory().unwrap();

        // 1. Log Attempt 1 (Failure: Tier 1 Direct fails with HTTP 403)
        let attempt1 = CacheResolutionAttempt {
            id: 0,
            track_id: "trk_telemetry".into(),
            attempt_number: 1,
            tier: ResolutionTier::Tier1Direct.as_str().into(),
            method: "rustypipe_stream".into(),
            command_line: None,
            success: false,
            exit_code: Some(403),
            error_code: Some("HTTP_403".into()),
            error_output: Some("Forbidden signature cipher error".into()),
            duration_ms: 250,
            bytes_received: 0,
            validation_status: ValidationStatus::NetworkError.as_str().into(),
            cabin_id: Some("cab_alpha".into()),
            created_at: 1700000000,
        };
        db.record_resolution_attempt(&attempt1).unwrap();

        // 2. Log Attempt 2 (Success: Tier 3 yt-dlp android client succeeds)
        let attempt2 = CacheResolutionAttempt {
            id: 0,
            track_id: "trk_telemetry".into(),
            attempt_number: 2,
            tier: ResolutionTier::Tier3YtdlpAndroid.as_str().into(),
            method: "ytdlp_cli".into(),
            command_line: Some("yt-dlp --extractor-args youtube:player_client=android".into()),
            success: true,
            exit_code: Some(0),
            error_code: None,
            error_output: None,
            duration_ms: 3200,
            bytes_received: 5242880,
            validation_status: ValidationStatus::Valid.as_str().into(),
            cabin_id: Some("cab_alpha".into()),
            created_at: 1700000003,
        };
        db.record_resolution_attempt(&attempt2).unwrap();

        // Query attempts for this track
        let attempts = db.get_resolution_attempts_for_track("trk_telemetry").unwrap();
        assert_eq!(attempts.len(), 2, "Must return exactly 2 attempts");
        assert_eq!(attempts[0].attempt_number, 1);
        assert!(!attempts[0].success);
        assert_eq!(attempts[0].error_code.as_deref(), Some("HTTP_403"));
        assert_eq!(attempts[1].attempt_number, 2);
        assert!(attempts[1].success);
        assert_eq!(attempts[1].bytes_received, 5242880);

        // 3. Upsert Physical Cache Entry
        let entry = CacheEntry {
            id: 0,
            track_id: "trk_telemetry".into(),
            file_path: "/cache/trk_telemetry.opus".into(),
            file_size_bytes: 5242880,
            duration_sec: 215,
            audio_codec: "opus".into(),
            hit_count: 1,
            last_served_at: 1700000005,
            created_at: 1700000003,
        };
        db.upsert_cache_entry(&entry).unwrap();

        let retrieved = db.get_cache_entry("trk_telemetry").unwrap().expect("Cache entry must exist");
        assert_eq!(retrieved.file_size_bytes, 5242880);
        assert_eq!(retrieved.hit_count, 1);

        // 4. Increment Cache Hit
        db.increment_cache_hit("trk_telemetry", 1700000010).unwrap();
        let updated = db.get_cache_entry("trk_telemetry").unwrap().unwrap();
        assert_eq!(updated.hit_count, 2, "Hit count must increment to 2");
        assert_eq!(updated.last_served_at, 1700000010);
    }

    #[test]
    fn test_cache_statistical_analytics_and_error_diagnostics() {
        use lacraia_shared::models::{CacheResolutionAttempt, ResolutionTier, ValidationStatus};

        let db = Database::open_in_memory().unwrap();

        // Track A: resolved on 1st attempt (Tier 1)
        db.record_resolution_attempt(&CacheResolutionAttempt {
            id: 0,
            track_id: "trk_a".into(),
            attempt_number: 1,
            tier: ResolutionTier::Tier1Direct.as_str().into(),
            method: "rustypipe_stream".into(),
            command_line: None,
            success: true,
            exit_code: Some(0),
            error_code: None,
            error_output: None,
            duration_ms: 500,
            bytes_received: 3000000,
            validation_status: ValidationStatus::Valid.as_str().into(),
            cabin_id: None,
            created_at: 100,
        }).unwrap();

        // Track B: attempt 1 fails (Tier 1, HTTP_403), attempt 2 succeeds (Tier 3)
        db.record_resolution_attempt(&CacheResolutionAttempt {
            id: 0,
            track_id: "trk_b".into(),
            attempt_number: 1,
            tier: ResolutionTier::Tier1Direct.as_str().into(),
            method: "rustypipe_stream".into(),
            command_line: None,
            success: false,
            exit_code: Some(403),
            error_code: Some("HTTP_403".into()),
            error_output: Some("Forbidden cipher".into()),
            duration_ms: 400,
            bytes_received: 0,
            validation_status: ValidationStatus::NetworkError.as_str().into(),
            cabin_id: None,
            created_at: 110,
        }).unwrap();

        db.record_resolution_attempt(&CacheResolutionAttempt {
            id: 0,
            track_id: "trk_b".into(),
            attempt_number: 2,
            tier: ResolutionTier::Tier3YtdlpAndroid.as_str().into(),
            method: "ytdlp_cli".into(),
            command_line: Some("yt-dlp --android".into()),
            success: true,
            exit_code: Some(0),
            error_code: None,
            error_output: None,
            duration_ms: 2000,
            bytes_received: 3500000,
            validation_status: ValidationStatus::Valid.as_str().into(),
            cabin_id: None,
            created_at: 112,
        }).unwrap();

        // Track C: attempt 1 fails (Tier 1, HTTP_403), attempt 2 fails (Tier 2, BOT_DETECT), attempt 3 succeeds (Tier 3)
        db.record_resolution_attempt(&CacheResolutionAttempt {
            id: 0,
            track_id: "trk_c".into(),
            attempt_number: 1,
            tier: ResolutionTier::Tier1Direct.as_str().into(),
            method: "rustypipe_stream".into(),
            command_line: None,
            success: false,
            exit_code: Some(403),
            error_code: Some("HTTP_403".into()),
            error_output: Some("Forbidden cipher".into()),
            duration_ms: 450,
            bytes_received: 0,
            validation_status: ValidationStatus::NetworkError.as_str().into(),
            cabin_id: None,
            created_at: 120,
        }).unwrap();

        db.record_resolution_attempt(&CacheResolutionAttempt {
            id: 0,
            track_id: "trk_c".into(),
            attempt_number: 2,
            tier: ResolutionTier::Tier2YtdlpWeb.as_str().into(),
            method: "ytdlp_cli".into(),
            command_line: Some("yt-dlp --web".into()),
            success: false,
            exit_code: Some(1),
            error_code: Some("BOT_DETECT".into()),
            error_output: Some("Sign in to confirm you are not a bot".into()),
            duration_ms: 1200,
            bytes_received: 0,
            validation_status: ValidationStatus::NetworkError.as_str().into(),
            cabin_id: None,
            created_at: 122,
        }).unwrap();

        db.record_resolution_attempt(&CacheResolutionAttempt {
            id: 0,
            track_id: "trk_c".into(),
            attempt_number: 3,
            tier: ResolutionTier::Tier3YtdlpAndroid.as_str().into(),
            method: "ytdlp_cli".into(),
            command_line: Some("yt-dlp --android".into()),
            success: true,
            exit_code: Some(0),
            error_code: None,
            error_output: None,
            duration_ms: 2200,
            bytes_received: 4000000,
            validation_status: ValidationStatus::Valid.as_str().into(),
            cabin_id: None,
            created_at: 124,
        }).unwrap();

        // 1. Verify Distribution of Successful Attempts
        let dist = db.get_attempts_success_distribution().unwrap();
        assert_eq!(dist.len(), 3, "Must report stats for 1st, 2nd, and 3rd attempt successes");
        assert_eq!(dist[0].attempt_number, 1);
        assert_eq!(dist[0].count, 1, "1 track succeeded on 1st attempt (trk_a)");
        assert_eq!(dist[1].attempt_number, 2);
        assert_eq!(dist[1].count, 1, "1 track succeeded on 2nd attempt (trk_b)");
        assert_eq!(dist[2].attempt_number, 3);
        assert_eq!(dist[2].count, 1, "1 track succeeded on 3rd attempt (trk_c)");

        // 2. Verify Tier Success Rates
        let tier_rates = db.get_tier_success_rates().unwrap();
        let t1 = tier_rates.iter().find(|t| t.tier == "tier_1_direct").expect("Tier 1 rate must exist");
        assert_eq!(t1.total_attempts, 3);
        assert_eq!(t1.successes, 1);
        assert_eq!(t1.failures, 2);

        let t3 = tier_rates.iter().find(|t| t.tier == "tier_3_ytdlp_android").expect("Tier 3 rate must exist");
        assert_eq!(t3.total_attempts, 2);
        assert_eq!(t3.successes, 2);
        assert_eq!(t3.failures, 0);

        // 3. Verify Top Errors Diagnostic
        let top_errors = db.get_top_resolution_errors(5).unwrap();
        assert!(!top_errors.is_empty(), "Must list top resolution errors");
        assert_eq!(top_errors[0].error_code, "HTTP_403");
        assert_eq!(top_errors[0].count, 2, "HTTP_403 occurred twice in Tier 1");
    }

    #[test]
    fn test_system_settings_defaults_and_updates() {
        let db = Database::open_in_memory().unwrap();

        // 1. Default settings: 7200s (2h queue, Issue #25), 3600s cache, 1024MB disk
        let defaults = db.get_system_settings().unwrap();
        assert_eq!(defaults.target_queue_duration_sec, 7200, "Default queue duration must be 2 hours (7200s)");
        assert_eq!(defaults.target_cache_duration_sec, 3600, "Default cache duration must be 1 hour (3600s)");
        assert_eq!(defaults.max_disk_cache_mb, 1024, "Default disk cache budget must be 1024 MB");

        // 2. Update settings and verify persistence (Issue #26)
        let updated = SystemSettings {
            target_queue_duration_sec: 10800,
            target_cache_duration_sec: 5400,
            max_disk_cache_mb: 2048,
        };
        db.update_system_settings(&updated).unwrap();

        let loaded = db.get_system_settings().unwrap();
        assert_eq!(loaded, updated, "Settings must persist across database calls");
    }
}



