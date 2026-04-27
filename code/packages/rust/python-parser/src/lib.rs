//! Python parser backed by compiled versioned parser grammars.

use coding_adventures_python_lexer::{tokenize_python, DEFAULT_VERSION};
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
            "Unsupported Python version '{version}'. Supported versions: {}",
            _grammar::SUPPORTED_VERSIONS
                .iter()
                .map(|value| format!("\"{}\"", value))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

pub fn create_python_parser(source: &str, version: &str) -> Result<GrammarParser, String> {
    let version = resolve_version(version)?;
    let tokens = tokenize_python(source, version)?;
    let grammar = _grammar::parser_grammar(version)
        .expect("compiled Python parser grammar missing supported version");
    Ok(GrammarParser::new(tokens, grammar))
}

pub fn parse_python(source: &str, version: &str) -> Result<GrammarASTNode, String> {
    let mut parser = create_python_parser(source, version)?;
    parser
        .parse()
        .map_err(|e| format!("Python parse failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_with_default_version() {
        let ast = parse_python("x = 1\n", "").unwrap();
        assert_eq!(ast.rule_name, "file");
    }

    #[test]
    fn parses_indented_block() {
        let ast = parse_python("def f():\n    return 1\n", "3.12").unwrap();
        assert_eq!(ast.rule_name, "file");
    }

    #[test]
    fn all_supported_versions_load() {
        for version in _grammar::SUPPORTED_VERSIONS {
            let ast = parse_python("", version).unwrap();
            assert!(
                !ast.rule_name.is_empty(),
                "version {version} should produce a root rule"
            );
        }
    }

    #[test]
    fn unsupported_version_returns_error() {
        let error = parse_python("x = 1\n", "4.0").unwrap_err();
        assert!(error.contains("4.0"));
    }
}
