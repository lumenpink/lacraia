use crate::db::{Database, TrackRecord};
use lacraia_shared::{is_title_clean, is_valid_duration_delta};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use thiserror::Error;
use tokio::process::Command as TokioCommand;
use tracing::{info, warn};

#[derive(Debug, Error)]
pub enum ResolverError {
    #[error("No valid candidate matching fuzzy criteria was found")]
    NoMatchingCandidate,
    #[error("IO/Process error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON deserialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Database error: {0}")]
    Db(#[from] crate::db::DbError),
    #[error("Extraction command failed: {0}")]
    ExtractionFailed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackResolution {
    pub track_id: String,
    pub title: String,
    pub artist: String,
    pub duration_sec: u64,
    pub stream_url: String,
    pub cache_path: String,
}

#[derive(Debug, Deserialize)]
pub struct YtDlpSearchResult {
    pub id: String,
    pub title: String,
    pub duration: Option<f64>,
    pub webpage_url: Option<String>,
}

#[derive(Clone)]
pub struct MediaResolver {
    cache_dir: PathBuf,
    db: Database,
    ytdlp_bin: String,
}

impl MediaResolver {
    pub fn new(cache_dir: PathBuf, db: Database) -> Self {
        let ytdlp_bin = std::env::var("YTDLP_BIN")
            .unwrap_or_else(|_| "/home/lumen/.local/bin/yt-dlp".to_string());
        Self {
            cache_dir,
            db,
            ytdlp_bin,
        }
    }

    /// Evaluates whether a search candidate satisfies duration delta (<=15s) and noise filter.
    pub fn evaluate_candidate(
        candidate_title: &str,
        candidate_duration_sec: u64,
        seed_title: &str,
        expected_duration_sec: u64,
    ) -> bool {
        if !is_valid_duration_delta(expected_duration_sec, candidate_duration_sec) {
            return false;
        }
        if !is_title_clean(candidate_title, seed_title) {
            return false;
        }
        true
    }

    /// Resolves and caches a track into /cache/{track_id}.opus
    pub async fn resolve_and_cache(
        &self,
        artist: &str,
        title: &str,
        expected_duration_sec: u64,
    ) -> Result<TrackResolution, ResolverError> {
        let track_id = format!("{}:{}", artist.to_lowercase(), title.to_lowercase());
        let safe_filename = format!(
            "{}.opus",
            track_id.replace(|c: char| !c.is_alphanumeric(), "_")
        );
        let target_path = self.cache_dir.join(&safe_filename);
        let target_path_str = target_path.to_string_lossy().to_string();

        // 1. Check local cache hit in SQLite and filesystem
        if let Ok(Some(existing)) = self.db.get_track(&track_id) {
            if let Some(ref path) = existing.cached_path {
                if Path::new(path).exists() {
                    info!("Cache hit for track '{}' at {}", track_id, path);
                    return Ok(TrackResolution {
                        track_id,
                        title: existing.title,
                        artist: existing.artist,
                        duration_sec: existing.duration_sec,
                        stream_url: format!("/cache/{}", safe_filename),
                        cache_path: path.clone(),
                    });
                }
            }
        }

        // 2. Perform search via yt-dlp JSON dump
        info!(
            "Searching audio candidate for '{} - {}' (expected: {}s)",
            artist, title, expected_duration_sec
        );

        let search_query = format!("ytsearch5:{} - {} audio", artist, title);
        let output = TokioCommand::new(&self.ytdlp_bin)
            .args(&[
                &search_query,
                "--dump-json",
                "--default-search",
                "ytsearch",
                "--no-playlist",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .await?;

        if !output.status.success() {
            warn!("yt-dlp search returned status {:?}", output.status);
        }

        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let mut chosen_candidate: Option<YtDlpSearchResult> = None;

        for line in stdout_str.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Ok(item) = serde_json::from_str::<YtDlpSearchResult>(line) {
                let duration_sec = item.duration.map(|d| d as u64).unwrap_or(0);
                if Self::evaluate_candidate(&item.title, duration_sec, title, expected_duration_sec)
                {
                    info!(
                        "Candidate accepted: '{}' ({}s) [id={}]",
                        item.title, duration_sec, item.id
                    );
                    chosen_candidate = Some(item);
                    break;
                } else {
                    info!(
                        "Candidate rejected: '{}' ({}s)",
                        item.title, duration_sec
                    );
                }
            }
        }

        let candidate = chosen_candidate.ok_or(ResolverError::NoMatchingCandidate)?;

        // 3. Extract and convert to Opus via yt-dlp + ffmpeg
        let video_url = candidate
            .webpage_url
            .unwrap_or_else(|| format!("https://www.youtube.com/watch?v={}", candidate.id));

        info!("Extracting audio from {} to {}", video_url, target_path_str);

        let dl_output = TokioCommand::new(&self.ytdlp_bin)
            .args(&[
                "-x",
                "--audio-format",
                "opus",
                "-o",
                &target_path_str,
                &video_url,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if !dl_output.status.success() {
            let err = String::from_utf8_lossy(&dl_output.stderr);
            return Err(ResolverError::ExtractionFailed(format!(
                "yt-dlp extraction failed: {}",
                err
            )));
        }

        // 4. Update SQLite record with cached_path
        let final_duration_sec = candidate
            .duration
            .map(|d| d as u64)
            .unwrap_or(expected_duration_sec);

        let record = TrackRecord {
            id: track_id.clone(),
            artist: artist.to_string(),
            title: title.to_string(),
            duration_sec: final_duration_sec,
            cached_path: Some(target_path_str.clone()),
            genres: None,
            last_played_at: None,
        };
        self.db.upsert_track(&record)?;
        self.db.mark_cached(&track_id, &target_path_str)?;

        Ok(TrackResolution {
            track_id,
            title: title.to_string(),
            artist: artist.to_string(),
            duration_sec: final_duration_sec,
            stream_url: format!("/cache/{}", safe_filename),
            cache_path: target_path_str,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_candidate_evaluation() {
        assert!(MediaResolver::evaluate_candidate(
            "David Bowie - Rebel Rebel (Official Audio)",
            270,
            "Rebel Rebel",
            274
        ));

        // Rejected: live version
        assert!(!MediaResolver::evaluate_candidate(
            "David Bowie - Rebel Rebel (Live at BBC)",
            270,
            "Rebel Rebel",
            274
        ));

        // Rejected: duration discrepancy > 15s
        assert!(!MediaResolver::evaluate_candidate(
            "David Bowie - Rebel Rebel (Extended)",
            310,
            "Rebel Rebel",
            274
        ));
    }
}
