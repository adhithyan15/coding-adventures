//! # MATLAB Parser — building a syntax tree for the MATLAB language.
//!
//! Tokenizes with [`coding_adventures_matlab_lexer`] and parses the result with
//! the generic [`GrammarParser`] driven by the compiled `matlab.grammar`. The
//! tree is a [`GrammarASTNode`] whose `rule_name`s match the grammar rules, so
//! the (future) `matlab-runtime` walks it by dispatching on rule name — the same
//! pattern S/R use. See `code/specs/MA01-matlab-language.md`.
//!
//! ## The one context-sensitive twist: `end`
//!
//! `end` is both a **block terminator** (`if … end`) and the **last-index
//! sentinel** (`A(end)`, `A(2:end)`). The grammar's `"end"` literals close
//! blocks; the index sentinel reaches the parser as an ordinary `NAME` thanks to
//! [`retag_index_end`], a pre-parse hook that rewrites every `end` sitting inside
//! `( )`/`[ ]`/`{ }` to a `NAME` token *before* parsing. So the two uses never
//! collide: a depth-0 `end` stays the keyword that closes a block; a bracketed
//! `end` is a name the runtime resolves to the dimension length.

use lexer::token::{Token, TokenType};
use parser::grammar_parser::{GrammarASTNode, GrammarParser};
mod _grammar;

/// Rewrite each `end` keyword that occurs inside `( )`, `[ ]`, or `{ }` into a
/// `NAME` token, leaving depth-0 `end`s (block terminators) untouched. Tracks
/// the combined bracket depth across all three bracket kinds.
fn retag_index_end(tokens: Vec<Token>) -> Vec<Token> {
    let mut depth: i32 = 0;
    tokens
        .into_iter()
        .map(|mut tok| {
            match tok.type_ {
                TokenType::LParen | TokenType::LBracket | TokenType::LBrace => depth += 1,
                TokenType::RParen | TokenType::RBracket | TokenType::RBrace => {
                    depth = depth.saturating_sub(1)
                }
                _ => {}
            }
            if depth > 0 && tok.effective_type_name() == "KEYWORD" && tok.value == "end" {
                tok.type_ = TokenType::Name;
                tok.type_name = None; // effective_type_name() now reports "NAME"
            }
            tok
        })
        .collect()
}

/// Build a [`GrammarParser`] for MATLAB source, with the `end`-retag hook
/// installed.
///
/// # Panics
///
/// Panics if tokenization fails. Use [`try_parse_matlab`] for a `Result`.
pub fn create_matlab_parser(source: &str) -> GrammarParser {
    let tokens = coding_adventures_matlab_lexer::tokenize_matlab(source);
    let mut parser = GrammarParser::new(tokens, _grammar::parser_grammar());
    parser.add_pre_parse(Box::new(retag_index_end));
    parser
}

/// Parse MATLAB source into a syntax tree rooted at the `program` rule.
///
/// # Panics
///
/// Panics on a lexical or syntax error. Use [`try_parse_matlab`] for a `Result`.
///
/// # Example
///
/// ```
/// use coding_adventures_matlab_parser::parse_matlab;
/// let tree = parse_matlab("A = [1 2; 3 4]\n");
/// assert_eq!(tree.rule_name, "program");
/// ```
pub fn parse_matlab(source: &str) -> GrammarASTNode {
    create_matlab_parser(source)
        .parse()
        .unwrap_or_else(|e| panic!("MATLAB parse failed: {e}"))
}

/// Parse MATLAB source, returning a `Result` instead of panicking.
pub fn try_parse_matlab(source: &str) -> Result<GrammarASTNode, String> {
    let tokens = coding_adventures_matlab_lexer::try_tokenize_matlab(source)?;
    let mut parser = GrammarParser::new(tokens, _grammar::parser_grammar());
    parser.add_pre_parse(Box::new(retag_index_end));
    parser.parse().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use parser::grammar_parser::ASTNodeOrToken;

    /// Does the tree contain a node with this `rule_name`?
    fn has_rule(node: &GrammarASTNode, name: &str) -> bool {
        node.rule_name == name
            || node.children.iter().any(|c| match c {
                ASTNodeOrToken::Node(n) => has_rule(n, name),
                ASTNodeOrToken::Token(_) => false,
            })
    }

    /// Count nodes with this `rule_name`.
    fn count_rule(node: &GrammarASTNode, name: &str) -> usize {
        let here = usize::from(node.rule_name == name);
        here + node
            .children
            .iter()
            .map(|c| match c {
                ASTNodeOrToken::Node(n) => count_rule(n, name),
                ASTNodeOrToken::Token(_) => 0,
            })
            .sum::<usize>()
    }

    fn parses(src: &str) {
        try_parse_matlab(src).unwrap_or_else(|e| panic!("expected {src:?} to parse: {e}"));
    }

    // --- Expressions and precedence -------------------------------------

    #[test]
    fn arithmetic_and_precedence() {
        parses("y = 2 + 3 * 4\n");
        parses("z = -2 ^ 2\n"); // unary looser than power
        parses("w = a:b:c\n"); // colon range with step
    }

    #[test]
    fn elementwise_and_matrix_operators() {
        parses("c = A .* B ./ C\n");
        parses("d = A * B \\ C\n");
        parses("e = A .^ 2\n");
    }

    #[test]
    fn transpose_is_postfix() {
        let t = parse_matlab("y = A'\n");
        assert!(has_rule(&t, "transpose_suffix"));
        parses("z = A.' * b\n"); // non-conjugate transpose
        parses("w = (A + B)'\n");
    }

    #[test]
    fn comparison_and_logical() {
        parses("r = a == b & c ~= d\n");
        parses("s = x > 0 && y < 1 || ~z\n");
    }

    // --- Matrix and cell literals ---------------------------------------

    #[test]
    fn matrix_literals() {
        let m = parse_matlab("M = [1 2; 3 4]\n");
        assert!(has_rule(&m, "matrix_literal"));
        assert_eq!(count_rule(&m, "matrix_row"), 2); // two rows
        parses("r = [1, 2, 3]\n"); // comma columns
        parses("col = [1; 2; 3]\n"); // semicolon rows
        parses("empty = []\n");
        // A newline inside [ ] separates rows (the lexer keeps it).
        let nl = parse_matlab("M = [1 2\n3 4]\n");
        assert_eq!(count_rule(&nl, "matrix_row"), 2);
    }

    #[test]
    fn cell_literal_and_concatenation() {
        parses("c = {1, 'two', 3}\n");
        parses("h = [A B]\n"); // horizontal concat (juxtaposition)
    }

    // --- Calls, indexing, and `end` -------------------------------------

    #[test]
    fn calls_and_indexing() {
        parses("y = f(x, 2)\n");
        let idx = parse_matlab("v = A(2, 3)\n");
        assert!(has_rule(&idx, "call_suffix"));
        parses("col = A(:, k)\n"); // whole-column
        parses("s = obj.field\n"); // field access
        parses("z = data.values(3)\n"); // field then index
    }

    #[test]
    fn end_is_the_index_sentinel_inside_brackets() {
        // `A(end)` and `A(2:end)` parse: the bracketed `end` is retagged to NAME.
        parses("last = A(end)\n");
        parses("tail = A(2:end)\n");
        parses("rev = A(end:-1:1)\n");
    }

    // --- Control flow (and `end` as block terminator) -------------------

    #[test]
    fn if_elseif_else() {
        let t = parse_matlab("if x > 0\n  y = 1\nelseif x < 0\n  y = -1\nelse\n  y = 0\nend\n");
        assert!(has_rule(&t, "if_stmt"));
        assert!(has_rule(&t, "elseif_clause"));
        assert!(has_rule(&t, "else_clause"));
    }

    #[test]
    fn for_and_while_loops() {
        let f = parse_matlab("for i = 1:10\n  s = s + i\nend\n");
        assert!(has_rule(&f, "for_stmt"));
        parses("while x > 0\n  x = x - 1\nend\n");
        parses("for k = 1:n\n  if k == 3\n    break\n  end\nend\n"); // nested, break
    }

    #[test]
    fn function_definitions() {
        parses("function y = sq(x)\n  y = x .^ 2\nend\n");
        let multi = parse_matlab("function [a, b] = two()\n  a = 1\n  b = 2\nend\n");
        assert!(has_rule(&multi, "func_returns"));
        parses("function noret(x)\n  disp(x)\nend\n");
    }

    #[test]
    fn switch_and_try() {
        parses("switch x\ncase 1\n  y = 1\notherwise\n  y = 0\nend\n");
        parses("try\n  risky()\ncatch err\n  handle(err)\nend\n");
    }

    #[test]
    fn anonymous_function() {
        let l = parse_matlab("sq = @(x) x .^ 2\n");
        assert!(has_rule(&l, "lambda"));
    }

    // --- Statement terminators ------------------------------------------

    #[test]
    fn semicolons_and_commas_separate_statements() {
        parses("a = 1; b = 2; c = 3\n"); // semicolons suppress display
        parses("x = 1, y = 2\n"); // comma separator
                                  // A trailing `;` is recorded as the statement terminator.
        let t = parse_matlab("a = 1;\n");
        assert!(has_rule(&t, "stmt_term"));
    }

    // --- Errors ---------------------------------------------------------

    #[test]
    fn unclosed_bracket_is_an_error() {
        assert!(try_parse_matlab("x = [1 2\n").is_err());
    }

    #[test]
    fn dangling_operator_is_an_error() {
        assert!(try_parse_matlab("y = 1 +\n").is_err());
    }
}
