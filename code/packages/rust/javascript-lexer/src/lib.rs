//! JavaScript lexer backed by compiled ECMAScript token grammars (es1 through es2025).

use coding_adventures_javascript_tokens::EsVersion;
use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

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

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::token::TokenType;

    #[test]
    fn tokenizes_es5_javascript() {
        let tokens = tokenize_javascript("var x = 1;", "es5").unwrap();
        assert_eq!(tokens[0].type_, TokenType::Keyword);
        assert_eq!(tokens[0].value, "var");
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
}
