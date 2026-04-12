//! F# lexer backed by compiled versioned token grammars.

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

mod _grammar;

pub const SUPPORTED_VERSIONS: &[&str] = _grammar::SUPPORTED_VERSIONS;
pub const DEFAULT_VERSION: &str = "10";

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
            "Unknown F# version '{version}'. Valid values: {}",
            SUPPORTED_VERSIONS
                .iter()
                .map(|value| format!("\"{}\"", value))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

pub fn create_fsharp_lexer<'src>(
    source: &'src str,
    version: &str,
) -> Result<GrammarLexer<'src>, String> {
    let version = resolve_version(version)?;
    let grammar =
        _grammar::token_grammar(version).expect("compiled F# token grammar missing supported version");
    Ok(GrammarLexer::new(source, &grammar))
}

pub fn tokenize_fsharp(source: &str, version: &str) -> Result<Vec<Token>, String> {
    let version = resolve_version(version)?;
    let grammar =
        _grammar::token_grammar(version).expect("compiled F# token grammar missing supported version");
    let mut lexer = GrammarLexer::new(source, &grammar);
    lexer
        .tokenize()
        .map_err(|e| format!("F# tokenization failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::token::TokenType;

    #[test]
    fn tokenizes_let_binding() {
        let tokens = tokenize_fsharp("let value = 1", "").unwrap();
        assert_eq!(tokens[0].type_, TokenType::Keyword);
        assert_eq!(tokens[0].value, "let");
    }

    #[test]
    fn all_supported_versions_load() {
        for version in SUPPORTED_VERSIONS {
            let tokens = tokenize_fsharp("let value = 1", version).unwrap();
            assert_eq!(tokens.last().unwrap().type_, TokenType::Eof, "version {version}");
        }
    }

    #[test]
    fn unknown_version_returns_error() {
        let error = tokenize_fsharp("let value = 1", "11").unwrap_err();
        assert!(error.contains("11"));
    }
}
