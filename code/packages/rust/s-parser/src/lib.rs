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

/// Recursion-depth cap for the S [`GrammarParser`] — see
/// [`GrammarParser::with_max_depth`] and
/// [`parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH`] for why the underlying
/// guard exists at all (deep recursion through `parse_rule` can overflow the
/// *native* thread stack — an uncatchable process abort — before this
/// crate's own `Result`-returning entry points ever get a chance to report
/// anything). `s-repl` feeds this parser arbitrary, untrusted source at an
/// interactive prompt, so this is a real, not theoretical, attack surface.
///
/// # Why not the shared [`DEFAULT_MAX_RULE_DEPTH`] (128), and why not just
/// reuse `r-parser`'s value unmeasured
///
/// `r-parser`'s own `r.grammar` deliberately reuses `s.grammar`'s rule
/// names verbatim, and its `MAX_RULE_DEPTH` doc comment found
/// `DEFAULT_MAX_RULE_DEPTH` (128) unsafe-by-rejection there (128 rule-frames
/// only covers 8 real nesting levels). Reusing that grammar-shape argument
/// to justify the SAME cap here without independently measuring this
/// grammar's own floor would be exactly the mistake `j-parser`'s doc comment
/// warns against — a sibling's measured floor is a reasonable prior, not a
/// substitute for measuring this grammar's own native-stack behaviour. So
/// this crate was measured the same way, independently:
///
/// Native-stack floor (uncapped parser, parenthesised nesting `(((…1…)))`,
/// default-stack `std::thread::spawn` worker, debug build): parses safely up
/// to **23 levels**, crashes the process at **24** — close to, but not
/// identical to, `r-parser`'s measured 21/22 floor, confirming the two
/// grammars are similar but not byte-identical in compiled shape. In
/// rule-frame terms (the cap bounds recursion directly, so re-measured
/// against candidate `with_max_depth` values on the same 5000-level
/// adversarial input): safe through at least 298 (unlike `r-parser`, this
/// grammar did not crash by 298; not measured further since 200 already
/// gives a comfortable margin).
///
/// `MAX_RULE_DEPTH` is set to **200** — matching `r-parser`'s independently
/// measured value, now that both have been confirmed safe. Measured real-
/// input headroom at 200 (using the *capped* parser, so no crash risk at
/// all): a parenthesised nesting parses cleanly up to 15 levels (16 trips
/// the cap) — comfortably past anything a hand-written S expression needs,
/// and independently confirmed not to crash a default-stack thread even
/// thousands of levels past the cap (see this crate's tests).
const MAX_RULE_DEPTH: usize = 200;

/// Create a [`GrammarParser`] wired to the S grammar and the tokens of
/// `source`, with the recursion-depth guard ([`MAX_RULE_DEPTH`]) enabled so
/// pathologically deep nesting fails cleanly instead of overflowing the
/// native stack.
///
/// # Panics
///
/// Panics if tokenization fails. Use [`try_parse_s`] for a non-panicking path.
pub fn create_s_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_s(source);
    GrammarParser::new(tokens, _grammar::parser_grammar()).with_max_depth(MAX_RULE_DEPTH)
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
        .with_max_depth(MAX_RULE_DEPTH)
        .parse()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
fn nested_paren_source(n: usize) -> String {
    "(".repeat(n) + "1" + &")".repeat(n) + "\n"
}

/// Deeply-nested input must produce a recoverable error, not overflow the
/// native stack. We parse 5000 levels — far past `MAX_RULE_DEPTH` — on a
/// worker thread with a generous 32 MiB stack, so the *guard* is what stops
/// the recursion, not the stack running out.
#[test]
fn test_deeply_nested_input_returns_error_not_overflow() {
    let handle = std::thread::Builder::new()
        .name("s-parser-depth-guard-regression".to_string())
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let result = try_parse_s(&nested_paren_source(5000));
            assert!(
                result.is_err(),
                "deeply-nested input must fail with an error, not parse or crash"
            );
        })
        .expect("failed to spawn worker thread");
    handle
        .join()
        .expect("depth guard must keep the worker thread from crashing");
}

/// Input that nests *exactly up to* `MAX_RULE_DEPTH` still parses cleanly,
/// and one layer deeper cleanly trips the guard. These exact boundary counts
/// (15 legitimate levels) were found empirically by binary-searching against
/// increasing nesting counts at the production cap — see `MAX_RULE_DEPTH`'s
/// doc comment.
#[test]
fn test_nesting_up_to_cap_still_parses() {
    assert!(try_parse_s(&nested_paren_source(15)).is_ok(), "15 levels must stay under the cap");
    assert!(
        try_parse_s(&nested_paren_source(16)).is_err(),
        "one nesting level past the cap's measured limit must fail"
    );
}

/// A caller relying on `MAX_RULE_DEPTH` must have the guard trip *before*
/// the native stack overflows on a default-stack thread — otherwise a
/// production caller (e.g. `s-repl`, or `cargo test`'s own per-test thread)
/// would still crash. We parse far-too-deep input on a worker thread with
/// **no** `stack_size` override (the same ~2 MiB a default thread gets). A
/// clean `Err` (not a `join()` failure from a crashed thread) proves
/// `MAX_RULE_DEPTH` sits safely below the native overflow point on the
/// default stack.
#[test]
fn test_opt_in_cap_trips_before_overflow_on_default_stack() {
    let handle = std::thread::spawn(|| {
        let result = try_parse_s(&nested_paren_source(5000));
        assert!(result.is_err(), "deeply-nested input must error, not crash");
    });
    handle
        .join()
        .expect("MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack");
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
    fn empty_named_argument_parses() {
        // R-19: `arg = NAME EQ [expr]` — a named argument may omit its value.
        // This is what `switch`'s empty-arm fall-through relies on.
        assert!(parses("switch(\"a\", a = , b = \"hit\")\n"));
        // An empty arm followed by `)` (last-arm-empty) parses too.
        assert!(parses("switch(\"b\", a = \"A\", b = )\n"));
        // Multiple consecutive empties.
        assert!(parses("switch(\"a\", a = , b = , c = \"z\")\n"));
        // The empty value is grammatically valid in any call (eval rejects it
        // outside switch, but it must PARSE).
        assert!(parses("f(x = )\n"));
        // A normal named arg with a value still parses (no regression).
        assert!(parses("f(x = 1, y = 2)\n"));
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
