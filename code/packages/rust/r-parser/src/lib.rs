//! # R Parser — building a syntax tree for the R language.
//!
//! Turns the token stream from [`coding_adventures_r_lexer`] into a parse tree
//! using the generic grammar-driven
//! [`GrammarParser`](parser::grammar_parser::GrammarParser), driven by the
//! embedded `r.grammar` (`src/_grammar.rs`). It hand-writes no parsing logic.
//!
//! ## Built to share the S evaluator
//!
//! R is "an implementation of the S language", so `r.grammar` deliberately uses
//! the **same rule names** as `s.grammar` (`assignment`, `comparison`,
//! `additive`, …, `postfix`, `call_suffix`, `primary`, …). The `s-runtime`
//! tree-walker dispatches on `rule_name`, so the very same evaluator can run R
//! programs once R-3 wires it up. The only grammar differences from S are the
//! places R departs from S:
//!
//! - **`=` and `->>` are assignment operators** (S uses `_` and lacks `->>`).
//! - **typed `NA`** atoms `NA_integer_` / `NA_real_` / `NA_character_`.
//!
//! ```text
//! R source
//!    |
//!    v
//! coding_adventures_r_lexer::tokenize_r   ->  Vec<Token>
//!    |
//!    v
//! parser::GrammarParser  (driven by the embedded r.grammar)
//!    |
//!    v
//! GrammarASTNode  <- the tree the (shared) s-runtime walks
//! ```

use coding_adventures_r_lexer::{tokenize_r, try_tokenize_r};
use parser::grammar_parser::{GrammarASTNode, GrammarParser};
mod _grammar;

/// Recursion-depth cap for the R [`GrammarParser`] — see
/// [`GrammarParser::with_max_depth`] and
/// [`parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH`] for why the underlying
/// guard exists at all (deep recursion through `parse_rule` can overflow the
/// *native* thread stack — an uncatchable process abort — before this
/// crate's own `Result`-returning entry points ever get a chance to report
/// anything). `r-repl` feeds this parser arbitrary, untrusted source at an
/// interactive prompt, so this is a real, not theoretical, attack surface.
///
/// # Why not the shared [`DEFAULT_MAX_RULE_DEPTH`] (128)
///
/// R's precedence chain (`assignment -> comparison -> range -> pipe ->
/// special -> additive -> multiplicative -> power -> unary -> postfix ->
/// primary -> LPAREN expr RPAREN -> expr -> …`) re-enters roughly a dozen
/// named rules for every one level of source nesting — measured directly
/// (binary search, parsing `(((…1…)))` at increasing depth with an
/// *uncapped* parser on a `std::thread::spawn` worker with the default
/// ~2 MiB stack, in a **debug** build to match this crate's own `cargo test`
/// conditions): 128 rule-frames only covers 8 real nesting levels before the
/// cap would trip, which is not implausible for real R code (deeply chained
/// function calls). `DEFAULT_MAX_RULE_DEPTH` would therefore be *safe*
/// (never risks a crash) but reject legitimate, non-adversarial input — the
/// same class of problem that gave `macsyma-parser` and `apl-parser`/
/// `j-parser` their own bespoke values.
///
/// Measured native-stack floor (uncapped parser, parenthesised nesting,
/// default-stack worker thread, debug build): parses safely up to **21
/// levels**, crashes the process at **22**. In rule-frame terms this maps to
/// a crash somewhere around rule-depth 298 (the cap itself, not real input
/// depth, is what bounds `GrammarParser`'s recursion, so the same floor was
/// re-measured directly against candidate `with_max_depth` values on the
/// **same 5000-level adversarial input**: safe through 297, crashes at 298).
///
/// `MAX_RULE_DEPTH` is set to **200** — about 33% below that 297-rule-frame
/// floor (comparable margin to `apl-parser`'s ~26.5% and `j-parser`'s ~30%),
/// coincidentally the same value `macsyma-parser` measured for its own,
/// similarly-shaped precedence chain. Measured real-input headroom at 200
/// (using the *capped* parser, so no crash risk at all): a parenthesised
/// nesting parses cleanly up to 14 levels (15 trips the cap) — comfortably
/// past anything a hand-written R expression needs, and independently
/// confirmed not to crash a default-stack thread even thousands of levels
/// past the cap (see this crate's tests).
const MAX_RULE_DEPTH: usize = 200;

/// Create a [`GrammarParser`] wired to the R grammar and the tokens of
/// `source`, with the recursion-depth guard ([`MAX_RULE_DEPTH`]) enabled so
/// pathologically deep nesting fails cleanly instead of overflowing the
/// native stack.
///
/// # Panics
///
/// Panics if tokenization fails. Use [`try_parse_r`] for a non-panicking path.
pub fn create_r_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_r(source);
    GrammarParser::new(tokens, _grammar::parser_grammar()).with_max_depth(MAX_RULE_DEPTH)
}

/// Parse R source text into a [`GrammarASTNode`] rooted at the `program` rule.
///
/// # Panics
///
/// Panics on a lexical or syntax error. Use [`try_parse_r`] to handle errors.
///
/// # Example
///
/// ```
/// use coding_adventures_r_parser::parse_r;
/// let ast = parse_r("data_frame <- c(1, 2, 3)\nmean(data_frame)\n");
/// assert_eq!(ast.rule_name, "program");
/// ```
pub fn parse_r(source: &str) -> GrammarASTNode {
    create_r_parser(source)
        .parse()
        .unwrap_or_else(|e| panic!("R parse failed: {e}"))
}

/// Parse R source text, returning a `Result` instead of panicking.
pub fn try_parse_r(source: &str) -> Result<GrammarASTNode, String> {
    let tokens = try_tokenize_r(source)?;
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
        .name("r-parser-depth-guard-regression".to_string())
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let result = try_parse_r(&nested_paren_source(5000));
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
/// (14 legitimate levels) were found empirically by binary-searching against
/// increasing nesting counts at the production cap — see `MAX_RULE_DEPTH`'s
/// doc comment.
#[test]
fn test_nesting_up_to_cap_still_parses() {
    assert!(try_parse_r(&nested_paren_source(14)).is_ok(), "14 levels must stay under the cap");
    assert!(
        try_parse_r(&nested_paren_source(15)).is_err(),
        "one nesting level past the cap's measured limit must fail"
    );
}

/// A caller relying on `MAX_RULE_DEPTH` must have the guard trip *before*
/// the native stack overflows on a default-stack thread — otherwise a
/// production caller (e.g. `r-repl`, or `cargo test`'s own per-test thread)
/// would still crash. We parse far-too-deep input on a worker thread with
/// **no** `stack_size` override (the same ~2 MiB a default thread gets). A
/// clean `Err` (not a `join()` failure from a crashed thread) proves
/// `MAX_RULE_DEPTH` sits safely below the native overflow point on the
/// default stack.
#[test]
fn test_opt_in_cap_trips_before_overflow_on_default_stack() {
    let handle = std::thread::spawn(|| {
        let result = try_parse_r(&nested_paren_source(5000));
        assert!(result.is_err(), "deeply-nested input must error, not crash");
    });
    handle
        .join()
        .expect("MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack");
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

    /// The value of the first operator/keyword token anywhere under `node`
    /// belonging to a node whose rule is `rule` (used to check which operator a
    /// construct matched).
    fn first_token_of(node: &GrammarASTNode, rule: &str) -> Option<String> {
        fn tok(n: &GrammarASTNode) -> Option<String> {
            n.children.iter().find_map(|c| match c {
                ASTNodeOrToken::Token(t) => Some(t.value.clone()),
                _ => None,
            })
        }
        fn search(n: &GrammarASTNode, rule: &str) -> Option<String> {
            if n.rule_name == rule {
                if let Some(t) = tok(n) {
                    return Some(t);
                }
            }
            n.children.iter().find_map(|c| match c {
                ASTNodeOrToken::Node(child) => search(child, rule),
                ASTNodeOrToken::Token(_) => None,
            })
        }
        search(node, rule)
    }

    fn parses(src: &str) -> bool {
        try_parse_r(src).is_ok()
    }

    #[test]
    fn program_is_the_root() {
        assert_eq!(parse_r("1\n").rule_name, "program");
    }

    #[test]
    fn underscore_name_parses_as_one_identifier() {
        // The R difference: `data_frame` is one name, used here as an lvalue.
        assert!(parses("data_frame <- 1\n"));
    }

    #[test]
    fn r_assignment_operators_parse() {
        for src in [
            "x <- 1\n",
            "x = 1\n", // R: `=` is assignment
            "x <<- 1\n",
            "1 -> x\n",
            "1 ->> x\n", // R: right super-assignment
        ] {
            assert!(parses(src), "should parse: {src:?}");
            assert!(contains_rule(&parse_r(src), "assignment"), "{src:?}");
        }
    }

    #[test]
    fn equals_assignment_uses_the_eq_token() {
        // `x = 1` should match the assignment rule with the `=` operator.
        assert_eq!(
            first_token_of(&parse_r("x = 1\n"), "assignment").as_deref(),
            Some("=")
        );
    }

    #[test]
    fn named_argument_still_distinct_from_assignment() {
        // Inside a call, `mean(x = 1)` is a named argument (tried before the
        // positional `expr` alternative), not a nested assignment statement.
        let ast = parse_r("mean(x = 1)\n");
        assert!(contains_rule(&ast, "arg_list"));
        assert!(parses("f(x == 1)\n")); // `==` stays a positional comparison
    }

    #[test]
    fn empty_named_argument_parses() {
        // R-19: `arg = NAME EQ [expr]` — a named argument may omit its value,
        // which `switch`'s empty-arm fall-through relies on.
        assert!(parses("switch(\"a\", a = , b = \"hit\")\n"));
        assert!(parses("switch(\"b\", a = \"A\", b = )\n"));
        assert!(parses("switch(\"a\", a = , b = , c = \"z\")\n"));
        // Empty value parses in any call (eval rejects it outside switch).
        assert!(parses("f(x = )\n"));
        assert!(parses("f(x = 1, y = 2)\n"));
    }

    #[test]
    fn typed_na_constants_parse() {
        for src in ["NA_integer_\n", "NA_real_\n", "NA_character_\n"] {
            assert!(parses(src), "should parse: {src:?}");
        }
    }

    #[test]
    fn shared_with_s_constructs_parse() {
        // Arithmetic precedence cascade, sequence, infix, indexing, $, calls.
        let ast = parse_r("y <- 1:3 + 2 * 3 ^ 2\n");
        for rule in ["additive", "multiplicative", "power", "range"] {
            assert!(contains_rule(&ast, rule), "missing {rule}");
        }
        assert!(contains_rule(&parse_r("x %in% c(1, 2)\n"), "special"));
        assert!(contains_rule(&parse_r("x[1]\n"), "index_suffix"));
        assert!(contains_rule(&parse_r("x[[1]]\n"), "dindex_suffix"));
        assert!(contains_rule(&parse_r("df$col\n"), "dollar_suffix"));
        assert!(contains_rule(&parse_r("c(1, 2, 3)\n"), "call_suffix"));
    }

    #[test]
    fn functions_and_control_flow_parse() {
        assert!(contains_rule(
            &parse_r("sq <- function(v) v * v\n"),
            "func_def"
        ));
        assert!(contains_rule(
            &parse_r("f <- function(x, n = 1) x + n\n"),
            "param_list"
        ));
        assert!(contains_rule(&parse_r("if (x > 0) 1 else -1\n"), "if_expr"));
        assert!(contains_rule(
            &parse_r("for (i in 1:3) print(i)\n"),
            "for_expr"
        ));
        assert!(contains_rule(
            &parse_r("while (x < 10) x <- x + 1\n"),
            "while_expr"
        ));
        assert!(contains_rule(&parse_r("repeat break\n"), "repeat_expr"));
        assert!(contains_rule(
            &parse_r("{\n  a <- 1\n  a + 1\n}\n"),
            "block"
        ));
    }

    #[test]
    fn multi_line_and_semicolons() {
        assert!(parses("sum(1,\n    2,\n    3)\n"));
        assert!(parses("x <- 1; y <- 2; x + y\n"));
        assert!(parses("mean(x)")); // trailing newline optional
    }

    #[test]
    fn syntax_error_is_reported() {
        assert!(try_parse_r("1 +\n").is_err());
    }

    #[test]
    fn the_canonical_r_session_parses() {
        assert!(parses(
            "data_frame <- c(1, 2, 3)\nmean(data_frame)\ndata_frame * 10 + c(1, 2)\n"
        ));
    }

    #[test]
    fn pipe_and_lambda_parse() {
        // The pipe produces a `pipe` node; the backslash lambda a `func_def`.
        assert!(contains_rule(&parse_r("x |> f()\n"), "pipe"));
        assert!(parses("c(3, 1, 2) |> sort() |> rev()\n"));
        assert!(contains_rule(&parse_r("sq <- \\(x) x ^ 2\n"), "func_def"));
        assert!(parses("sapply(1:3, \\(n) n * n)\n"));
    }
}
