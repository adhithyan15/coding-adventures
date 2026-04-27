//! F# parser backed by compiled versioned parser grammars.

use coding_adventures_fsharp_lexer::{tokenize_fsharp, DEFAULT_VERSION};
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

mod _grammar;

fn resolve_version(version: &str) -> Result<&str, String> {
    let resolved = if version.is_empty() {
        DEFAULT_VERSION
    } else {
        version
    };

    if _grammar::SUPPORTED_VERSIONS.contains(&resolved) {
        Ok(resolved)
    } else {
        Err(format!(
            "Unknown F# version '{version}'. Valid values: {}",
            _grammar::SUPPORTED_VERSIONS
                .iter()
                .map(|value| format!("\"{}\"", value))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

pub fn create_fsharp_parser(source: &str, version: &str) -> Result<GrammarParser, String> {
    let version = resolve_version(version)?;
    let tokens = tokenize_fsharp(source, version)?;
    let grammar = _grammar::parser_grammar(version)
        .expect("compiled F# parser grammar missing supported version");
    Ok(GrammarParser::new(tokens, grammar))
}

pub fn parse_fsharp(source: &str, version: &str) -> Result<GrammarASTNode, String> {
    let mut parser = create_fsharp_parser(source, version)?;
    parser
        .parse()
        .map_err(|e| format!("F# parse failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_let_binding() {
        let ast = parse_fsharp("let value = 1", "").unwrap();
        assert_eq!(ast.rule_name, "compilation_unit");
    }

    #[test]
    fn all_supported_versions_load() {
        for version in _grammar::SUPPORTED_VERSIONS {
            let ast = parse_fsharp("let value = 1", version).unwrap();
            assert_eq!(ast.rule_name, "compilation_unit", "version {version}");
        }
    }

    #[test]
    fn unknown_version_returns_error() {
        let error = parse_fsharp("let value = 1", "11").unwrap_err();
        assert!(error.contains("11"));
    }
}
