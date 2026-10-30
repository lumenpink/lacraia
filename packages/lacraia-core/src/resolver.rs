use crate::db::{Database, TrackRecord};
use lacraia_shared::{
    is_title_clean, is_title_matching, is_valid_duration_delta, strip_accents, CacheEntry,
    CacheResolutionAttempt,
};
use rustypipe::client::RustyPipe;
use rustypipe::model::{AudioCodec, TrackItem};
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
    rp: RustyPipe,
    offline: bool,
}

impl MediaResolver {
    pub fn new(cache_dir: PathBuf, db: Database) -> Self {
        let ytdlp_bin = std::env::var("YTDLP_BIN").unwrap_or_else(|_| "yt-dlp".to_string());
        let rp = RustyPipe::new();
        let offline = std::env::var("LACRAIA_OFFLINE")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        Self {
            cache_dir,
            db,
            ytdlp_bin,
            rp,
            offline,
        }
    }

    pub fn new_offline(cache_dir: PathBuf, db: Database) -> Self {
        let ytdlp_bin = std::env::var("YTDLP_BIN").unwrap_or_else(|_| "yt-dlp".to_string());
        let rp = RustyPipe::new();
        Self {
            cache_dir,
            db,
            ytdlp_bin,
            rp,
            offline: true,
        }
    }

    pub fn build_ytdlp_search_args(search_query: &str) -> Vec<String> {
        vec![
            search_query.to_string(),
            "--dump-json".to_string(),
            "--default-search".to_string(),
            "ytsearch".to_string(),
            "--no-playlist".to_string(),
        ]
    }

    pub fn build_ytdlp_download_args(target_path_str: &str, video_url: &str) -> Vec<String> {
        vec![
            "-f".to_string(),
            "ba[acodec=opus]/ba/bestaudio".to_string(),
            "-x".to_string(),
            "--audio-format".to_string(),
            "opus".to_string(),
            "-o".to_string(),
            target_path_str.to_string(),
            video_url.to_string(),
        ]
    }

    /// Evaluates whether a search candidate satisfies duration delta (<=15s) and noise filter.
    pub fn evaluate_candidate(
        candidate_title: &str,
        candidate_duration_sec: u64,
        seed_title: &str,
        expected_duration_sec: u64,
    ) -> bool {
        // If expected duration is the generic placeholder (210s), allow standard musical range (30s - 30min)
        if expected_duration_sec == 210 {
            if !(30..=1800).contains(&candidate_duration_sec) {
                return false;
            }
        } else if !is_valid_duration_delta(expected_duration_sec, candidate_duration_sec) {
            return false;
        }
        if !is_title_clean(candidate_title, seed_title) {
            return false;
        }
        if !is_title_matching(candidate_title, seed_title) {
            return false;
        }
        true
    }

    /// Returns ffmpeg transcoding/remuxing arguments based on whether the input audio stream is already Opus.
    pub fn ffmpeg_audio_codec_args(is_opus: bool) -> Vec<&'static str> {
        if is_opus {
            vec!["-c:a", "copy"]
        } else {
            vec!["-c:a", "libopus", "-b:a", "128k"]
        }
    }

    /// Primary resolver attempt using native Rust crate (RustyPipe)
    async fn resolve_via_rustypipe(
        &self,
        artist: &str,
        title: &str,
        expected_duration_sec: u64,
        target_path: &Path,
    ) -> Result<u64, String> {
        let query = format!("{} {}", artist, title);
        info!(
            "MediaResolver (Tier 2): Querying YouTube Music via native RustyPipe for '{}'",
            query
        );

        let search_res = self
            .rp
            .query()
            .music_search::<TrackItem, _>(&query, None)
            .await
            .map_err(|e| format!("RustyPipe music search failed: {}", e))?;

        let mut candidate_id = None;
        let mut candidate_duration = expected_duration_sec;

        for item in search_res.items.items {
            let duration_sec = item.duration.unwrap_or(expected_duration_sec as u32) as u64;
            if Self::evaluate_candidate(&item.name, duration_sec, title, expected_duration_sec) {
                info!(
                    "RustyPipe candidate accepted: '{}' ({}s) [id={}]",
                    item.name, duration_sec, item.id
                );
                candidate_id = Some(item.id);
                candidate_duration = duration_sec;
                break;
            } else {
                info!(
                    "RustyPipe candidate rejected: '{}' ({}s)",
                    item.name, duration_sec
                );
            }
        }

        let video_id = match candidate_id {
            Some(id) => id,
            None => {
                // Secondary check in standard YouTube search
                let yt_search = self
                    .rp
                    .query()
                    .search::<rustypipe::model::VideoItem, _>(&query)
                    .await
                    .map_err(|e| format!("RustyPipe fallback video search failed: {}", e))?;

                let mut found = None;
                for item in yt_search.items.items {
                    let d = item.duration.unwrap_or(expected_duration_sec as u32) as u64;
                    if Self::evaluate_candidate(&item.name, d, title, expected_duration_sec) {
                        found = Some((item.id, d));
                        break;
                    }
                }
                let (id, d) = found.ok_or_else(|| {
                    "No candidate matched fuzzy duration/filter in RustyPipe".to_string()
                })?;
                candidate_duration = d;
                id
            }
        };

        info!(
            "RustyPipe fetching player stream for video ID: {}",
            video_id
        );
        let player = self
            .rp
            .query()
            .player(&video_id)
            .await
            .map_err(|e| format!("RustyPipe player query failed: {}", e))?;

        let audio_stream = player
            .audio_streams
            .iter()
            .find(|s| s.codec == AudioCodec::Opus)
            .or_else(|| player.audio_streams.first())
            .cloned()
            .ok_or_else(|| "No suitable audio stream found in RustyPipe response".to_string())?;

        let stream_url = audio_stream.url;
        let is_opus = audio_stream.codec == AudioCodec::Opus;
        let codec_args = Self::ffmpeg_audio_codec_args(is_opus);
        info!(
            "RustyPipe saving stream to target (is_opus={}, codec_args={:?})",
            is_opus, codec_args
        );

        // Remux or transcode stream to target Opus file using ffmpeg with browser User-Agent and headers
        let mut ffmpeg_cmd = TokioCommand::new("ffmpeg");
        ffmpeg_cmd.args([
            "-y",
            "-headers", "Referer: https://music.youtube.com/\r\nUser-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36\r\n",
            "-user_agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36",
            "-i", &stream_url,
        ]);
        ffmpeg_cmd.args(&codec_args);
        ffmpeg_cmd.arg(target_path.to_str().unwrap());

        let child = ffmpeg_cmd
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn ffmpeg: {}", e))?;

        let output = child
            .wait_with_output()
            .await
            .map_err(|e| format!("ffmpeg execution error: {}", e))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(format!("ffmpeg transcoding error: {}", err));
        }

        info!(
            "RustyPipe: successfully saved and transcoded to {}",
            target_path.display()
        );
        Ok(candidate_duration)
    }

    /// Secondary fallback resolver using yt-dlp CLI
    async fn resolve_via_ytdlp(
        &self,
        artist: &str,
        title: &str,
        expected_duration_sec: u64,
        target_path_str: &str,
    ) -> Result<u64, ResolverError> {
        info!(
            "MediaResolver (Tier 3 Fallback): Searching audio candidate via yt-dlp for '{} - {}'",
            artist, title
        );

        let clean_artist = strip_accents(artist)
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c.is_whitespace() {
                    c
                } else {
                    ' '
                }
            })
            .collect::<String>();
        let clean_title = strip_accents(title)
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c.is_whitespace() {
                    c
                } else {
                    ' '
                }
            })
            .collect::<String>();
        let search_query = format!(
            "ytsearch5:{} {} audio",
            clean_artist.trim(),
            clean_title.trim()
        );
        let search_args = Self::build_ytdlp_search_args(&search_query);
        let output = TokioCommand::new(&self.ytdlp_bin)
            .args(&search_args)
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
                        "yt-dlp candidate accepted: '{}' ({}s) [id={}]",
                        item.title, duration_sec, item.id
                    );
                    chosen_candidate = Some(item);
                    break;
                } else {
                    info!(
                        "yt-dlp candidate rejected: '{}' ({}s)",
                        item.title, duration_sec
                    );
                }
            }
        }

        let candidate = chosen_candidate.ok_or(ResolverError::NoMatchingCandidate)?;

        let video_url = candidate
            .webpage_url
            .unwrap_or_else(|| format!("https://www.youtube.com/watch?v={}", candidate.id));

        info!(
            "Extracting audio via yt-dlp from {} to {}",
            video_url, target_path_str
        );

        let dl_args = Self::build_ytdlp_download_args(target_path_str, &video_url);
        let dl_output = TokioCommand::new(&self.ytdlp_bin)
            .args(&dl_args)
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

        let final_duration_sec = candidate
            .duration
            .map(|d| d as u64)
            .unwrap_or(expected_duration_sec);

        Ok(final_duration_sec)
    }

    /// Resolves and caches a track into /cache/{track_id}.opus with tier 1 (cache), tier 2 (rustypipe) and tier 3 (yt-dlp)
    pub async fn resolve_and_cache(
        &self,
        artist: &str,
        title: &str,
        expected_duration_sec: u64,
    ) -> Result<TrackResolution, ResolverError> {
        let clean_artist = strip_accents(artist).to_lowercase();
        let clean_title = strip_accents(title).to_lowercase();
        let track_id = format!("{}:{}", clean_artist.trim(), clean_title.trim());
        let raw_track_id = format!(
            "{}:{}",
            artist.to_lowercase().trim(),
            title.to_lowercase().trim()
        );
        let safe_filename = format!(
            "{}.opus",
            track_id.replace(|c: char| !c.is_alphanumeric(), "_")
        );
        let target_path = self.cache_dir.join(&safe_filename);
        let target_path_str = target_path.to_string_lossy().to_string();

        // 1. Check if DB already knows a cached_path for this track (either track_id or raw_track_id)
        let db_existing = self
            .db
            .get_track(&track_id)
            .ok()
            .flatten()
            .or_else(|| self.db.get_track(&raw_track_id).ok().flatten());

        if let Some(ref existing) = db_existing {
            if let Some(ref path) = existing.cached_path {
                let p = Path::new(path);
                let hit_path = if p.exists() {
                    Some(p.to_path_buf())
                } else if let Some(fname) = p.file_name() {
                    let in_cache = self.cache_dir.join(fname);
                    if in_cache.exists() {
                        Some(in_cache)
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(hit) = hit_path {
                    let hit_file = hit
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(&safe_filename)
                        .to_string();
                    let hit_path_str = hit.to_string_lossy().to_string();
                    info!(
                        "Tier 0 Cache hit for track '{}' from DB record at {}",
                        track_id, hit_path_str
                    );
                    let file_size_bytes = std::fs::metadata(&hit).map(|m| m.len()).unwrap_or(0);
                    let now = chrono::Utc::now().timestamp();
                    let _ = self.db.upsert_cache_entry(&CacheEntry {
                        id: 0,
                        track_id: existing.id.clone(),
                        file_path: hit_path_str.clone(),
                        file_size_bytes,
                        duration_sec: existing.duration_sec,
                        audio_codec: "opus".to_string(),
                        hit_count: 1,
                        last_served_at: now,
                        created_at: now,
                    });
                    let _ = self.db.record_resolution_attempt(&CacheResolutionAttempt {
                        id: 0,
                        track_id: existing.id.clone(),
                        attempt_number: 1,
                        tier: "tier_0_local".to_string(),
                        method: "db_cache".to_string(),
                        command_line: None,
                        success: true,
                        exit_code: Some(0),
                        error_code: None,
                        error_output: None,
                        duration_ms: 0,
                        bytes_received: file_size_bytes,
                        validation_status: "valid".to_string(),
                        cabin_id: None,
                        created_at: now,
                    });
                    return Ok(TrackResolution {
                        track_id: existing.id.clone(),
                        title: existing.title.clone(),
                        artist: existing.artist.clone(),
                        duration_sec: existing.duration_sec,
                        stream_url: format!("/cache/{}", hit_file),
                        cache_path: hit_path_str,
                    });
                }
            }
        }

        // 2. Check local cache hit in filesystem
        let found_path = if target_path.exists() {
            Some((safe_filename.clone(), target_path.clone()))
        } else {
            let lower_target = safe_filename.to_lowercase();
            let alt_path = self.cache_dir.join(&lower_target);
            if alt_path.exists() {
                Some((lower_target, alt_path))
            } else {
                // Fuzzy check in cache_dir: ignore accents & case in filenames
                let mut matched = None;
                if let Ok(entries) = std::fs::read_dir(&self.cache_dir) {
                    for entry in entries.flatten() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if strip_accents(&name).to_lowercase() == safe_filename {
                            matched = Some((name, entry.path()));
                            break;
                        }
                    }
                }
                matched
            }
        };

        if let Some((hit_file, hit_path)) = found_path {
            let hit_path_str = hit_path.to_string_lossy().to_string();
            info!(
                "Tier 0 Filesystem Cache hit for track '{}' at {}",
                track_id, hit_path_str
            );
            let (tit, art, dur) = if let Some(existing) = db_existing {
                (existing.title, existing.artist, existing.duration_sec)
            } else {
                (title.to_string(), artist.to_string(), expected_duration_sec)
            };
            let _ = self.db.mark_cached(&track_id, &hit_file);
            let _ = self.db.mark_cached(&raw_track_id, &hit_file);
            let file_size_bytes = std::fs::metadata(&hit_path).map(|m| m.len()).unwrap_or(0);
            let now = chrono::Utc::now().timestamp();
            let _ = self.db.upsert_cache_entry(&CacheEntry {
                id: 0,
                track_id: track_id.clone(),
                file_path: hit_path_str.clone(),
                file_size_bytes,
                duration_sec: dur,
                audio_codec: "opus".to_string(),
                hit_count: 1,
                last_served_at: now,
                created_at: now,
            });
            let _ = self.db.record_resolution_attempt(&CacheResolutionAttempt {
                id: 0,
                track_id: track_id.clone(),
                attempt_number: 1,
                tier: "tier_0_local".to_string(),
                method: "filesystem_cache".to_string(),
                command_line: None,
                success: true,
                exit_code: Some(0),
                error_code: None,
                error_output: None,
                duration_ms: 0,
                bytes_received: file_size_bytes,
                validation_status: "valid".to_string(),
                cabin_id: None,
                created_at: now,
            });
            return Ok(TrackResolution {
                track_id,
                title: tit,
                artist: art,
                duration_sec: dur,
                stream_url: format!("/cache/{}", hit_file),
                cache_path: hit_path_str,
            });
        }

        if self.offline {
            return Err(ResolverError::ExtractionFailed(
                "offline mode: network resolution disabled".to_string(),
            ));
        }

        // Tier 1: Try native Rust crate (RustyPipe) - isolated in task to catch third-party unwrap panics
        let mut attempt_num = 1;
        let t0 = std::time::Instant::now();
        let self_clone = self.clone();
        let artist_s = artist.to_string();
        let title_s = title.to_string();
        let target_path_buf = target_path.to_path_buf();
        let rp_handle = tokio::spawn(async move {
            self_clone
                .resolve_via_rustypipe(&artist_s, &title_s, expected_duration_sec, &target_path_buf)
                .await
        });
        let rp_res = match rp_handle.await {
            Ok(res) => res,
            Err(join_err) => Err(format!("RustyPipe task panicked or failed: {}", join_err)),
        };
        let rp_duration_ms = t0.elapsed().as_millis() as u64;

        let resolved_duration = match rp_res {
            Ok(duration) => {
                let file_size_bytes = std::fs::metadata(&target_path)
                    .map(|m| m.len())
                    .unwrap_or(0);
                let now = chrono::Utc::now().timestamp();
                let _ = self.db.record_resolution_attempt(&CacheResolutionAttempt {
                    id: 0,
                    track_id: track_id.clone(),
                    attempt_number: attempt_num,
                    tier: "tier_1_direct".to_string(),
                    method: "rustypipe_music_search".to_string(),
                    command_line: None,
                    success: true,
                    exit_code: Some(0),
                    error_code: None,
                    error_output: None,
                    duration_ms: rp_duration_ms,
                    bytes_received: file_size_bytes,
                    validation_status: "valid".to_string(),
                    cabin_id: None,
                    created_at: now,
                });
                duration
            }
            Err(rp_err) => {
                let now = chrono::Utc::now().timestamp();
                let _ = self.db.record_resolution_attempt(&CacheResolutionAttempt {
                    id: 0,
                    track_id: track_id.clone(),
                    attempt_number: attempt_num,
                    tier: "tier_1_direct".to_string(),
                    method: "rustypipe_music_search".to_string(),
                    command_line: None,
                    success: false,
                    exit_code: Some(1),
                    error_code: Some("RUSTYPIPE_FAILED".to_string()),
                    error_output: Some(rp_err.clone()),
                    duration_ms: rp_duration_ms,
                    bytes_received: 0,
                    validation_status: "network_error".to_string(),
                    cabin_id: None,
                    created_at: now,
                });
                attempt_num += 1;

                warn!(
                    "MediaResolver: Tier 1 (RustyPipe) failed: {}. Activating Tier 3 (yt-dlp fallback)...",
                    rp_err
                );
                let t1 = std::time::Instant::now();
                let ytdlp_res = self
                    .resolve_via_ytdlp(artist, title, expected_duration_sec, &target_path_str)
                    .await;
                let ytdlp_duration_ms = t1.elapsed().as_millis() as u64;
                let now = chrono::Utc::now().timestamp();

                match ytdlp_res {
                    Ok(dur) => {
                        let file_size_bytes = std::fs::metadata(&target_path)
                            .map(|m| m.len())
                            .unwrap_or(0);
                        let _ = self.db.record_resolution_attempt(&CacheResolutionAttempt {
                            id: 0,
                            track_id: track_id.clone(),
                            attempt_number: attempt_num,
                            tier: "tier_3_ytdlp_android".to_string(),
                            method: "ytdlp_android_client".to_string(),
                            command_line: Some(format!("{} ytsearch5:...", self.ytdlp_bin)),
                            success: true,
                            exit_code: Some(0),
                            error_code: None,
                            error_output: None,
                            duration_ms: ytdlp_duration_ms,
                            bytes_received: file_size_bytes,
                            validation_status: "valid".to_string(),
                            cabin_id: None,
                            created_at: now,
                        });
                        dur
                    }
                    Err(e) => {
                        let err_msg = e.to_string();
                        let _ = self.db.record_resolution_attempt(&CacheResolutionAttempt {
                            id: 0,
                            track_id: track_id.clone(),
                            attempt_number: attempt_num,
                            tier: "tier_3_ytdlp_android".to_string(),
                            method: "ytdlp_android_client".to_string(),
                            command_line: Some(format!("{} ytsearch5:...", self.ytdlp_bin)),
                            success: false,
                            exit_code: Some(1),
                            error_code: Some("YTDLP_FAILED".to_string()),
                            error_output: Some(err_msg),
                            duration_ms: ytdlp_duration_ms,
                            bytes_received: 0,
                            validation_status: "empty_file".to_string(),
                            cabin_id: None,
                            created_at: now,
                        });
                        return Err(e);
                    }
                }
            }
        };

        // Update SQLite record with cached_path
        let record = TrackRecord {
            id: track_id.clone(),
            artist: artist.to_string(),
            title: title.to_string(),
            duration_sec: resolved_duration,
            cached_path: Some(target_path_str.clone()),
            genres: None,
            last_played_at: None,
        };
        self.db.upsert_track(&record)?;
        self.db.mark_cached(&track_id, &target_path_str)?;

        let file_size_bytes = std::fs::metadata(&target_path)
            .map(|m| m.len())
            .unwrap_or(0);
        let now = chrono::Utc::now().timestamp();
        let _ = self.db.upsert_cache_entry(&CacheEntry {
            id: 0,
            track_id: track_id.clone(),
            file_path: target_path_str.clone(),
            file_size_bytes,
            duration_sec: resolved_duration,
            audio_codec: "opus".to_string(),
            hit_count: 1,
            last_served_at: now,
            created_at: now,
        });

        Ok(TrackResolution {
            track_id,
            title: title.to_string(),
            artist: artist.to_string(),
            duration_sec: resolved_duration,
            stream_url: format!("/cache/{}", safe_filename),
            cache_path: target_path_str,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ytdlp_args_resilience_and_omits_android_client() {
        let search_args =
            MediaResolver::build_ytdlp_search_args("ytsearch5:chico buarque construcao audio");
        assert!(
            !search_args
                .iter()
                .any(|arg| arg.contains("player_client=android")),
            "Search args must not contain player_client=android to prevent SABR extraction failure"
        );

        let dl_args = MediaResolver::build_ytdlp_download_args(
            "/tmp/out.opus",
            "https://youtube.com/watch?v=123",
        );
        assert!(
            !dl_args.iter().any(|arg| arg.contains("player_client=android")),
            "Download args must not contain player_client=android to prevent SABR extraction failure"
        );
    }

    #[tokio::test]
    async fn test_media_resolver_offline_mode_fails_fast_without_network() {
        let tmp_dir = std::env::temp_dir().join(format!("lacraia_off_{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&tmp_dir);
        let db = Database::open_in_memory().unwrap();

        let resolver = MediaResolver::new_offline(tmp_dir.clone(), db.clone());
        let res = resolver
            .resolve_and_cache("Ghost Artist", "Ghost Title", 180)
            .await;
        assert!(
            res.is_err(),
            "Offline resolver must return error on uncached tracks"
        );
        match res.unwrap_err() {
            ResolverError::ExtractionFailed(msg) => {
                assert!(msg.contains("offline"), "Error must indicate offline mode");
            }
            other => panic!("Expected ResolverError::ExtractionFailed, got: {:?}", other),
        }
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

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

        // Rejected: false positive with unrelated title
        assert!(!MediaResolver::evaluate_candidate(
            "RIO", 210, "lacraia", 210
        ));

        // Accepted: fuzzy matching with diacritics
        assert!(MediaResolver::evaluate_candidate(
            "Construção | Chico Buarque (Vídeo Oficial)",
            258,
            "Construcao",
            258
        ));
    }

    #[tokio::test]
    async fn test_media_resolver_attempts_logging_and_cache_inventory() {
        let tmp_dir = std::env::temp_dir().join(format!(
            "lacraia_test_cache_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp_dir).unwrap();

        let db = Database::open_in_memory().unwrap();
        let resolver = MediaResolver::new(tmp_dir.clone(), db.clone());

        // 1. Simulate an existing file in cache (Tier 0 local cache hit)
        let safe_filename = "chico_buarque_construcao.opus";
        let file_path = tmp_dir.join(safe_filename);
        std::fs::write(&file_path, b"FAKE_OPUS_AUDIO_BYTES_TEST").unwrap();

        let res = resolver
            .resolve_and_cache("Chico Buarque", "Construção", 258)
            .await
            .unwrap();
        assert_eq!(res.track_id, "chico buarque:construcao");

        // Must create/update cache_entries
        let entry = db
            .get_cache_entry("chico buarque:construcao")
            .unwrap()
            .expect("Cache entry must be recorded in DB");
        assert_eq!(entry.file_path, file_path.to_string_lossy().to_string());
        assert_eq!(entry.file_size_bytes, 26);
        assert!(entry.hit_count >= 1);

        // Must record a Tier 0 success attempt
        let attempts = db
            .get_resolution_attempts_for_track("chico buarque:construcao")
            .unwrap();
        assert!(
            !attempts.is_empty(),
            "Resolution attempts must record Tier 0 local cache hit"
        );
        assert!(attempts
            .iter()
            .any(|a| a.tier == "tier_0_local" && a.success));

        // Cleanup
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_ffmpeg_audio_codec_args_fast_remux() {
        assert_eq!(
            MediaResolver::ffmpeg_audio_codec_args(true),
            vec!["-c:a", "copy"]
        );
        assert_eq!(
            MediaResolver::ffmpeg_audio_codec_args(false),
            vec!["-c:a", "libopus", "-b:a", "128k"]
        );
    }

    #[tokio::test]
    async fn test_ffmpeg_direct_stream_copy_execution() {
        let tmp_dir = std::env::temp_dir().join(format!(
            "lacraia_remux_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp_dir).unwrap();

        let src_file = tmp_dir.join("src.opus");
        let dst_file = tmp_dir.join("dst.opus");

        let gen = TokioCommand::new("ffmpeg")
            .args([
                "-y",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=0.2",
                "-c:a",
                "libopus",
                src_file.to_str().unwrap(),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;
        if let Ok(st) = gen {
            if st.success() {
                let codec_args = MediaResolver::ffmpeg_audio_codec_args(true);
                let mut cmd = TokioCommand::new("ffmpeg");
                cmd.args(["-y", "-i", src_file.to_str().unwrap()]);
                cmd.args(&codec_args);
                cmd.arg(dst_file.to_str().unwrap());
                let res = cmd.status().await.unwrap();
                assert!(res.success());
                assert!(dst_file.exists());
                assert!(std::fs::metadata(&dst_file).unwrap().len() > 0);
            }
        }

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }
}
