//! Java parser backed by compiled versioned parser grammars.

use coding_adventures_java_lexer::tokenize_java;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

mod _grammar;

fn validate_version(version: &str) -> Result<&str, String> {
    if _grammar::SUPPORTED_VERSIONS.contains(&version) {
        Ok(version)
    } else {
        Err(format!(
            "Unknown Java version '{version}'. Valid values: {}",
            _grammar::SUPPORTED_VERSIONS
                .iter()
                .map(|value| format!("\"{}\"", value))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

pub fn create_java_parser(source: &str, version: &str) -> Result<GrammarParser, String> {
    let version = validate_version(version)?;
    let tokens = tokenize_java(source, version)?;
    let grammar = _grammar::parser_grammar(version)
        .expect("compiled Java parser grammar missing supported version");
    Ok(GrammarParser::new(tokens, grammar))
}

pub fn parse_java(source: &str, version: &str) -> Result<GrammarASTNode, String> {
    let mut parser = create_java_parser(source, version)?;
    parser
        .parse()
        .map_err(|e| format!("Java parse failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_class() {
        let ast = parse_java("class Hello { }", "21").unwrap();
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn all_supported_versions_load() {
        for version in _grammar::SUPPORTED_VERSIONS {
            let ast = parse_java("", version).unwrap();
            assert_eq!(ast.rule_name, "program", "version {version}");
        }
    }

    #[test]
    fn unknown_version_returns_error() {
        let error = parse_java("class Hello { }", "99").unwrap_err();
        assert!(error.contains("99"));
    }
}
