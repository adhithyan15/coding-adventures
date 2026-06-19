//! # Wolfram Parser — building a syntax tree for the Wolfram Language.
//!
//! Turns the token stream from [`coding_adventures_wolfram_lexer`] into a parse
//! tree using the generic
//! [`GrammarParser`](parser::grammar_parser::GrammarParser), driven by the
//! embedded `wolfram.grammar` (`src/_grammar.rs`). It hand-writes no parsing
//! logic. A sibling of `r-parser` / `macsyma-parser`. See
//! `code/specs/MA04-wolfram-language.md`.
//!
//! ## What the tree captures
//!
//! Everything in Wolfram is `head[args]`; this parser produces the surface tree
//! whose rule names (`assignment`, `replaceall`, `rule`, `additive`,
//! `multiplicative`, `power`, `postfix`, `atom`, `list`, …) the W-4
//! `wolfram-runtime` will lower into the canonical `symbolic-ir` heads
//! (`Plus`/`Times`/`Power`/`List`/`Rule`/`ReplaceAll`/`Set`/…).
//!
//! ```text
//! Wolfram source
//!    |
//!    v
//! coding_adventures_wolfram_lexer::tokenize_wolfram  ->  Vec<Token>
//!    |
//!    v
//! parser::GrammarParser  (driven by the embedded wolfram.grammar)
//!    |
//!    v
//! GrammarASTNode  <- the tree W-4 lowers to symbolic-ir
//! ```

use coding_adventures_wolfram_lexer::{tokenize_wolfram, try_tokenize_wolfram};
use parser::grammar_parser::{GrammarASTNode, GrammarParser};
mod _grammar;

/// Create a [`GrammarParser`] wired to the Wolfram grammar and the tokens of
/// `source`.
///
/// # Panics
///
/// Panics if tokenization fails. Use [`try_parse_wolfram`] for a non-panicking
/// path.
pub fn create_wolfram_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_wolfram(source);
    GrammarParser::new(tokens, _grammar::parser_grammar())
}

/// Parse Wolfram source text into a [`GrammarASTNode`] rooted at `program`.
///
/// # Panics
///
/// Panics on a lexical or syntax error. Use [`try_parse_wolfram`] to handle
/// errors.
///
/// # Example
///
/// ```
/// use coding_adventures_wolfram_parser::parse_wolfram;
/// let ast = parse_wolfram("Sin[x] + 1\n");
/// assert_eq!(ast.rule_name, "program");
/// ```
pub fn parse_wolfram(source: &str) -> GrammarASTNode {
    create_wolfram_parser(source)
        .parse()
        .unwrap_or_else(|e| panic!("Wolfram parse failed: {e}"))
}

/// Parse Wolfram source text, returning a `Result` instead of panicking.
pub fn try_parse_wolfram(source: &str) -> Result<GrammarASTNode, String> {
    let tokens = try_tokenize_wolfram(source)?;
    GrammarParser::new(tokens, _grammar::parser_grammar())
        .parse()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use parser::grammar_parser::ASTNodeOrToken;

    fn contains_rule(node: &GrammarASTNode, name: &str) -> bool {
        node.rule_name == name
            || node.children.iter().any(|c| match c {
                ASTNodeOrToken::Node(n) => contains_rule(n, name),
                ASTNodeOrToken::Token(_) => false,
            })
    }

    /// The value of the first token directly under the first node whose rule is
    /// `rule` — used to check which operator a construct matched.
    fn first_token_of(node: &GrammarASTNode, rule: &str) -> Option<String> {
        fn tok(n: &GrammarASTNode) -> Option<String> {
            n.children.iter().find_map(|c| match c {
                ASTNodeOrToken::Token(t) => Some(t.value.clone()),
                _ => None,
            })
        }
        if node.rule_name == rule {
            if let Some(t) = tok(node) {
                return Some(t);
            }
        }
        node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Node(child) => first_token_of(child, rule),
            ASTNodeOrToken::Token(_) => None,
        })
    }

    fn parses(src: &str) -> bool {
        try_parse_wolfram(src).is_ok()
    }

    #[test]
    fn program_is_the_root() {
        assert_eq!(parse_wolfram("1\n").rule_name, "program");
    }

    #[test]
    fn square_bracket_application_parses() {
        // `Sin[x]` is application (postfix), not a list.
        let ast = parse_wolfram("Sin[x]\n");
        assert!(contains_rule(&ast, "postfix"));
    }

    #[test]
    fn nested_application_parses() {
        assert!(parses("f[g[x]]\n"));
        assert!(parses("f[x, y, z]\n"));
        assert!(parses("f[]\n")); // empty arg list
    }

    #[test]
    fn brace_list_parses() {
        assert!(contains_rule(&parse_wolfram("{a, b, c}\n"), "list"));
        assert!(parses("{}\n")); // empty list
        assert!(parses("{1, {2, 3}, 4}\n")); // nested list
    }

    #[test]
    fn arithmetic_precedence_cascade() {
        // x + 2*y^3 exercises additive > multiplicative > power.
        let ast = parse_wolfram("x + 2*y^3\n");
        for rule in ["additive", "multiplicative", "power"] {
            assert!(contains_rule(&ast, rule), "missing {rule}");
        }
    }

    #[test]
    fn replacement_operators_parse() {
        assert!(contains_rule(&parse_wolfram("x /. a -> b\n"), "replaceall"));
        assert_eq!(
            first_token_of(&parse_wolfram("a -> b\n"), "rule").as_deref(),
            Some("->")
        );
        assert!(parses("x /. a :> b\n")); // RuleDelayed
                                          // `x /. a -> b` is ReplaceAll[x, Rule[a, b]] — rule binds tighter than /.
        let ast = parse_wolfram("x /. a -> b\n");
        assert!(contains_rule(&ast, "replaceall") && contains_rule(&ast, "rule"));
    }

    #[test]
    fn assignment_set_and_setdelayed() {
        assert_eq!(
            first_token_of(&parse_wolfram("x = 5\n"), "assignment").as_deref(),
            Some("=")
        );
        assert_eq!(
            first_token_of(&parse_wolfram("f[x_] := x^2\n"), "assignment").as_deref(),
            Some(":=")
        );
    }

    #[test]
    fn pattern_blanks_parse() {
        assert!(parses("x_\n")); // Pattern[x, Blank[]]
        assert!(parses("_\n")); // Blank[]
        assert!(parses("_Integer\n")); // Blank[Integer]
        assert!(parses("x_Integer\n")); // Pattern[x, Blank[Integer]]
        assert!(parses("f[x_] := x\n")); // a pattern in a function definition
    }

    #[test]
    fn comparison_logic_and_grouping() {
        assert!(contains_rule(&parse_wolfram("a == b\n"), "comparison"));
        assert!(contains_rule(&parse_wolfram("a && b || c\n"), "logical_or"));
        assert!(parses("!x\n"));
        assert!(parses("(a + b) * c\n")); // grouping
    }

    #[test]
    fn newlines_inside_brackets_let_a_form_span_lines() {
        // The lexer drops interior newlines; the parser sees one statement.
        assert!(parses("f[\n  a,\n  b\n]\n"));
        assert!(parses("{\n  1,\n  2\n}\n"));
    }

    #[test]
    fn statement_separators_and_trailing_newline() {
        assert!(parses("a; b; c\n"));
        assert!(parses("x = 1;\n")); // a `;` suppresses, still parses
        assert!(parses("1 + 1")); // trailing newline optional
    }

    // --- W-6 operator sugar: /@, @@, [[ ]] ------------------------------

    #[test]
    fn map_and_apply_sugar_parse_via_mapapply() {
        // `f /@ x` and `f @@ x` match the new `mapapply` infix level.
        assert_eq!(
            first_token_of(&parse_wolfram("f /@ x\n"), "mapapply").as_deref(),
            Some("/@")
        );
        assert_eq!(
            first_token_of(&parse_wolfram("f @@ x\n"), "mapapply").as_deref(),
            Some("@@")
        );
        // Over a list literal (the common form), and chained.
        assert!(parses("f /@ {1, 2}\n"));
        assert!(parses("Plus @@ {1, 2, 3}\n"));
        assert!(parses("g @@ f /@ x\n")); // left-folds: Apply[g, Map[f, x]]
    }

    #[test]
    fn double_bracket_part_sugar_parses_via_postfix() {
        // `x[[i]]` is a postfix, like `f[…]` application.
        assert!(contains_rule(&parse_wolfram("x[[2]]\n"), "postfix"));
        assert!(parses("{a, b, c}[[2]]\n"));
        // Chained / nested part: `m[[1]][[2]]` and a multi-index `m[[1, 2]]`.
        assert!(parses("{{1, 2}, {3, 4}}[[1]][[2]]\n"));
        assert!(parses("m[[1, 2]]\n"));
        // Interleaves with application: `f[x][[1]]`, `x[[1]][y]`.
        assert!(parses("f[x][[1]]\n"));
        assert!(parses("x[[1]][y]\n"));
    }

    #[test]
    fn empty_double_brackets_are_a_syntax_error() {
        // `[[ ]]` requires at least one index (unlike `f[]`).
        assert!(try_parse_wolfram("x[[]]\n").is_err());
    }

    #[test]
    fn syntax_error_is_reported() {
        assert!(try_parse_wolfram("1 +\n").is_err());
        assert!(try_parse_wolfram("f[x\n").is_err()); // unclosed bracket
        assert!(try_parse_wolfram("x[[1\n").is_err()); // unclosed double bracket
        assert!(try_parse_wolfram("f /@\n").is_err()); // map with no right operand
    }

    #[test]
    fn a_small_wolfram_program_parses() {
        assert!(parses(
            "f[x_] := x^2\nf[3] + Sin[0]\n{1, 2, 3} /. a_ -> a + 1\n"
        ));
    }
}
