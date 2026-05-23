//! JSDoc parser.
//!
//! Parses the **interior** of a `/** ... */` block comment into a tree of
//! tags and type expressions per CLOC05 §"`jsdoc.grammar` outline." The
//! enclosing markers are expected to be stripped by an upstream
//! comment-extractor stage (deferred follow-up).
//!
//! The parser is grammar-driven: the actual rules live in
//! [`code/grammars/jsdoc/jsdoc.grammar`](../../../grammars/jsdoc/jsdoc.grammar),
//! compiled to native Rust at build time via `grammar-tools compile-grammar`
//! and embedded as `mod _grammar`.

use coding_adventures_jsdoc_lexer::tokenize_jsdoc;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

mod _grammar;

/// Construct a JSDoc [`GrammarParser`] over `source`. The caller is
/// responsible for stripping the surrounding `/** */` markers; see the
/// crate-level docs.
pub fn create_jsdoc_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_jsdoc(source);
    let grammar = _grammar::parser_grammar();
    GrammarParser::new(tokens, grammar)
}

/// Parse `source` into a [`GrammarASTNode`] rooted at the `document`
/// rule. Returns `Err` on parse failure.
pub fn parse_jsdoc(source: &str) -> Result<GrammarASTNode, String> {
    let mut parser = create_jsdoc_parser(source);
    parser
        .parse()
        .map_err(|e| format!("JSDoc parse failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_empty_document() {
        let ast = parse_jsdoc("").unwrap();
        assert_eq!(ast.rule_name, "document");
    }

    #[test]
    fn parses_type_tag() {
        let ast = parse_jsdoc("@type {number}\n").unwrap();
        assert_eq!(ast.rule_name, "document");
        // The tree should mention the AT_TAG token's text somewhere.
        let dumped = format!("{:?}", ast);
        assert!(dumped.contains("@type"));
    }

    #[test]
    fn parses_returns_tag() {
        let ast = parse_jsdoc("@returns {string}\n").unwrap();
        assert_eq!(ast.rule_name, "document");
    }

    #[test]
    fn parses_param_tag_with_name() {
        let ast = parse_jsdoc("@param {string} name\n").unwrap();
        assert_eq!(ast.rule_name, "document");
        let dumped = format!("{:?}", ast);
        assert!(dumped.contains("@param"));
        assert!(dumped.contains("name"));
    }

    #[test]
    fn parses_multiple_tags() {
        let src = "@param {string} name\n@returns {boolean}\n";
        let ast = parse_jsdoc(src).unwrap();
        assert_eq!(ast.rule_name, "document");
        let dumped = format!("{:?}", ast);
        assert!(dumped.contains("@param"));
        assert!(dumped.contains("@returns"));
    }

    #[test]
    fn parses_nullable_type() {
        let ast = parse_jsdoc("@type {?Foo}\n").unwrap();
        assert_eq!(ast.rule_name, "document");
    }

    #[test]
    fn parses_array_type() {
        let ast = parse_jsdoc("@type {string[]}\n").unwrap();
        assert_eq!(ast.rule_name, "document");
    }

    #[test]
    fn parses_dotted_nominal_type() {
        let ast = parse_jsdoc("@type {Foo.Bar.Baz}\n").unwrap();
        assert_eq!(ast.rule_name, "document");
    }

    #[test]
    fn unknown_tag_is_tolerated() {
        // @throws isn't in the v1 named-tag list, but the unknown_tag
        // rule should sweep it up so the parse still succeeds.
        let ast = parse_jsdoc("@throws {Error} when bad\n").unwrap();
        assert_eq!(ast.rule_name, "document");
        let dumped = format!("{:?}", ast);
        assert!(dumped.contains("@throws"));
    }

    #[test]
    fn create_jsdoc_parser_returns_working_parser() {
        let mut parser = create_jsdoc_parser("@type {string}\n");
        let ast = parser.parse().expect("parse should succeed");
        assert_eq!(ast.rule_name, "document");
    }
}
