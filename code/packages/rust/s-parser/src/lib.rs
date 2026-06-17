//! # S Parser — building a syntax tree for the historical Bell Labs S language.
//!
//! This crate turns the token stream from the [`coding_adventures_s_lexer`]
//! crate into a parse tree using the generic grammar-driven
//! [`GrammarParser`](parser::grammar_parser::GrammarParser). It hand-writes no
//! parsing logic: the S grammar lives in `code/grammars/s.grammar`, compiled
//! ahead of time into the embedded `src/_grammar.rs`.
//!
//! ```text
//! S source
//!    |
//!    v
//! coding_adventures_s_lexer::tokenize_s   →  Vec<Token>
//!    |
//!    v
//! parser::GrammarParser  (driven by the embedded s.grammar)
//!    |
//!    v
//! GrammarASTNode  ← the tree the s-runtime walks
//! ```
//!
//! ## The shape of the tree
//!
//! [`GrammarASTNode`] is generic: each node carries a `rule_name` (the grammar
//! rule that matched, e.g. `"assignment"`, `"additive"`, `"call_suffix"`) and a
//! list of children that are either deeper nodes or raw tokens. Operator
//! precedence is encoded by the nesting depth, exactly as the grammar's
//! precedence cascade prescribes (assignment loosest, calls/indexing tightest).
//! The `s-runtime` crate interprets this tree directly.
//!
//! Because the grammar references `NEWLINE`, the parser runs in
//! newlines-significant mode automatically.

use coding_adventures_s_lexer::{tokenize_s, try_tokenize_s};
use parser::grammar_parser::{GrammarASTNode, GrammarParser};
mod _grammar;

/// Create a [`GrammarParser`] wired to the S grammar and the tokens of `source`.
///
/// # Panics
///
/// Panics if tokenization fails. Use [`try_parse_s`] for a non-panicking path.
pub fn create_s_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_s(source);
    GrammarParser::new(tokens, _grammar::parser_grammar())
}

/// Parse S source text into a [`GrammarASTNode`] whose `rule_name` is
/// `"program"`.
///
/// # Panics
///
/// Panics on a lexical or syntax error. Use [`try_parse_s`] to handle errors.
///
/// # Example
///
/// ```
/// use coding_adventures_s_parser::parse_s;
/// let ast = parse_s("x <- c(1, 2, 3)\nmean(x)\n");
/// assert_eq!(ast.rule_name, "program");
/// ```
pub fn parse_s(source: &str) -> GrammarASTNode {
    create_s_parser(source)
        .parse()
        .unwrap_or_else(|e| panic!("S parse failed: {e}"))
}

/// Parse S source text, returning a `Result` instead of panicking.
pub fn try_parse_s(source: &str) -> Result<GrammarASTNode, String> {
    let tokens = try_tokenize_s(source)?;
    GrammarParser::new(tokens, _grammar::parser_grammar())
        .parse()
        .map_err(|e| e.to_string())
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use parser::grammar_parser::ASTNodeOrToken;

    /// Does any node in the tree have this `rule_name`?
    fn contains_rule(node: &GrammarASTNode, name: &str) -> bool {
        if node.rule_name == name {
            return true;
        }
        node.children.iter().any(|child| match child {
            ASTNodeOrToken::Node(n) => contains_rule(n, name),
            ASTNodeOrToken::Token(_) => false,
        })
    }

    /// Collect every token value appearing under a node, in order.
    fn token_values(node: &GrammarASTNode, out: &mut Vec<String>) {
        for child in &node.children {
            match child {
                ASTNodeOrToken::Node(n) => token_values(n, out),
                ASTNodeOrToken::Token(t) => out.push(t.value.clone()),
            }
        }
    }

    fn parses(src: &str) -> bool {
        try_parse_s(src).is_ok()
    }

    #[test]
    fn program_is_the_root() {
        assert_eq!(parse_s("1\n").rule_name, "program");
    }

    #[test]
    fn all_assignment_forms_parse() {
        for src in [
            "x <- 1\n",
            "x _ 1\n",
            "1 -> x\n",
            "x <<- 1\n",
            "a <- b <- 3\n",
        ] {
            assert!(parses(src), "should parse: {src:?}");
            assert!(contains_rule(&parse_s(src), "assignment"), "{src:?}");
        }
    }

    #[test]
    fn combine_call_with_positional_args() {
        let ast = parse_s("c(1, 2, 3)\n");
        assert!(contains_rule(&ast, "call_suffix"));
        let mut vals = Vec::new();
        token_values(&ast, &mut vals);
        assert!(vals.contains(&"c".to_string()) && vals.contains(&"3".to_string()));
    }

    #[test]
    fn named_arguments_parse() {
        assert!(parses("mean(x, na.rm = TRUE)\n"));
        // `==` inside a call is a positional comparison, not a named arg.
        assert!(parses("f(x == 1)\n"));
    }

    #[test]
    fn arithmetic_and_precedence_nodes() {
        let ast = parse_s("1 + 2 * 3 ^ 2\n");
        for rule in ["additive", "multiplicative", "power"] {
            assert!(contains_rule(&ast, rule), "missing {rule}");
        }
    }

    #[test]
    fn comparison_and_sequence() {
        assert!(contains_rule(&parse_s("a < b\n"), "comparison"));
        assert!(contains_rule(&parse_s("1:10\n"), "range"));
    }

    #[test]
    fn indexing_parses() {
        assert!(contains_rule(&parse_s("x[1]\n"), "index_suffix"));
    }

    #[test]
    fn function_definition_parses() {
        let ast = parse_s("sq <- function(v) v * v\n");
        assert!(contains_rule(&ast, "func_def"));
        assert!(contains_rule(&ast, "param_list"));
    }

    #[test]
    fn function_with_default_argument() {
        assert!(parses("f <- function(x, n = 1) x + n\n"));
    }

    #[test]
    fn if_else_expression() {
        let ast = parse_s("if (x > 0) 1 else -1\n");
        assert!(contains_rule(&ast, "if_expr"));
    }

    #[test]
    fn for_and_while_and_repeat() {
        assert!(contains_rule(
            &parse_s("for (i in 1:3) print(i)\n"),
            "for_expr"
        ));
        assert!(contains_rule(
            &parse_s("while (x < 10) x <- x + 1\n"),
            "while_expr"
        ));
        assert!(contains_rule(&parse_s("repeat break\n"), "repeat_expr"));
    }

    #[test]
    fn multi_statement_block_with_newlines() {
        let ast = parse_s("{\n  x <- 1\n  y <- 2\n  x + y\n}\n");
        assert!(contains_rule(&ast, "block"));
    }

    #[test]
    fn semicolons_separate_statements() {
        assert!(parses("x <- 1; y <- 2; x + y\n"));
    }

    #[test]
    fn call_spanning_multiple_lines() {
        // Interior newlines were dropped by the lexer, so this is one call.
        assert!(parses("sum(1,\n    2,\n    3)\n"));
    }

    #[test]
    fn trailing_newline_optional() {
        assert!(parses("mean(x)"));
    }

    #[test]
    fn syntax_error_is_reported() {
        // A dangling binary operator has no right operand.
        assert!(try_parse_s("1 +\n").is_err());
    }

    #[test]
    fn the_canonical_session_parses() {
        assert!(parses(
            "x <- c(1, 2, 3)\nmean(x)\nx * 10 + c(1, 2)\nsd(x)\n"
        ));
    }

    // --- v2 grammar additions -------------------------------------------

    #[test]
    fn infix_operators_parse() {
        assert!(contains_rule(&parse_s("x %% 3\n"), "special"));
        assert!(parses("a %in% b\n"));
        assert!(parses("\"%plus%\" <- function(a, b) a + b\n"));
    }

    #[test]
    fn dollar_and_double_bracket_parse() {
        assert!(contains_rule(&parse_s("df$x\n"), "dollar_suffix"));
        assert!(contains_rule(&parse_s("df[[\"x\"]]\n"), "dindex_suffix"));
        assert!(contains_rule(&parse_s("df[1, 2]\n"), "index_suffix"));
    }

    #[test]
    fn precedence_fix_keeps_colon_inside_arithmetic() {
        // `1:3+1` must parse (range nested under additive).
        let ast = parse_s("1:3+1\n");
        assert!(contains_rule(&ast, "additive"));
        assert!(contains_rule(&ast, "range"));
    }
}
