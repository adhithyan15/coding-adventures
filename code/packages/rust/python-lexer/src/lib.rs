//! Python lexer backed by compiled versioned token grammars.

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

mod _grammar;

pub const SUPPORTED_VERSIONS: &[&str] = _grammar::SUPPORTED_VERSIONS;
pub const DEFAULT_VERSION: &str = "3.12";

fn resolve_version(version: &str) -> Result<&str, String> {
    let resolved = if version.is_empty() {
        DEFAULT_VERSION
    } else {
        version
    };

    if SUPPORTED_VERSIONS.contains(&resolved) {
        Ok(resolved)
    } else {
        Err(format!(
            "Unsupported Python version '{version}'. Supported versions: {}",
            SUPPORTED_VERSIONS
                .iter()
                .map(|value| format!("\"{}\"", value))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

pub fn create_python_lexer<'src>(
    source: &'src str,
    version: &str,
) -> Result<GrammarLexer<'src>, String> {
    let version = resolve_version(version)?;
    let grammar = _grammar::token_grammar(version)
        .expect("compiled Python token grammar missing supported version");
    Ok(GrammarLexer::new(source, &grammar))
}

pub fn tokenize_python(source: &str, version: &str) -> Result<Vec<Token>, String> {
    let version = resolve_version(version)?;
    let grammar = _grammar::token_grammar(version)
        .expect("compiled Python token grammar missing supported version");
    let mut lexer = GrammarLexer::new(source, &grammar);
    lexer
        .tokenize()
        .map_err(|e| format!("Python tokenization failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::token::TokenType;

    #[test]
    fn tokenizes_python_with_default_version() {
        let tokens = tokenize_python("x = 1\n", "").unwrap();
        assert_eq!(tokens[0].type_, TokenType::Name);
        assert_eq!(tokens[0].value, "x");
    }

    #[test]
    fn supports_indentation_tokens() {
        let tokens = tokenize_python("def f():\n    return 1\n", "3.12").unwrap();
        assert!(tokens.iter().any(|token| token.type_ == TokenType::Indent));
        assert!(tokens.iter().any(|token| token.type_ == TokenType::Dedent));
    }

    #[test]
    fn soft_keywords_remain_names() {
        let tokens = tokenize_python("match = 42\n", "3.12").unwrap();
        assert_eq!(tokens[0].type_, TokenType::Name);
        assert_eq!(tokens[0].value, "match");
    }

    #[test]
    fn all_supported_versions_load() {
        for version in SUPPORTED_VERSIONS {
            let tokens = tokenize_python("x = 1\n", version).unwrap();
            assert_eq!(tokens.last().unwrap().type_, TokenType::Eof, "version {version}");
        }
    }

    #[test]
    fn unsupported_version_returns_error() {
        let error = tokenize_python("x = 1\n", "4.0").unwrap_err();
        assert!(error.contains("4.0"));
    }
}
