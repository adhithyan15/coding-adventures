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
use grammar_tools::token_grammar::{PatternGroup, TokenDefinition, TokenGrammar};
use lexer::grammar_lexer::{GrammarLexer, LexerContext};
use lexer::token::{Token, TOKEN_PRECEDED_BY_NEWLINE};
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

// ===========================================================================
// Annex B: SingleLineHTMLCloseComment (`-->` at the start of a line)
// ===========================================================================
//
// Browsers have always let a script hide itself from pre-`<script>` browsers
// inside an HTML comment, and ECMA-262 Annex B §B.1.1 ("HTML-like Comments")
// keeps that working. One of its forms is the *close* comment: a `-->` that
// starts a line (only whitespace and `/* … */` comments may come before it on
// that line) runs to the end of the line and is ignored, exactly like `//`:
//
// ```text
//     var foo = [23]
//     -->[0];            ← all of `-->[0];` is a comment
//     foo[0] === 23      ← so `foo` is still the array [23]
// ```
//
// The token grammars cannot say "at the start of a line" (a skip pattern is
// a regex anchored at the current position, with no look-behind), so the
// grammar alone lexes that `-->` as `--` then `>`. The lexer does know which
// tokens a line terminator preceded: it sets TOKEN_PRECEDED_BY_NEWLINE from
// the trivia it skipped, and that trivia includes any `/* … */` comments. So
// an on-token callback recognises the comment where the grammar can't:
//
// ```text
//   MINUS_MINUS, line-leading, next char `>`   →  drop it, push the
//                                                 `html_close_comment` group
//   HTML_CLOSE_COMMENT_REST (`>` … end of line) →  drop it, pop the group
// ```
//
// The group holds that single pattern and is exclusive (it is never a
// `set-mode` target, so it does not inherit the default patterns). Both
// tokens are suppressed, so no F10 mode transition fires on them and the
// regex-vs-division mode on the far side of the comment is the one before it.
// The line terminator that ends the comment is ordinary skipped trivia, so the
// next token is still flagged as newline-preceded, which is what ASI needs.
//
// "Line-leading" also covers the very start of the input. ES2024 test262
// accepts `-->` as the first thing in a Script.
//
// This applies to every edition and to module code too, which Annex B does
// not ask for. It is safe because, read as code, a line-leading `-->` can
// never be part of a valid program: a postfix `--` may not follow a line
// terminator, so the `--` would have to be a prefix operator, and `>` cannot
// start its operand. Treating it as a comment therefore changes no program
// that was valid before. (Before this, javascript-parser's newline-led
// `++`/`--` split, #17840, turned `[23]⏎-->[0]` into `[23]; -- > [0]` and
// rejected the program. Before that split, the grammar accepted it only by
// misreading it as `[23]-- > [0]`.)
//
// The *open* comment `<!--` is not handled. Unlike `-->`, it changes the
// meaning of valid module code (`a<!--b` is `a < !(--b)` there), so it needs
// a script-vs-module goal that the lexer does not have.

/// The pattern group pushed after a line-leading `--` whose next char is `>`.
const HTML_CLOSE_COMMENT_GROUP: &str = "html_close_comment";

/// The rest of an HTML close comment: the `>` of `-->`, then everything up to
/// (not including) the next LineTerminator (LF, CR, LS or PS).
const HTML_CLOSE_COMMENT_REST: &str = "HTML_CLOSE_COMMENT_REST";

/// Build the GrammarLexer every entry point in this crate uses.
///
/// Sources that do not contain `-->` at all (almost all of them) get the plain
/// grammar lexer with no callback, so they pay nothing for Annex B. The others
/// get the `html_close_comment` group and [`skip_html_close_comment`].
fn javascript_lexer<'src>(source: &'src str, mut grammar: TokenGrammar) -> GrammarLexer<'src> {
    if !source.contains("-->") {
        return GrammarLexer::new(source, &grammar);
    }
    grammar.groups.insert(
        HTML_CLOSE_COMMENT_GROUP.to_string(),
        PatternGroup {
            name: HTML_CLOSE_COMMENT_GROUP.to_string(),
            definitions: vec![TokenDefinition {
                name: HTML_CLOSE_COMMENT_REST.to_string(),
                pattern: r">[^\n\r\u{2028}\u{2029}]*".to_string(),
                is_regex: true,
                line_number: 0,
                alias: None,
            }],
        },
    );
    let mut lexer = GrammarLexer::new(source, &grammar);
    lexer.set_on_token(Some(Box::new(skip_html_close_comment)));
    lexer
}

/// The on-token callback that turns a line-leading `-->` into a comment. See
/// the section comment above for why this lives in the lexer.
fn skip_html_close_comment(token: &Token, ctx: &mut LexerContext) {
    match token.type_name.as_deref() {
        Some("MINUS_MINUS") if starts_line(token, ctx) && ctx.peek(1) == ">" => {
            ctx.suppress();
            ctx.push_group(HTML_CLOSE_COMMENT_GROUP)
                .expect("javascript_lexer registers the html_close_comment group");
        }
        Some(HTML_CLOSE_COMMENT_REST) => {
            ctx.suppress();
            ctx.pop_group();
        }
        _ => {}
    }
}

/// Whether `token` is the first token on its line: a line terminator was
/// skipped since the previous token, or nothing has been emitted yet.
fn starts_line(token: &Token, ctx: &LexerContext) -> bool {
    token.flags.unwrap_or(0) & TOKEN_PRECEDED_BY_NEWLINE != 0 || ctx.previous_token().is_none()
}

pub fn create_javascript_lexer<'src>(
    source: &'src str,
    version: &str,
) -> Result<GrammarLexer<'src>, String> {
    let version = validate_version(version)?;
    let grammar = _grammar::token_grammar(version)
        .expect("compiled JavaScript token grammar missing supported version");
    Ok(javascript_lexer(source, grammar))
}

pub fn tokenize_javascript(source: &str, version: &str) -> Result<Vec<Token>, String> {
    let version = validate_version(version)?;
    let grammar = _grammar::token_grammar(version)
        .expect("compiled JavaScript token grammar missing supported version");
    let mut lexer = javascript_lexer(source, grammar);
    lexer
        .tokenize()
        .map_err(|e| format!("JavaScript tokenization failed: {e}"))
}

/// Typed version of [`create_javascript_lexer`]. Takes an [`EsVersion`]
/// directly; cannot fail with an unknown-version error.
pub fn create_javascript_lexer_typed<'src>(
    source: &'src str,
    version: EsVersion,
) -> GrammarLexer<'src> {
    let grammar = _grammar::token_grammar(version.as_str())
        .expect("compiled JavaScript token grammar missing supported version");
    javascript_lexer(source, grammar)
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
    let mut lexer = javascript_lexer(source, grammar);
    lexer
        .tokenize()
        .map_err(|e| format!("JavaScript tokenization failed: {e}"))
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
    let mut lexer = javascript_lexer(source, grammar);
    lexer
        .tokenize()
        .map_err(|e| format!("JavaScript tokenization failed: {e}"))
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

    /// Token values (empty EOF dropped) for an ES2025 lex.
    fn values(src: &str) -> Vec<String> {
        tokenize_javascript_typed(src, EsVersion::Es2025)
            .unwrap()
            .into_iter()
            .map(|t| t.value)
            .filter(|v| !v.is_empty())
            .collect()
    }

    #[test]
    fn block_comments_ending_in_extra_stars_are_comments() {
        // `([^*]|\*[^/])*` ate the `*` that starts `*/` (#17888).
        for src in ["/***/ a", "/* x **/ a", "/****/ a", "/* * / */ a", "/**\n * doc\n **/ a"] {
            assert_eq!(values(src), vec!["a"], "{src:?}");
        }
    }

    #[test]
    fn line_comments_and_hashbangs_end_at_a_carriage_return() {
        // `\r` ends a line in JavaScript; `[^\n]*` swallowed the code after it.
        assert_eq!(values("a // c\rb"), vec!["a", "b"]);
        // HASHBANG is a token; it ends at the `\r`, and `b` is code.
        assert_eq!(values("#!/usr/bin/env node\rb"), vec!["#!/usr/bin/env node", "b"]);
    }

    #[test]
    fn a_regex_never_spans_a_carriage_return() {
        // A regex literal can't hold a line terminator, escaped or not.
        let toks = tokenize_javascript_typed("x = /a\rb/", EsVersion::Es2025);
        assert!(toks.map_or(true, |t| t.iter().all(|t| t.type_name.as_deref() != Some("REGEX"))));
        let toks = tokenize_javascript_typed("x = /a\\\rb/", EsVersion::Es2025);
        assert!(toks.map_or(true, |t| t.iter().all(|t| t.type_name.as_deref() != Some("REGEX"))));
        // Ordinary regexes still lex.
        assert!(values("x = /a\\/b/g").contains(&"/a\\/b/g".to_string()));
    }

    #[test]
    fn a_dollar_before_the_closing_backtick_ends_the_template() {
        // `\$[^{]` consumed `$` + backtick, so `\`$\`` ran on to the next
        // backtick, swallowing code (#17886).
        assert_eq!(values("x = `$` + y"), vec!["x", "=", "`$`", "+", "y"]);
        assert_eq!(values("x = `a$$` ; y"), vec!["x", "=", "`a$$`", ";", "y"]);
        // `$${` is a literal `$` then a substitution.
        let toks = tokenize_javascript_typed("x = `$${y}`", EsVersion::Es2025).unwrap();
        let kinds: Vec<_> = toks.iter().filter_map(|t| t.type_name.clone()).collect();
        assert!(kinds.iter().any(|k| k == "TEMPLATE_HEAD"), "{kinds:?}");
        assert!(kinds.iter().any(|k| k == "TEMPLATE_TAIL"), "{kinds:?}");
        // `$` before a substitution's closing tail, and a lone `$x`.
        assert!(tokenize_javascript_typed("x = `${a}$`", EsVersion::Es2025).is_ok());
        assert_eq!(values("x = `$a`"), vec!["x", "=", "`$a`"]);
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

    // -----------------------------------------------------------------------
    // Annex B SingleLineHTMLCloseComment (test262
    // annexB/language/comments/single-line-html-close*.js)
    // -----------------------------------------------------------------------

    fn token_values(tokens: &[lexer::token::Token]) -> Vec<&str> {
        tokens.iter().map(|t| t.value.as_str()).collect()
    }

    #[test]
    fn line_leading_html_close_comment_is_skipped_in_every_version() {
        // test262 single-line-html-close-asi.js: `-->[0];` is a comment, so
        // `foo` stays the array and the next line is a new statement.
        for &version in EsVersion::ALL {
            let tokens =
                tokenize_javascript_typed("var foo = [23]\n-->[0];\nfoo", version).unwrap();
            assert_eq!(
                token_values(&tokens),
                ["var", "foo", "=", "[", "23", "]", "foo", ""],
                "{version}"
            );
            assert!(preceded_by_newline(&tokens[6]), "{version}");
        }
    }

    #[test]
    fn html_close_comment_may_follow_whitespace_and_comments_on_its_line() {
        // Every HTMLCloseComment shape test262 uses: after a line terminator,
        // then optional whitespace and same-line `/* */` comments; after a
        // `/* */` that itself contains a line terminator (even when a token
        // precedes it on its first line); and at the very start of the input.
        for src in [
            "a\n-->",
            "a\n-->the comment extends to these characters\nb",
            "a\n   -->x\nb",
            "a\n/* x */-->x\nb",
            "a\n/**/ /* y */-->x\nb",
            "a\n/*\n*/-->x\nb",
            "a\n0/*\n*/-->x\nb",
            "--> a comment\nb",
            "   --> a comment\nb",
            "/* a */ /* b */--> a comment\nb",
            "a\r\n-->x\r\nb",
            "a\n-->>= not an operator\nb",
            "a\n--> unterminated ' \" ` /* are all comment text\nb",
        ] {
            let tokens = tokenize_javascript_typed(src, EsVersion::Es2025)
                .unwrap_or_else(|e| panic!("{src:?}: {e}"));
            assert!(
                tokens.iter().all(|t| t.value != "--" && t.value != ">"),
                "{src:?} -> {:?}",
                token_values(&tokens)
            );
            let last = &tokens[tokens.len() - 2];
            if last.value == "b" {
                assert!(preceded_by_newline(last), "{src:?}");
            }
        }
    }

    #[test]
    fn html_close_comment_ends_at_every_line_terminator() {
        // SingleLineCommentChars stop at LF, CR, LS and PS.
        let tokens = tokenize_javascript_typed("a\n-->x\rb", EsVersion::Es2025).unwrap();
        assert_eq!(token_values(&tokens), ["a", "b", ""]);
    }

    #[test]
    fn dash_dash_gt_that_does_not_lead_its_line_is_still_code() {
        // Mid-line, `-->` is `--` then `>`; `-- >` is never a comment.
        assert_eq!(
            token_values(&tokenize_javascript_typed("a-->b", EsVersion::Es2025).unwrap()),
            ["a", "--", ">", "b", ""]
        );
        assert_eq!(
            token_values(&tokenize_javascript_typed("a\n-- >b", EsVersion::Es2025).unwrap()),
            ["a", "--", ">", "b", ""]
        );
        // A `-->` at the start of a line inside a template literal or a
        // block comment is part of that token or comment, not a comment of
        // its own.
        let template = tokenize_javascript_typed("x = `\n-->`;", EsVersion::Es2025).unwrap();
        assert_eq!(template[2].value, "`\n-->`");
        let block = tokenize_javascript_typed("a /*\n-->*/ - b", EsVersion::Es2025).unwrap();
        assert_eq!(token_values(&block), ["a", "-", "b", ""]);
    }

    #[test]
    fn html_close_comment_keeps_the_regex_or_division_mode() {
        // After `a` a `/` is division. The suppressed comment tokens fire no
        // F10 transition (an emitted `>` would switch to regex mode and lex
        // `/b/g` as one REGEX).
        let tokens = tokenize_javascript_typed("a\n-->x\n/b/g", EsVersion::Es2025).unwrap();
        assert_eq!(token_values(&tokens), ["a", "/", "b", "/", "g", ""]);
        // After `=` a `/` starts a regex, and still does past the comment.
        let tokens = tokenize_javascript_typed("a =\n-->x\n/b/g", EsVersion::Es2025).unwrap();
        assert_eq!(token_values(&tokens), ["a", "=", "/b/g", ""]);
    }

    #[test]
    fn every_entry_point_skips_html_close_comments() {
        let src = "a\n-->x\nb";
        let expected = ["a", "b", ""];
        assert_eq!(token_values(&tokenize_javascript(src, "es2025").unwrap()), expected);
        assert_eq!(
            token_values(&tokenize_javascript_division_only(src, EsVersion::Es2025).unwrap()),
            expected
        );
        assert_eq!(
            token_values(&create_javascript_lexer(src, "es2025").unwrap().tokenize().unwrap()),
            expected
        );
        assert_eq!(
            token_values(&create_javascript_lexer_typed(src, EsVersion::Es2025).tokenize().unwrap()),
            expected
        );
        let mut cv = CVLog::new(true);
        let with_cv = tokenize_javascript_with_cv(src, "in.js", EsVersion::Es2025, &mut cv).unwrap();
        assert_eq!(with_cv.len(), 3);
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
