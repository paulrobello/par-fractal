//! Shared launch-parameter parsing (QA-036).
//!
//! One native-testable home for the quality-level parser and the URL query
//! grammar, previously duplicated between `main.rs` (CLI) and the web entry
//! (`web/src/lib.rs`).

/// Parse a quality level string to LOD level index (0=ultra, 1=high, 2=medium,
/// 3=low). Shared by the CLI (`--quality`) and the web `?quality=` parameter.
pub fn parse_quality_level(s: &str) -> Option<usize> {
    match s.to_lowercase().as_str() {
        "ultra" | "u" => Some(0),
        "high" | "h" => Some(1),
        "medium" | "med" | "m" => Some(2),
        "low" | "l" => Some(3),
        _ => None,
    }
}

/// Parse one query/hash segment (`"quality=high&preset=Mandelbulb"`, with or
/// without a leading `?`/`#`) into `(preset_name, quality_level)`. Keys are
/// case-insensitive; `quality`/`q` set the level, `preset`/`p` the name.
/// Unknown keys are ignored; a key without `=` is skipped.
pub fn parse_url_query(input: &str) -> (Option<String>, Option<usize>) {
    let mut preset_name: Option<String> = None;
    let mut quality_level: Option<usize> = None;

    let query = input.trim_start_matches(['?', '#']);
    for param in query.split('&') {
        if let Some((key, value)) = param.split_once('=') {
            match key.to_lowercase().as_str() {
                "quality" | "q" => {
                    quality_level = parse_quality_level(value);
                }
                "preset" | "p" => {
                    preset_name = Some(value.to_string());
                }
                _ => {}
            }
        }
    }

    (preset_name, quality_level)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_levels_parse_with_aliases() {
        assert_eq!(parse_quality_level("ultra"), Some(0));
        assert_eq!(parse_quality_level("U"), Some(0));
        assert_eq!(parse_quality_level("high"), Some(1));
        assert_eq!(parse_quality_level("h"), Some(1));
        assert_eq!(parse_quality_level("Medium"), Some(2));
        assert_eq!(parse_quality_level("med"), Some(2));
        assert_eq!(parse_quality_level("low"), Some(3));
        assert_eq!(parse_quality_level("l"), Some(3));
        assert_eq!(parse_quality_level("nonsense"), None);
        assert_eq!(parse_quality_level(""), None);
    }

    #[test]
    fn url_query_parses_quality_and_preset() {
        let (preset, quality) = parse_url_query("?quality=high&preset=Mandelbulb");
        assert_eq!(preset.as_deref(), Some("Mandelbulb"));
        assert_eq!(quality, Some(1));

        // Short forms, reversed order, hash prefix.
        let (preset, quality) = parse_url_query("#p=Mandelbrot&q=l");
        assert_eq!(preset.as_deref(), Some("Mandelbrot"));
        assert_eq!(quality, Some(3));

        // Case-insensitive keys.
        let (_, quality) = parse_url_query("QUALITY=ultra");
        assert_eq!(quality, Some(0));
    }

    #[test]
    fn url_query_ignores_unknown_and_malformed() {
        let (preset, quality) = parse_url_query("?foo=1&q=medium&novalue&=x");
        assert_eq!(preset, None);
        assert_eq!(quality, Some(2));

        let (preset, quality) = parse_url_query("?q=notalevel");
        assert_eq!(preset, None);
        assert_eq!(quality, None, "invalid quality value must not set a level");
    }
}
