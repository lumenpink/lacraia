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
    local_window: usize,
    remote_window: usize,
    target_cache_duration_sec: u64,
    target_queue_duration_sec: u64,
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
            local_window: 5,
            remote_window: 10,
            target_cache_duration_sec: 3600,
            target_queue_duration_sec: 3600,
            poll_interval,
        }
    }

    pub fn new_with_sliding_window(
        db: Database,
        resolver: Arc<MediaResolver>,
        similarity: Arc<CompositeSimilarityEngine>,
        hub: Arc<PlaybackHub>,
        local_window: usize,
        remote_window: usize,
        poll_interval: Duration,
    ) -> Self {
        Self {
            db,
            resolver,
            similarity,
            hub,
            lookahead_window: local_window,
            local_window,
            remote_window,
            target_cache_duration_sec: 0,
            target_queue_duration_sec: 0,
            poll_interval,
        }
    }

    pub fn new_with_temporal_window(
        db: Database,
        resolver: Arc<MediaResolver>,
        similarity: Arc<CompositeSimilarityEngine>,
        hub: Arc<PlaybackHub>,
        target_cache_duration_sec: u64,
        target_queue_duration_sec: u64,
        poll_interval: Duration,
    ) -> Self {
        Self {
            db,
            resolver,
            similarity,
            hub,
            lookahead_window: 5,
            local_window: 5,
            remote_window: 10,
            target_cache_duration_sec,
            target_queue_duration_sec,
            poll_interval,
        }
    }

    pub fn find_prefetch_targets_for_window(
        &self,
        queue: &[(QueueEntry, TrackRecord)],
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

        let mut cached_count = 0;
        let mut cached_duration_sec: u64 = 0;
        let mut uncached_candidates = Vec::new();

        for (_, (entry, track)) in queue.iter().enumerate().skip(start_index) {
            if entry.state == QueueState::Failed {
                continue;
            }
            if track.cached_path.is_some() || entry.state == QueueState::Cached {
                cached_count += 1;
                cached_duration_sec += track.duration_sec;
            } else if entry.state == QueueState::Pending {
                uncached_candidates.push(track.clone());
            }
        }

        if self.target_cache_duration_sec > 0 {
            if cached_duration_sec >= self.target_cache_duration_sec {
                return vec![];
            }
            let mut targets = Vec::new();
            let mut accumulated = cached_duration_sec;
            for cand in uncached_candidates {
                accumulated += cand.duration_sec;
                targets.push(cand);
                if accumulated >= self.target_cache_duration_sec {
                    break;
                }
            }
            targets
        } else {
            if cached_count >= self.local_window {
                return vec![];
            }
            let needed = self.local_window - cached_count;
            uncached_candidates.into_iter().take(needed).collect()
        }
    }

    pub async fn maintain_autopilot_queue_sliding(
        &self,
        cabin_id: &str,
    ) -> Result<usize, QueueDaemonError> {
        if !self.hub.is_cabin_non_stop(cabin_id).await {
            return Ok(0);
        }

        let queue = self.db.get_queue_for_cabin(cabin_id)?;

        let mut playing_index = None;
        let mut upcoming_count = 0;
        let mut upcoming_duration_sec: u64 = 0;
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

        for (_, (entry, track)) in queue.iter().enumerate().skip(start_index) {
            if entry.state == QueueState::Pending || entry.state == QueueState::Cached {
                upcoming_count += 1;
                upcoming_duration_sec += track.duration_sec;
            }
        }

        let is_playing_in_hub = self.hub.is_cabin_playing(cabin_id).await;
        let has_playing_in_queue = queue.iter().any(|(e, _)| e.state == QueueState::Playing);
        if !is_playing_in_hub && !has_playing_in_queue {
            return Ok(0);
        }

        let is_temporal = self.target_queue_duration_sec > 0;
        if is_temporal {
            if upcoming_duration_sec >= self.target_queue_duration_sec {
                return Ok(0);
            }
        } else if upcoming_count >= self.remote_window {
            return Ok(0);
        }

        let deficit_sec = if is_temporal {
            self.target_queue_duration_sec.saturating_sub(upcoming_duration_sec)
        } else {
            0
        };

        let deficit_count = if is_temporal {
            ((deficit_sec / 60) + 5) as usize
        } else {
            self.remote_window.saturating_sub(upcoming_count)
        };

        // Multi-seed collection across recent history and active queue
        let mut seeds: Vec<(String, String, String)> = Vec::new();
        let mut seed_ids: Vec<String> = Vec::new();

        // 1. Recent playback history from SQLite (up to 5 recent tracks)
        if let Ok(history) = self.db.get_playback_history(Some(cabin_id), None, 5) {
            for h in history {
                if !seed_ids.contains(&h.track_id) {
                    seeds.push((h.artist, h.title, h.track_id.clone()));
                    seed_ids.push(h.track_id);
                }
            }
        }

        // 2. Currently playing / queued tracks
        if let Some(current) = self.hub.get_current_track_for_cabin(cabin_id).await {
            if !seed_ids.contains(&current.id) {
                seeds.push((current.artist, current.title, current.id.clone()));
                seed_ids.push(current.id);
            }
        }
        for (_, t) in queue.iter().rev() {
            if !seed_ids.contains(&t.id) {
                seeds.push((t.artist.clone(), t.title.clone(), t.id.clone()));
                seed_ids.push(t.id.clone());
            }
            if seeds.len() >= 5 {
                break;
            }
        }

        if seeds.is_empty() {
            return Ok(0);
        }

        info!(
            "Autopilot Sliding: Cabin '{}' has {} upcoming (target count: {}, target duration: {}s). Multi-seed count: {}",
            cabin_id, upcoming_count, self.remote_window, self.target_queue_duration_sec, seeds.len()
        );

        let mut candidates_to_add: Vec<(String, String, String, u64)> = Vec::new();
        let mut added_duration_sec: u64 = 0;

        // Query multi-seed similarity engine
        let query_limit = (deficit_count * 2).max(10);
        if let Ok(sim_results) = self.similarity.find_similar_multi(&seeds, query_limit).await {
            for c in sim_results {
                if !exclude_ids.contains(&c.id)
                    && !candidates_to_add.iter().any(|(id, ..)| id == &c.id)
                {
                    let dur = c.duration_sec.unwrap_or(210);
                    candidates_to_add.push((c.id, c.artist, c.title, dur));
                    added_duration_sec += dur;
                    if is_temporal {
                        if added_duration_sec >= deficit_sec {
                            break;
                        }
                    } else if candidates_to_add.len() >= deficit_count {
                        break;
                    }
                }
            }
        }

        let still_needed = if is_temporal {
            added_duration_sec < deficit_sec
        } else {
            candidates_to_add.len() < deficit_count
        };

        if still_needed {
            let mut local_exclude = exclude_ids.clone();
            for (id, ..) in &candidates_to_add {
                local_exclude.push(id.clone());
            }
            let needed_query = if is_temporal {
                let rem_sec = deficit_sec.saturating_sub(added_duration_sec);
                ((rem_sec / 60) + 5) as usize
            } else {
                deficit_count - candidates_to_add.len()
            };
            if let Ok(local_recs) = self.db.get_recommendations_for_multi_seeds(&seed_ids, &local_exclude, needed_query) {
                for r in local_recs {
                    if !candidates_to_add.iter().any(|(id, ..)| id == &r.id) {
                        candidates_to_add.push((r.id, r.artist, r.title, r.duration_sec));
                        added_duration_sec += r.duration_sec;
                        if is_temporal {
                            if added_duration_sec >= deficit_sec {
                                break;
                            }
                        } else if candidates_to_add.len() >= deficit_count {
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
            if self.db.enqueue_track_with_meta(&id, Some("AUTOPILOT"), cabin_id).is_ok() {
                enqueued += 1;
            }
        }

        if enqueued > 0 {
            info!("Autopilot Sliding: Enqueued {} tracks for cabin '{}'", enqueued, cabin_id);
            self.hub.broadcast_state_for_cabin(cabin_id).await;
        }

        Ok(enqueued)
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
        } else {
            current_track_in_hub.as_ref().map(|current| (current.artist.clone(), current.title.clone(), current.id.clone()))
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
            if self.db.enqueue_track_with_meta(&id, Some("AUTOPILOT"), cabin_id).is_ok() {
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
            let _ = self.maintain_autopilot_queue_sliding(cid).await;
        }

        let mut all_queues = Vec::new();
        for cid in cabin_ids {
            if let Ok(q) = self.db.get_queue_for_cabin(&cid) {
                all_queues.extend(q);
            }
        }

        let targets = self.find_prefetch_targets_for_window(&all_queues);

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

    #[tokio::test]
    async fn test_5_local_and_10_remote_lookahead_sliding_window() {
        let db = Database::open_in_memory().unwrap();
        let (tx, _rx) = tokio::sync::broadcast::channel(100);
        let hub = Arc::new(PlaybackHub::new(db.clone(), tx));
        let resolver = Arc::new(crate::resolver::MediaResolver::new(std::path::PathBuf::from("/tmp/cache"), db.clone()));
        let similarity = Arc::new(CompositeSimilarityEngine::new(None, db.clone()));

        // Default lookahead: 5 local, 10 remote
        let daemon = QueueDaemon::new_with_sliding_window(
            db.clone(),
            resolver,
            similarity,
            hub.clone(),
            5,  // local cached window
            10, // remote queue window
            Duration::from_millis(50),
        );

        // 1. Initial cabin playing a seed
        let seed = TrackRecord {
            id: "seed_1".into(),
            artist: "MC Serginho".into(),
            title: "Vai Lacraia".into(),
            duration_sec: 240,
            cached_path: Some("/cache/vai_lacraia.opus".into()),
            genres: Some("funk".into()),
            last_played_at: None,
        };
        db.upsert_track(&seed).unwrap();
        for i in 1..=15 {
            let trk = TrackRecord {
                id: format!("cand_{}", i),
                artist: "MC Serginho".into(),
                title: format!("Funk Track {}", i),
                duration_sec: 180 + i as u64,
                cached_path: None,
                genres: Some("funk".into()),
                last_played_at: None,
            };
            db.upsert_track(&trk).unwrap();
            db.insert_edge("seed_1", &format!("cand_{}", i), 0.95 - (i as f64 * 0.02), "test").unwrap();
        }
        db.play_now_with_meta("seed_1", Some("PILOTO"), "cab_lookahead").unwrap();
        let _ = hub.dispatch_command_with_role(lacraia_shared::CommandAction::Play, "cab_lookahead", Some(lacraia_shared::Role::Pilot)).await;

        // 2. Maintain autopilot with non-stop enabled -> must populate until 10 upcoming tracks
        let enqueued = daemon.maintain_autopilot_queue_sliding("cab_lookahead").await.unwrap();
        assert_eq!(enqueued, 10, "Autopilot must enqueue 10 remote upcoming tracks");

        let queue = db.get_queue_for_cabin("cab_lookahead").unwrap();
        let upcoming: Vec<_> = queue.iter().filter(|(e, _)| e.state != QueueState::Playing).collect();
        assert_eq!(upcoming.len(), 10, "Queue must contain 10 upcoming remote tracks");

        // 3. Find prefetch targets -> must target exactly 5 local tracks
        let all_queues = db.get_queue_for_cabin("cab_lookahead").unwrap();
        let prefetch_targets = daemon.find_prefetch_targets_for_window(&all_queues);
        assert_eq!(prefetch_targets.len(), 5, "Daemon must prefetch exactly 5 local cached tracks");

        // 4. When non-stop is disabled for this cabin, maintain_autopilot does not add more
        hub.set_cabin_non_stop("cab_lookahead", false).await;
        let enqueued_disabled = daemon.maintain_autopilot_queue_sliding("cab_lookahead").await.unwrap();
        assert_eq!(enqueued_disabled, 0, "When non-stop is disabled, autopilot must not auto-populate");
    }

    #[tokio::test]
    async fn test_queue_daemon_temporal_lookahead_duration_based() {
        let db = Database::open_in_memory().unwrap();
        let (tx, _rx) = tokio::sync::broadcast::channel(100);
        let hub = Arc::new(PlaybackHub::new(db.clone(), tx));
        let resolver = Arc::new(crate::resolver::MediaResolver::new(std::path::PathBuf::from("/tmp/cache"), db.clone()));
        let similarity = Arc::new(CompositeSimilarityEngine::new(None, db.clone()));

        // Temporal lookahead daemon: 600s cache target, 1200s queue target
        let daemon = QueueDaemon::new_with_temporal_window(
            db.clone(),
            resolver,
            similarity,
            hub.clone(),
            600,  // target_cache_duration_sec: 600s (10 min)
            1200, // target_queue_duration_sec: 1200s (20 min)
            Duration::from_millis(50),
        );

        let dummy_track = |id: &str, dur: u64| TrackRecord {
            id: id.into(),
            artist: "Garotos Podres".into(),
            title: id.into(),
            duration_sec: dur,
            cached_path: None,
            genres: Some("punk".into()),
            last_played_at: None,
        };

        let dummy_entry = |id: i64, state: QueueState| QueueEntry {
            id,
            track_id: format!("punk_{}", id),
            position: id,
            state,
            created_at: 0,
            cabin_id: "cab_temporal".into(),
            requested_by: None,
        };

        // Track 1 playing (60s), followed by 15 pending short tracks of 60s each
        let mut queue = vec![(dummy_entry(1, QueueState::Playing), dummy_track("punk_1", 60))];
        for i in 2..=16 {
            queue.push((dummy_entry(i, QueueState::Pending), dummy_track(&format!("punk_{}", i), 60)));
        }

        // To satisfy 600s of cache with 60s tracks, it must select 10 tracks, NOT 5!
        let prefetch_targets = daemon.find_prefetch_targets_for_window(&queue);
        assert_eq!(prefetch_targets.len(), 10, "With 60s tracks and 600s target, must select 10 tracks");

        // Autopilot sliding: seed cabin playing, and DB has 25 candidate tracks of 60s each.
        // With target_queue_duration_sec = 1200s, it must enqueue 20 tracks (20 * 60s = 1200s), NOT just 10!
        let seed = TrackRecord {
            id: "seed_temporal".into(),
            artist: "Garotos Podres".into(),
            title: "Papai Noel".into(),
            duration_sec: 60,
            cached_path: Some("/cache/papai_noel.opus".into()),
            genres: Some("punk".into()),
            last_played_at: None,
        };
        db.upsert_track(&seed).unwrap();
        for i in 1..=25 {
            let trk = TrackRecord {
                id: format!("punk_cand_{}", i),
                artist: "Garotos Podres".into(),
                title: format!("Punk Song {}", i),
                duration_sec: 60,
                cached_path: None,
                genres: Some("punk".into()),
                last_played_at: None,
            };
            db.upsert_track(&trk).unwrap();
            db.insert_edge("seed_temporal", &format!("punk_cand_{}", i), 0.99 - (i as f64 * 0.01), "test").unwrap();
        }
        db.play_now_with_meta("seed_temporal", Some("PILOTO"), "cab_temporal").unwrap();
        let _ = hub.dispatch_command_with_role(lacraia_shared::CommandAction::Play, "cab_temporal", Some(lacraia_shared::Role::Pilot)).await;

        let enqueued = daemon.maintain_autopilot_queue_sliding("cab_temporal").await.unwrap();
        assert_eq!(enqueued, 20, "Autopilot must enqueue 20 tracks (60s each) to satisfy 1200s target queue duration");

        let queue = db.get_queue_for_cabin("cab_temporal").unwrap();
        let upcoming_duration: u64 = queue.iter()
            .filter(|(e, _)| e.state != QueueState::Playing)
            .map(|(_, t)| t.duration_sec)
            .sum();
        assert!(upcoming_duration >= 1200, "Total upcoming queue duration must be at least 1200s");
    }
}

