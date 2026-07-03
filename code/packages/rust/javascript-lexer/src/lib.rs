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
        let id = cv.create(Some(origin));
        out.push(TokenWithCv { token, cv: id });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::token::{TokenType, TOKEN_PRECEDED_BY_NEWLINE};

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
