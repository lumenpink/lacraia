use crate::db::{Database, TrackRecord};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;
use tracing::{info, warn};

#[derive(Debug, Error)]
pub enum SimilarityError {
    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Database error: {0}")]
    Db(#[from] crate::db::DbError),
    #[error("No similarity results available from any provider")]
    NoResults,
    #[error("Provider error: {0}")]
    Provider(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimilarityCandidate {
    pub id: String,
    pub artist: String,
    pub title: String,
    pub match_score: f64,
    pub mbid: Option<String>,
    pub provider: String,
    pub duration_sec: Option<u64>,
}

#[async_trait]
pub trait SimilarityEngine: Send + Sync {
    async fn find_similar(
        &self,
        artist: &str,
        track: &str,
        limit: usize,
    ) -> Result<Vec<SimilarityCandidate>, SimilarityError>;
}

// ─────────────────────────────────────────────────────────────────────────────
// Last.fm Provider
// ─────────────────────────────────────────────────────────────────────────────

pub struct LastFmProvider {
    client: Client,
    api_key: String,
}

impl LastFmProvider {
    pub fn new(api_key: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_default();
        Self { client, api_key }
    }

    pub async fn search_tracks(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SimilarityCandidate>, SimilarityError> {
        let url = "https://ws.audioscrobbler.com/2.0/";
        let resp = self
            .client
            .get(url)
            .query(&[
                ("method", "track.search"),
                ("track", query),
                ("api_key", &self.api_key),
                ("format", "json"),
                ("limit", &limit.to_string()),
            ])
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(SimilarityError::Provider(format!(
                "Last.fm track.search returned HTTP {}",
                resp.status()
            )));
        }

        let body: LastFmSearchResponse = resp.json().await?;
        let tracks = body
            .results
            .and_then(|r| r.trackmatches)
            .and_then(|tm| tm.track)
            .unwrap_or_default();

        let candidates = tracks
            .into_iter()
            .map(|t| {
                let id = format!("{}:{}", t.artist.to_lowercase(), t.name.to_lowercase());
                SimilarityCandidate {
                    id,
                    artist: t.artist,
                    title: t.name,
                    match_score: 0.9,
                    mbid: t.mbid.filter(|s| !s.is_empty()),
                    provider: "lastfm".to_string(),
                    duration_sec: Some(210),
                }
            })
            .collect();

        Ok(candidates)
    }
}

#[derive(Deserialize)]
struct LastFmSearchResponse {
    results: Option<LastFmSearchResults>,
}

#[derive(Deserialize)]
struct LastFmSearchResults {
    trackmatches: Option<LastFmTrackMatches>,
}

#[derive(Deserialize)]
struct LastFmTrackMatches {
    track: Option<Vec<LastFmSearchTrack>>,
}

#[derive(Deserialize)]
struct LastFmSearchTrack {
    name: String,
    artist: String,
    mbid: Option<String>,
}

#[derive(Deserialize)]
struct LastFmResponse {
    similartracks: Option<LastFmSimilarTracks>,
}

#[derive(Deserialize)]
struct LastFmSimilarTracks {
    track: Option<Vec<LastFmTrack>>,
}

#[derive(Deserialize)]
struct LastFmTrack {
    name: String,
    artist: LastFmArtist,
    #[serde(rename = "match")]
    match_score: Option<serde_json::Value>,
    mbid: Option<String>,
    duration: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct LastFmArtist {
    name: String,
}

#[async_trait]
impl SimilarityEngine for LastFmProvider {
    async fn find_similar(
        &self,
        artist: &str,
        track: &str,
        limit: usize,
    ) -> Result<Vec<SimilarityCandidate>, SimilarityError> {
        let url = "https://ws.audioscrobbler.com/2.0/";
        let resp = self
            .client
            .get(url)
            .query(&[
                ("method", "track.getsimilar"),
                ("artist", artist),
                ("track", track),
                ("api_key", &self.api_key),
                ("format", "json"),
                ("limit", &limit.to_string()),
            ])
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(SimilarityError::Provider(format!(
                "Last.fm returned HTTP {}",
                resp.status()
            )));
        }

        let body: LastFmResponse = resp.json().await?;
        let tracks = body
            .similartracks
            .and_then(|st| st.track)
            .unwrap_or_default();

        let candidates = tracks
            .into_iter()
            .map(|t| {
                let score = match t.match_score {
                    Some(serde_json::Value::Number(n)) => n.as_f64().unwrap_or(0.0),
                    Some(serde_json::Value::String(s)) => s.parse::<f64>().unwrap_or(0.0),
                    _ => 0.0,
                };

                let duration_sec = match t.duration {
                    Some(serde_json::Value::Number(n)) => n.as_u64(),
                    Some(serde_json::Value::String(s)) => s.parse::<u64>().ok(),
                    _ => None,
                };

                let id = format!("{}:{}", t.artist.name.to_lowercase(), t.name.to_lowercase());

                SimilarityCandidate {
                    id,
                    artist: t.artist.name,
                    title: t.name,
                    match_score: score,
                    mbid: t.mbid.filter(|s| !s.is_empty()),
                    provider: "lastfm".to_string(),
                    duration_sec,
                }
            })
            .collect();

        Ok(candidates)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// MetaBrainz / ListenBrainz Provider (Open Data, No Key Required)
// ─────────────────────────────────────────────────────────────────────────────

pub struct MetaBrainzProvider {
    client: Client,
}

impl MetaBrainzProvider {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(6))
            .user_agent("LaCraia/0.1.0 ( https://lacraia.2lp.in )")
            .build()
            .unwrap_or_default();
        Self { client }
    }

    pub async fn search_tracks(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SimilarityCandidate>, SimilarityError> {
        let clean = query.replace('"', "").replace('\\', "");
        let query_str = format!("recording:\"{}\"^3 OR artist:\"{}\"^2 OR \"{}\"", clean, clean, clean);
        let url = "https://musicbrainz.org/ws/2/recording";
        let resp = self
            .client
            .get(url)
            .query(&[("query", query_str.as_str()), ("fmt", "json"), ("limit", &limit.to_string())])
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(SimilarityError::Provider(format!(
                "MusicBrainz returned HTTP {}",
                resp.status()
            )));
        }

        let body: MusicBrainzSearchResponse = resp.json().await?;
        let recordings = body.recordings.unwrap_or_default();

        let candidates = recordings
            .into_iter()
            .take(limit)
            .map(|r| {
                let artist_name = r
                    .artist_credit
                    .and_then(|ac| ac.into_iter().next())
                    .map(|a| a.name)
                    .unwrap_or_else(|| "Desconhecido".to_string());

                let score = r.score.unwrap_or(80) as f64 / 100.0;
                let duration_sec = r.length.map(|ms| ms / 1000);
                let id = format!("{}:{}", artist_name.to_lowercase(), r.title.to_lowercase());

                SimilarityCandidate {
                    id,
                    artist: artist_name,
                    title: r.title,
                    match_score: score,
                    mbid: Some(r.id),
                    provider: "metabrainz".to_string(),
                    duration_sec,
                }
            })
            .collect();

        Ok(candidates)
    }
}

impl Default for MetaBrainzProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
struct MusicBrainzSearchResponse {
    recordings: Option<Vec<MbRecording>>,
}

#[derive(Deserialize)]
struct MbRecording {
    id: String,
    title: String,
    length: Option<u64>,
    #[serde(rename = "artist-credit")]
    artist_credit: Option<Vec<MbArtistCredit>>,
    score: Option<u32>,
}

#[derive(Deserialize)]
struct MbArtistCredit {
    name: String,
}

#[async_trait]
impl SimilarityEngine for MetaBrainzProvider {
    async fn find_similar(
        &self,
        artist: &str,
        track: &str,
        limit: usize,
    ) -> Result<Vec<SimilarityCandidate>, SimilarityError> {
        // Query MusicBrainz search for recording matches
        let query = format!("artist:\"{}\" AND recording:\"{}\"", artist, track);
        let url = "https://musicbrainz.org/ws/2/recording";
        let resp = self
            .client
            .get(url)
            .query(&[("query", query.as_str()), ("fmt", "json"), ("limit", "5")])
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(SimilarityError::Provider(format!(
                "MusicBrainz returned HTTP {}",
                resp.status()
            )));
        }

        let body: MusicBrainzSearchResponse = resp.json().await?;
        let recordings = body.recordings.unwrap_or_default();

        let candidates = recordings
            .into_iter()
            .take(limit)
            .map(|r| {
                let artist_name = r
                    .artist_credit
                    .and_then(|ac| ac.into_iter().next())
                    .map(|a| a.name)
                    .unwrap_or_else(|| artist.to_string());

                let score = r.score.unwrap_or(80) as f64 / 100.0;
                let duration_sec = r.length.map(|ms| ms / 1000);
                let id = format!("{}:{}", artist_name.to_lowercase(), r.title.to_lowercase());

                SimilarityCandidate {
                    id,
                    artist: artist_name,
                    title: r.title,
                    match_score: score,
                    mbid: Some(r.id),
                    provider: "metabrainz".to_string(),
                    duration_sec,
                }
            })
            .collect();

        Ok(candidates)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Composite Engine (Auto-Indexing & Offline Resilient)
// ─────────────────────────────────────────────────────────────────────────────

pub struct CompositeSimilarityEngine {
    lastfm: Option<LastFmProvider>,
    metabrainz: MetaBrainzProvider,
    db: Database,
}

impl CompositeSimilarityEngine {
    pub fn new(lastfm_key: Option<String>, db: Database) -> Self {
        let lastfm = lastfm_key.filter(|k| !k.is_empty()).map(LastFmProvider::new);
        let metabrainz = MetaBrainzProvider::new();
        Self {
            lastfm,
            metabrainz,
            db,
        }
    }

    /// Automatically indexes discovered similarity candidates and edges into SQLite
    pub fn persist_candidates(
        &self,
        seed_id: &str,
        candidates: &[SimilarityCandidate],
    ) {
        for c in candidates {
            let record = TrackRecord {
                id: c.id.clone(),
                artist: c.artist.clone(),
                title: c.title.clone(),
                duration_sec: c.duration_sec.unwrap_or(210),
                cached_path: None,
                genres: None,
                last_played_at: None,
            };

            if let Err(e) = self.db.upsert_track(&record) {
                warn!("Failed to index track {}: {}", c.id, e);
            }

            if let Err(e) = self.db.insert_edge(seed_id, &c.id, c.match_score, &c.provider) {
                warn!("Failed to index edge {} -> {}: {}", seed_id, c.id, e);
            }
        }
    }

    /// Performs online search via Last.fm and/or MetaBrainz, persisting all candidates
    pub async fn search_tracks(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SimilarityCandidate>, SimilarityError> {
        // 1. Try Last.fm if configured
        if let Some(ref lfm) = self.lastfm {
            match lfm.search_tracks(query, limit).await {
                Ok(results) if !results.is_empty() => {
                    self.persist_search_candidates(&results);
                    return Ok(results);
                }
                _ => {}
            }
        }

        // 2. Try MetaBrainz (Open Data, no key required)
        match self.metabrainz.search_tracks(query, limit).await {
            Ok(results) if !results.is_empty() => {
                self.persist_search_candidates(&results);
                return Ok(results);
            }
            Ok(_) => {}
            Err(e) => {
                warn!("MusicBrainz search error: {}", e);
            }
        }

        Ok(vec![])
    }

    pub fn persist_search_candidates(&self, candidates: &[SimilarityCandidate]) {
        for c in candidates {
            let record = TrackRecord {
                id: c.id.clone(),
                artist: c.artist.clone(),
                title: c.title.clone(),
                duration_sec: c.duration_sec.unwrap_or(210),
                cached_path: None,
                genres: Some("jukebox".to_string()),
                last_played_at: None,
            };
            if let Err(e) = self.db.upsert_track(&record) {
                warn!("Failed to persist search candidate {}: {}", c.id, e);
            }
        }
    }

    /// Queries similarities across multiple seed tracks (artist, title, id),
    /// combining and ranking candidates according to collective affinity.
    pub async fn find_similar_multi(
        &self,
        seeds: &[(String, String, String)],
        limit: usize,
    ) -> Result<Vec<SimilarityCandidate>, SimilarityError> {
        if seeds.is_empty() {
            return Err(SimilarityError::NoResults);
        }

        let mut candidate_map: std::collections::HashMap<String, (SimilarityCandidate, f64, usize)> = std::collections::HashMap::new();
        let mut seed_filter = std::collections::HashSet::new();
        for (artist, title, id) in seeds {
            seed_filter.insert(id.to_lowercase());
            seed_filter.insert(crate::db::strip_accents(&id.to_lowercase()));
            seed_filter.insert(format!("{}:{}", artist.to_lowercase(), title.to_lowercase()));
            seed_filter.insert(crate::db::strip_accents(&format!("{}:{}", artist.to_lowercase(), title.to_lowercase())));
            seed_filter.insert(title.to_lowercase());
            seed_filter.insert(crate::db::strip_accents(&title.to_lowercase()));
        }

        // 1. Try online/fallback queries for each individual seed
        for (artist, title, _seed_id) in seeds {
            if let Ok(sims) = self.find_similar(artist, title, limit).await {
                for c in sims {
                    let norm_id = crate::db::strip_accents(&c.id.to_lowercase());
                    let norm_title = crate::db::strip_accents(&c.title.to_lowercase());
                    if !seed_filter.contains(&c.id.to_lowercase())
                        && !seed_filter.contains(&norm_id)
                        && !seed_filter.contains(&c.title.to_lowercase())
                        && !seed_filter.contains(&norm_title)
                    {
                        let entry = candidate_map.entry(c.id.clone()).or_insert_with(|| (c.clone(), 0.0, 0));
                        entry.1 += c.match_score;
                        entry.2 += 1;
                    }
                }
            }
        }

        let seed_ids: Vec<String> = seeds.iter().map(|(_, _, id)| id.clone()).collect();
        // 2. Also check local SQLite graph across all seeds
        if let Ok(local_recs) = self.db.get_recommendations_for_multi_seeds(&seed_ids, &seed_ids, limit * 2) {
            for (idx, r) in local_recs.into_iter().enumerate() {
                let norm_id = crate::db::strip_accents(&r.id.to_lowercase());
                let norm_title = crate::db::strip_accents(&r.title.to_lowercase());
                if !seed_filter.contains(&r.id.to_lowercase())
                    && !seed_filter.contains(&norm_id)
                    && !seed_filter.contains(&r.title.to_lowercase())
                    && !seed_filter.contains(&norm_title)
                {
                    let score = 0.95 - (idx as f64 * 0.05).min(0.5);
                    let entry = candidate_map.entry(r.id.clone()).or_insert_with(|| (
                        SimilarityCandidate {
                            id: r.id.clone(),
                            artist: r.artist,
                            title: r.title,
                            match_score: score,
                            mbid: None,
                            provider: "local_multi_seed".to_string(),
                            duration_sec: Some(r.duration_sec),
                        },
                        0.0,
                        0,
                    ));
                    entry.1 += score;
                    entry.2 += 1;
                }
            }
        }

        if candidate_map.is_empty() {
            return Err(SimilarityError::NoResults);
        }

        let mut scored_candidates: Vec<SimilarityCandidate> = candidate_map
            .into_values()
            .map(|(mut cand, total_score, count)| {
                let boost = 1.0 + 0.4 * (count.saturating_sub(1) as f64);
                cand.match_score = (total_score / (count as f64)) * boost;
                cand
            })
            .collect();

        scored_candidates.sort_by(|a, b| b.match_score.partial_cmp(&a.match_score).unwrap_or(std::cmp::Ordering::Equal));
        scored_candidates.truncate(limit);
        Ok(scored_candidates)
    }
}

#[async_trait]
impl SimilarityEngine for CompositeSimilarityEngine {
    async fn find_similar(
        &self,
        artist: &str,
        track: &str,
        limit: usize,
    ) -> Result<Vec<SimilarityCandidate>, SimilarityError> {
        let seed_id = format!("{}:{}", artist.to_lowercase(), track.to_lowercase());

        // Ensure seed track itself exists in local DB
        let seed_record = TrackRecord {
            id: seed_id.clone(),
            artist: artist.to_string(),
            title: track.to_string(),
            duration_sec: 210,
            cached_path: None,
            genres: None,
            last_played_at: None,
        };
        let _ = self.db.upsert_track(&seed_record);

        // 1. Try Last.fm if configured
        if let Some(ref lfm) = self.lastfm {
            match lfm.find_similar(artist, track, limit).await {
                Ok(results) if !results.is_empty() => {
                    info!("Obtained {} similar tracks from Last.fm", results.len());
                    self.persist_candidates(&seed_id, &results);
                    return Ok(results);
                }
                Ok(_) => {
                    warn!("Last.fm returned empty results, attempting fallback");
                }
                Err(e) => {
                    warn!("Last.fm provider failed: {}, attempting fallback", e);
                }
            }
        }

        // 2. Try MetaBrainz fallback
        match self.metabrainz.find_similar(artist, track, limit).await {
            Ok(results) if !results.is_empty() => {
                info!("Obtained {} similar tracks from MetaBrainz", results.len());
                self.persist_candidates(&seed_id, &results);
                return Ok(results);
            }
            Ok(_) => {
                warn!("MetaBrainz returned empty results, falling back to local DB");
            }
            Err(e) => {
                warn!("MetaBrainz provider failed: {}, falling back to local DB", e);
            }
        }

        // 3. 100% Offline Mode: Fallback to local similarity graph in SQLite
        info!("Executing local offline similarity graph search for '{}'", seed_id);
        let local_matches = self.db.get_similar_tracks(&seed_id, false, limit)?;
        if !local_matches.is_empty() {
            let candidates = local_matches
                .into_iter()
                .map(|(t, weight)| SimilarityCandidate {
                    id: t.id,
                    artist: t.artist,
                    title: t.title,
                    match_score: weight,
                    mbid: None,
                    provider: "local_sqlite".to_string(),
                    duration_sec: Some(t.duration_sec),
                })
                .collect();
            return Ok(candidates);
        }

        Err(SimilarityError::NoResults)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_composite_engine_offline_fallback() {
        let db = Database::open_in_memory().unwrap();

        let seed = TrackRecord {
            id: "joy division:disorder".into(),
            artist: "Joy Division".into(),
            title: "Disorder".into(),
            duration_sec: 212,
            cached_path: Some("/cache/disorder.opus".into()),
            genres: Some("post-punk".into()),
            last_played_at: None,
        };

        let sim = TrackRecord {
            id: "new order:ceremony".into(),
            artist: "New Order".into(),
            title: "Ceremony".into(),
            duration_sec: 265,
            cached_path: Some("/cache/ceremony.opus".into()),
            genres: Some("post-punk".into()),
            last_played_at: None,
        };

        db.upsert_track(&seed).unwrap();
        db.upsert_track(&sim).unwrap();
        db.insert_edge("joy division:disorder", "new order:ceremony", 0.94, "lastfm").unwrap();

        // Create engine with invalid key so online fails, forcing offline DB fallback
        let engine = CompositeSimilarityEngine::new(None, db);
        // Querying non-existent online will fall back to local graph
        let results = engine
            .db
            .get_similar_tracks("joy division:disorder", true, 5)
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0.title, "Ceremony");
        assert_eq!(results[0].1, 0.94);
    }

    #[tokio::test]
    async fn test_composite_engine_multi_seed_query() {
        let db = Database::open_in_memory().unwrap();

        let s1 = TrackRecord {
            id: "mc:vai_lacraia".into(),
            artist: "MC Serginho".into(),
            title: "Vai Lacraia".into(),
            duration_sec: 240,
            cached_path: None,
            genres: Some("funk".into()),
            last_played_at: None,
        };
        let s2 = TrackRecord {
            id: "tigrao:cerol".into(),
            artist: "Bonde do Tigrao".into(),
            title: "Cerol na Mao".into(),
            duration_sec: 180,
            cached_path: None,
            genres: Some("funk".into()),
            last_played_at: None,
        };
        let rec1 = TrackRecord {
            id: "valesca:beijinho".into(),
            artist: "Valesca Popozuda".into(),
            title: "Beijinho no Ombro".into(),
            duration_sec: 170,
            cached_path: None,
            genres: Some("funk".into()),
            last_played_at: None,
        };

        db.upsert_track(&s1).unwrap();
        db.upsert_track(&s2).unwrap();
        db.upsert_track(&rec1).unwrap();
        db.insert_edge("mc:vai_lacraia", "valesca:beijinho", 0.90, "test").unwrap();
        db.insert_edge("tigrao:cerol", "valesca:beijinho", 0.85, "test").unwrap();

        let engine = CompositeSimilarityEngine::new(None, db);
        let seeds = vec![
            ("MC Serginho".to_string(), "Vai Lacraia".to_string(), "mc:vai_lacraia".to_string()),
            ("Bonde do Tigrao".to_string(), "Cerol na Mao".to_string(), "tigrao:cerol".to_string()),
        ];

        let results = engine.find_similar_multi(&seeds, 5).await.unwrap();
        assert!(!results.is_empty(), "find_similar_multi must return candidates from seeds");
        assert!(results.iter().any(|c| c.title == "Beijinho no Ombro"), "Candidates must include Beijinho no Ombro");
    }
}
