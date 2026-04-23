//! Haskell parser backed by compiled versioned parser grammars.

use coding_adventures_haskell_lexer::tokenize_haskell;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

mod _grammar;

fn validate_version(version: &str) -> Result<&str, String> {
    if _grammar::SUPPORTED_VERSIONS.contains(&version) {
        Ok(version)
    } else {
        Err(format!(
            "Unknown Haskell version '{version}'. Valid values: {}",
            _grammar::SUPPORTED_VERSIONS
                .iter()
                .map(|value| format!("\"{}\"", value))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

pub fn create_haskell_parser(source: &str, version: &str) -> Result<GrammarParser, String> {
    let version = validate_version(version)?;
    let tokens = tokenize_haskell(source, version)?;
    let grammar = _grammar::parser_grammar(version)
        .expect("compiled Haskell parser grammar missing supported version");
    Ok(GrammarParser::new(tokens, grammar))
}

pub fn parse_haskell(source: &str, version: &str) -> Result<GrammarASTNode, String> {
    let mut parser = create_haskell_parser(source, version)?;
    parser
        .parse()
        .map_err(|e| format!("Haskell parse failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_name() {
        let ast = parse_haskell("x", "2010").unwrap();
        assert_eq!(ast.rule_name, "file");
    }

    #[test]
    fn all_supported_versions_load() {
        for version in _grammar::SUPPORTED_VERSIONS {
            let ast = parse_haskell("x", version).unwrap();
            assert_eq!(ast.rule_name, "file", "version {version}");
        }
    }

    #[test]
    fn unknown_version_returns_error() {
        let error = parse_haskell("x", "99").unwrap_err();
        assert!(error.contains("99"));
    }
}
