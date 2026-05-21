//! JavaScript parser backed by compiled ECMAScript parser grammars (es1 through es2025).

use coding_adventures_javascript_lexer::{tokenize_javascript, tokenize_javascript_typed};
use coding_adventures_javascript_tokens::EsVersion;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

/// Typed default version. New code should prefer this over the string form.
pub const DEFAULT_ES_VERSION: EsVersion = EsVersion::Es2025;

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

/// Typed version of [`create_javascript_parser`]. Takes an [`EsVersion`]
/// directly; cannot fail with an unknown-version error.
pub fn create_javascript_parser_typed(
    source: &str,
    version: EsVersion,
) -> Result<GrammarParser, String> {
    let tokens = tokenize_javascript_typed(source, version)?;
    let grammar = _grammar::parser_grammar(version.as_str())
        .expect("compiled JavaScript parser grammar missing supported version");
    Ok(GrammarParser::new(tokens, grammar))
}

/// Typed version of [`parse_javascript`]. Takes an [`EsVersion`] directly.
pub fn parse_javascript_typed(
    source: &str,
    version: EsVersion,
) -> Result<GrammarASTNode, String> {
    let mut parser = create_javascript_parser_typed(source, version)?;
    parser
        .parse()
        .map_err(|e| format!("JavaScript parse failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_es5_javascript() {
        let ast = parse_javascript("var x = 1;", "es5").unwrap();
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

    #[test]
    fn parse_typed_es2015() {
        let ast = parse_javascript_typed("let x = 1;", EsVersion::Es2015).unwrap();
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn default_es_version_constant_is_es2025() {
        assert_eq!(DEFAULT_ES_VERSION, EsVersion::Es2025);
    }

    #[test]
    fn all_typed_versions_load() {
        for &version in EsVersion::ALL {
            let ast = parse_javascript_typed("", version).unwrap();
            assert_eq!(ast.rule_name, "program", "version {version}");
        }
    }

    #[test]
    fn create_parser_typed() {
        let _parser = create_javascript_parser_typed("var x = 1;", EsVersion::Es5).unwrap();
    }
}
