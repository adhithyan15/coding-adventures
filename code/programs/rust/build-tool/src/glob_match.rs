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
// We support four portable pattern forms used by BUILD source declarations:
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
// | `[…]`    | Matches one Unicode scalar from a literal set or an   |
// |          | ascending range, optionally negated by leading `!`.  |
//
// ==========================================================================
// Chapter 3: The Matching Algorithm
// ==========================================================================
//
// The algorithm first compiles Unicode scalar tokens without consulting the
// filesystem. A whole-segment `**` is the only wildcard that can cross path
// boundaries; embedded repeated stars collapse to one segment-local `*`.
//
// The top-level flow:
//
//  1. Normalize both pattern and path: replace `\` with `/`.
//  2. Split on `/`, discard empty compatibility components, and compile each
//     segment. Consecutive whole-segment globstars collapse.
//  3. Match the segment list and candidate path with a rolling-row dynamic
//     program. Each state is visited once.
//  4. Match tokens inside one segment with a second rolling-row program.
//
// Bracket parsing pre-indexes the next `]`, so even long unmatched `[` runs
// take linear parser work. No recursive suffix enumeration remains.

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

use std::fmt;

/// Stable failure returned for portable-glob syntax that has no unambiguous
/// cross-runtime meaning.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GlobPatternError {
    /// The class contains a descending range or `--`, `&&`, `~~`, or `||`.
    AmbiguousOrDescendingCharacterClass,
}

impl fmt::Display for GlobPatternError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AmbiguousOrDescendingCharacterClass => {
                formatter.write_str("ambiguous or descending character class in glob pattern")
            }
        }
    }
}

impl std::error::Error for GlobPatternError {}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ClassMember {
    Literal(char),
    Range(char, char),
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Token {
    Literal(char),
    Star,
    Question,
    CharacterClass {
        negated: bool,
        members: Vec<ClassMember>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum CompiledSegment {
    GlobStar,
    Tokens(Vec<Token>),
}

/// One validated portable glob, compiled once for repeated candidate matching.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CompiledPattern {
    segments: Vec<CompiledSegment>,
}

/// Match a file path against a glob pattern.
///
/// Supports the portable BUILD glob forms:
///   - `**` — matches zero or more path segments (crosses `/` boundaries)
///   - `*`  — matches zero or more characters within one segment (no `/`)
///   - `?`  — matches exactly one character (not `/`)
///   - `[…]` — matches one Unicode scalar from a class or ascending range
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
    try_match_path(pattern, path).unwrap_or_else(|error| panic!("{error}"))
}

/// Match a path while preserving invalid portable syntax as a typed error.
pub fn try_match_path(pattern: &str, path: &str) -> Result<bool, GlobPatternError> {
    let compiled = compile_pattern(pattern)?;
    Ok(match_compiled_path(&compiled, path))
}

/// Validate one pattern without matching a candidate path.
pub fn validate_pattern(pattern: &str) -> Result<(), GlobPatternError> {
    compile_pattern(pattern).map(|_| ())
}

/// Compile a complete declared list before callers enumerate candidates.
pub(crate) fn compile_patterns(
    patterns: &[String],
) -> Result<Vec<CompiledPattern>, GlobPatternError> {
    patterns
        .iter()
        .map(|pattern| compile_pattern(pattern))
        .collect()
}

// ---------------------------------------------------------------------------
// Internal matching engine
// ---------------------------------------------------------------------------

fn compile_pattern(pattern: &str) -> Result<CompiledPattern, GlobPatternError> {
    let normalized = pattern.replace('\\', "/");
    let mut segments = Vec::new();

    for segment in normalized.split('/').filter(|segment| !segment.is_empty()) {
        if segment == "**" {
            if !matches!(segments.last(), Some(CompiledSegment::GlobStar)) {
                segments.push(CompiledSegment::GlobStar);
            }
        } else {
            segments.push(CompiledSegment::Tokens(parse_segment(segment)?));
        }
    }

    Ok(CompiledPattern { segments })
}

fn parse_segment(segment: &str) -> Result<Vec<Token>, GlobPatternError> {
    parse_segment_with_state_count(segment).map(|(tokens, _)| tokens)
}

fn parse_segment_with_state_count(
    segment: &str,
) -> Result<(Vec<Token>, usize), GlobPatternError> {
    let scalars: Vec<char> = segment.chars().collect();
    let mut next_closing_bracket = vec![None; scalars.len() + 1];
    let mut next_closing = None;

    for index in (0..scalars.len()).rev() {
        if scalars[index] == ']' {
            next_closing = Some(index);
        }
        next_closing_bracket[index] = next_closing;
    }

    let mut tokens = Vec::new();
    let mut index = 0;
    let mut visited = scalars.len();
    while index < scalars.len() {
        visited += 1;
        match scalars[index] {
            '*' => {
                if !matches!(tokens.last(), Some(Token::Star)) {
                    tokens.push(Token::Star);
                }
                index += 1;
            }
            '?' => {
                tokens.push(Token::Question);
                index += 1;
            }
            '[' => match parse_character_class(&scalars, index, &next_closing_bracket)? {
                Some((token, next_index)) => {
                    tokens.push(token);
                    index = next_index;
                }
                None => {
                    tokens.push(Token::Literal('['));
                    index += 1;
                }
            },
            literal => {
                tokens.push(Token::Literal(literal));
                index += 1;
            }
        }
    }

    Ok((tokens, visited))
}

fn parse_character_class(
    scalars: &[char],
    opening: usize,
    next_closing_bracket: &[Option<usize>],
) -> Result<Option<(Token, usize)>, GlobPatternError> {
    let mut cursor = opening + 1;
    let negated = cursor < scalars.len() && scalars[cursor] == '!';
    if negated {
        cursor += 1;
    }

    let closing = match next_closing_bracket[cursor] {
        Some(candidate) if candidate == cursor => next_closing_bracket[cursor + 1],
        candidate => candidate,
    };
    let Some(closing) = closing else {
        return Ok(None);
    };
    let body = &scalars[cursor..closing];

    if body.windows(2).any(|pair| {
        pair[0] == pair[1] && matches!(pair[0], '-' | '&' | '~' | '|')
    }) {
        return Err(GlobPatternError::AmbiguousOrDescendingCharacterClass);
    }

    let mut members = Vec::new();
    let mut member_index = 0;
    while member_index < body.len() {
        if member_index + 2 < body.len() && body[member_index + 1] == '-' {
            let start = body[member_index];
            let end = body[member_index + 2];
            if start > end {
                return Err(GlobPatternError::AmbiguousOrDescendingCharacterClass);
            }
            members.push(ClassMember::Range(start, end));
            member_index += 3;
        } else {
            members.push(ClassMember::Literal(body[member_index]));
            member_index += 1;
        }
    }

    Ok(Some((
        Token::CharacterClass { negated, members },
        closing + 1,
    )))
}

pub(crate) fn match_compiled_path(pattern: &CompiledPattern, path: &str) -> bool {
    match_compiled_path_with_state_count(pattern, path).0
}

#[cfg(test)]
fn match_path_with_state_count(
    pattern: &str,
    path: &str,
) -> Result<(bool, usize), GlobPatternError> {
    let compiled = compile_pattern(pattern)?;
    Ok(match_compiled_path_with_state_count(&compiled, path))
}

fn match_compiled_path_with_state_count(
    pattern: &CompiledPattern,
    path: &str,
) -> (bool, usize) {
    let normalized = path.replace('\\', "/");
    let path_segments: Vec<Vec<char>> = normalized
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.chars().collect())
        .collect();
    let path_count = path_segments.len();
    let mut next_row = vec![false; path_count + 1];
    next_row[path_count] = true;
    let mut visited = path_count + 1;

    for segment in pattern.segments.iter().rev() {
        let mut row = vec![false; path_count + 1];
        visited += path_count + 1;
        match segment {
            CompiledSegment::GlobStar => {
                row[path_count] = next_row[path_count];
                for path_index in (0..path_count).rev() {
                    row[path_index] = next_row[path_index] || row[path_index + 1];
                }
            }
            CompiledSegment::Tokens(tokens) => {
                for path_index in (0..path_count).rev() {
                    row[path_index] = next_row[path_index + 1]
                        && match_segment(tokens, &path_segments[path_index]);
                }
            }
        }
        next_row = row;
    }

    (next_row[0], visited)
}

fn match_segment(tokens: &[Token], value: &[char]) -> bool {
    match_segment_with_state_count(tokens, value).0
}

fn match_segment_with_state_count(tokens: &[Token], value: &[char]) -> (bool, usize) {
    let value_count = value.len();
    let mut next_row = vec![false; value_count + 1];
    next_row[value_count] = true;

    for token in tokens.iter().rev() {
        let mut row = vec![false; value_count + 1];
        match token {
            Token::Star => {
                row[value_count] = next_row[value_count];
                for value_index in (0..value_count).rev() {
                    row[value_index] = next_row[value_index] || row[value_index + 1];
                }
            }
            Token::Question => {
                row[..value_count].copy_from_slice(&next_row[1..=value_count]);
            }
            Token::Literal(_) | Token::CharacterClass { .. } => {
                for value_index in 0..value_count {
                    row[value_index] = next_row[value_index + 1]
                        && token_matches(token, value[value_index]);
                }
            }
        }
        next_row = row;
    }

    (next_row[0], (tokens.len() + 1) * (value_count + 1))
}

fn token_matches(token: &Token, value: char) -> bool {
    match token {
        Token::Literal(expected) => *expected == value,
        Token::CharacterClass { negated, members } => {
            let included = members.iter().any(|member| match member {
                ClassMember::Literal(expected) => *expected == value,
                ClassMember::Range(start, end) => *start <= value && value <= *end,
            });
            if *negated {
                !included
            } else {
                included
            }
        }
        Token::Star | Token::Question => false,
    }
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
    fn test_leading_slash_is_ignored_for_compatibility() {
        assert!(match_path("?.py", "/a.py"));
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

    #[test]
    fn question_and_classes_consume_unicode_scalars() {
        assert!(try_match_path("?.txt", "🐍.txt").unwrap());
        assert!(try_match_path("[🐀-🙏].txt", "🐍.txt").unwrap());
        assert!(!try_match_path("[🐀-🙏].txt", "a.txt").unwrap());

        let decomposed = "e\u{301}";
        assert!(!try_match_path("?", decomposed).unwrap());
        assert!(try_match_path("??", decomposed).unwrap());
    }

    #[test]
    fn portable_character_class_edges_match_python_fnmatchcase() {
        for (pattern, matching, nonmatching) in [
            ("[^a].txt", "^.txt", "b.txt"),
            ("[]a].txt", "].txt", "b.txt"),
            ("[-a].txt", "-.txt", "b.txt"),
            ("[a-].txt", "-.txt", "b.txt"),
            ("[a-c].txt", "b.txt", "z.txt"),
            ("[!a-c].txt", "z.txt", "b.txt"),
        ] {
            assert!(try_match_path(pattern, matching).unwrap(), "{pattern}");
            assert!(!try_match_path(pattern, nonmatching).unwrap(), "{pattern}");
        }

        assert!(try_match_path("[^a].txt", "a.txt").unwrap());
        assert!(try_match_path("[!]].txt", "a.txt").unwrap());
        assert!(!try_match_path("[!]].txt", "].txt").unwrap());
    }

    #[test]
    fn unmatched_opening_bracket_is_a_literal() {
        for (pattern, path) in [
            ("[", "["),
            ("prefix[", "prefix["),
            ("[]", "[]"),
            ("[!]", "[!]"),
        ] {
            assert!(try_match_path(pattern, path).unwrap(), "{pattern}");
        }
        assert!(!try_match_path("prefix[", "prefixx").unwrap());
    }

    #[test]
    fn rejected_classes_return_one_stable_typed_error() {
        for pattern in [
            "[z-a].txt",
            "[a--b].txt",
            "[a&&b].txt",
            "[a~~b].txt",
            "[a||b].txt",
        ] {
            assert_eq!(
                validate_pattern(pattern),
                Err(GlobPatternError::AmbiguousOrDescendingCharacterClass),
                "{pattern}"
            );
            assert_eq!(
                try_match_path(pattern, "a.txt"),
                Err(GlobPatternError::AmbiguousOrDescendingCharacterClass),
                "{pattern}"
            );
        }
    }

    #[test]
    fn unmatched_bracket_parser_work_is_linear() {
        let pattern = "[".repeat(16_384);
        let (tokens, visited) = parse_segment_with_state_count(&pattern).unwrap();

        assert_eq!(tokens.len(), pattern.chars().count());
        assert_eq!(visited, 2 * pattern.chars().count());
    }

    #[test]
    fn adversarial_globstar_near_miss_visits_each_path_state_once() {
        let pattern = std::iter::repeat_n(["**", "a"], 12)
            .flatten()
            .chain(["z"])
            .collect::<Vec<_>>()
            .join("/");
        let path = std::iter::repeat_n("a", 24)
            .chain(["y"])
            .collect::<Vec<_>>()
            .join("/");

        let (matched, visited) = match_path_with_state_count(&pattern, &path).unwrap();

        assert!(!matched);
        let pattern_states = pattern.split('/').count() + 1;
        let path_states = path.split('/').count() + 1;
        assert_eq!(visited, pattern_states * path_states);
    }

    #[test]
    fn adversarial_segment_near_miss_visits_each_state_once() {
        let tokens = parse_segment("*a*a*a*a*a*b").unwrap();
        let value: Vec<char> = "aaaaaaaaaaaaaaaaac".chars().collect();

        let (matched, visited) = match_segment_with_state_count(&tokens, &value);

        assert!(!matched);
        assert_eq!(visited, (tokens.len() + 1) * (value.len() + 1));
    }

    #[test]
    fn repeated_stars_and_globstars_collapse_without_semantic_drift() {
        assert!(try_match_path("**/**/*.py", "a/b/main.py").unwrap());
        assert!(try_match_path("**/**/**", "x/y/z").unwrap());
        assert!(try_match_path("src/**", "src").unwrap());
        assert!(try_match_path("src//main.py", "src/main.py").unwrap());
        assert!(try_match_path("src/", "src").unwrap());
        assert!(!try_match_path("*", "").unwrap());
    }

    #[test]
    fn braces_are_literal_and_embedded_double_star_stays_in_one_segment() {
        assert!(try_match_path("{a,b}.txt", "{a,b}.txt").unwrap());
        assert!(!try_match_path("{a,b}.txt", "a.txt").unwrap());
        assert!(try_match_path("foo**bar", "fooxbar").unwrap());
        assert!(!try_match_path("foo**bar", "foo/x/bar").unwrap());
    }
}
