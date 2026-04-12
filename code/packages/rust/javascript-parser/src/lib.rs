//! JavaScript parser backed by compiled generic and ECMAScript parser grammars.

use coding_adventures_javascript_lexer::tokenize_javascript;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

mod _grammar;

fn validate_version(version: &str) -> Result<&str, String> {
    if _grammar::SUPPORTED_VERSIONS.contains(&version) {
        Ok(version)
    } else {
        Err(format!(
            "Unknown JavaScript/ECMAScript version '{version}'. Valid values: {}",
            _grammar::SUPPORTED_VERSIONS
                .iter()
                .map(|value| format!("\"{}\"", value))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

pub fn create_javascript_parser(source: &str, version: &str) -> Result<GrammarParser, String> {
    let version = validate_version(version)?;
    let tokens = tokenize_javascript(source, version)?;
    let grammar = _grammar::parser_grammar(version)
        .expect("compiled JavaScript parser grammar missing supported version");
    Ok(GrammarParser::new(tokens, grammar))
}

pub fn parse_javascript(source: &str, version: &str) -> Result<GrammarASTNode, String> {
    let mut parser = create_javascript_parser(source, version)?;
    parser
        .parse()
        .map_err(|e| format!("JavaScript parse failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_generic_javascript() {
        let ast = parse_javascript("var x = 1;", "").unwrap();
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn parses_versioned_ecmascript() {
        let ast = parse_javascript("let x = 1;", "es2015").unwrap();
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn all_supported_versions_load() {
        for version in _grammar::SUPPORTED_VERSIONS {
            let ast = parse_javascript("", version).unwrap();
            assert_eq!(ast.rule_name, "program", "version {version:?}");
        }
    }
}
