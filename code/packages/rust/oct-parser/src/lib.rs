//! # Oct parser — OCT02 phase 1.
//!
//! Parses Oct source text into a grammar AST using the generic
//! `GrammarParser` and the auto-generated `_grammar.rs` (compiled from
//! `code/grammars/oct.grammar` via `grammar-tools`).  Mirrors the
//! Nib parser's structure exactly — Oct's grammar is similar enough
//! that a thin wrapper is sufficient.
//!
//! ## Usage
//!
//! ```
//! use coding_adventures_oct_parser::parse_oct;
//!
//! let ast = parse_oct("fn main() { let x: u8 = 5; }").unwrap();
//! assert_eq!(ast.rule_name, "program");
//! ```

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use coding_adventures_oct_lexer::tokenize_oct;
use parser::grammar_parser::{GrammarASTNode, GrammarParseError, GrammarParser};

mod _grammar;

/// Create a `GrammarParser` over an Oct source string.  Most callers
/// want [`parse_oct`] instead.
pub fn create_oct_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_oct(source);
    let grammar = _grammar::parser_grammar();
    GrammarParser::new(tokens, grammar)
}

/// Parse an Oct source string into a grammar AST rooted at `program`.
// `GrammarParseError` is a large error type owned by the shared `grammar-tools`
// crate; boxing it here would diverge from every other grammar frontend's API.
#[allow(clippy::result_large_err)]
pub fn parse_oct(source: &str) -> Result<GrammarASTNode, GrammarParseError> {
    let mut parser = create_oct_parser(source);
    parser.parse()
}

#[cfg(test)]
mod tests {
    use super::*;
    use parser::grammar_parser::ASTNodeOrToken;

    fn has_rule(node: &GrammarASTNode, rule: &str) -> bool {
        if node.rule_name == rule { return true; }
        node.children.iter().any(|c| match c {
            ASTNodeOrToken::Node(inner) => has_rule(inner, rule),
            _ => false,
        })
    }

    #[test]
    fn parses_minimal_main() {
        let ast = parse_oct("fn main() { }").expect("parse ok");
        assert_eq!(ast.rule_name, "program");
        assert!(has_rule(&ast, "fn_decl"));
    }

    #[test]
    fn parses_let_with_type_annotation() {
        let ast = parse_oct("fn main() { let x: u8 = 5; }").expect("parse ok");
        assert!(has_rule(&ast, "let_stmt"));
    }

    #[test]
    fn parses_return_statement() {
        let ast = parse_oct("fn add(a: u8, b: u8) -> u8 { return a + b; }")
            .expect("parse ok");
        assert!(has_rule(&ast, "return_stmt"));
        assert!(has_rule(&ast, "add_expr"));
    }

    #[test]
    fn parses_if_else() {
        let ast = parse_oct("fn t() { if 1 == 1 { } else { } }").expect("parse ok");
        assert!(has_rule(&ast, "if_stmt"));
    }

    #[test]
    fn parses_while_loop() {
        let ast = parse_oct("fn t() { while 1 == 1 { } }").expect("parse ok");
        assert!(has_rule(&ast, "while_stmt"));
    }

    #[test]
    fn parses_loop_and_break() {
        let ast = parse_oct("fn t() { loop { break; } }").expect("parse ok");
        assert!(has_rule(&ast, "loop_stmt"));
        assert!(has_rule(&ast, "break_stmt"));
    }

    #[test]
    fn parses_intrinsic_call() {
        // `out(port, value)` is an intrinsic, not a regular call.
        let ast = parse_oct("fn t() { out(1, 0); }").expect("parse ok");
        assert!(has_rule(&ast, "intrinsic_call"));
    }

    #[test]
    fn parses_user_function_call() {
        let ast = parse_oct("fn forty_two() -> u8 { return 42; } \
                             fn main() { let r: u8 = forty_two(); }")
            .expect("parse ok");
        assert!(has_rule(&ast, "call_expr"));
    }

    #[test]
    fn parses_static_decl() {
        let ast = parse_oct("static counter: u8 = 0;\nfn main() { }")
            .expect("parse ok");
        assert!(has_rule(&ast, "static_decl"));
    }

    #[test]
    fn parses_expression_precedence() {
        // Bitwise above additive, additive above relational.
        let ast = parse_oct("fn t() { if 1 + 2 == 3 { } }").expect("parse ok");
        assert!(has_rule(&ast, "add_expr"));
        assert!(has_rule(&ast, "eq_expr"));
    }

    #[test]
    fn rejects_syntax_errors() {
        // Missing closing brace.
        let err = parse_oct("fn main() {").unwrap_err();
        // Just confirm we got an error — exact message format is the
        // grammar engine's concern.
        let _ = format!("{err}");
    }
}
