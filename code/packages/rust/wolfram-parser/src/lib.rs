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

/// Recursion-depth cap for the Wolfram [`GrammarParser`] — see
/// [`GrammarParser::with_max_depth`] and
/// [`parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH`] for why the underlying
/// guard exists at all (deep `(((…)))`/`f[g[h[…]]]` nesting recurses once per
/// `parse_rule` call and can overflow the *native* thread stack — an
/// uncatchable process abort — before this crate's own `Result`-returning
/// entry points ever get a chance to report anything).
///
/// # Why not the shared [`DEFAULT_MAX_RULE_DEPTH`] (128)
///
/// Wolfram's precedence cascade is unusually deep. One layer of `(...)`
/// grouping or `f[...]` application loops all the way back through the
/// *entire* cascade — `assignment -> replaceall -> rule -> condition ->
/// alternatives -> logical_or -> logical_and -> logical_not -> amp ->
/// comparison -> additive -> multiplicative -> unary -> power -> mapapply ->
/// patterntest -> postfix -> atom -> group -> expr` — 20 named-rule calls per
/// source-nesting level, several times deeper than a shallow ECMAScript-style
/// cascade. `DEFAULT_MAX_RULE_DEPTH` (tuned for that shallow shape) allows
/// only ~5 real nesting levels here (measured) — too easy for completely
/// ordinary nested Wolfram function calls (`f[g[h[x]]]` chains are routine)
/// to trip.
///
/// # How this number was derived — and a real conflict that changed it
///
/// Following the exact methodology behind `DEFAULT_MAX_RULE_DEPTH`: a
/// throwaway, isolated subprocess (never run in-process — a genuine overflow
/// aborts the whole process, so this must be explored somewhere a crash is
/// safe) built `((((…0…))))` with thousands of nesting levels through this
/// crate's own `create_wolfram_parser`, and binary-searched — on a worker
/// thread with the **default ~2 MiB stack** (no `stack_size` override) — for
/// the `with_max_depth` value at which the parse stops overflowing and starts
/// returning a clean `Err`. Result (debug build, this toolchain, stable
/// across 5 repeated trials): safe at 275, overflowing at 278. A cap can only
/// refuse to recurse further, it cannot shrink the frames already pushed, so
/// ~276 `parse_rule` frames is the hard ceiling this grammar can ever reach
/// on a 2 MiB stack — about 11 real bracket-nesting levels at the absolute
/// edge (zero margin). A first pass at this constant used `200` (comfortable
/// margin below that floor, ~8 real levels) — safe for a bare default-stack
/// caller, matching `macsyma-parser` and `matlab-parser`'s own caps.
///
/// **That value broke `wolfram-runtime`'s existing, passing
/// `moderate_nesting_still_evaluates` test** (40 levels of `(...)` nesting,
/// expected to evaluate — a legitimate case, not a DoS probe). Investigating
/// why revealed `wolfram-runtime` does not rely on a bare thread at all: its
/// `eval_to_outputs` spawns the *actual* parse onto a worker thread with a
/// **512 MiB** stack (`EVAL_STACK_SIZE`) and gates input with its own
/// *token-count* cap (`MAX_STATEMENT_TOKENS = 2000`, checked against the real
/// lexer output **before** the parser ever runs) — not a bracket-depth
/// count. That is exactly the "frontend that already guards itself on an
/// enlarged stack" scenario `DEFAULT_MAX_RULE_DEPTH`'s own doc comment warns
/// a parser-level cap would preempt (it names
/// `python-to-semantic-ir`/`javascript-to-semantic-ir` as the same pattern).
/// Wolfram's 20-rule-per-level cascade makes this unavoidable: **even 40
/// levels of real nesting needs ~840 `parse_rule` frames — already past the
/// bare-2-MiB-stack crash floor (~276) *regardless of any cap we pick*.**
/// `wolfram-runtime`'s reliance on its own big-stack thread is load-bearing,
/// not incidental; no `with_max_depth` value can simultaneously (a) sit below
/// ~276 (to protect a bare-stack caller) and (b) sit above ~840 (to keep this
/// existing, legitimate case working). Those two requirements are
/// mathematically incompatible for this grammar, so this crate cannot be the
/// thing that makes a *bare*-stack caller of `parse_wolfram` safe from ~12+
/// levels of nesting — only `wolfram-runtime`'s own enlarged-stack + token-cap
/// design (unaffected by anything in this crate) does that today.
///
/// Given that, this constant is calibrated for the **only real consumer's**
/// actual deployment (an enlarged-stack worker thread), not a bare default
/// stack: `2000` gives 98 real nesting levels (measured: 98 parses cleanly,
/// 99 trips the cap) — well past the tested 40, with margin for later
/// "moderate" examples — while comfortably tripping (fast: well under a
/// second) before `wolfram-runtime`'s own 512 MiB stack could ever overflow
/// (512 MiB is ~256× the 2 MiB floor above, i.e. safe past ~70,000 raw
/// frames). It does *not* try to reach `wolfram-runtime`'s own theoretical
/// worst case (`MAX_STATEMENT_TOKENS = 2000` tokens could encode up to ~999
/// nesting levels) — measured directly, parsing real nesting anywhere near
/// that scale takes **tens of seconds** even on a big stack (this
/// implementation's packrat memo keys are `format!`-allocated strings, and
/// the furthest-failure tracking is an O(n) `Vec::contains` scan per
/// attempt — a real, separate performance concern, flagged as follow-up work
/// rather than fixed here), so a cap that size would trade one DoS vector
/// (stack overflow) for another (a multi-second-to-multi-minute CPU burn per
/// request). `2000` stays inside the fast, practical range while fully
/// covering every currently-legitimate case.
///
/// **The upshot for future callers**: unlike `macsyma-parser` /
/// `matlab-parser` (whose `200` genuinely protects a bare default-stack
/// caller), this crate's cap does *not* make `parse_wolfram` /
/// `create_wolfram_parser` / `try_parse_wolfram` safe to call directly on an
/// ordinary thread with deeply-nested input — a caller without its own
/// enlarged-stack + complexity-budget strategy (like `wolfram-runtime`'s)
/// remains exposed to a native stack overflow past ~11 real nesting levels,
/// exactly as before this change. Such a caller should either reuse
/// `wolfram-runtime`'s pattern (parse on a large `stack_size` thread, gate
/// input by token count first) or explicitly tighten the cap itself, e.g.
/// `create_wolfram_parser(src).with_max_depth(150)`, accepting a much lower
/// real-nesting ceiling in exchange for bare-thread safety.
const MAX_RULE_DEPTH: usize = 2000;

/// Create a [`GrammarParser`] wired to the Wolfram grammar and the tokens of
/// `source`, with the recursion-depth guard ([`MAX_RULE_DEPTH`]) enabled so
/// pathologically deep nesting fails cleanly instead of overflowing the
/// native stack.
///
/// # Panics
///
/// Panics if tokenization fails. Use [`try_parse_wolfram`] for a non-panicking
/// path.
pub fn create_wolfram_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_wolfram(source);
    GrammarParser::new(tokens, _grammar::parser_grammar()).with_max_depth(MAX_RULE_DEPTH)
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
        .with_max_depth(MAX_RULE_DEPTH)
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

    // --- W-21 pattern operator sugar: |, /;, ?, //. ----------------------

    #[test]
    fn alternatives_operator_parses_via_alternatives_rule() {
        // `a | b | c` matches the new `alternatives` infix level.
        assert_eq!(
            first_token_of(&parse_wolfram("a | b\n"), "alternatives").as_deref(),
            Some("|")
        );
        assert!(contains_rule(&parse_wolfram("a | b | c\n"), "alternatives"));
        // `||` (OR) still wins over `|` (longest-match by declaration order): the
        // `alternatives` rule is a transparent wrapper here (always in the cascade),
        // so it matches NO `|` token — `first_token_of` finds no ALTERNATIVES op.
        assert!(contains_rule(&parse_wolfram("a || b\n"), "logical_or"));
        assert_eq!(
            first_token_of(&parse_wolfram("a || b\n"), "alternatives"),
            None
        );
    }

    #[test]
    fn condition_operator_parses_and_binds_looser_than_alternatives() {
        assert_eq!(
            first_token_of(&parse_wolfram("p /; t\n"), "condition").as_deref(),
            Some("/;")
        );
        // `a | b /; t` — the `condition` node sits ABOVE the `alternatives` node
        // (so `/;` is looser than `|`): the AST contains both, with condition
        // wrapping alternatives.
        let ast = parse_wolfram("a | b /; t\n");
        assert!(contains_rule(&ast, "condition") && contains_rule(&ast, "alternatives"));
        // A real Condition example.
        assert!(parses("x_ /; x > 2\n"));
    }

    #[test]
    fn patterntest_operator_parses_via_patterntest_rule() {
        assert_eq!(
            first_token_of(&parse_wolfram("p ? f\n"), "patterntest").as_deref(),
            Some("?")
        );
        // `_?EvenQ` — the `?` binds tighter than application/list, so it parses.
        assert!(parses("_?EvenQ\n"));
        assert!(parses("x_?IntegerQ\n"));
        // Chained `?` (left-associative).
        assert!(parses("_?IntegerQ?Positive\n"));
    }

    #[test]
    fn replacerepeated_operator_parses_at_the_replaceall_level() {
        // `//.` shares the `replaceall` rule with `/.`.
        assert_eq!(
            first_token_of(&parse_wolfram("e //. r\n"), "replaceall").as_deref(),
            Some("//.")
        );
        // `//.` must NOT lex as `/` `/.` — the whole operator is one token, so
        // `{1, 2, 3} //. 2 -> 99` parses as ReplaceRepeated of the rule.
        assert!(parses("{1, 2, 3} //. 2 -> 99\n"));
        // Mixed chain with `/.` at the same level.
        assert!(parses("x /. a //. b\n"));
    }

    #[test]
    fn syntax_error_is_reported() {
        assert!(try_parse_wolfram("1 +\n").is_err());
        assert!(try_parse_wolfram("f[x\n").is_err()); // unclosed bracket
        assert!(try_parse_wolfram("x[[1\n").is_err()); // unclosed double bracket
        assert!(try_parse_wolfram("f /@\n").is_err()); // map with no right operand
        assert!(try_parse_wolfram("a |\n").is_err()); // W-21: `|` with no right operand
        assert!(try_parse_wolfram("p ?\n").is_err()); // W-21: `?` with no right operand
        assert!(try_parse_wolfram("e //.\n").is_err()); // W-21: `//.` with no rules
    }

    #[test]
    fn a_small_wolfram_program_parses() {
        assert!(parses(
            "f[x_] := x^2\nf[3] + Sin[0]\n{1, 2, 3} /. a_ -> a + 1\n"
        ));
    }

    // --- W-11 pure functions: #, #n, ##, & ------------------------------

    #[test]
    fn slot_forms_parse_via_the_slot_rule() {
        // `#`, `#2`, `##` are all atoms matching the new `slot` rule.
        assert!(contains_rule(&parse_wolfram("#\n"), "slot"));
        assert!(contains_rule(&parse_wolfram("#2\n"), "slot"));
        assert!(contains_rule(&parse_wolfram("##\n"), "slot"));
        // A numbered slot is HASH then NUMBER inside the slot node.
        assert_eq!(
            first_token_of(&parse_wolfram("#2\n"), "slot").as_deref(),
            Some("#")
        );
    }

    #[test]
    fn ampersand_postfix_parses_via_the_amp_level() {
        // `#^2 &` matches the new `amp` postfix level.
        assert!(contains_rule(&parse_wolfram("#^2 &\n"), "amp"));
        assert!(parses("(#1 + #2) &\n"));
        assert!(parses("# &\n"));
        assert!(parses("(#^2) &\n"));
    }

    #[test]
    fn amp_binds_looser_than_power_so_hash_pow_2_amp_is_a_function_of_a_power() {
        // The pinned precedence: `#^2 &` is `(#^2)&`, NOT `#^(2&)`. Concretely,
        // the `&`'s operand (the `power` under `amp`) must contain the POWER
        // operator — i.e. the `^` is INSIDE the function body, below the `amp`.
        let ast = parse_wolfram("#^2 &\n");
        // The amp level must be present, and a `power` node carrying `^` must
        // sit beneath it (the body), proving `&` captured the whole `#^2`.
        assert!(contains_rule(&ast, "amp"), "amp level missing");
        assert!(
            contains_rule(&ast, "power"),
            "the `^` must parse as a power INSIDE the function body"
        );
        // And the whole thing still parses as one statement.
        assert!(parses("#^2 &\n"));
    }

    #[test]
    fn named_function_long_form_parses_as_ordinary_application() {
        // `Function[x, x^2]` and `Function[{x, y}, x + y]` are plain `Head[args]`
        // applications — no special grammar, lowered to the Function head later.
        assert!(parses("Function[x, x^2]\n"));
        assert!(parses("Function[{x, y}, x + y]\n"));
        // Applied immediately: `Function[x, x^2][5]` is postfix application.
        assert!(parses("Function[x, x^2][5]\n"));
        assert!(contains_rule(&parse_wolfram("Function[x, x^2][5]\n"), "postfix"));
    }

    #[test]
    fn pure_function_applied_immediately_parses() {
        // The whole point: `(#^2)&[5]` and `(#1+#2)&[3,4]` apply a pure function.
        assert!(parses("(#^2)&[5]\n"));
        assert!(parses("(#1 + #2)&[3, 4]\n"));
        assert!(parses("#&[9]\n"));
        // Composes inside higher-order builtins.
        assert!(parses("Map[#^2 &, {1, 2, 3}]\n"));
        assert!(parses("Select[{1, 2, 3, 4}, Mod[#, 2] == 0 &]\n"));
        assert!(parses("Nest[# + 1 &, 0, 3]\n"));
    }

    #[test]
    fn malformed_pure_function_inputs_are_syntax_errors() {
        // A bare `&` with nothing to its left has no operand.
        assert!(try_parse_wolfram("&\n").is_err());
        // A `&` whose body is itself incomplete (`+ &` has no left operand for
        // the `+`) cannot parse.
        assert!(try_parse_wolfram("+ &\n").is_err());
        // An applied pure function with an unclosed bracket is a syntax error.
        assert!(try_parse_wolfram("(#^2)&[5\n").is_err());
    }

    // --- Recursion-depth guard (DoS hardening) --------------------------
    //
    // These three tests mirror the exact methodology used to validate
    // `parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH` (see that file's own
    // `test_deeply_nested_input_returns_error_not_overflow` /
    // `test_nesting_up_to_cap_still_parses` /
    // `test_opt_in_cap_trips_before_overflow_on_default_stack`), but exercise
    // the REAL Wolfram grammar and the crate's actual `MAX_RULE_DEPTH` (2000).
    //
    // Unlike `macsyma-parser`/`matlab-parser`, these tests run on an
    // **enlarged** worker thread (matching `wolfram-runtime`'s own
    // `EVAL_STACK_SIZE`), not the bare default ~2 MiB stack — see
    // `MAX_RULE_DEPTH`'s doc comment for why: Wolfram's 20-rule-per-level
    // cascade means even the legitimate, already-tested 40-level nesting case
    // (`wolfram-runtime`'s `moderate_nesting_still_evaluates`) needs more
    // native stack than a bare default thread has, with or without this
    // crate's cap. Testing on a bare thread here would just prove the
    // (already-known, unrelated-to-this-cap) fact that a bare-stack caller
    // crashes on moderate Wolfram nesting; it would not tell us anything about
    // whether `MAX_RULE_DEPTH` itself is well-calibrated.

    /// Build `n` nested parens around a `0`, e.g. `((0))\n` for `n == 2`.
    fn nested_paren_source(n: usize) -> String {
        format!("{}0{}\n", "(".repeat(n), ")".repeat(n))
    }

    /// Deeply-nested input must produce a recoverable error, not overflow the
    /// native stack (an uncatchable process abort). We parse 5000 levels — far
    /// past `MAX_RULE_DEPTH` — on a worker thread with a 1 GiB stack (double
    /// `wolfram-runtime`'s own 512 MiB `EVAL_STACK_SIZE`), so the *guard* is
    /// what stops the recursion, not the stack running out.
    ///
    /// Note: unlike the synthetic single-rule grammar in `grammar_parser.rs`,
    /// Wolfram's entry rule (`program = { statement_line }`) is a zero-or-more
    /// repetition. When the single top-level statement fails deep inside
    /// (because the depth cap refused to recurse further), the repetition
    /// itself still succeeds trivially with *zero* statements matched, so the
    /// `GrammarParseError` surfaced by `parse()` is the generic "unexpected
    /// leftover token" message rather than the specific "nests deeper than
    /// the supported limit" phrasing `grammar_parser.rs`'s tests see for a
    /// grammar whose entry point IS the recursive rule. Either way the parse
    /// still fails cleanly with a `Result::Err` instead of crashing, which is
    /// the property under test here.
    #[test]
    fn test_deeply_nested_input_returns_error_not_overflow() {
        let handle = std::thread::Builder::new()
            .name("wolfram-depth-guard-regression".to_string())
            .stack_size(1024 * 1024 * 1024)
            .spawn(|| {
                let source = nested_paren_source(5000);
                let result = try_parse_wolfram(&source);
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
    /// and one layer deeper cleanly trips the guard. These exact boundary
    /// counts (98 legitimate levels — comfortably past `wolfram-runtime`'s own
    /// tested 40-level `moderate_nesting_still_evaluates` case) were found
    /// empirically by binary-searching `create_wolfram_parser` against
    /// increasing nesting counts at the production cap, on a worker thread
    /// sized like `wolfram-runtime`'s own 512 MiB `EVAL_STACK_SIZE` — see
    /// `MAX_RULE_DEPTH`'s doc comment.
    #[test]
    fn test_nesting_up_to_cap_still_parses() {
        let handle = std::thread::Builder::new()
            .stack_size(512 * 1024 * 1024)
            .spawn(|| {
                let ok_source = nested_paren_source(98);
                let ast = parse_wolfram(&ok_source);
                assert_eq!(ast.rule_name, "program");

                let tripped_source = nested_paren_source(99);
                assert!(
                    try_parse_wolfram(&tripped_source).is_err(),
                    "one nesting level past the cap's measured limit must fail"
                );
            })
            .expect("failed to spawn worker thread");
        handle.join().expect("worker thread must not crash");
    }

    /// A caller relying on `MAX_RULE_DEPTH` must have the guard trip *before*
    /// the native stack overflows — otherwise a production caller would still
    /// crash. We parse far-too-deep input on a worker thread sized like
    /// `wolfram-runtime`'s own 512 MiB `EVAL_STACK_SIZE` (the realistic
    /// deployment this cap is calibrated for — see `MAX_RULE_DEPTH`'s doc
    /// comment for why a *bare* default-stack thread cannot be the target
    /// here). A clean `Err` (not a `join()` failure from a crashed thread, and
    /// fast — not the multi-second-plus cost of parsing thousands of real
    /// nesting levels) proves `MAX_RULE_DEPTH` sits safely below the native
    /// overflow point on that stack, and trips quickly rather than doing
    /// expensive work first.
    #[test]
    fn test_opt_in_cap_trips_before_overflow_on_enlarged_stack() {
        let handle = std::thread::Builder::new()
            .stack_size(512 * 1024 * 1024)
            .spawn(|| {
                let source = nested_paren_source(5000);
                let result = try_parse_wolfram(&source);
                assert!(result.is_err(), "deeply-nested input must error, not crash");
            })
            .expect("failed to spawn worker thread");
        handle
            .join()
            .expect("MAX_RULE_DEPTH must trip BEFORE native overflow on the enlarged stack");
    }
}
