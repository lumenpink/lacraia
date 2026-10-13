use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LyricLine {
    #[serde(rename = "timestampMs")]
    pub timestamp_ms: u64,
    pub text: String,
}

static LRC_REGEX: OnceLock<Regex> = OnceLock::new();

fn get_lrc_regex() -> &'static Regex {
    LRC_REGEX.get_or_init(|| Regex::new(r"\[(\d{2,}):(\d{2})(?:\.(\d{2,3}))?\](.*)").unwrap())
}

/// Parses raw LRC lyrics string into sorted LyricLine entries.
pub fn parse_lrc(raw: &str) -> Vec<LyricLine> {
    let re = get_lrc_regex();
    let mut lines = Vec::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(caps) = re.captures(trimmed) {
            let minutes: u64 = caps[1].parse().unwrap_or(0);
            let seconds: u64 = caps[2].parse().unwrap_or(0);
            let subsec_str = caps.get(3).map(|m| m.as_str()).unwrap_or("0");
            let millis: u64 = if subsec_str.len() == 2 {
                subsec_str.parse::<u64>().unwrap_or(0) * 10
            } else {
                subsec_str.parse::<u64>().unwrap_or(0)
            };

            let timestamp_ms = (minutes * 60 * 1000) + (seconds * 1000) + millis;
            let text = caps
                .get(4)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();

            lines.push(LyricLine { timestamp_ms, text });
        }
    }

    lines.sort_by_key(|l| l.timestamp_ms);
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_lrc() {
        let sample = "\
[00:12.50]Line one text
[00:15.800]Line two text
[01:02.10]Line three in next minute
";
        let parsed = parse_lrc(sample);
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].timestamp_ms, 12500);
        assert_eq!(parsed[0].text, "Line one text");
        assert_eq!(parsed[1].timestamp_ms, 15800);
        assert_eq!(parsed[2].timestamp_ms, 62100);
    }

    #[test]
    fn test_parse_out_of_order() {
        let sample = "\
[00:20.00]Second line
[00:05.00]First line
";
        let parsed = parse_lrc(sample);
        assert_eq!(parsed[0].timestamp_ms, 5000);
        assert_eq!(parsed[0].text, "First line");
        assert_eq!(parsed[1].timestamp_ms, 20000);
        assert_eq!(parsed[1].text, "Second line");
    }
}
