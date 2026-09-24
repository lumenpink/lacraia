use crate::db::{Database, QueueEntry, QueueState, TrackRecord};
use crate::resolver::{MediaResolver, ResolverError};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::time::sleep;
use tracing::{error, info, warn};

#[derive(Debug, Error)]
pub enum QueueDaemonError {
    #[error("Database error: {0}")]
    Db(#[from] crate::db::DbError),
    #[error("Resolver error: {0}")]
    Resolver(#[from] ResolverError),
}

pub struct QueueDaemon {
    db: Database,
    resolver: Arc<MediaResolver>,
    lookahead_window: usize,
    poll_interval: Duration,
}

impl QueueDaemon {
    pub fn new(
        db: Database,
        resolver: Arc<MediaResolver>,
        lookahead_window: usize,
        poll_interval: Duration,
    ) -> Self {
        Self {
            db,
            resolver,
            lookahead_window,
            poll_interval,
        }
    }

    /// Identifies pending tracks within the lookahead window ahead of active playback.
    pub fn find_prefetch_targets(
        queue: &[(QueueEntry, TrackRecord)],
        lookahead_window: usize,
    ) -> Vec<TrackRecord> {
        let mut playing_index = None;
        for (i, (entry, _)) in queue.iter().enumerate() {
            if entry.state == QueueState::Playing {
                playing_index = Some(i);
                break;
            }
        }

        let start_index = match playing_index {
            Some(idx) => idx + 1,
            None => 0,
        };

        let mut targets = Vec::new();
        for (_, (entry, track)) in queue.iter().enumerate().skip(start_index) {
            if targets.len() >= lookahead_window {
                break;
            }
            if entry.state == QueueState::Pending && track.cached_path.is_none() {
                targets.push(track.clone());
            }
        }

        targets
    }

    /// Executes a single prefetch cycle for the lookahead window.
    pub async fn step(&self) -> Result<usize, QueueDaemonError> {
        let mut all_queues = Vec::new();
        let mut cabin_ids = vec!["default".to_string()];
        if let Ok(cabins) = self.db.list_cabins() {
            for c in cabins {
                if !cabin_ids.contains(&c.id) {
                    cabin_ids.push(c.id);
                }
            }
        }
        for cid in cabin_ids {
            if let Ok(q) = self.db.get_queue_for_cabin(&cid) {
                all_queues.extend(q);
            }
        }

        let targets = Self::find_prefetch_targets(&all_queues, self.lookahead_window);

        let mut processed = 0;
        for track in targets {
            info!(
                "QueueDaemon: Pre-caching track '{} - {}' (expected {}s)",
                track.artist, track.title, track.duration_sec
            );

            match self
                .resolver
                .resolve_and_cache(&track.artist, &track.title, track.duration_sec)
                .await
            {
                Ok(res) => {
                    info!(
                        "QueueDaemon: Successfully cached '{}' at {}",
                        res.title, res.cache_path
                    );
                    self.db.mark_cached(&track.id, &res.cache_path)?;
                    processed += 1;
                }
                Err(e) => {
                    warn!(
                        "QueueDaemon: Pre-caching failed for '{}': {}",
                        track.title, e
                    );
                    if let Err(db_err) = self.db.mark_failed(&track.id) {
                        warn!("QueueDaemon: Failed to mark track as failed: {}", db_err);
                    }
                }
            }
        }

        Ok(processed)
    }


    /// Spawns the daemon background task.
    pub fn start(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            info!(
                "QueueDaemon started with lookahead window of {} tracks",
                self.lookahead_window
            );
            loop {
                if let Err(e) = self.step().await {
                    error!("QueueDaemon step encountered error: {}", e);
                }
                sleep(self.poll_interval).await;
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookahead_window_selection() {
        let dummy_track = |id: &str| TrackRecord {
            id: id.into(),
            artist: "Artist".into(),
            title: id.into(),
            duration_sec: 200,
            cached_path: None,
            genres: None,
            last_played_at: None,
        };

        let dummy_entry = |id: i64, state: QueueState| QueueEntry {
            id,
            track_id: format!("trk_{}", id),
            position: id,
            state,
            created_at: 0,
            cabin_id: "default".into(),
            requested_by: None,
        };

        // Scenario: Track 1 is Playing, Track 2 is Pending, Track 3 is Pending, Track 4 is Pending
        let queue = vec![
            (dummy_entry(1, QueueState::Playing), dummy_track("trk_1")),
            (dummy_entry(2, QueueState::Pending), dummy_track("trk_2")),
            (dummy_entry(3, QueueState::Pending), dummy_track("trk_3")),
            (dummy_entry(4, QueueState::Pending), dummy_track("trk_4")),
        ];

        let targets = QueueDaemon::find_prefetch_targets(&queue, 2);
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].id, "trk_2");
        assert_eq!(targets[1].id, "trk_3");

        // Scenario: Track 2 already cached, should take Track 3 and Track 4
        let queue2 = vec![
            (dummy_entry(1, QueueState::Playing), dummy_track("trk_1")),
            (dummy_entry(2, QueueState::Cached), dummy_track("trk_2")),
            (dummy_entry(3, QueueState::Pending), dummy_track("trk_3")),
            (dummy_entry(4, QueueState::Pending), dummy_track("trk_4")),
        ];

        let targets2 = QueueDaemon::find_prefetch_targets(&queue2, 2);
        assert_eq!(targets2.len(), 2);
        assert_eq!(targets2[0].id, "trk_3");
        assert_eq!(targets2[1].id, "trk_4");

        // Scenario: Nothing is Playing yet (cold start queue)
        let queue3 = vec![
            (dummy_entry(1, QueueState::Pending), dummy_track("trk_1")),
            (dummy_entry(2, QueueState::Pending), dummy_track("trk_2")),
            (dummy_entry(3, QueueState::Pending), dummy_track("trk_3")),
        ];

        let targets3 = QueueDaemon::find_prefetch_targets(&queue3, 2);
        assert_eq!(targets3.len(), 2);
        assert_eq!(targets3[0].id, "trk_1");
        assert_eq!(targets3[1].id, "trk_2");
    }

    #[test]
    fn test_lookahead_all_cached() {
        let dummy_track = |id: &str| TrackRecord {
            id: id.into(),
            artist: "Artist".into(),
            title: id.into(),
            duration_sec: 200,
            cached_path: Some("/cache/file.opus".into()),
            genres: None,
            last_played_at: None,
        };

        let dummy_entry = |id: i64, state: QueueState| QueueEntry {
            id,
            track_id: format!("trk_{}", id),
            position: id,
            state,
            created_at: 0,
            cabin_id: "default".into(),
            requested_by: None,
        };

        let queue = vec![
            (dummy_entry(1, QueueState::Playing), dummy_track("trk_1")),
            (dummy_entry(2, QueueState::Cached), dummy_track("trk_2")),
            (dummy_entry(3, QueueState::Cached), dummy_track("trk_3")),
        ];

        let targets = QueueDaemon::find_prefetch_targets(&queue, 2);
        assert_eq!(targets.len(), 0);
    }

    #[test]
    fn test_lookahead_skips_failed() {
        let dummy_track = |id: &str| TrackRecord {
            id: id.into(),
            artist: "Artist".into(),
            title: id.into(),
            duration_sec: 200,
            cached_path: None,
            genres: None,
            last_played_at: None,
        };

        let dummy_entry = |id: i64, state: QueueState| QueueEntry {
            id,
            track_id: format!("trk_{}", id),
            position: id,
            state,
            created_at: 0,
            cabin_id: "default".into(),
            requested_by: None,
        };

        // Scenario: Track 1 Playing, Track 2 Failed, Track 3 Pending
        let queue = vec![
            (dummy_entry(1, QueueState::Playing), dummy_track("trk_1")),
            (dummy_entry(2, QueueState::Failed), dummy_track("trk_2")),
            (dummy_entry(3, QueueState::Pending), dummy_track("trk_3")),
        ];

        let targets = QueueDaemon::find_prefetch_targets(&queue, 2);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].id, "trk_3");
    }
}

