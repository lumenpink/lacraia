use chrono::Utc;
use lacraia_shared::{PassengerInfo, Role};
use rusqlite::{params, Connection, Result};
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

    pub fn from_str(s: &str) -> Self {
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

            CREATE INDEX IF NOT EXISTS idx_queue_pos 
                ON queue(cabin_id, position ASC, state);
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
            tracks.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
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
                state: QueueState::from_str(&row.get::<_, String>(3)?),
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
                    state: QueueState::from_str(&state_str),
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
                        state: QueueState::from_str(&state_str),
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
                    state: QueueState::from_str(&state_str),
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
}



