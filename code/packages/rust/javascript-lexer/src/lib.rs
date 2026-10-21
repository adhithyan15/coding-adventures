//! JavaScript lexer backed by compiled ECMAScript token grammars (es1 through es2025).
//!
//! # Correlation-vector plumbing
//!
//! Per [CLOC03](../../../specs/CLOC03-correlation-vector-plumbing.md)
//! §"Stage 1 — Lexer," when called via [`tokenize_javascript_with_cv`] the
//! lexer assigns a fresh correlation-vector ID to every emitted token via
//! `CVLog::create(Some(Origin{ ... }))`. The `Origin` records the source
//! filename and a `line:column` location string built from the token's
//! own positional info. No `Contribution` is appended at this stage —
//! lexing is the act of *creation*, and there is nothing yet to contribute
//! about.

use coding_adventures_correlation_vector::{CVLog, Origin};
use coding_adventures_javascript_tokens::EsVersion;
use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;
use std::collections::HashMap;

mod _grammar;

pub const SUPPORTED_VERSIONS: &[&str] = _grammar::SUPPORTED_VERSIONS;
pub const DEFAULT_VERSION: &str = "es2025";

/// Typed default version. New code should prefer this over [`DEFAULT_VERSION`].
pub const DEFAULT_ES_VERSION: EsVersion = EsVersion::Es2025;

fn validate_version(version: &str) -> Result<&str, String> {
    if SUPPORTED_VERSIONS.contains(&version) {
        Ok(version)
    } else {
        Err(format!(
            "Unknown JavaScript/ECMAScript version '{version}'. Valid values: {}",
            SUPPORTED_VERSIONS
                .iter()
                .map(|value| format!("\"{}\"", value))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

pub fn create_javascript_lexer<'src>(
    source: &'src str,
    version: &str,
) -> Result<GrammarLexer<'src>, String> {
    let version = validate_version(version)?;
    let grammar = _grammar::token_grammar(version)
        .expect("compiled JavaScript token grammar missing supported version");
    Ok(GrammarLexer::new(source, &grammar))
}

pub fn tokenize_javascript(source: &str, version: &str) -> Result<Vec<Token>, String> {
    let version = validate_version(version)?;
    let grammar = _grammar::token_grammar(version)
        .expect("compiled JavaScript token grammar missing supported version");
    let mut lexer = GrammarLexer::new(source, &grammar);
    let tokens = lexer
        .tokenize()
        .map_err(|e| format!("JavaScript tokenization failed: {e}"))?;
    refuse_html_like_comments(source, &tokens)?;
    Ok(tokens)
}

/// Typed version of [`create_javascript_lexer`]. Takes an [`EsVersion`]
/// directly; cannot fail with an unknown-version error.
pub fn create_javascript_lexer_typed<'src>(
    source: &'src str,
    version: EsVersion,
) -> GrammarLexer<'src> {
    let grammar = _grammar::token_grammar(version.as_str())
        .expect("compiled JavaScript token grammar missing supported version");
    GrammarLexer::new(source, &grammar)
}

/// Typed version of [`tokenize_javascript`]. Takes an [`EsVersion`] directly;
/// cannot fail with an unknown-version error. The only error path is
/// tokenization itself.
pub fn tokenize_javascript_typed(
    source: &str,
    version: EsVersion,
) -> Result<Vec<Token>, String> {
    let grammar = _grammar::token_grammar(version.as_str())
        .expect("compiled JavaScript token grammar missing supported version");
    let mut lexer = GrammarLexer::new(source, &grammar);
    let tokens = lexer
        .tokenize()
        .map_err(|e| format!("JavaScript tokenization failed: {e}"))?;
    refuse_html_like_comments(source, &tokens)?;
    Ok(tokens)
}

/// Tokenize a JavaScript subset that does not admit regex literals.
///
/// This is an explicit division-only goal for LANG78's numeric frontend,
/// not context-sensitive disambiguation for general JavaScript. Filter the
/// compiled rules before matching: splitting a REGEX token afterwards would
/// lose comments, string boundaries and source locations. All other rules,
/// including comment trivia and division assignment, retain their priority.
/// Callers must reject unsupported syntax after parsing and enforce their
/// source/work limits, just as with [`tokenize_javascript_typed`].
pub fn tokenize_javascript_division_only(
    source: &str,
    version: EsVersion,
) -> Result<Vec<Token>, String> {
    let mut grammar = _grammar::token_grammar(version.as_str())
        .expect("compiled JavaScript token grammar missing supported version");
    grammar.definitions.retain(|rule| rule.name != "REGEX");
    grammar
        .error_definitions
        .retain(|rule| rule.name != "BAD_REGEX");
    let mut lexer = GrammarLexer::new(source, &grammar);
    let tokens = lexer
        .tokenize()
        .map_err(|e| format!("JavaScript tokenization failed: {e}"))?;
    refuse_html_like_comments(source, &tokens)?;
    Ok(tokens)
}

// ---------------------------------------------------------------------------
// Annex B HTML-like comments: refused, never misread
// ---------------------------------------------------------------------------
//
// In a script, ECMA-262 Annex B (B.1.1) has two comment forms this token
// grammar can't express, because they depend on "start of a line" or on
// three tokens together:
//
//   SingleLineHTMLOpenComment   `<!--` …to end of line
//   SingleLineHTMLCloseComment  a line terminator, then optional whitespace
//                               and comments, then `-->` …to end of line
//
// Lexed as ordinary tokens they become CODE: `a = b⏎-->f()` reads as
// `b-- > f()`, and `x = 1 <!--y` as `1 < !--y`. Either way, comment text
// would run (#17840 review). Modelling them correctly takes context the
// token grammar lacks: two attempts were defeated by regex, template and
// `\r` corner cases. So both forms are REFUSED with a clear error.
//
// A line-leading `--` is judged twice. Either the lexer's line count rose
// since the previous token (it counts `\n`), or the source between the two
// tokens holds another line terminator (`\r`, U+2028, U+2029) the line
// count misses. The span starts at the previous token, so a multi-line
// template right before `--` is also refused; `--` after a template is an
// early error anyway.

/// Error if `tokens` contain an Annex B HTML-like comment.
fn refuse_html_like_comments(source: &str, tokens: &[Token]) -> Result<(), String> {
    let adjacent = |a: &Token, b: &Token, width: usize| {
        b.line == a.line && b.column == a.column + width
    };
    // Source offsets of tokens, found by a cursor that only moves forward:
    // tokens come in source order, so mapping every one costs O(source)
    // in total, however long a line is.
    let mut cursor = SourceCursor::new(source);
    let has_hidden_breaks = source.contains(['\r', '\u{2028}', '\u{2029}']);
    for i in 0..tokens.len() {
        let t = &tokens[i];
        // After a `}` the token grammar always expects a regex, but when that
        // `}` closes an object, function or class expression a `/` there is
        // DIVISION: `x = {}⏎/ 2 <!--/; f()` is `x = {} / 2` plus a comment.
        // The lexer then hides the `<!--` inside one REGEX token and the rest
        // of the comment runs (#17840 review). A regex right after `}` that
        // holds `<!--` is refused; elsewhere `/<!--/` stays an ordinary regex.
        if t.type_name.as_deref() == Some("REGEX")
            && t.value.contains("<!--")
            && i.checked_sub(1).is_some_and(|p| tokens[p].value == "}")
        {
            return Err(html_comment_error("<!--", t));
        }
        // A regex literal may never hold a line terminator, escaped or not
        // (ECMA-262 RegularExpressionChar). The REGEX token pattern only
        // excludes `\n`, so `\r`, U+2028 or U+2029 inside one means the lexer
        // misread division (after `}`) and swallowed a line break, with a
        // line-leading `-->` behind it (#17840 review round 3). No valid
        // program is refused.
        if t.type_name.as_deref() == Some("REGEX")
            && t.value.contains(['\r', '\u{2028}', '\u{2029}'])
        {
            return Err(format!(
                "JavaScript tokenization failed: line terminator inside a regular expression \
                 literal at {}:{}",
                t.line, t.column
            ));
        }
        // A `--` here is one `--` token (ES grammars) or two touching `-`
        // tokens (the generic subset grammar has no `--`); `next` is the
        // index of the token after it.
        let next = if t.value == "--" && t.type_name.as_deref() == Some("MINUS_MINUS") {
            i + 1
        } else if t.value == "-"
            && tokens.get(i + 1).is_some_and(|m| m.value == "-" && adjacent(t, m, 1))
        {
            i + 2
        } else {
            continue;
        };
        // `<!--`: `<` `!` `--`, touching.
        if i >= 2
            && tokens[i - 2].value == "<"
            && tokens[i - 1].value == "!"
            && adjacent(&tokens[i - 2], &tokens[i - 1], 1)
            && adjacent(&tokens[i - 1], t, 1)
        {
            return Err(html_comment_error("<!--", &tokens[i - 2]));
        }
        // `-->` at the start of a line (or of the source). Maximal munch may
        // fuse the `>` with what follows (`-->>x` is `--` `>>`, `-->=x` is
        // `--` `>=`), so only the token's FIRST character must be `>`.
        let Some(gt) = tokens.get(next) else { continue };
        if !gt.value.starts_with('>') || !adjacent(t, gt, 2) {
            continue;
        }
        let line_leading = match i.checked_sub(1).map(|p| &tokens[p]) {
            None => true,
            Some(prev) => {
                prev.line < t.line
                    // Only `\r`, U+2028 and U+2029 can hide a line break from
                    // the line count; without them, skip the offset walk.
                    || (has_hidden_breaks
                        && match (cursor.offset(prev), cursor.offset(t)) {
                            (Some(a), Some(b)) if a <= b => source[a..b]
                                .contains(['\r', '\u{2028}', '\u{2029}']),
                            // Positions that don't map back: refuse.
                            _ => true,
                        })
            }
        };
        if line_leading {
            return Err(html_comment_error("-->", t));
        }
    }
    Ok(())
}

/// Maps a token's 1-based (line, column), with lines counted by `\n` and
/// columns by `char`, to its byte offset. Queries must not go backwards, by
/// more than the one token the caller re-asks for, so the walk is linear.
struct SourceCursor<'s> {
    source: &'s str,
    byte: usize,
    line: usize,
    column: usize,
}

impl<'s> SourceCursor<'s> {
    fn new(source: &'s str) -> Self {
        SourceCursor { source, byte: 0, line: 1, column: 1 }
    }

    fn offset(&mut self, t: &Token) -> Option<usize> {
        let target = (t.line, t.column);
        if target < (self.line, self.column) {
            // Re-asked for an earlier token: restart (rare, and bounded by
            // the one-token look-back of the caller).
            *self = SourceCursor::new(self.source);
        }
        while (self.line, self.column) < target {
            let ch = self.source[self.byte..].chars().next()?;
            self.byte += ch.len_utf8();
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
        ((self.line, self.column) == target).then_some(self.byte)
    }
}

fn html_comment_error(form: &str, at: &Token) -> String {
    format!(
        "JavaScript tokenization failed: unsupported Annex B HTML-like comment `{form}` \
         at {}:{} (not modelled; refused so its text can't run as code)",
        at.line, at.column
    )
}

/// A token paired with the correlation-vector ID assigned to it by
/// [`tokenize_javascript_with_cv`].
///
/// The CV ID is a string in the same format that
/// [`CVLog`](coding_adventures_correlation_vector::CVLog) returns — e.g.
/// `"a3f1.1"`. Downstream consumers (the parser, the AST) can look it up
/// in the same `CVLog` they passed in.
#[derive(Debug, Clone)]
pub struct TokenWithCv {
    /// The token as produced by [`tokenize_javascript_typed`]. Fields are
    /// unchanged; nothing about the token itself depends on CV plumbing.
    pub token: Token,
    /// The CV ID assigned to this token. Use it to look up the
    /// `CVEntry` (origin, contributions, parents) in the same `CVLog`.
    pub cv: String,
}

/// Tokenize and assign a fresh correlation-vector ID to every emitted token.
///
/// Per CLOC03 §"Stage 1 — Lexer", every token gets exactly one
/// `CVLog::create(Some(Origin{ ... }))` call with an `Origin` whose
/// `source` is `source_file` and whose `location` is `"line:col"` built
/// from the token's own `line` and `column` fields. No `Contribution` is
/// appended; lexing is creation, not modification.
///
/// `source_file` should be the path or display name of the input file
/// (e.g. `"src/api.js"`). For stdin input, conventions vary — the existing
/// repo uses `"stdin"`. The string ends up in `Origin.source` and is what
/// the source-map generator resolves back to.
///
/// The `cv` log is borrowed mutably for the duration of the call. The same
/// log is then handed to the parser, typechecker, and every downstream
/// pass — see CLOC03 for the full lifecycle.
pub fn tokenize_javascript_with_cv(
    source: &str,
    source_file: &str,
    version: EsVersion,
    cv: &mut CVLog,
) -> Result<Vec<TokenWithCv>, String> {
    let tokens = tokenize_javascript_typed(source, version)?;
    let mut out = Vec::with_capacity(tokens.len());
    for token in tokens {
        let origin = Origin {
            source: source_file.to_string(),
            location: format!("{}:{}", token.line, token.column),
            timestamp: None,
            meta: HashMap::new(),
        };
        let id = cv.try_create(Some(origin))
            .map_err(|e| format!("JavaScript lexer CV creation failed: {e}"))?;
        out.push(TokenWithCv { token, cv: id });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::token::{TokenType, TOKEN_PRECEDED_BY_NEWLINE};

    fn lex(src: &str) -> Result<Vec<Token>, String> {
        tokenize_javascript_typed(src, EsVersion::Es2025)
    }

    #[test]
    fn a_line_leading_html_close_comment_is_refused() {
        // test262 annexB/language/comments/single-line-html-close-asi.js
        assert!(lex("var foo = [23]\n-->[0];").is_err());
        // Its text would otherwise run as code (#17840 review).
        assert!(lex("a = b\n-->console.log(1)").is_err());
        assert!(lex("a\n  /* x */ -->y").is_err());
        assert!(lex("a\n/*\n*/-->b()").is_err());
        assert!(lex("-->x").is_err());
        // Maximal munch fuses the `>` with the comment text (#17840 review).
        assert!(lex("a\n-->>b").is_err());
        assert!(lex("a\n-->=b").is_err());
        assert!(lex("a\r-->>>=b").is_err());
        assert!(lex("-->>x").is_err());
    }

    #[test]
    fn a_line_break_the_line_count_misses_still_counts() {
        // `\r`, U+2028 and U+2029 end a line too; the lexer counts only `\n`.
        assert!(lex("var a=1;a/*\r*/-->console.log(1)").is_err());
        assert!(lex("a\r-->b").is_err());
    }

    #[test]
    fn the_check_is_linear_on_one_long_line() {
        // Many same-line `a-->b` pairs, plus a `\r` so the offset walk runs:
        // offsets come from a forward-only cursor, not a per-token rescan.
        let src = format!("x\r;{}", "a-->b;".repeat(50_000));
        let start = std::time::Instant::now();
        assert!(lex(&src).is_ok());
        assert!(start.elapsed() < std::time::Duration::from_secs(30), "{:?}", start.elapsed());
    }

    #[test]
    fn an_html_open_comment_is_refused() {
        assert!(lex("var r = 1 <!--y").is_err());
        assert!(lex("<!-- x").is_err());
        // The generic subset grammar lexes `--` as two `-` tokens.
        assert!(tokenize_javascript("var r = 1 <!--y", "").is_err());
        assert!(tokenize_javascript("a\n-->b", "").is_err());
        assert!(tokenize_javascript("a - -b", "").is_ok());
        // Hidden in what the lexer takes for a regex after `}` (#17840 review).
        assert!(lex("var x = {}\n/ 2 <!--/; console.log(1)").is_err());
        assert!(lex("var x = function(){}\n/ 2 <!--/; f()").is_err());
        // A line break the REGEX pattern lets through hides a `-->` (round 3).
        assert!(lex("var x = {}\n/ 2\u{2028}-->/; console.log(1)").is_err());
        assert!(lex("var x = {}\n/ 2\u{2029}-->/; console.log(1)").is_err());
        assert!(lex("var x = {}\n/ 2\r-->/; f()").is_err());
        assert!(lex("var r = /a\u{2028}-->/; f()").is_err());
        // A regex that matches HTML comments is fine elsewhere.
        assert!(lex("var re = /<!--.*-->/;").is_ok());
    }

    #[test]
    fn ordinary_decrements_and_comparisons_are_untouched() {
        // Same line: `a-->b` is `a-- > b`.
        assert!(lex("a-->b").is_ok());
        assert!(lex("a\n-- >b").is_ok());
        assert!(lex("a < !--b").is_ok());
        assert!(lex("a<! --b").is_ok());
        // Inside strings, templates, regexes and comments, nothing is a token.
        assert!(lex("x = '-->'; y = `\n-->`; z = /-->/; /* \n--> */ // -->").is_ok());
        assert!(lex("s = '<!--'").is_ok());
    }

    #[test]
    fn division_only_goal_keeps_slashes_separate_in_every_version() {
        for &version in EsVersion::ALL {
            let tokens = tokenize_javascript_division_only("8/2/2;", version).unwrap();
            let values: Vec<_> = tokens.iter().map(|t| t.value.as_str()).collect();
            assert_eq!(&values[..6], &["8", "/", "2", "/", "2", ";"], "{version}");
            assert_eq!(tokens[1].type_, TokenType::Slash);
            assert_eq!(tokens[3].type_, TokenType::Slash);
            assert!(tokens[1].type_name.is_none());
            assert!(tokens[3].type_name.is_none());
            assert!(tokens
                .iter()
                .all(|t| t.type_name.as_deref() != Some("REGEX")));
            // Early editions have no REGEX rule. Only regex-capable editions
            // can prove preservation of a complete regex token.
            let grammar = _grammar::token_grammar(version.as_str()).unwrap();
            if grammar.definitions.iter().any(|rule| rule.name == "REGEX") {
                let full = tokenize_javascript_typed("var r=/x/g;", version).unwrap();
                assert!(
                    full.iter()
                        .any(|t| t.type_name.as_deref() == Some("REGEX") && t.value == "/x/g"),
                    "{version}"
                );
            }
        }
    }

    #[test]
    fn division_only_goal_preserves_trivia_locations_strings_and_errors() {
        let tokens =
            tokenize_javascript_division_only("8 /* / */ / 2\n/ 2", EsVersion::Es2020).unwrap();
        assert_eq!((tokens[1].line, tokens[1].column), (1, 11));
        assert_eq!((tokens[3].line, tokens[3].column), (2, 1));
        assert!(preceded_by_newline(&tokens[3]));
        for source in [
            "'a/b/c'; // / ignored /\n42;",
            "\"/x/\"; /* / ignored / */ 42;",
            "8 /= 2;",
        ] {
            assert_eq!(
                tokenize_javascript_division_only(source, EsVersion::Es2020).unwrap(),
                tokenize_javascript_typed(source, EsVersion::Es2020).unwrap(),
                "{source}"
            );
        }
        for source in ["'unterminated", "\"unterminated", "§"] {
            assert!(tokenize_javascript_division_only(source, EsVersion::Es2020).is_err());
        }
    }

    #[test]
    fn checked_cv_allocation_failure_returns_a_lexer_error() {
        let mut cv = CVLog::new_checked_compact(coding_adventures_correlation_vector::GraphLimits {
            max_nodes: 0,
            ..Default::default()
        })
        .unwrap();
        let error = tokenize_javascript_with_cv("1;", "input.js", EsVersion::Es5, &mut cv).unwrap_err();
        assert!(
            error.contains("CV") && error.contains("nodes limit"),
            "{error}"
        );
        assert!(cv.entries().is_empty());
    }

    fn preceded_by_newline(t: &lexer::token::Token) -> bool {
        t.flags.unwrap_or(0) & TOKEN_PRECEDED_BY_NEWLINE != 0
    }

    #[test]
    fn tokenizes_es5_javascript() {
        let tokens = tokenize_javascript("var x = 1;", "es5").unwrap();
        assert_eq!(tokens[0].type_, TokenType::Keyword);
        assert_eq!(tokens[0].value, "var");
    }

    #[test]
    fn token_after_newline_has_preceded_by_newline_flag() {
        // `a = 1` then `b = 2` on the next line. `b` (index 3) is preceded by a
        // line terminator; the tokens up to and including the `1` are not.
        let tokens = tokenize_javascript("a = 1\nb = 2", "es2025").unwrap();
        // [a, =, 1, b, =, 2, EOF]
        assert_eq!(tokens[3].value, "b");
        assert!(
            preceded_by_newline(&tokens[3]),
            "`b` after a newline must carry TOKEN_PRECEDED_BY_NEWLINE"
        );
        // The `=` and `1` on the first line must NOT carry the flag.
        assert!(!preceded_by_newline(&tokens[1]));
        assert!(!preceded_by_newline(&tokens[2]));
    }

    #[test]
    fn same_line_tokens_have_no_newline_flag() {
        let tokens = tokenize_javascript("a = 1 b = 2", "es2025").unwrap();
        // Everything is on one line; no token is newline-preceded.
        assert!(tokens.iter().all(|t| !preceded_by_newline(t)));
    }

    #[test]
    fn newline_inside_a_template_does_not_set_the_flag() {
        // The `\n` lives INSIDE the multi-line template (consumed by token
        // matching, not trivia), and `x` follows on the same source line as the
        // template's closing backtick — so `x` is NOT preceded by a newline.
        let tokens = tokenize_javascript("var t = `a\nb`; x", "es2025").unwrap();
        let x = tokens
            .iter()
            .find(|t| t.value == "x")
            .expect("found x token");
        assert!(
            !preceded_by_newline(x),
            "a newline inside a template must not flag the following token"
        );
    }

    #[test]
    fn tokenizes_versioned_ecmascript() {
        let tokens = tokenize_javascript("let x = 1;", "es2015").unwrap();
        assert_eq!(tokens[0].type_, TokenType::Keyword);
        assert_eq!(tokens[0].value, "let");
    }

    #[test]
    fn default_version_resolves_to_es2025() {
        let tokens = tokenize_javascript("let x = 1;", DEFAULT_VERSION).unwrap();
        assert_eq!(tokens[0].type_, TokenType::Keyword);
        assert_eq!(tokens[0].value, "let");
    }

    #[test]
    fn all_supported_versions_load() {
        for version in SUPPORTED_VERSIONS {
            let tokens = tokenize_javascript("42;", version).unwrap();
            assert_eq!(tokens[0].type_, TokenType::Number, "version {version:?}");
        }
    }

    #[test]
    fn tokenize_typed_es2015() {
        let tokens = tokenize_javascript_typed("let x = 1;", EsVersion::Es2015).unwrap();
        assert_eq!(tokens[0].type_, TokenType::Keyword);
        assert_eq!(tokens[0].value, "let");
    }

    #[test]
    fn default_es_version_constant_is_es2025() {
        assert_eq!(DEFAULT_ES_VERSION, EsVersion::Es2025);
        // And it must agree with the string DEFAULT_VERSION.
        assert_eq!(DEFAULT_ES_VERSION.as_str(), DEFAULT_VERSION);
    }

    #[test]
    fn all_typed_versions_load() {
        for &version in EsVersion::ALL {
            let tokens = tokenize_javascript_typed("42;", version).unwrap();
            assert_eq!(tokens[0].type_, TokenType::Number, "version {version}");
        }
    }

    #[test]
    fn create_lexer_typed_returns_grammar_lexer() {
        // Constructor returns infallibly (no unknown-version path).
        let _lexer = create_javascript_lexer_typed("var x = 1;", EsVersion::Es5);
    }

    // gap-096 (CLOC12.99) regression: the ES2024/ES2025 REGEX token's
    // flag character class once read `[dgimsvy]`, accidentally dropping
    // the ES2015 `u` (unicode) flag when `v` (unicodeSets) was added.
    // A regex carrying every modern flag must lex as ONE token, not a
    // truncated regex followed by a stray identifier of the leftover
    // flags. We assert the whole `/x/dgimsuy` literal survives intact.
    #[test]
    fn es2025_regex_accepts_all_modern_flags_as_one_token() {
        let tokens =
            tokenize_javascript_typed("var r=/x/dgimsuy;", EsVersion::Es2025).unwrap();
        let regex = tokens
            .iter()
            .find(|t| t.value.starts_with("/x/"))
            .expect("expected a single regex token beginning with /x/");
        assert_eq!(
            regex.value, "/x/dgimsuy",
            "all of d,g,i,m,s,u,y must be consumed as part of the regex; \
             a split here means a flag is missing from the grammar's class"
        );
        // And crucially there is NO stray identifier `uy` left behind.
        assert!(
            tokens.iter().all(|t| t.value != "uy" && t.value != "u"),
            "regex flags must not split off into a separate identifier"
        );
    }

    // The same flag set under ES2024 (the other grammar that carried the
    // typo) must likewise lex as one token.
    #[test]
    fn es2024_regex_accepts_u_flag() {
        let tokens =
            tokenize_javascript_typed("var r=/x/gimsuy;", EsVersion::Es2024).unwrap();
        let regex = tokens
            .iter()
            .find(|t| t.value.starts_with("/x/"))
            .expect("expected a single regex token beginning with /x/");
        assert_eq!(regex.value, "/x/gimsuy");
    }

    // ----- CV-plumbed tokenization (CLOC03 Stage 1) -----

    #[test]
    fn tokenize_with_cv_assigns_an_id_per_token() {
        let mut cv = CVLog::new(true);
        let tokens =
            tokenize_javascript_with_cv("var x = 1;", "src/test.js", EsVersion::Es5, &mut cv)
                .unwrap();
        assert!(!tokens.is_empty(), "expected at least one token");
        for t in &tokens {
            assert!(!t.cv.is_empty(), "expected a non-empty CV id");
        }
    }

    #[test]
    fn tokenize_with_cv_ids_are_unique() {
        let mut cv = CVLog::new(true);
        let tokens =
            tokenize_javascript_with_cv("var x = 1; var y = 2;", "u.js", EsVersion::Es5, &mut cv)
                .unwrap();
        let mut ids: Vec<&str> = tokens.iter().map(|t| t.cv.as_str()).collect();
        let len = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), len, "all CV ids should be unique");
    }

    #[test]
    fn tokenize_with_cv_entries_resolvable_in_log() {
        let mut cv = CVLog::new(true);
        let tokens =
            tokenize_javascript_with_cv("var x = 1;", "lookup.js", EsVersion::Es5, &mut cv)
                .unwrap();
        for t in &tokens {
            let entry = cv
                .get(&t.cv)
                .unwrap_or_else(|| panic!("CV id {:?} not found in log", t.cv));
            let origin = entry
                .origin
                .as_ref()
                .expect("token CV must have an Origin");
            assert_eq!(origin.source, "lookup.js");
            // location is "line:col" — must contain a colon.
            assert!(
                origin.location.contains(':'),
                "expected line:col location, got {:?}",
                origin.location
            );
        }
    }

    // ----- gap-044b: template literals with non-identifier expressions -----
    //
    // Before the fix, any substitution expression that triggered an F10
    // flat-mode transition (e.g. `on NAME -> set-mode div`) caused the lexer
    // to lose the template context.  A subsequent `}` was then consumed as
    // RBRACE instead of TEMPLATE_TAIL, producing a LexerError.
    //
    // We verify six representative shapes; each must tokenize successfully
    // and the closing `}...`` token must be classified as TEMPLATE_TAIL.

    fn find_template_tail(tokens: &[lexer::token::Token]) -> Option<&lexer::token::Token> {
        tokens.iter().find(|t| t.type_name.as_deref() == Some("TEMPLATE_TAIL"))
    }

    #[test]
    fn gap044b_template_member_expr_no_lexer_error() {
        // `${obj.name}` — DOT triggers set-mode default, then NAME sets div
        let tokens = tokenize_javascript_typed(
            "var s = `prefix ${obj.name} suffix`;",
            EsVersion::Es2025,
        ).expect("should tokenize `${obj.name}` without LexerError");
        assert!(find_template_tail(&tokens).is_some(),
            "expected a TEMPLATE_TAIL token after `${{obj.name}}`");
    }

    #[test]
    fn gap044b_template_binary_expr_no_lexer_error() {
        // `${a + b}` — PLUS triggers set-mode default, then b sets div
        let tokens = tokenize_javascript_typed(
            "var s = `sum ${a + b} end`;",
            EsVersion::Es2025,
        ).expect("should tokenize `${{a + b}}` without LexerError");
        assert!(find_template_tail(&tokens).is_some(),
            "expected a TEMPLATE_TAIL token after `${{a + b}}`");
    }

    #[test]
    fn gap044b_template_call_expr_no_lexer_error() {
        // `${f()}` — LPAREN triggers set-mode default, RPAREN triggers set-mode div
        let tokens = tokenize_javascript_typed(
            "var s = `call ${f()} end`;",
            EsVersion::Es2025,
        ).expect("should tokenize `${{f()}}` without LexerError");
        assert!(find_template_tail(&tokens).is_some(),
            "expected a TEMPLATE_TAIL token after `${{f()}}`");
    }

    #[test]
    fn gap044b_template_object_expr_no_lexer_error() {
        // `${{a:1}}` — nested braces inside the substitution
        let tokens = tokenize_javascript_typed(
            "var s = `obj ${{a:1}} end`;",
            EsVersion::Es2025,
        ).expect("should tokenize template with nested object literal without LexerError");
        assert!(find_template_tail(&tokens).is_some(),
            "expected a TEMPLATE_TAIL token after nested object literal in template");
    }

    #[test]
    fn gap044b_template_ternary_expr_no_lexer_error() {
        // `${x ? y : z}` — COLON triggers set-mode default, z NAME sets div
        let tokens = tokenize_javascript_typed(
            "var s = `tern ${x ? y : z} end`;",
            EsVersion::Es2025,
        ).expect("should tokenize ternary template without LexerError");
        assert!(find_template_tail(&tokens).is_some(),
            "expected a TEMPLATE_TAIL token after ternary expression in template");
    }

    #[test]
    fn gap044b_template_multiple_substitutions_no_lexer_error() {
        // Two substitutions: each `}${` is TEMPLATE_MIDDLE, closing `}` is TEMPLATE_TAIL
        let tokens = tokenize_javascript_typed(
            "var s = `${a.x} mid ${b.y} end`;",
            EsVersion::Es2025,
        ).expect("should tokenize multiple substitutions without LexerError");
        assert!(find_template_tail(&tokens).is_some(),
            "expected a TEMPLATE_TAIL token in multi-substitution template");
        let middles: Vec<_> = tokens.iter()
            .filter(|t| t.type_name.as_deref() == Some("TEMPLATE_MIDDLE"))
            .collect();
        assert_eq!(middles.len(), 1,
            "expected exactly one TEMPLATE_MIDDLE between two substitutions");
    }

    #[test]
    fn gap044b_simple_template_still_works() {
        // Sanity check: simple `${x}` (only NAME, no operators) must still work.
        let tokens = tokenize_javascript_typed(
            "var s = `hello ${x}!`;",
            EsVersion::Es2025,
        ).expect("simple template must tokenize");
        assert!(find_template_tail(&tokens).is_some(),
            "expected TEMPLATE_TAIL for simple template substitution");
    }

    #[test]
    fn tokenize_with_cv_disabled_log_still_returns_tokens() {
        // Per CLOC03, when the log is disabled, create() still returns IDs
        // (so call sites stay shape-identical) but no entries get stored.
        let mut cv = CVLog::new(false);
        let tokens =
            tokenize_javascript_with_cv("var x = 1;", "off.js", EsVersion::Es5, &mut cv).unwrap();
        assert!(!tokens.is_empty());
        // Log has no entries, but tokens do have (synthetic) IDs.
        for t in &tokens {
            assert!(!t.cv.is_empty());
        }
    }
}
