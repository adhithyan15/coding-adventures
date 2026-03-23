// Pure string-based glob matching for file paths.
//
// ==========================================================================
// Chapter 1: Why Glob Matching?
// ==========================================================================
//
// BUILD files in our monorepo declare source patterns like `src/**/*.py` or
// `tests/*.rs`. The build tool needs to determine which actual files on disk
// match these patterns — for change detection, hashing, and dependency
// tracking.
//
// Rather than shelling out to a system glob or pulling in a large library,
// we implement glob matching as pure string comparison. This has three
// benefits:
//
//   1. **No filesystem access**: We can test patterns against paths without
//      touching the disk. This makes the module fast and side-effect-free.
//   2. **Cross-platform**: Forward and back slashes are normalized before
//      matching, so patterns work the same on macOS, Linux, and Windows.
//   3. **Predictable**: The matching rules are simple and documented inline.
//      No surprises from platform-specific glob implementations.
//
// ==========================================================================
// Chapter 2: Supported Wildcards
// ==========================================================================
//
// We support three wildcards, matching the behavior expected by Bazel-style
// BUILD files:
//
// | Wildcard | Meaning                                               |
// |----------|-------------------------------------------------------|
// | `**`     | Matches zero or more path segments (directories).     |
// |          | For example, `src/**/*.py` matches `src/a.py`,        |
// |          | `src/foo/a.py`, and `src/foo/bar/a.py`.               |
// | `*`      | Matches zero or more characters within a single path  |
// |          | segment. Does NOT cross `/` boundaries.               |
// |          | For example, `*.py` matches `foo.py` but not          |
// |          | `dir/foo.py`.                                         |
// | `?`      | Matches exactly one character (not `/`).              |
// |          | For example, `?.py` matches `a.py` but not `ab.py`.  |
//
// ==========================================================================
// Chapter 3: The Matching Algorithm
// ==========================================================================
//
// The algorithm is recursive with memoization-friendly structure (though we
// use simple recursion here since patterns and paths are short).
//
// The key insight is that `**` is the only wildcard that can cross path
// segment boundaries (`/`). So we split the pattern on `**` first, then
// match each segment using `*` and `?` within a single path segment.
//
// The top-level flow:
//
//  1. Normalize both pattern and path: replace `\` with `/`.
//  2. Split the pattern on `**` to get "segments".
//  3. For each segment, try to match it against the remaining path.
//  4. `**` between segments can consume zero or more path components.
//
// For single-segment matching (no `**`), we use character-by-character
// comparison with `*` and `?` handling.

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Match a file path against a glob pattern.
///
/// Supports three wildcards:
///   - `**` — matches zero or more path segments (crosses `/` boundaries)
///   - `*`  — matches zero or more characters within one segment (no `/`)
///   - `?`  — matches exactly one character (not `/`)
///
/// Both the pattern and path are normalized to use `/` as the separator
/// before matching, so this works correctly on all platforms.
///
/// # Examples
///
/// ```
/// use build_tool::glob_match::match_path;
///
/// // ** matches any depth of directories
/// assert!(match_path("src/**/*.py", "src/foo/bar.py"));
/// assert!(match_path("src/**/*.py", "src/bar.py"));
///
/// // * matches within a single segment
/// assert!(match_path("*.py", "hello.py"));
/// assert!(!match_path("*.py", "dir/hello.py"));
///
/// // ? matches exactly one character
/// assert!(match_path("?.py", "a.py"));
/// assert!(!match_path("?.py", "ab.py"));
/// ```
pub fn match_path(pattern: &str, path: &str) -> bool {
    // Step 1: Normalize separators.
    //
    // Windows uses backslashes, Unix uses forward slashes. We normalize
    // everything to forward slashes so patterns work cross-platform.
    let pattern = pattern.replace('\\', "/");
    let path = path.replace('\\', "/");

    // Delegate to the recursive matcher.
    do_match(pattern.as_bytes(), path.as_bytes())
}

// ---------------------------------------------------------------------------
// Internal matching engine
// ---------------------------------------------------------------------------

/// Recursive matching engine operating on byte slices for efficiency.
///
/// This function handles all three wildcards:
///
/// - When we encounter `**`, we try matching the rest of the pattern
///   against every possible suffix of the path (consuming zero or more
///   complete path segments).
///
/// - When we encounter `*`, we try consuming zero or more non-`/`
///   characters from the path.
///
/// - When we encounter `?`, we consume exactly one non-`/` character.
///
/// - Literal characters must match exactly.
///
/// The recursion terminates when either the pattern or path is exhausted.
/// A match succeeds only if BOTH are exhausted simultaneously (or the
/// remaining pattern consists entirely of `**` and `/` separators).
fn do_match(pattern: &[u8], path: &[u8]) -> bool {
    // Base case: both pattern and path are fully consumed — success.
    if pattern.is_empty() {
        return path.is_empty();
    }

    // Check for `**` (double-star) at the current position.
    //
    // `**` is special because it crosses `/` boundaries. It matches
    // zero or more complete path segments. We handle it by trying
    // every possible "skip" of the path:
    //
    //   - Skip 0 characters (** matches nothing)
    //   - Skip to after the next `/` (** matches one segment)
    //   - Skip to after the second `/` (** matches two segments)
    //   - ... and so on until the path is exhausted.
    if pattern.len() >= 2 && pattern[0] == b'*' && pattern[1] == b'*' {
        // Consume the `**` from the pattern.
        let rest_pattern = &pattern[2..];

        // Also consume a trailing `/` after `**` if present, since `**/`
        // means "any number of directories followed by a separator".
        let rest_pattern = if !rest_pattern.is_empty() && rest_pattern[0] == b'/' {
            &rest_pattern[1..]
        } else {
            rest_pattern
        };

        // Also handle leading `/` before `**` — try without it.
        // Try matching rest_pattern against every suffix of path.
        //
        // Attempt 1: ** matches zero segments (path unchanged).
        if do_match(rest_pattern, path) {
            return true;
        }

        // Attempt 2+: ** matches one or more segments.
        // Walk through the path, and at each `/` boundary, try matching
        // the rest.
        for i in 0..path.len() {
            if path[i] == b'/' {
                if do_match(rest_pattern, &path[i + 1..]) {
                    return true;
                }
            }
        }

        // Attempt: ** matches the entire remaining path.
        // This handles the case where ** is at the end of the pattern.
        if rest_pattern.is_empty() {
            return true;
        }

        return false;
    }

    // Check for single `*` — matches zero or more non-`/` characters.
    //
    // The key difference from `**` is that `*` does NOT cross `/`
    // boundaries. So `*.py` matches `foo.py` but not `dir/foo.py`.
    if pattern[0] == b'*' {
        let rest_pattern = &pattern[1..];

        // Try consuming 0, 1, 2, ... characters from path (but not `/`).
        for i in 0..=path.len() {
            // Stop if we would cross a `/` boundary.
            if i > 0 && path[i - 1] == b'/' {
                break;
            }
            if do_match(rest_pattern, &path[i..]) {
                return true;
            }
        }

        return false;
    }

    // Check for `?` — matches exactly one non-`/` character.
    if pattern[0] == b'?' {
        if !path.is_empty() && path[0] != b'/' {
            return do_match(&pattern[1..], &path[1..]);
        }
        return false;
    }

    // Literal character — must match exactly.
    if !path.is_empty() && pattern[0] == path[0] {
        return do_match(&pattern[1..], &path[1..]);
    }

    // No match.
    false
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
//
// The test suite covers all three wildcards (`**`, `*`, `?`) plus edge
// cases like empty patterns, empty paths, consecutive wildcards, and
// cross-platform separator normalization.

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Literal matching (no wildcards)
    // -----------------------------------------------------------------------

    #[test]
    fn test_exact_match() {
        assert!(match_path("foo.py", "foo.py"));
    }

    #[test]
    fn test_exact_match_with_path() {
        assert!(match_path("src/foo.py", "src/foo.py"));
    }

    #[test]
    fn test_exact_mismatch() {
        assert!(!match_path("foo.py", "bar.py"));
    }

    #[test]
    fn test_empty_pattern_empty_path() {
        assert!(match_path("", ""));
    }

    #[test]
    fn test_empty_pattern_nonempty_path() {
        assert!(!match_path("", "foo.py"));
    }

    #[test]
    fn test_nonempty_pattern_empty_path() {
        assert!(!match_path("foo.py", ""));
    }

    // -----------------------------------------------------------------------
    // Single star (*)
    // -----------------------------------------------------------------------

    #[test]
    fn test_star_matches_filename() {
        assert!(match_path("*.py", "hello.py"));
    }

    #[test]
    fn test_star_matches_empty_string() {
        assert!(match_path("*.py", ".py"));
    }

    #[test]
    fn test_star_does_not_cross_slash() {
        // `*` should NOT match across directory boundaries.
        assert!(!match_path("*.py", "dir/hello.py"));
    }

    #[test]
    fn test_star_in_middle() {
        assert!(match_path("test_*.py", "test_logic.py"));
        assert!(match_path("test_*.py", "test_.py"));
        assert!(!match_path("test_*.py", "test_dir/file.py"));
    }

    #[test]
    fn test_star_at_end() {
        assert!(match_path("src/*", "src/foo.py"));
        assert!(match_path("src/*", "src/bar"));
        assert!(!match_path("src/*", "src/sub/foo.py"));
    }

    #[test]
    fn test_star_at_beginning() {
        assert!(match_path("*/foo.py", "src/foo.py"));
        assert!(!match_path("*/foo.py", "src/sub/foo.py"));
    }

    #[test]
    fn test_multiple_stars_same_segment() {
        assert!(match_path("*_test_*.py", "unit_test_gates.py"));
        assert!(!match_path("*_test_*.py", "dir/unit_test_gates.py"));
    }

    // -----------------------------------------------------------------------
    // Double star (**)
    // -----------------------------------------------------------------------

    #[test]
    fn test_doublestar_matches_zero_segments() {
        // `**/*.py` should match `foo.py` (zero directories).
        assert!(match_path("**/*.py", "foo.py"));
    }

    #[test]
    fn test_doublestar_matches_one_segment() {
        assert!(match_path("**/*.py", "src/foo.py"));
    }

    #[test]
    fn test_doublestar_matches_multiple_segments() {
        assert!(match_path("**/*.py", "src/foo/bar/baz.py"));
    }

    #[test]
    fn test_doublestar_at_end() {
        // `src/**` should match everything under src/.
        assert!(match_path("src/**", "src/foo.py"));
        assert!(match_path("src/**", "src/a/b/c.py"));
    }

    #[test]
    fn test_doublestar_in_middle() {
        assert!(match_path("src/**/*.py", "src/foo.py"));
        assert!(match_path("src/**/*.py", "src/sub/foo.py"));
        assert!(match_path("src/**/*.py", "src/a/b/c/foo.py"));
    }

    #[test]
    fn test_doublestar_does_not_match_wrong_prefix() {
        assert!(!match_path("src/**/*.py", "lib/foo.py"));
    }

    #[test]
    fn test_doublestar_alone_matches_everything() {
        assert!(match_path("**", "anything"));
        assert!(match_path("**", "a/b/c/d.py"));
        assert!(match_path("**", ""));
    }

    #[test]
    fn test_doublestar_with_exact_suffix() {
        assert!(match_path("**/BUILD", "code/packages/python/logic-gates/BUILD"));
        assert!(match_path("**/BUILD", "BUILD"));
    }

    // -----------------------------------------------------------------------
    // Question mark (?)
    // -----------------------------------------------------------------------

    #[test]
    fn test_question_matches_one_char() {
        assert!(match_path("?.py", "a.py"));
    }

    #[test]
    fn test_question_does_not_match_zero_chars() {
        assert!(!match_path("?.py", ".py"));
    }

    #[test]
    fn test_question_does_not_match_two_chars() {
        assert!(!match_path("?.py", "ab.py"));
    }

    #[test]
    fn test_question_does_not_match_slash() {
        assert!(!match_path("?.py", "/a.py"));
    }

    #[test]
    fn test_multiple_questions() {
        assert!(match_path("???.py", "abc.py"));
        assert!(!match_path("???.py", "ab.py"));
        assert!(!match_path("???.py", "abcd.py"));
    }

    // -----------------------------------------------------------------------
    // Combined wildcards
    // -----------------------------------------------------------------------

    #[test]
    fn test_star_and_question() {
        assert!(match_path("test_?_*.py", "test_a_foo.py"));
        assert!(!match_path("test_?_*.py", "test_ab_foo.py"));
    }

    #[test]
    fn test_doublestar_and_star() {
        assert!(match_path("**/*_test.py", "src/foo_test.py"));
        assert!(match_path("**/*_test.py", "a/b/c/foo_test.py"));
        assert!(match_path("**/*_test.py", "foo_test.py"));
    }

    #[test]
    fn test_doublestar_star_question() {
        assert!(match_path("**/test_?.py", "src/test_a.py"));
        assert!(!match_path("**/test_?.py", "src/test_ab.py"));
    }

    // -----------------------------------------------------------------------
    // Cross-platform separator normalization
    // -----------------------------------------------------------------------

    #[test]
    fn test_backslash_normalization_in_path() {
        assert!(match_path("src/**/*.py", "src\\foo\\bar.py"));
    }

    #[test]
    fn test_backslash_normalization_in_pattern() {
        assert!(match_path("src\\**\\*.py", "src/foo/bar.py"));
    }

    // -----------------------------------------------------------------------
    // Edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_pattern_longer_than_path() {
        assert!(!match_path("a/b/c/d", "a/b"));
    }

    #[test]
    fn test_path_longer_than_pattern() {
        assert!(!match_path("a/b", "a/b/c/d"));
    }

    #[test]
    fn test_only_star() {
        // `*` alone matches any single segment.
        assert!(match_path("*", "foo"));
        assert!(match_path("*", "foo.py"));
        assert!(!match_path("*", "foo/bar"));
    }

    #[test]
    fn test_trailing_slash_in_pattern() {
        // Trailing slash should not affect matching of files.
        assert!(!match_path("src/", "src/foo.py"));
    }

    #[test]
    fn test_real_world_python_pattern() {
        let pat = "src/**/*.py";
        assert!(match_path(pat, "src/logic_gates/__init__.py"));
        assert!(match_path(pat, "src/logic_gates/gates.py"));
        assert!(!match_path(pat, "tests/test_gates.py"));
    }

    #[test]
    fn test_real_world_rust_pattern() {
        let pat = "src/**/*.rs";
        assert!(match_path(pat, "src/main.rs"));
        assert!(match_path(pat, "src/lib/parser.rs"));
        assert!(!match_path(pat, "benches/bench.rs"));
    }
}
