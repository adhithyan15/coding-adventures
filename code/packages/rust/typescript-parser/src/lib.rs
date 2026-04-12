//! TypeScript parser backed by compiled generic and versioned parser grammars.

use coding_adventures_typescript_lexer::tokenize_typescript;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

mod _grammar;

fn validate_version(version: &str) -> Result<&str, String> {
    if _grammar::SUPPORTED_VERSIONS.contains(&version) {
        Ok(version)
    } else {
        Err(format!(
            "Unknown TypeScript version '{version}'. Valid values: {}",
            _grammar::SUPPORTED_VERSIONS
                .iter()
                .map(|value| format!("\"{}\"", value))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

pub fn create_typescript_parser(source: &str, version: &str) -> Result<GrammarParser, String> {
    let version = validate_version(version)?;
    let tokens = tokenize_typescript(source, version)?;
    let grammar = _grammar::parser_grammar(version)
        .expect("compiled TypeScript parser grammar missing supported version");
    Ok(GrammarParser::new(tokens, grammar))
}

pub fn parse_typescript(source: &str, version: &str) -> Result<GrammarASTNode, String> {
    let mut parser = create_typescript_parser(source, version)?;
    parser
        .parse()
        .map_err(|e| format!("TypeScript parse failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_generic_typescript() {
        let ast = parse_typescript("let x = 1;", "").unwrap();
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn parses_versioned_typescript() {
        let ast = parse_typescript("let x = 1;", "ts5.8").unwrap();
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn all_supported_versions_load() {
        for version in _grammar::SUPPORTED_VERSIONS {
            let ast = parse_typescript("", version).unwrap();
            assert_eq!(ast.rule_name, "program", "version {version:?}");
        }
    }
}
