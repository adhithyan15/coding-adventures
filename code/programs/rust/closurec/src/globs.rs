//! `globs` — minimal hand-rolled glob expansion for `--js`.
//!
//! # Why hand-rolled
//!
//! The repo's working principle is **zero-dep where reasonable**.
//! Glob matching is a well-bounded problem — a few hundred lines of
//! careful code beats pulling in `glob` + its transitive deps. The
//! upstream Java Closure Compiler likewise rolls its own (see
//! `CommandLineRunner.findJsFiles`).
//!
//! # What's supported
//!
//! Exactly the glob features the Closure CLI honors:
//!
//! - `*` — matches any sequence of characters within a *single*
//!   path segment (does NOT cross `/`).
//! - `**` — matches across any number of path segments (zero or
//!   more directories), but ONLY as a complete segment.
//!   `src/**/*.js` works; `src/**.js` is treated literally per
//!   CC's documented behavior.
//! - `?` — matches exactly one character within a segment.
//! - `[abc]` / `[a-z]` — character classes. `[!abc]` negates.
//! - literal text — must match byte-for-byte.
//!
//! All other special characters (`{,}`, brace expansion;
//! parentheses for capture; etc.) are treated as literal text.
//!
//! # Exclusion semantics — the `!` prefix
//!
//! When a `--js` value starts with `!`, it's an **exclusion
//! pattern**. After the inclusion patterns have produced the
//! candidate set (in the order they appeared on the command
//! line), each exclusion removes everything it matches. So:
//!
//! ```text
//! --js 'src/**/*.js' --js '!src/legacy/**' --js 'tests/foo.js'
//! ```
//!
//! includes every JS file under `src/`, then removes any under
//! `src/legacy/`, then appends `tests/foo.js` to the result. This
//! matches Closure's behavior.
//!
//! # Walk strategy
//!
//! For each inclusion pattern we identify the longest "fixed"
//! (glob-free) prefix and walk the filesystem under that prefix
//! only. So `src/components/**/*.js` walks `src/components/`,
//! not the entire CWD — same optimisation the upstream Java tool
//! makes. Filesystem walks honor Unicode path components but
//! treat the glob alphabet as ASCII (matches CC).
//!
//! # Determinism
//!
//! Directory entries are sorted lexicographically before
//! recursion, so glob expansion is deterministic across runs and
//! filesystems with different `readdir` ordering.

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Things that can go wrong while expanding `--js` patterns.
///
/// Closure-compatible behavior: a pattern matching zero files is
/// an error (CC's `--js src/**/*.js` over an empty dir errors with
/// "JSC_NO_JS_FILES_FOUND_FOR_PATTERN"). An invalid pattern (e.g.
/// unbalanced `[`) is also an error.
#[derive(Debug, Clone, PartialEq)]
pub enum GlobError {
    /// A pattern produced zero matches.
    NoMatches(String),
    /// A pattern was syntactically invalid.
    InvalidPattern { pattern: String, reason: String },
    /// Filesystem walk failed under a pattern's fixed prefix.
    WalkError {
        pattern: String,
        path: PathBuf,
        kind: io::ErrorKind,
        message: String,
    },
}

impl std::fmt::Display for GlobError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GlobError::NoMatches(p) => write!(f, "--js {p:?}: no JS files matched"),
            GlobError::InvalidPattern { pattern, reason } => {
                write!(f, "--js {pattern:?}: invalid glob pattern ({reason})")
            }
            GlobError::WalkError { pattern, path, message, .. } => {
                write!(
                    f,
                    "--js {pattern:?}: failed to read {}: {message}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for GlobError {}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Expand a list of `--js` patterns into the concrete list of
/// files to compile. Patterns are processed in order; each
/// inclusion pattern appends to the result set, each exclusion
/// pattern (leading `!`) removes from it.
///
/// Order within the result is the same as Closure's: inclusions
/// in the order they appeared, with each inclusion's matches in
/// lexicographic path order. Exclusions don't re-order — they
/// just remove.
///
/// # Closure parity
///
/// - Zero matches across all patterns is an error.
/// - A literal (glob-free) inclusion that doesn't exist is also
///   an error (CC treats `--js foo.js` as "I expect this file"
///   not "match if present").
/// - Duplicate matches (same path matched by two patterns) are
///   deduplicated, preserving the first occurrence. Closure
///   logs a warning here; v1 silently dedupes — we can add the
///   warning when the diagnostics pipeline lands (CLOC11 Track 5).
pub fn expand_js_patterns(patterns: &[String]) -> Result<Vec<PathBuf>, GlobError> {
    let mut included: Vec<PathBuf> = Vec::new();

    for raw in patterns {
        if let Some(excl) = raw.strip_prefix('!') {
            // Exclusion: remove everything that matches.
            let matcher = compile_pattern(excl).map_err(|reason| GlobError::InvalidPattern {
                pattern: raw.clone(),
                reason,
            })?;
            included.retain(|p| !matches_path(&matcher, p));
        } else {
            // Inclusion.
            let matches = expand_single(raw)?;
            for path in matches {
                if !included.iter().any(|p| p == &path) {
                    included.push(path);
                }
            }
        }
    }

    if included.is_empty() {
        // Closure errors with NO_JS_FILES_FOUND_FOR_PATTERN listing
        // the offending pattern. For a multi-pattern command line
        // we report the *first inclusion* — that's what
        // CommandLineRunner.java does too.
        let first_inclusion = patterns
            .iter()
            .find(|p| !p.starts_with('!'))
            .cloned()
            .unwrap_or_default();
        return Err(GlobError::NoMatches(first_inclusion));
    }

    Ok(included)
}

/// Expand a single inclusion pattern.
///
/// Two paths through:
///
/// - **No glob characters.** Treat as a literal file path. Must
///   exist (CC behavior); returns the path as a one-element vec.
/// - **Glob characters present.** Find the longest fixed prefix,
///   walk under it, match each candidate against the full pattern.
fn expand_single(pattern: &str) -> Result<Vec<PathBuf>, GlobError> {
    if !has_glob_chars(pattern) {
        let path = PathBuf::from(pattern);
        if !path.exists() {
            return Err(GlobError::NoMatches(pattern.to_string()));
        }
        return Ok(vec![path]);
    }

    let matcher = compile_pattern(pattern).map_err(|reason| GlobError::InvalidPattern {
        pattern: pattern.to_string(),
        reason,
    })?;

    let (fixed_prefix, _rest) = split_fixed_prefix(pattern);
    let start_dir = if fixed_prefix.is_empty() {
        PathBuf::from(".")
    } else {
        PathBuf::from(&fixed_prefix)
    };

    let mut matches = Vec::new();
    walk_and_match(&start_dir, &matcher, pattern, &mut matches)?;
    matches.sort();

    if matches.is_empty() {
        return Err(GlobError::NoMatches(pattern.to_string()));
    }
    Ok(matches)
}

fn has_glob_chars(s: &str) -> bool {
    s.contains('*') || s.contains('?') || s.contains('[')
}

/// Split a pattern into (longest_literal_prefix, remainder). The
/// literal prefix is the part up to but not including the first
/// path segment that contains a glob character.
///
/// Absolute paths are preserved as-is: `/var/x/*.js` splits to
/// `("/var/x", "*.js")`, not `("var/x", "*.js")`. Without this,
/// the walker would start at the wrong directory.
fn split_fixed_prefix(pattern: &str) -> (String, String) {
    // Track absolute vs relative explicitly so we don't lose the
    // leading `/` when stripping segments off for the recurse.
    let (mut prefix, rest) = if let Some(stripped) = pattern.strip_prefix('/') {
        ("/".to_string(), stripped)
    } else {
        (String::new(), pattern)
    };
    let mut iter = rest.split('/').peekable();
    // `first_seg` controls whether we need to insert a `/` between
    // segments we already appended. It starts true (no segments
    // appended yet). After we push the first non-glob segment, we
    // flip to false and subsequent segments get a separator.
    let mut first_seg = true;
    while let Some(seg) = iter.peek() {
        if has_glob_chars(seg) {
            break;
        }
        // Only push a separator if `prefix` doesn't already end
        // in one (which it does for absolute paths after the
        // initial `/`).
        if !first_seg && !prefix.ends_with('/') {
            prefix.push('/');
        }
        prefix.push_str(seg);
        first_seg = false;
        iter.next();
    }
    let rest_parts: Vec<&str> = iter.collect();
    (prefix, rest_parts.join("/"))
}

/// Maximum directory depth we'll recurse into during glob
/// expansion. Defends against symlink loops and ridiculous
/// real-world trees alike. 64 is well past anything sensible
/// — node_modules clocks in at ~20 in the wild — so legitimate
/// invocations never bump it.
const MAX_WALK_DEPTH: usize = 64;

fn walk_and_match(
    dir: &Path,
    matcher: &CompiledPattern,
    pattern: &str,
    out: &mut Vec<PathBuf>,
) -> Result<(), GlobError> {
    walk_and_match_inner(dir, matcher, pattern, out, 0)
}

fn walk_and_match_inner(
    dir: &Path,
    matcher: &CompiledPattern,
    pattern: &str,
    out: &mut Vec<PathBuf>,
    depth: usize,
) -> Result<(), GlobError> {
    if depth > MAX_WALK_DEPTH {
        // Defensive: cap recursion. Almost certainly a symlink
        // loop or a maliciously deep tree. We stop walking but
        // don't fail the whole expansion — caller's matches so
        // far stay valid.
        return Ok(());
    }
    if !dir.exists() {
        return Ok(());
    }
    // Use symlink_metadata to check what `dir` itself is without
    // following symlinks. Following a symlink-to-dir at the top
    // of a walk is fine (the user pointed --js there explicitly);
    // it's the entries we encounter mid-walk that we don't follow.
    if dir.is_file() {
        if matches_path(matcher, dir) {
            out.push(dir.to_path_buf());
        }
        return Ok(());
    }
    let mut entries: Vec<_> = fs::read_dir(dir)
        .map_err(|e| GlobError::WalkError {
            pattern: pattern.to_string(),
            path: dir.to_path_buf(),
            kind: e.kind(),
            message: e.to_string(),
        })?
        .filter_map(|r| r.ok())
        .collect();
    entries.sort_by_key(|e| e.path());
    for entry in entries {
        // Use `file_type` rather than `path.is_dir()` so we don't
        // follow symlinks mid-walk. A symlink loop
        // (`a/loop -> ..`) would otherwise blow the stack;
        // benign-but-noisy symlinks would silently double-count
        // files. Either way, we skip them.
        let ft = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        if ft.is_symlink() {
            continue;
        }
        let path = entry.path();
        if ft.is_dir() {
            walk_and_match_inner(&path, matcher, pattern, out, depth + 1)?;
        } else if ft.is_file() && matches_path(matcher, &path) {
            out.push(path);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Pattern compilation + matching
// ---------------------------------------------------------------------------
//
// A compiled pattern is a list of per-segment matchers plus a
// flag for trailing `**` (which matches "this segment and any
// descendant"). Glob matching against a path is then:
//
//   1. Normalise the candidate path into segments.
//   2. Walk the segment-matchers and path-segments in lockstep,
//      with `**` consuming zero or more.
//
// Within a segment we compile to a small Token stream that the
// matcher walks character-by-character with backtracking on `*`.
// Backtracking is fine: paths have bounded length and `*` is
// rare per segment.

/// One token in a single segment's matcher.
#[derive(Debug, Clone, PartialEq)]
enum Token {
    /// Match a literal character.
    Lit(char),
    /// Match exactly one character (`?`).
    AnyChar,
    /// Match any sequence of characters within this segment (`*`).
    AnySeq,
    /// Match one character in a class (`[abc]`, `[a-z]`, `[!abc]`).
    CharClass { negated: bool, ranges: Vec<(char, char)> },
}

#[derive(Debug, Clone, PartialEq)]
enum SegmentMatcher {
    /// Match exactly one path segment against this Token sequence.
    Tokens(Vec<Token>),
    /// `**` — match zero or more *whole* segments.
    DoubleStar,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompiledPattern {
    segments: Vec<SegmentMatcher>,
}

fn compile_pattern(pattern: &str) -> Result<CompiledPattern, String> {
    let mut segments = Vec::new();
    // Skip a single leading empty segment that appears when the
    // pattern is absolute (e.g. `/foo/*.js` → ["", "foo", "*.js"]).
    // `matches_path` only emits `Component::Normal` segments, so
    // an absolute path's components are `["foo", "*.js"]` — we
    // must align with that.
    let parts: Vec<&str> = if let Some(rest) = pattern.strip_prefix('/') {
        rest.split('/').collect()
    } else {
        pattern.split('/').collect()
    };
    for seg in parts {
        if seg == "**" {
            segments.push(SegmentMatcher::DoubleStar);
        } else {
            segments.push(SegmentMatcher::Tokens(compile_segment(seg)?));
        }
    }
    Ok(CompiledPattern { segments })
}

fn compile_segment(seg: &str) -> Result<Vec<Token>, String> {
    // Iterate by char *index* directly instead of computing the
    // current position via `seg.len() - chars.count()`. The latter
    // mixes byte-length and char-count, which panics with a usize
    // underflow on any non-ASCII segment containing `[`. Tracking
    // (byte_idx, char) up front avoids that whole class of bugs.
    let mut tokens = Vec::new();
    let indices: Vec<(usize, char)> = seg.char_indices().collect();
    let mut i = 0;
    while i < indices.len() {
        let (byte_idx, c) = indices[i];
        match c {
            '*' => {
                // Coalesce `**` inside a segment to `*` (per spec,
                // `**` only has special meaning as a full segment).
                let mut j = i + 1;
                while j < indices.len() && indices[j].1 == '*' {
                    j += 1;
                }
                i = j;
                tokens.push(Token::AnySeq);
            }
            '?' => {
                tokens.push(Token::AnyChar);
                i += 1;
            }
            '[' => {
                let (cls, consumed_chars) = compile_char_class(&seg[byte_idx..])?;
                tokens.push(cls);
                i += consumed_chars;
            }
            ch => {
                tokens.push(Token::Lit(ch));
                i += 1;
            }
        }
    }
    Ok(tokens)
}

fn compile_char_class(rest: &str) -> Result<(Token, usize), String> {
    // `rest` starts with the `[` that opened the class. Returns
    // the compiled token and the number of *chars* consumed
    // (including the closing `]`), so the caller can advance its
    // char-index correctly even when `rest` contains multi-byte
    // characters.
    let bytes: Vec<char> = rest.chars().collect();
    if bytes.first() != Some(&'[') {
        return Err("char class compile called without leading '['".into());
    }
    let mut i = 1;
    let negated = if bytes.get(i) == Some(&'!') {
        i += 1;
        true
    } else {
        false
    };
    let mut ranges: Vec<(char, char)> = Vec::new();
    let mut closed = false;
    while i < bytes.len() {
        let c = bytes[i];
        if c == ']' && !ranges.is_empty() {
            i += 1;
            closed = true;
            break;
        }
        // Range: `a-z` consumes 3 chars.
        if i + 2 < bytes.len() && bytes[i + 1] == '-' && bytes[i + 2] != ']' {
            ranges.push((c, bytes[i + 2]));
            i += 3;
        } else {
            ranges.push((c, c));
            i += 1;
        }
    }
    if !closed {
        return Err(format!("unterminated character class in {:?}", rest));
    }
    Ok((Token::CharClass { negated, ranges }, i))
}

fn matches_path(pattern: &CompiledPattern, path: &Path) -> bool {
    // Only walk the `Normal` segments of the path. RootDir, CurDir,
    // and ParentDir are filesystem decorations — the *content*
    // segments are what we glob against, mirroring how
    // `compile_pattern` strips a leading `/` from absolute
    // patterns to align with this same Normal-only segment list.
    let segments: Vec<String> = path
        .components()
        .filter_map(|c| match c {
            Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();
    match_segments(&pattern.segments, &segments)
}

fn match_segments(matchers: &[SegmentMatcher], path: &[String]) -> bool {
    match (matchers.first(), path.first()) {
        (None, None) => true,
        (None, Some(_)) => false,
        (Some(SegmentMatcher::DoubleStar), _) => {
            // `**` matches zero or more path segments. Try each.
            // Greedy from zero up, returning on first success.
            for k in 0..=path.len() {
                if match_segments(&matchers[1..], &path[k..]) {
                    return true;
                }
            }
            false
        }
        (Some(SegmentMatcher::Tokens(_)), None) => false,
        (Some(SegmentMatcher::Tokens(toks)), Some(seg)) => {
            if !match_segment(toks, seg) {
                return false;
            }
            match_segments(&matchers[1..], &path[1..])
        }
    }
}

fn match_segment(tokens: &[Token], text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    match_tokens(tokens, &chars)
}

/// Iterative glob matcher using Krauss's two-pointer algorithm
/// with `*`-backtracking via saved positions.
///
/// This avoids the catastrophic exponential blowup of the naive
/// recursive backtracker on adversarial patterns like
/// `*a*a*a*...`. Time is `O(n * m)` where `n` is the segment
/// length and `m` is the token count. Memory is `O(1)` aside
/// from the input slices.
///
/// The algorithm:
///   - walk tokens and chars in lockstep.
///   - on a literal / `?` / class mismatch, fall back to the
///     last `*` (if any) and advance the saved char position
///     by one.
///   - on `*`, record a backtrack point at the next token and
///     current char position; advance the token pointer past `*`.
///   - succeed when we've consumed every token AND every char.
fn match_tokens(tokens: &[Token], chars: &[char]) -> bool {
    let mut ti: usize = 0;
    let mut ci: usize = 0;
    let mut star_ti: Option<usize> = None;
    let mut star_ci: usize = 0;

    while ci < chars.len() {
        if ti < tokens.len() {
            match &tokens[ti] {
                Token::AnySeq => {
                    star_ti = Some(ti + 1);
                    star_ci = ci;
                    ti += 1;
                    continue;
                }
                Token::Lit(want) if *want == chars[ci] => {
                    ti += 1;
                    ci += 1;
                    continue;
                }
                Token::AnyChar => {
                    ti += 1;
                    ci += 1;
                    continue;
                }
                Token::CharClass { negated, ranges } => {
                    let in_class = ranges.iter().any(|(lo, hi)| *lo <= chars[ci] && chars[ci] <= *hi);
                    if in_class != *negated {
                        ti += 1;
                        ci += 1;
                        continue;
                    }
                }
                Token::Lit(_) => {}
            }
        }
        // Mismatch or out-of-tokens — backtrack to last `*` if any.
        match star_ti {
            Some(st) => {
                ti = st;
                star_ci += 1;
                ci = star_ci;
            }
            None => return false,
        }
    }

    // Consumed all chars; remaining tokens must all be `*`.
    tokens[ti..].iter().all(|t| matches!(t, Token::AnySeq))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    // -- Pure-function tests (no filesystem) --------------------------------

    #[test]
    fn no_glob_chars_means_literal() {
        assert!(!has_glob_chars("src/foo.js"));
        assert!(has_glob_chars("src/*.js"));
        assert!(has_glob_chars("src/**/foo.js"));
        assert!(has_glob_chars("src/foo?.js"));
        assert!(has_glob_chars("src/foo[ab].js"));
    }

    #[test]
    fn split_fixed_prefix_handles_full_literal() {
        assert_eq!(
            split_fixed_prefix("src/components/foo.js"),
            ("src/components/foo.js".to_string(), "".to_string()),
        );
    }

    #[test]
    fn split_fixed_prefix_stops_at_first_glob_segment() {
        assert_eq!(
            split_fixed_prefix("src/components/*.js"),
            ("src/components".to_string(), "*.js".to_string()),
        );
        assert_eq!(
            split_fixed_prefix("src/**/foo.js"),
            ("src".to_string(), "**/foo.js".to_string()),
        );
    }

    #[test]
    fn split_fixed_prefix_handles_no_prefix() {
        assert_eq!(
            split_fixed_prefix("*.js"),
            ("".to_string(), "*.js".to_string()),
        );
    }

    #[test]
    fn segment_matcher_literal() {
        let m = compile_pattern("foo.js").unwrap();
        assert!(matches_path(&m, Path::new("foo.js")));
        assert!(!matches_path(&m, Path::new("bar.js")));
    }

    #[test]
    fn segment_matcher_star_within_segment() {
        let m = compile_pattern("*.js").unwrap();
        assert!(matches_path(&m, Path::new("a.js")));
        assert!(matches_path(&m, Path::new("foo.js")));
        assert!(matches_path(&m, Path::new(".js")));
        assert!(!matches_path(&m, Path::new("foo.css")));
        assert!(!matches_path(&m, Path::new("dir/foo.js")));
    }

    #[test]
    fn segment_matcher_double_star_crosses_dirs() {
        let m = compile_pattern("src/**/*.js").unwrap();
        assert!(matches_path(&m, Path::new("src/a.js")));
        assert!(matches_path(&m, Path::new("src/components/a.js")));
        assert!(matches_path(&m, Path::new("src/a/b/c/d.js")));
        assert!(!matches_path(&m, Path::new("a.js")));
        assert!(!matches_path(&m, Path::new("src/a.css")));
    }

    #[test]
    fn segment_matcher_question_mark() {
        let m = compile_pattern("foo?.js").unwrap();
        assert!(matches_path(&m, Path::new("foo1.js")));
        assert!(matches_path(&m, Path::new("fooX.js")));
        assert!(!matches_path(&m, Path::new("foo.js")));   // ? requires exactly 1
        assert!(!matches_path(&m, Path::new("foo12.js"))); // ? matches only 1
    }

    #[test]
    fn segment_matcher_char_class() {
        let m = compile_pattern("foo[ab].js").unwrap();
        assert!(matches_path(&m, Path::new("fooa.js")));
        assert!(matches_path(&m, Path::new("foob.js")));
        assert!(!matches_path(&m, Path::new("fooc.js")));
        let r = compile_pattern("test_[0-9].js").unwrap();
        assert!(matches_path(&r, Path::new("test_3.js")));
        assert!(!matches_path(&r, Path::new("test_x.js")));
        let n = compile_pattern("foo[!ab].js").unwrap();
        assert!(matches_path(&n, Path::new("fooc.js")));
        assert!(!matches_path(&n, Path::new("fooa.js")));
    }

    #[test]
    fn invalid_pattern_unterminated_char_class() {
        let err = compile_pattern("foo[ab.js");
        assert!(err.is_err());
    }

    #[test]
    fn glob_error_display() {
        let e = GlobError::NoMatches("src/**/*.js".into());
        assert!(e.to_string().contains("no JS files matched"));
        let e = GlobError::InvalidPattern {
            pattern: "x[".into(),
            reason: "bad".into(),
        };
        assert!(e.to_string().contains("invalid"));
        let _: &dyn std::error::Error = &GlobError::NoMatches("x".into());
    }

    // -- Filesystem-backed tests ------------------------------------------

    fn temp_dir(suffix: &str) -> PathBuf {
        let id = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let p = env::temp_dir().join(format!("closurec-cloc11-02-{id}-{nanos}-{suffix}"));
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn cleanup(p: &Path) {
        let _ = fs::remove_dir_all(p);
    }

    #[test]
    fn expand_literal_path_returns_it() {
        let dir = temp_dir("literal");
        let a = dir.join("a.js");
        fs::write(&a, "// a").unwrap();
        let out = expand_js_patterns(&[a.to_string_lossy().to_string()]).unwrap();
        assert_eq!(out, vec![a.clone()]);
        cleanup(&dir);
    }

    #[test]
    fn expand_literal_missing_errors() {
        let err = expand_js_patterns(&["/nonexistent/missing.js".to_string()])
            .expect_err("missing literal must error");
        match err {
            GlobError::NoMatches(p) => assert_eq!(p, "/nonexistent/missing.js"),
            other => panic!("expected NoMatches, got {other:?}"),
        }
    }

    #[test]
    fn expand_star_glob() {
        let dir = temp_dir("star");
        for name in &["a.js", "b.js", "c.css"] {
            fs::write(dir.join(name), "// content").unwrap();
        }
        let pattern = format!("{}/*.js", dir.display());
        let out = expand_js_patterns(&[pattern]).unwrap();
        assert_eq!(out.len(), 2);
        assert!(out.iter().any(|p| p.ends_with("a.js")));
        assert!(out.iter().any(|p| p.ends_with("b.js")));
        // CSS is not in the result.
        assert!(out.iter().all(|p| !p.to_string_lossy().ends_with("c.css")));
        cleanup(&dir);
    }

    #[test]
    fn expand_double_star_recurses() {
        let dir = temp_dir("dstar");
        fs::create_dir_all(dir.join("nested/deeper")).unwrap();
        fs::write(dir.join("top.js"), "// top").unwrap();
        fs::write(dir.join("nested/mid.js"), "// mid").unwrap();
        fs::write(dir.join("nested/deeper/leaf.js"), "// leaf").unwrap();

        let pattern = format!("{}/**/*.js", dir.display());
        let out = expand_js_patterns(&[pattern]).unwrap();
        assert_eq!(out.len(), 3);
        assert!(out.iter().any(|p| p.ends_with("top.js")));
        assert!(out.iter().any(|p| p.ends_with("mid.js")));
        assert!(out.iter().any(|p| p.ends_with("leaf.js")));
        cleanup(&dir);
    }

    #[test]
    fn expand_with_exclusion() {
        let dir = temp_dir("exclude");
        fs::write(dir.join("keep.js"), "// keep").unwrap();
        fs::write(dir.join("drop.js"), "// drop").unwrap();
        let include = format!("{}/*.js", dir.display());
        let exclude = format!("!{}/drop.js", dir.display());
        let out = expand_js_patterns(&[include, exclude]).unwrap();
        assert_eq!(out.len(), 1);
        assert!(out[0].ends_with("keep.js"));
        cleanup(&dir);
    }

    #[test]
    fn expand_no_matches_errors() {
        let dir = temp_dir("empty");
        let pattern = format!("{}/*.js", dir.display());
        let err = expand_js_patterns(std::slice::from_ref(&pattern))
            .expect_err("no matches must error");
        match err {
            GlobError::NoMatches(p) => assert_eq!(p, pattern),
            other => panic!("expected NoMatches, got {other:?}"),
        }
        cleanup(&dir);
    }

    #[test]
    fn expand_invalid_pattern_errors() {
        let dir = temp_dir("invalid");
        fs::write(dir.join("a.js"), "// a").unwrap();
        // Unterminated char class in a glob pattern.
        let pattern = format!("{}/x[ab.js", dir.display());
        let err = expand_js_patterns(&[pattern])
            .expect_err("invalid pattern must error");
        match err {
            GlobError::InvalidPattern { .. } => {}
            other => panic!("expected InvalidPattern, got {other:?}"),
        }
        cleanup(&dir);
    }

    #[test]
    fn expand_preserves_inclusion_order_across_patterns() {
        let dir = temp_dir("order");
        fs::write(dir.join("z.js"), "// z").unwrap();
        fs::write(dir.join("a.js"), "// a").unwrap();
        // First pattern matches z.js only; second matches everything.
        // Order should be: z.js, a.js (then z.js is deduped on second).
        let p1 = format!("{}/z.js", dir.display());
        let p2 = format!("{}/*.js", dir.display());
        let out = expand_js_patterns(&[p1, p2]).unwrap();
        assert_eq!(out.len(), 2);
        assert!(out[0].ends_with("z.js"));
        assert!(out[1].ends_with("a.js"));
        cleanup(&dir);
    }

    #[test]
    fn expand_dedupes_overlapping_inclusions() {
        let dir = temp_dir("dedupe");
        fs::write(dir.join("foo.js"), "// foo").unwrap();
        let p = format!("{}/*.js", dir.display());
        let out = expand_js_patterns(&[p.clone(), p]).unwrap();
        assert_eq!(out.len(), 1);
        cleanup(&dir);
    }

    // -- Regression tests for security-review findings ---------------------

    #[test]
    fn compile_segment_handles_unicode_before_char_class() {
        // Earlier draft used `seg.len() - chars.count() - 1` which
        // mixes byte-length and char-count → usize underflow
        // panic on any non-ASCII segment containing `[`. The fix
        // tracks byte indices explicitly.
        let m = compile_pattern("é[ab].js").expect("should compile, not panic");
        assert!(matches_path(&m, Path::new("éa.js")));
        assert!(matches_path(&m, Path::new("éb.js")));
        assert!(!matches_path(&m, Path::new("éc.js")));
    }

    #[test]
    fn match_tokens_no_catastrophic_backtracking() {
        // Adversarial pattern that the previous recursive
        // backtracker would have taken exponential time on. The
        // Krauss two-pointer algorithm makes this O(n*m).
        // We just assert it returns in a reasonable time — if
        // we're back to exponential, the test will hang the suite
        // and CI will time out, which is the signal we want.
        let pat = "*a*a*a*a*a*a*a*a*a*a*a*b";
        let text: String = "a".repeat(50);
        let m = compile_pattern(pat).unwrap();
        let start = std::time::Instant::now();
        let result = matches_path(&m, Path::new(&text));
        let elapsed = start.elapsed();
        assert!(!result);
        assert!(
            elapsed.as_millis() < 100,
            "catastrophic backtracking suspected: {elapsed:?}"
        );
    }

    #[test]
    fn walk_skips_symlinks() {
        // Symlink loops (`a/loop -> ..`) would otherwise cause
        // unbounded recursion. We skip symlinks mid-walk.
        let dir = temp_dir("symlink");
        fs::write(dir.join("real.js"), "// real").unwrap();
        // Create a self-referencing symlink. If symlink creation
        // fails on this platform (Windows without symlink privs),
        // skip the test rather than fail it.
        #[cfg(unix)]
        let made_link = std::os::unix::fs::symlink(&dir, dir.join("loop")).is_ok();
        #[cfg(not(unix))]
        let made_link = false;
        if !made_link {
            cleanup(&dir);
            return;
        }
        let pattern = format!("{}/**/*.js", dir.display());
        let out = expand_js_patterns(&[pattern]).unwrap();
        // We see real.js exactly once and don't recurse through
        // the symlink loop.
        assert_eq!(out.len(), 1, "got: {out:?}");
        assert!(out[0].ends_with("real.js"));
        cleanup(&dir);
    }

    #[test]
    fn expand_exclusion_glob_matches_subtree() {
        let dir = temp_dir("excl_subtree");
        fs::create_dir_all(dir.join("legacy")).unwrap();
        fs::write(dir.join("modern.js"), "// modern").unwrap();
        fs::write(dir.join("legacy/old.js"), "// old").unwrap();
        let include = format!("{}/**/*.js", dir.display());
        let exclude = format!("!{}/legacy/**", dir.display());
        let out = expand_js_patterns(&[include, exclude]).unwrap();
        assert_eq!(out.len(), 1);
        assert!(out[0].ends_with("modern.js"));
        cleanup(&dir);
    }
}
