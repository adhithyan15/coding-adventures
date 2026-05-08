//! Glob matcher for capability targets.
//!
//! The matching rules (from `capability-cage-rust.md` and the Go
//! conformance suite):
//!
//! ```text
//! Pattern              Matches
//! foo                  exactly `foo`
//! *                    any single path component (no separators)
//! **                   any number of components
//! *.tokens             any single component ending in `.tokens`
//! ./grammars/*.tokens  one-level files under `./grammars/`
//! ./grammars/**/*.tokens  any-depth `.tokens` under `./grammars/`
//! host:port            literal host and port
//! *:443                any host on port 443
//! api.weather.gov:*    any port on `api.weather.gov`
//! ```
//!
//! Path separators are `/` (we normalize Windows backslashes before
//! matching). Net targets use `:` as the separator between host and
//! port.

const PATH_SEP: char = '/';

/// Returns true if `candidate` (a literal call argument) matches
/// `pattern` (a target string from the manifest).
pub fn match_target(pattern: &str, candidate: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    // Net targets use `:` to separate host and port; if both pattern
    // and candidate look like net targets, match each side
    // independently.
    if let (Some(p_idx), Some(c_idx)) = (pattern.rfind(':'), candidate.rfind(':')) {
        let (p_host, p_port) = (&pattern[..p_idx], &pattern[p_idx + 1..]);
        let (c_host, c_port) = (&candidate[..c_idx], &candidate[c_idx + 1..]);
        // Heuristic: if neither side contains `/`, treat as net
        // target. Path globs never contain `:` between segments
        // (Windows drive letters would, but our paths use `./` or
        // start with `/`).
        let looks_like_net =
            !pattern.contains('/') && !pattern.contains('\\') && !p_port.contains(' ');
        if looks_like_net {
            return match_host(p_host, c_host) && match_port(p_port, c_port);
        }
    }

    // Otherwise treat as path match.
    let pattern = normalize_separators(pattern);
    let candidate = normalize_separators(candidate);
    match_path(&pattern, &candidate)
}

fn normalize_separators(s: &str) -> String {
    s.replace('\\', "/")
}

fn match_host(pattern: &str, candidate: &str) -> bool {
    pattern == "*" || pattern == candidate
}

fn match_port(pattern: &str, candidate: &str) -> bool {
    pattern == "*" || pattern == candidate
}

fn match_path(pattern: &str, candidate: &str) -> bool {
    let p_segs: Vec<&str> = pattern.split(PATH_SEP).collect();
    let c_segs: Vec<&str> = candidate.split(PATH_SEP).collect();
    match_segments(&p_segs, &c_segs)
}

fn match_segments(p: &[&str], c: &[&str]) -> bool {
    match (p.first(), c.first()) {
        (None, None) => true,
        (None, Some(_)) => false,
        (Some(&"**"), _) => {
            // ** matches zero or more components.
            // Try matching the rest of the pattern against
            // increasing suffixes of the candidate.
            let rest_pattern = &p[1..];
            let mut idx = 0;
            loop {
                if match_segments(rest_pattern, &c[idx..]) {
                    return true;
                }
                if idx >= c.len() {
                    return false;
                }
                idx += 1;
            }
        }
        (Some(p_seg), Some(c_seg)) => {
            if !match_one_segment(p_seg, c_seg) {
                return false;
            }
            match_segments(&p[1..], &c[1..])
        }
        (Some(_), None) => false,
    }
}

/// Match a single path component (no `/` allowed in either side).
/// Supports `*` wildcard within the component.
fn match_one_segment(pattern: &str, candidate: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if !pattern.contains('*') {
        return pattern == candidate;
    }
    // Pattern with `*` chars: split into literal pieces and match
    // sequentially. e.g. "*.tokens" → ["", ".tokens"]; "foo*.bar" →
    // ["foo", ".bar"].
    let pieces: Vec<&str> = pattern.split('*').collect();
    let mut cursor = 0;
    for (i, piece) in pieces.iter().enumerate() {
        if piece.is_empty() {
            continue;
        }
        if i == 0 {
            // First piece must be a prefix.
            if !candidate[cursor..].starts_with(piece) {
                return false;
            }
            cursor += piece.len();
        } else if i == pieces.len() - 1 {
            // Last piece must be a suffix at end-of-string.
            if !candidate[cursor..].ends_with(piece) {
                return false;
            }
            // Ensure suffix starts no earlier than current cursor.
            if candidate.len() < cursor + piece.len() {
                return false;
            }
        } else {
            // Middle piece: find anywhere from current cursor.
            match candidate[cursor..].find(piece) {
                Some(idx) => cursor += idx + piece.len(),
                None => return false,
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn star_matches_everything() {
        assert!(match_target("*", "anything"));
        assert!(match_target("*", "/etc/passwd"));
        assert!(match_target("*", "api.weather.gov:443"));
    }

    #[test]
    fn literal_match() {
        assert!(match_target("foo", "foo"));
        assert!(!match_target("foo", "bar"));
        assert!(!match_target("foo", "foobar"));
    }

    #[test]
    fn single_star_one_component() {
        assert!(match_target("./grammars/*", "./grammars/json.tokens"));
        assert!(!match_target(
            "./grammars/*",
            "./grammars/sub/json.tokens"
        ));
    }

    #[test]
    fn double_star_any_depth() {
        assert!(match_target("./**/*.tokens", "./grammars/json.tokens"));
        assert!(match_target(
            "./**/*.tokens",
            "./grammars/sub/deep/json.tokens"
        ));
        assert!(!match_target("./**/*.tokens", "./grammars/json.json"));
    }

    #[test]
    fn extension_glob_within_segment() {
        assert!(match_target("*.tokens", "json.tokens"));
        assert!(!match_target("*.tokens", "json.toml"));
        assert!(match_target("./grammars/*.tokens", "./grammars/json.tokens"));
    }

    #[test]
    fn middle_wildcard() {
        assert!(match_target("foo*.bar", "fooXY.bar"));
        assert!(match_target("foo*.bar", "foo.bar"));
        assert!(!match_target("foo*.bar", "foo.baz"));
    }

    #[test]
    fn net_target_match() {
        assert!(match_target("api.weather.gov:443", "api.weather.gov:443"));
        assert!(!match_target("api.weather.gov:443", "api.weather.gov:80"));
        assert!(!match_target(
            "api.weather.gov:443",
            "evil.example.com:443"
        ));
    }

    #[test]
    fn net_wildcard_port() {
        assert!(match_target("api.weather.gov:*", "api.weather.gov:443"));
        assert!(match_target("api.weather.gov:*", "api.weather.gov:80"));
        assert!(!match_target("api.weather.gov:*", "evil.example.com:443"));
    }

    #[test]
    fn net_wildcard_host() {
        assert!(match_target("*:443", "api.weather.gov:443"));
        assert!(match_target("*:443", "evil.example.com:443"));
        assert!(!match_target("*:443", "api.weather.gov:80"));
    }

    #[test]
    fn windows_paths_normalize() {
        assert!(match_target(
            "./grammars/*.tokens",
            ".\\grammars\\json.tokens"
        ));
    }
}
