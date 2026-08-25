//! Java lexer backed by compiled versioned token grammars.

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

mod _grammar;

pub const SUPPORTED_VERSIONS: &[&str] = _grammar::SUPPORTED_VERSIONS;
pub const DEFAULT_VERSION: &str = "21";

fn validate_version(version: &str) -> Result<&str, String> {
    if SUPPORTED_VERSIONS.contains(&version) {
        Ok(version)
    } else {
        Err(format!(
            "Unknown Java version '{version}'. Valid values: {}",
            SUPPORTED_VERSIONS
                .iter()
                .map(|value| format!("\"{}\"", value))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

pub fn create_java_lexer<'src>(
    source: &'src str,
    version: &str,
) -> Result<GrammarLexer<'src>, String> {
    let version = validate_version(version)?;
    let grammar =
        _grammar::token_grammar(version).expect("compiled Java token grammar missing supported version");
    Ok(GrammarLexer::new(source, &grammar))
}

pub fn tokenize_java(source: &str, version: &str) -> Result<Vec<Token>, String> {
    let version = validate_version(version)?;
    let grammar =
        _grammar::token_grammar(version).expect("compiled Java token grammar missing supported version");
    let mut lexer = GrammarLexer::new(source, &grammar);
    lexer
        .tokenize()
        .map_err(|e| format!("Java tokenization failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::token::TokenType;

    #[test]
    fn tokenizes_basic_class() {
        let tokens = tokenize_java("class Hello { }", "21").unwrap();
        assert_eq!(tokens[0].type_, TokenType::Keyword);
        assert_eq!(tokens[0].value, "class");
    }

    #[test]
    fn all_supported_versions_load() {
        for version in SUPPORTED_VERSIONS {
            let tokens = tokenize_java("42;", version).unwrap();
            assert_eq!(tokens[0].type_, TokenType::Number, "version {version}");
        }
    }

    #[test]
    fn unknown_version_returns_error() {
        let error = tokenize_java("int x = 1;", "99").unwrap_err();
        assert!(error.contains("99"));
    }

    // ── Real construct coverage (JV02 M0: the original 3 tests here only
    // exercised a bare `class Hello { }` and a bare `42;` — nothing that
    // exercises the token shapes an actual interface/generic/lambda/
    // exception/annotation source file needs). This crate's `TokenType`
    // enum (shared with every other grammar-tools-based lexer) has no
    // dedicated variant for `<`/`>`/`->`/`@`/`&`/`::`/`...` — they all fall
    // through to the generic `Name` type, with the literal symbol in
    // `.value`. That means the *parser*, not the lexer, is what
    // disambiguates them; these tests pin the one thing the lexer itself
    // is responsible for getting right: which characters group into which
    // token, and in what order.

    #[test]
    fn interface_keyword_and_generic_angle_brackets_tokenize() {
        let tokens = tokenize_java("interface Shape<T> { }", "21").unwrap();
        assert_eq!(tokens[0].type_, TokenType::Keyword);
        assert_eq!(tokens[0].value, "interface");
        // `<` / `>` have no dedicated TokenType (see module doc above) --
        // both fall through to `Name`, distinguished only by `.value`.
        let values: Vec<&str> = tokens.iter().map(|t| t.value.as_str()).collect();
        assert_eq!(
            values,
            vec!["interface", "Shape", "<", "T", ">", "{", "}", ""]
        );
    }

    #[test]
    fn lambda_arrow_is_a_single_token_not_minus_then_greater_than() {
        // `->` must NOT split into a `-` token followed by a `>` token --
        // that would make every lambda unparseable downstream. This is the
        // one lexer-level fact a lambda-bearing frontend needs guaranteed.
        let tokens = tokenize_java("Runnable r = () -> f();", "21").unwrap();
        let arrow = tokens.iter().find(|t| t.value == "->");
        assert!(
            arrow.is_some(),
            "expected a single \"->\" token, got: {:?}",
            tokens.iter().map(|t| &t.value).collect::<Vec<_>>()
        );
        assert!(
            !tokens.iter().any(|t| t.value == "-"),
            "arrow must not have been split into a bare \"-\" token"
        );
    }

    #[test]
    fn nested_generic_closing_angle_brackets_merge_into_one_token() {
        // `List<Integer>>` -- the LAST TWO `>` characters merge into a
        // single ">>"-valued token at the lexer level, exactly like a
        // real right-shift operator would. This is not a bug in the
        // lexer (a context-free lexer cannot know it should NOT merge
        // them without knowing it's inside a generic-argument list) --
        // it is the reason the *parser* needs its own contextual
        // token-splitting logic to re-derive two separate `>` closers.
        // Pinned here so a future lexer change that silently "fixes"
        // this by *not* merging doesn't go unnoticed -- the parser-side
        // fix (tracked separately) depends on this exact merged shape.
        let tokens = tokenize_java("List<List<Integer>> x;", "21").unwrap();
        let values: Vec<&str> = tokens.iter().map(|t| t.value.as_str()).collect();
        assert!(
            values.contains(&">>"),
            "expected a merged \">>\" token, got: {:?}",
            values
        );
        assert!(
            !values.contains(&">"),
            "did not expect a separate \">\" token once merging occurred, got: {:?}",
            values
        );
    }

    #[test]
    fn annotation_at_sign_and_identifier_are_separate_tokens() {
        let tokens = tokenize_java("@Override", "21").unwrap();
        let values: Vec<&str> = tokens.iter().map(|t| t.value.as_str()).collect();
        assert_eq!(values, vec!["@", "Override", ""]);
    }

    #[test]
    fn varargs_ellipsis_is_a_single_token() {
        let tokens = tokenize_java("void f(int... xs) { }", "21").unwrap();
        assert!(
            tokens.iter().any(|t| t.value == "..."),
            "expected a single \"...\" token, got: {:?}",
            tokens.iter().map(|t| &t.value).collect::<Vec<_>>()
        );
    }

    #[test]
    fn method_reference_double_colon_is_a_single_token() {
        let tokens = tokenize_java("xs::hashCode", "21").unwrap();
        let values: Vec<&str> = tokens.iter().map(|t| t.value.as_str()).collect();
        assert_eq!(values, vec!["xs", "::", "hashCode", ""]);
    }

    #[test]
    fn try_catch_finally_and_throw_keywords_tokenize() {
        let tokens = tokenize_java(
            "try { throw new E(); } catch (E e) { } finally { }",
            "21",
        )
        .unwrap();
        let keywords: Vec<&str> = tokens
            .iter()
            .filter(|t| t.type_ == TokenType::Keyword)
            .map(|t| t.value.as_str())
            .collect();
        for kw in ["try", "throw", "new", "catch", "finally"] {
            assert!(
                keywords.contains(&kw),
                "expected keyword \"{kw}\" among {keywords:?}"
            );
        }
    }

    #[test]
    fn string_and_char_literals_tokenize_with_escapes() {
        let tokens = tokenize_java(r#"String s = "a\nb"; char c = 'x';"#, "21").unwrap();
        let string_tok = tokens
            .iter()
            .find(|t| t.type_ == TokenType::String)
            .expect("expected a String token");
        assert_eq!(string_tok.value, "a\nb", "escape sequence must be decoded");
    }
}
