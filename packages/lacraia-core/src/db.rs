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
}

impl QueueState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Cached => "cached",
            Self::Playing => "playing",
            Self::Played => "played",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "cached" => Self::Cached,
            "playing" => Self::Playing,
            "played" => Self::Played,
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

        // Ensure backwards compatibility with earlier queue table versions
        let _ = conn.execute("ALTER TABLE queue ADD COLUMN cabin_id TEXT NOT NULL DEFAULT 'default'", []);
        let _ = conn.execute("ALTER TABLE queue ADD COLUMN requested_by TEXT", []);

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

        conn.execute(
            r#"
            INSERT INTO queue (track_id, position, state, created_at, cabin_id, requested_by)
            VALUES (?1, ?2, 'pending', ?3, ?4, ?5)
            "#,
            params![track_id, next_pos, now, cabin_id, requested_by],
        )?;

        Ok(conn.last_insert_rowid())
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
            JOIN tracks t ON t.id = q.track_id
            WHERE q.state != 'played' AND q.cabin_id = ?1
            ORDER BY q.position ASC
            "#,
        )?;

        let rows = stmt.query_map(params![cabin_id], |row| {
            let state_str: String = row.get(3)?;
            Ok((
                QueueEntry {
                    id: row.get(0)?,
                    track_id: row.get(1)?,
                    position: row.get(2)?,
                    state: QueueState::from_str(&state_str),
                    created_at: row.get(4)?,
                    cabin_id: row.get(5)?,
                    requested_by: row.get(6)?,
                },
                TrackRecord {
                    id: row.get(1)?,
                    artist: row.get(7)?,
                    title: row.get(8)?,
                    duration_sec: row.get(9)?,
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

    pub fn get_or_create_cabin(&self, id: &str, name: &str, pilot_token: &str) -> Result<CabinRecord, DbError> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp();
        conn.execute(
            r#"
            INSERT INTO cabins (id, name, is_locked, pilot_token, created_at)
            VALUES (?1, ?2, 0, ?3, ?4)
            ON CONFLICT(id) DO NOTHING
            "#,
            params![id, name, pilot_token, now],
        )?;

        let cabin = conn.query_row(
            "SELECT id, name, is_locked, pilot_token, created_at FROM cabins WHERE id = ?1",
            params![id],
            |row| {
                Ok(CabinRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    is_locked: row.get::<_, i64>(2)? == 1,
                    pilot_token: row.get(3)?,
                    created_at: row.get(4)?,
                })
            },
        )?;
        Ok(cabin)
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

    pub fn validate_token(&self, cabin_id: &str, token: &str) -> Result<Option<Role>, DbError> {
        let conn = self.conn.lock().unwrap();
        // Check if token matches cabin pilot
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

        // Check if token matches active passenger
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
    fn test_db_cabin_and_passenger_operations() {
        let db = Database::open_in_memory().unwrap();

        // 1. Create cabin
        let cabin = db.get_or_create_cabin("van-furacao", "Van Furacão", "pilot-secret-123").unwrap();
        assert_eq!(cabin.id, "van-furacao");
        assert_eq!(cabin.name, "Van Furacão");
        assert!(!cabin.is_locked);

        // 2. Validate pilot token
        let role = db.validate_token("van-furacao", "pilot-secret-123").unwrap();
        assert_eq!(role, Some(Role::Pilot));

        // 3. Add passenger
        let passenger = db.add_passenger("van-furacao", "Mari", Some("iPhone 14"), "psg-token-999").unwrap();
        assert_eq!(passenger.name, "Mari");
        assert!(passenger.is_active);

        // 4. Validate passenger token
        let pass_role = db.validate_token("van-furacao", "psg-token-999").unwrap();
        assert_eq!(pass_role, Some(Role::Passenger));

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
        let invalid_role = db.validate_token("van-furacao", "psg-token-999").unwrap();
        assert_eq!(invalid_role, None);
    }
}

