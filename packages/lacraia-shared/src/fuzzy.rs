/// Maximum acceptable duration discrepancy in seconds
pub const MAX_DURATION_DELTA_SEC: u64 = 15;

/// Checks whether obtained duration deviates by no more than MAX_DURATION_DELTA_SEC from expected.
pub fn is_valid_duration_delta(expected_sec: u64, obtained_sec: u64) -> bool {
    let diff = expected_sec.abs_diff(obtained_sec);
    diff <= MAX_DURATION_DELTA_SEC
}

/// Checks whether the search result title is free of undesirable noise terms.
pub fn is_title_clean(title: &str, seed_title: &str) -> bool {
    let title_lower = title.to_lowercase();
    let seed_lower = seed_title.to_lowercase();

    let noise_terms = [
        "live",
        "ao vivo",
        "en vivo",
        "cover",
        "tribute",
        "10 hours",
        "1 hour",
        "slowed",
        "reverb",
        "parody",
        "paródia",
    ];

    for term in noise_terms {
        // If noise term is in candidate title but NOT in original seed title, reject candidate
        if title_lower.contains(term) && !seed_lower.contains(term) {
            return false;
        }
    }

    // Remix handling: allowed only if seed explicitly contains "remix"
    if title_lower.contains("remix") && !seed_lower.contains("remix") {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duration_delta() {
        assert!(is_valid_duration_delta(200, 210));
        assert!(is_valid_duration_delta(200, 185));
        assert!(is_valid_duration_delta(200, 215));
        assert!(!is_valid_duration_delta(200, 216));
        assert!(!is_valid_duration_delta(200, 180));
    }

    #[test]
    fn test_title_filter() {
        assert!(is_title_clean("David Bowie - Rebel Rebel (Official Audio)", "Rebel Rebel"));
        assert!(!is_title_clean("David Bowie - Rebel Rebel (Live at Wembley)", "Rebel Rebel"));
        assert!(!is_title_clean("Rebel Rebel (Cover by John)", "Rebel Rebel"));
        assert!(!is_title_clean("Rebel Rebel (10 Hours Loop)", "Rebel Rebel"));
        assert!(!is_title_clean("Rebel Rebel Remix 2024", "Rebel Rebel"));
        // Allowed if seed requested remix
        assert!(is_title_clean("Rebel Rebel (Club Remix)", "Rebel Rebel Remix"));
    }
}
