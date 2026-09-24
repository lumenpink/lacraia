use crate::db::{Database, QueueEntry, QueueState, TrackRecord};
use crate::resolver::{MediaResolver, ResolverError};
use crate::similarity::{CompositeSimilarityEngine, SimilarityEngine};
use crate::ws::PlaybackHub;
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
    similarity: Arc<CompositeSimilarityEngine>,
    hub: Arc<PlaybackHub>,
    lookahead_window: usize,
    poll_interval: Duration,
}

impl QueueDaemon {
    pub fn new(
        db: Database,
        resolver: Arc<MediaResolver>,
        similarity: Arc<CompositeSimilarityEngine>,
        hub: Arc<PlaybackHub>,
        lookahead_window: usize,
        poll_interval: Duration,
    ) -> Self {
        Self {
            db,
            resolver,
            similarity,
            hub,
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

    /// Ensures that for each active cabin, there are at least `target_ahead` (e.g. 5) tracks
    /// ahead in the queue (pending or cached), auto-populating from similarity recommendations.
    pub async fn maintain_autopilot_queue(
        &self,
        cabin_id: &str,
        target_ahead: usize,
    ) -> Result<usize, QueueDaemonError> {
        let queue = self.db.get_queue_for_cabin(cabin_id)?;

        let mut playing_index = None;
        let mut upcoming_count = 0;
        let mut exclude_ids = Vec::new();

        for (i, (entry, track)) in queue.iter().enumerate() {
            exclude_ids.push(track.id.clone());
            if entry.state == QueueState::Playing {
                playing_index = Some(i);
            }
        }

        let start_index = match playing_index {
            Some(idx) => idx + 1,
            None => 0,
        };

        for (_, (entry, _)) in queue.iter().enumerate().skip(start_index) {
            if entry.state == QueueState::Pending || entry.state == QueueState::Cached {
                upcoming_count += 1;
            }
        }

        // The autopilot activates only when the cabin is actively playing (either in the hub
        // or has a track marked as Playing in the queue). If dormant/idle, it waits for the user.
        let is_playing_in_hub = self.hub.is_cabin_playing(cabin_id).await;
        let has_playing_in_queue = queue.iter().any(|(e, _)| e.state == QueueState::Playing);
        if !is_playing_in_hub && !has_playing_in_queue {
            return Ok(0);
        }

        if upcoming_count >= target_ahead {
            return Ok(0);
        }

        let deficit = target_ahead - upcoming_count;

        let current_track_in_hub = self.hub.get_current_track_for_cabin(cabin_id).await;
        // Determine seed track: last track in queue, or current playing track in hub
        let seed: Option<(String, String, String)> = if let Some((_, last_track)) = queue.last() {
            Some((last_track.artist.clone(), last_track.title.clone(), last_track.id.clone()))
        } else if let Some(ref current) = current_track_in_hub {
            Some((current.artist.clone(), current.title.clone(), current.id.clone()))
        } else {
            None
        };

        let (seed_artist, seed_title, seed_id) = match seed {
            Some(s) => s,
            None => return Ok(0),
        };

        info!(
            "Autopilot: Cabin '{}' has {} tracks ahead (target: {}). Querying similarity for '{} - {}' (deficit: {})",
            cabin_id, upcoming_count, target_ahead, seed_artist, seed_title, deficit
        );

        let mut candidates_to_add: Vec<(String, String, String, u64)> = Vec::new();

        if let Ok(sim_results) = self.similarity.find_similar(&seed_artist, &seed_title, 10).await {
            for c in sim_results {
                if !exclude_ids.contains(&c.id)
                    && !candidates_to_add.iter().any(|(id, ..)| id == &c.id)
                {
                    candidates_to_add.push((c.id, c.artist, c.title, c.duration_sec.unwrap_or(210)));
                    if candidates_to_add.len() >= deficit {
                        break;
                    }
                }
            }
        }

        if candidates_to_add.len() < deficit {
            let mut local_exclude = exclude_ids.clone();
            for (id, ..) in &candidates_to_add {
                local_exclude.push(id.clone());
            }
            if let Ok(local_recs) = self.db.get_recommendations_for_seed(&seed_id, &local_exclude, deficit - candidates_to_add.len()) {
                for r in local_recs {
                    if !candidates_to_add.iter().any(|(id, ..)| id == &r.id) {
                        candidates_to_add.push((r.id, r.artist, r.title, r.duration_sec));
                        if candidates_to_add.len() >= deficit {
                            break;
                        }
                    }
                }
            }
        }

        let mut enqueued = 0;
        for (id, artist, title, dur) in candidates_to_add {
            let record = TrackRecord {
                id: id.clone(),
                artist,
                title,
                duration_sec: dur,
                cached_path: None,
                genres: None,
                last_played_at: None,
            };
            let _ = self.db.upsert_track(&record);
            if let Ok(_) = self.db.enqueue_track_with_meta(&id, Some("AUTOPILOT"), cabin_id) {
                enqueued += 1;
            }
        }

        if enqueued > 0 {
            info!("Autopilot: Successfully enqueued {} tracks for cabin '{}'", enqueued, cabin_id);
            self.hub.broadcast_state_for_cabin(cabin_id).await;
        }

        Ok(enqueued)
    }

    /// Executes a single prefetch cycle for the lookahead window.
    pub async fn step(&self) -> Result<usize, QueueDaemonError> {
        let mut cabin_ids = vec!["default".to_string()];
        if let Ok(cabins) = self.db.list_cabins() {
            for c in cabins {
                if !cabin_ids.contains(&c.id) {
                    cabin_ids.push(c.id);
                }
            }
        }

        for cid in &cabin_ids {
            let _ = self.maintain_autopilot_queue(cid, self.lookahead_window).await;
        }

        let mut all_queues = Vec::new();
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

