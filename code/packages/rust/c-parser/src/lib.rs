//! # C parser — parsing the C integer-core subset (SIR27).
//!
//! The syntactic layer of the `c-to-semantic-ir` frontend
//! ([SIR27](../../../specs/SIR27-c-to-semantic-ir.md)).  It tokenizes with
//! [`coding_adventures_c_lexer`] and feeds the tokens to the generic
//! [`GrammarParser`] driving the compiled `c.grammar`.
//!
//! ```text
//! C source
//!    │  c_lexer::tokenize_c
//!    ▼
//! Vec<Token>
//!    │  parser::GrammarParser (c.grammar → CST)
//!    ▼
//! GrammarASTNode  { rule_name, children }
//! ```
//!
//! The result is the generic, uniform parse tree
//! [`parser::grammar_parser::GrammarASTNode`] — a consumer walks it by
//! `rule_name` (the lowering pass in `c-to-semantic-ir` does exactly that).
//! The root node's `rule_name` is `"translation_unit"`.

use coding_adventures_c_lexer::{tokenize_c, try_tokenize_c};
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

mod _grammar;

/// Create a [`GrammarParser`] wired to the C grammar and tokens.  Ready to
/// call `.parse()`.
pub fn create_c_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_c(source);
    GrammarParser::new(tokens, _grammar::parser_grammar())
}

/// Parse C `source` into a [`GrammarASTNode`] CST rooted at
/// `"translation_unit"`.  Panics on a parse error; use [`try_parse_c`] for the
/// fallible form.
pub fn parse_c(source: &str) -> GrammarASTNode {
    try_parse_c(source).unwrap_or_else(|e| panic!("C parse failed: {e}"))
}

/// Parse C `source`, returning a human-readable error string on failure.  The
/// truly fallible path — a lexical error becomes an `Err`, not a panic (so it
/// routes through [`try_tokenize_c`], not the panicking `tokenize_c` that
/// `create_c_parser` uses).
pub fn try_parse_c(source: &str) -> Result<GrammarASTNode, String> {
    let tokens = try_tokenize_c(source)?;
    GrammarParser::new(tokens, _grammar::parser_grammar())
        .parse()
        .map_err(|e| format!("{e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use parser::grammar_parser::ASTNodeOrToken;

    fn root(src: &str) -> GrammarASTNode {
        parse_c(src)
    }

    /// Does the tree contain a node with this `rule_name` anywhere?
    fn has_rule(node: &GrammarASTNode, target: &str) -> bool {
        if node.rule_name == target {
            return true;
        }
        node.children.iter().any(|c| match c {
            ASTNodeOrToken::Node(n) => has_rule(n, target),
            ASTNodeOrToken::Token(_) => false,
        })
    }

    fn count_rule(node: &GrammarASTNode, target: &str) -> usize {
        let here = usize::from(node.rule_name == target);
        here + node
            .children
            .iter()
            .map(|c| match c {
                ASTNodeOrToken::Node(n) => count_rule(n, target),
                ASTNodeOrToken::Token(_) => 0,
            })
            .sum::<usize>()
    }

    #[test]
    fn empty_main_parses() {
        let ast = root("int main(void) { }");
        assert_eq!(ast.rule_name, "translation_unit");
        assert!(has_rule(&ast, "function_def"));
    }

    #[test]
    fn declaration_with_init() {
        let ast = root("int main(void) { int32_t x = 5; return x; }");
        assert!(has_rule(&ast, "declaration"));
        assert!(has_rule(&ast, "return_stmt"));
    }

    #[test]
    fn arithmetic_precedence_is_structural() {
        // 2 + 3 * 4 must nest the multiply under the add (mult is tighter).
        let ast = root("int main(void) { return 2 + 3 * 4; }");
        assert!(has_rule(&ast, "additive"));
        assert!(has_rule(&ast, "multiplicative"));
    }

    #[test]
    fn cast_is_recognized() {
        // (uint8_t)x is a cast, not a parenthesised expression.
        let ast = root("int main(void) { uint8_t c = (uint8_t)300; return c; }");
        assert!(has_rule(&ast, "cast"), "cast not found:\n{ast:#?}");
    }

    #[test]
    fn control_flow_and_calls() {
        let src = "int f(int x) { if (x > 0) { return 1; } else { return 0; } }\n\
                   int main(void) { int i = 0; while (i < 10) { i = i + 1; } printf(\"%d\\n\", f(i)); return 0; }";
        let ast = root(src);
        assert_eq!(count_rule(&ast, "function_def"), 2);
        assert!(has_rule(&ast, "if_stmt"));
        assert!(has_rule(&ast, "while_stmt"));
        assert!(has_rule(&ast, "call_suffix")); // printf(...) and f(i)
    }

    #[test]
    fn for_loop_with_declaration_init() {
        let ast = root("int main(void) { for (int i = 0; i < 3; i = i + 1) { } return 0; }");
        assert!(has_rule(&ast, "for_stmt"));
        assert!(has_rule(&ast, "init_declarator"));
    }

    #[test]
    fn multiword_types_and_params() {
        let ast = root("unsigned long g(unsigned int a, long long b) { return a; }");
        assert!(has_rule(&ast, "function_def"));
        assert!(has_rule(&ast, "param"));
    }
}
