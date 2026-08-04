//! # Reduce Parser — building a syntax tree for Reduce (a subset).
//!
//! Turns the token stream from [`coding_adventures_reduce_lexer`] into a
//! parse tree using the generic
//! [`GrammarParser`](parser::grammar_parser::GrammarParser), driven by the
//! embedded `reduce.grammar` (`src/_grammar.rs`). It hand-writes no parsing
//! logic. A sibling of `derive-parser` / `macsyma-parser` / `wolfram-parser`.
//! See `code/specs/MA08-reduce-language.md`.
//!
//! ## What the tree captures
//!
//! Every Reduce expression parses down to ordinary infix/postfix operators
//! over `head(args)`-shaped calls, with a small statement layer (`:=`,
//! `if/then/else`, `<< ... >>`) on top; this parser produces the surface
//! tree whose rule names (`assignment`, `if_expr`, `group_expr`,
//! `logical_or`, `comparison`, `cons`, `additive`, `multiplicative`,
//! `power`, `postfix`, `atom`, `list_literal`, …) a future `reduce-runtime`
//! (R-4) will lower into the canonical `symbolic-ir` heads (`Plus`/`Times`/
//! `Power`/`Assign`/`Define`/`If`/`CompoundExpression`/`List`/`Cons`/…).
//!
//! ```text
//! Reduce source
//!    |
//!    v
//! coding_adventures_reduce_lexer::tokenize_reduce  ->  Vec<Token>
//!    |
//!    v
//! parser::GrammarParser  (driven by the embedded reduce.grammar)
//!    |
//!    v
//! GrammarASTNode  <- the tree R-4 lowers to symbolic-ir
//! ```

use coding_adventures_reduce_lexer::{tokenize_reduce, try_tokenize_reduce};
use parser::grammar_parser::{GrammarASTNode, GrammarParser};
mod _grammar;

/// Recursion-depth cap for the Reduce [`GrammarParser`] — see
/// [`GrammarParser::with_max_depth`] and
/// [`parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH`] for why the underlying
/// guard exists at all (deep recursion through `parse_rule` can overflow the
/// *native* thread stack — an uncatchable process abort — before this
/// crate's own `Result`-returning entry points ever get a chance to report
/// anything).
///
/// # Five recursion shapes, not one — measured independently, per MA06 §6's
/// established methodology
///
/// `reduce.grammar` has no analogue of `apl.grammar`/`j.grammar`'s own
/// right-recursive `noun_expr` chain — every "chain of operators at one
/// precedence tier" production here (`logical_or`, `logical_and`,
/// `additive`, `multiplicative`, `postfix`'s call chain, `arglist`,
/// `group_expr`'s statement sequence) is written with EBNF `{ x }`
/// repetition, not right-recursive self-reference. This was checked
/// directly, not assumed: a throwaway probe grammar built from
/// [`GrammarElement::Repetition`](grammar_tools::parser_grammar::GrammarElement)
/// alone (mirroring `arglist = expr { COMMA expr }`'s exact shape) was fed
/// up to *one million* repeated items on a default-stack (~2 MiB) worker
/// thread with an uncapped parser — zero crashes. `match_element`'s own
/// `Repetition` arm (`parser::grammar_parser`) is a plain `loop { ... }`:
/// each iteration's `match_element` call returns before the next iteration
/// begins, so the *native* call stack never grows with iteration count,
/// only with the nesting *within* one iteration's own match. Width alone is
/// not a recursion-depth risk in this grammar engine — measuring it as if
/// it might be (the way MA06 §6 asked future crates to treat `j.grammar`'s
/// own `verb_train` flat repetition) would have been assuming a *shape*
/// analogy the actual `Repetition` implementation does not bear out here.
///
/// What genuinely recurses in `reduce.grammar` are its five distinct
/// self-referential (right-recursive, `[ x ]`-optional-wrapped or
/// ordered-choice-cycling) productions, each measured independently below
/// with the same methodology every sibling `*-parser` crate's own
/// `MAX_RULE_DEPTH` doc comment uses: binary search, an *uncapped*
/// `GrammarParser` (`max_depth = usize::MAX`), a `std::thread::spawn`
/// worker with the **default ~2 MiB stack** (no `stack_size` override), one
/// fresh **subprocess per data point** (a real native-stack overflow calls
/// `process::abort()`, which kills the whole process, not just the
/// offending thread — an in-process loop cannot survive past the first
/// crash to report a clean number), in a **debug** build (`cargo test`'s
/// own default) since debug frames are meaningfully larger than release
/// frames.
///
/// 1. **Parenthesised nesting**, `((((…5…))))` — `group -> expr -> (
///    if_expr/group_expr fail ) -> assignment -> logical_or -> logical_and
///    -> logical_not -> comparison -> cons -> additive -> multiplicative ->
///    unary -> power -> postfix -> atom -> group -> …` — cycles through the
///    *entire* precedence cascade every nesting level. Measured: parses
///    safely up to **19 levels**, crashes the process at **20**.
/// 2. **A flat, right-associative `:=` chain**, `a:=a:=a:=…:=5` —
///    `assignment`'s own `[ ASSIGN expr ]` continuation. Measured: parses
///    safely up to **102 levels**, crashes at **103**.
/// 3. **A flat `if`/`else` chain**, `if 1 then 1 else if 1 then 1 else …
///    else 5` — `if_expr`'s own `[ "else" expr ]` continuation, where
///    `expr` tries (and immediately commits to) `if_expr` again. Measured:
///    parses safely up to **102 levels**, crashes at **103**.
/// 4. **A flat cons (`.`) chain**, `a.a.a. … .a` — `cons`'s own `[ DOT cons
///    ]` continuation. Built from `NAME` atoms, not `NUMBER` literals: an
///    early draft of this measurement used `1.1.1. … .1`, but `NUMBER`'s
///    own regex (`[0-9]+\.?[0-9]*`) greedily absorbs one trailing
///    `.digit` run per token, so `1.1.1.1` lexes as `NUMBER("1.1") DOT
///    NUMBER("1.1")`, not `NUMBER("1") DOT NUMBER("1") DOT NUMBER("1") DOT
///    NUMBER("1")` — silently *halving* the intended chain length. Caught
///    by cross-checking this shape's nesting-count floor against its
///    independently-measured rule-frame floor (see the table below): the
///    numeric-literal version measured a suspiciously high 327-safe /
///    328-crash floor with no per-level frame cost consistent with the
///    other four shapes; switching to `NAME` atoms (immune to the digit-
///    merging ambiguity) produced a floor that *is* consistent. Measured
///    (`NAME` version): parses safely up to **163 levels**, crashes at
///    **164**.
/// 5. **A flat power (`^`) chain**, `1^1^1^ … ^1` — `power`'s own `[ CARET
///    unary ]` continuation (through `unary`, which falls back to `power`
///    absent a leading `-`; `^` shares no characters with `NUMBER`'s own
///    pattern, so it cannot suffer the cons-chain's digit-merging
///    ambiguity). Measured: parses safely up to **102 levels**, crashes at
///    **103**.
///
/// # The binding constraint is a rule-frame floor, not a nesting-count one
///
/// Naively, the *nesting-count* floors above suggest parenthesised nesting
/// binds (crashing at the lowest count, 19) — matching `derive-parser`'s
/// own finding, the opposite of `j-parser`'s "genuine surprise". But
/// `MAX_RULE_DEPTH` caps `self.depth`, a raw count of *named-rule*
/// invocations on the call stack — nesting-count doesn't directly say how
/// many `self.depth` units that costs, because a `Sequence`/`Optional`/
/// `Alternation` match element recurses through `match_element` regardless
/// of whether it crosses into a new *named* rule, and those crossings are
/// invisible to `self.depth` yet still consume real native stack. So each
/// nesting-count floor was independently re-measured in **rule-frame
/// terms**: binary search over candidate `with_max_depth` values against a
/// fixed 5000-level/link input of that shape (deep enough that the *cap
/// itself*, not the input's own finite length, is always what triggers
/// first) — the same conversion `derive-parser`'s own doc comment performs
/// for its one measured shape, done here for all five:
///
/// | Shape | Nesting-count floor | Rule-frame floor |
/// |---|---|---|
/// | Parens | 19 safe / 20 crash | 289 safe / 290 crash |
/// | `:=` chain | 102 safe / 103 crash | 220 safe / 221 crash |
/// | `if`/`else` chain | 102 safe / 103 crash | 220 safe / 221 crash |
/// | cons (`.`) chain | 163 safe / 164 crash | **179 safe / 180 crash** |
/// | power (`^`) chain | 102 safe / 103 crash | 220 safe / 221 crash |
///
/// The cons chain — which *tolerates the most nesting levels* of the five
/// (163) — has the *lowest* rule-frame floor (179): each `cons` link's
/// *persistent* per-level cost is just one `cons` rule-frame (its leaf
/// operand, `additive`, fully parses and pops before the `[ DOT cons ]`
/// continuation is ever attempted, so that sub-chain's own several rule-
/// frames never accumulate across levels) — but whatever native-stack
/// bytes `cons`'s own call chain uses per crossing evidently costs more
/// than parens'/assign's/if-else's/power's own per-frame cost, so it
/// reaches the native ceiling at a *lower total frame count* despite
/// needing *more* levels to get there. This generalises MA06 §6's warning
/// one step further: not just "does a shape's floor transfer from a
/// sibling crate", but "does a shape's *nesting-count* floor even predict
/// which shape *actually* binds the frame-count cap this crate's own guard
/// enforces" — it does not, here, and assuming parenthesised nesting (the
/// nesting-count-lowest, and every sibling `*-parser`'s own historically-
/// dominant shape) was the binding constraint without this second,
/// rule-frame-terms measurement would have shipped a cap unsafe for cons
/// chains specifically.
///
/// `MAX_RULE_DEPTH` is set to **128** — about 28.5% below the binding
/// cons-chain rule-frame floor of 179 (comparable margin to `apl-parser`'s
/// own ~26.5%, `j-parser`'s ~30%, `derive-parser`'s ~33%), and therefore
/// safely below all four other rule-frame floors (220, 220, 220, 289) as
/// well. `128` also happens to equal
/// [`DEFAULT_MAX_RULE_DEPTH`](parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH)
/// — a coincidence of this grammar's own measured floors, not a decision
/// to reuse the shared default; it was derived independently from the
/// table above and simply landed there.
///
/// Measured real-input headroom at `128` (using the CAPPED parser, i.e.
/// `create_reduce_parser`/`try_parse_reduce`, so no crash risk at all):
/// parenthesised nesting parses cleanly up to 8 levels (9 trips the cap);
/// a `:=` chain, an `if`/`else` chain, and a power chain each parse cleanly
/// up to 56 levels (57 trips); a cons chain parses cleanly up to 112
/// levels (113 trips) — all comfortably beyond anything a hand-written
/// Reduce expression needs, and all five independently confirmed not to
/// crash a default-stack thread even thousands of levels/links past the
/// cap (see this crate's tests).
const MAX_RULE_DEPTH: usize = 128;

/// Create a [`GrammarParser`] wired to the Reduce grammar and the tokens of
/// `source`, with the recursion-depth guard ([`MAX_RULE_DEPTH`]) enabled so
/// pathologically deep nesting fails cleanly instead of overflowing the
/// native stack.
pub fn create_reduce_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_reduce(source);
    GrammarParser::new(tokens, _grammar::parser_grammar()).with_max_depth(MAX_RULE_DEPTH)
}

/// Parse Reduce source text into a [`GrammarASTNode`] rooted at `program`.
///
/// # Panics
///
/// Panics on a lexical or syntax error. Use [`try_parse_reduce`] to handle
/// errors.
///
/// # Example
///
/// ```
/// use coding_adventures_reduce_parser::parse_reduce;
/// let ast = parse_reduce("x := 5;\n");
/// assert_eq!(ast.rule_name, "program");
/// ```
pub fn parse_reduce(source: &str) -> GrammarASTNode {
    create_reduce_parser(source)
        .parse()
        .unwrap_or_else(|e| panic!("Reduce parse failed: {e}"))
}

/// Parse Reduce source text, returning a `Result` instead of panicking.
pub fn try_parse_reduce(source: &str) -> Result<GrammarASTNode, String> {
    let tokens = try_tokenize_reduce(source)?;
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

    fn parses(src: &str) -> bool {
        try_parse_reduce(src).is_ok()
    }

    #[test]
    fn program_is_the_root() {
        assert_eq!(parse_reduce("1;\n").rule_name, "program");
    }

    #[test]
    fn bare_trailing_statement_with_no_terminator_parses() {
        assert!(parses("1"));
    }

    #[test]
    fn semi_and_dollar_are_interchangeable_terminators() {
        assert!(parses("x := 1; y := 2$"));
    }

    #[test]
    fn function_call_uses_ordinary_parens() {
        let ast = parse_reduce("df(x, z);\n");
        assert!(contains_rule(&ast, "postfix"));
        assert!(contains_rule(&ast, "arglist"));
    }

    #[test]
    fn array_subscript_read_shares_the_call_production() {
        // MA08 §3: `a(5)`/`b(i, q)` parse the same as an ordinary call.
        assert!(parses("a(5);\n"));
        assert!(parses("b(i, q);\n"));
    }

    #[test]
    fn assignment_and_procedure_definition_share_one_production() {
        assert!(parses("x := 5;\n"));
        let ast = parse_reduce("h(l, m) := x - 2*y;\n");
        assert!(contains_rule(&ast, "assignment"));
    }

    #[test]
    fn assignment_right_associates() {
        // a := b := c evaluates as a := (b := c) (manual §2.7).
        assert!(parses("a := b := 5;\n"));
    }

    #[test]
    fn eq_is_equation_distinct_from_assign() {
        assert!(parses("x = 4;\n"));
        assert!(contains_rule(&parse_reduce("x = 4;\n"), "comparison"));
    }

    #[test]
    fn every_comparison_operator_parses() {
        for op in ["=", "neq", "<", ">", "<=", ">="] {
            let src = format!("a {op} b;\n");
            assert!(parses(&src), "`{src}` should parse");
        }
    }

    #[test]
    fn list_literal_uses_curly_braces() {
        let ast = parse_reduce("{a, b, c};\n");
        assert!(contains_rule(&ast, "list_literal"));
    }

    #[test]
    fn list_function_call_spelling_parses_like_any_other_call() {
        assert!(parses("list(a, b, c);\n"));
    }

    #[test]
    fn cons_operator_parses() {
        let ast = parse_reduce("a . {b, c};\n");
        assert!(contains_rule(&ast, "cons"));
    }

    #[test]
    fn cons_binds_looser_than_additive() {
        // `1+2 . {3,4}` must parse as `(1+2) . {3,4}` -- `cons`'s own
        // production wraps `additive`, so the additive sub-expression is
        // fully reduced before cons ever sees it. Structurally: the tree
        // must contain both `additive` and `cons`.
        let ast = parse_reduce("1+2 . {3,4};\n");
        assert!(contains_rule(&ast, "additive"));
        assert!(contains_rule(&ast, "cons"));
    }

    #[test]
    fn list_accessor_and_constructor_calls_parse() {
        for src in [
            "first(l);\n",
            "second(l);\n",
            "third(l);\n",
            "rest(l);\n",
            "part(l, n);\n",
            "append(l1, l2);\n",
            "reverse(l);\n",
        ] {
            assert!(parses(src), "`{src}` should parse");
        }
    }

    #[test]
    fn arithmetic_precedence() {
        let ast = parse_reduce("2 + 3 * 4 ^ 2;\n");
        assert!(contains_rule(&ast, "additive"));
        assert!(contains_rule(&ast, "multiplicative"));
        assert!(contains_rule(&ast, "power"));
    }

    #[test]
    fn caret_and_double_star_are_the_same_operator() {
        assert!(parses("a ^ b;\n"));
        assert!(parses("a ** b;\n"));
    }

    #[test]
    fn unary_minus_binds_looser_than_power() {
        let ast = parse_reduce("-x^2;\n");
        assert!(contains_rule(&ast, "unary"));
        assert!(contains_rule(&ast, "power"));
    }

    #[test]
    fn power_is_right_associative_by_shape() {
        assert!(parses("4^3^2;\n"));
    }

    #[test]
    fn grouping_parens_parse() {
        assert!(parses("(1 + 2) * 3;\n"));
        assert!(contains_rule(&parse_reduce("(1 + 2) * 3;\n"), "group"));
    }

    #[test]
    fn boolean_keywords_parse() {
        assert!(parses("a and b or not c;\n"));
        assert!(contains_rule(&parse_reduce("a and b;\n"), "logical_and"));
        assert!(contains_rule(&parse_reduce("a or b;\n"), "logical_or"));
        assert!(contains_rule(&parse_reduce("not a;\n"), "logical_not"));
    }

    #[test]
    fn uppercase_keyword_spellings_lex_as_plain_names_not_keywords() {
        // reduce.tokens' keywords are lowercase-only (the mirror image of
        // derive.tokens' uppercase AND/OR/NOT) -- `AND` here is just a NAME,
        // so this is a bare `a AND b` juxtaposition with no operator
        // between two names, which is not valid syntax in this subset
        // (implicit multiplication by juxtaposition is out of scope, MA08
        // §4) and must fail to parse as a single statement.
        assert!(try_parse_reduce("a AND b;\n").is_err());
    }

    #[test]
    fn bare_juxtaposed_names_with_no_operator_is_rejected() {
        // `a AND b` (with `AND` lexing as an ordinary NAME, not a keyword —
        // see the test above) has three bare names with no operator
        // between them. Implicit multiplication by juxtaposition is out of
        // scope (MA08 §4), so this must be a syntax error -- NOT silently
        // accepted as three separate terminator-less statements. This is a
        // regression test for exactly the bug `program`'s own grammar
        // comment documents: an earlier draft folded the "no terminator"
        // case into `statement_line` itself (tried per repetition
        // iteration, not just once at the very end), which let `a`, `AND`,
        // and `b;` each parse as their own bare statement and silently
        // consumed the entire input as three separate program items.
        assert!(try_parse_reduce("a AND b;\n").is_err());
        assert!(try_parse_reduce("a b\n").is_err());
    }

    #[test]
    fn if_then_parses_without_an_else() {
        let ast = parse_reduce("if a then b;\n");
        assert!(contains_rule(&ast, "if_expr"));
    }

    #[test]
    fn if_then_else_parses() {
        let ast = parse_reduce("if a=b then c else d;\n");
        assert!(contains_rule(&ast, "if_expr"));
    }

    #[test]
    fn if_is_usable_as_an_expression() {
        // MA08 §3: "usable as an expression, returning whichever branch
        // ran" -- nested directly as an assignment's right-hand side.
        let ast = parse_reduce("x := if a>0 then 1 else -1;\n");
        assert!(contains_rule(&ast, "if_expr"));
        assert!(contains_rule(&ast, "assignment"));
    }

    #[test]
    fn dangling_else_attaches_to_the_nearest_if() {
        // `if a then if b then c else d` must parse as
        // `if a then (if b then c else d)`, not `(if a then if b then c)
        // else d` -- there is only one `if_expr` node nested inside the
        // outer `then`-branch's own recursive `expr`, matching ordinary
        // recursive-descent dangling-else resolution.
        assert!(parses("if a then if b then c else d;\n"));
    }

    #[test]
    fn group_statement_parses() {
        let ast = parse_reduce("<< a := 1; a + 1 >>;\n");
        assert!(contains_rule(&ast, "group_expr"));
    }

    #[test]
    fn group_statement_is_usable_as_an_expression() {
        // MA08 §3: "evaluates to its last statement's value".
        let ast = parse_reduce("x := << a := 1; a + 1 >>;\n");
        assert!(contains_rule(&ast, "group_expr"));
        assert!(contains_rule(&ast, "assignment"));
    }

    #[test]
    fn group_statement_with_a_single_statement_parses() {
        assert!(parses("<< a + 1 >>;\n"));
    }

    #[test]
    fn nested_function_calls_parse() {
        assert!(parses("log(y/m);\n"));
    }

    #[test]
    fn multi_arg_calls_parse() {
        assert!(parses("df(x, z, a, b);\n"));
    }

    #[test]
    fn a_multi_statement_program_parses() {
        let ast = parse_reduce("x := 1; y := 2; x + y;\n");
        let stmt_lines: usize = ast
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "statement_line"))
            .count();
        assert_eq!(stmt_lines, 3);
    }

    #[test]
    fn syntax_error_is_reported() {
        assert!(try_parse_reduce("1 +\n").is_err());
        assert!(try_parse_reduce("(1 + 2\n").is_err());
    }

    // -------------------------------------------------------------------
    // Recursion-depth guard (DoS hardening) -- exercises all five
    // independently-measured shapes documented on `MAX_RULE_DEPTH`.
    // -------------------------------------------------------------------

    fn nested_paren_source(n: usize) -> String {
        format!("{}5{};\n", "(".repeat(n), ")".repeat(n))
    }

    fn assign_chain_source(n: usize) -> String {
        let mut s = String::new();
        for _ in 0..n {
            s.push_str("a:=");
        }
        s.push_str("5;\n");
        s
    }

    fn if_else_chain_source(n: usize) -> String {
        let mut s = String::new();
        for _ in 0..n {
            s.push_str("if 1 then 1 else ");
        }
        s.push_str("5;\n");
        s
    }

    fn cons_chain_source(n: usize) -> String {
        // NAME atoms, not NUMBER literals -- see MAX_RULE_DEPTH's doc
        // comment ("A flat cons (`.`) chain") for why `1.1.1...` silently
        // halves the intended chain length via NUMBER's own greedy
        // `.digit`-absorbing regex.
        let mut s = String::from("a");
        for _ in 0..n {
            s.push_str(".a");
        }
        s.push_str(";\n");
        s
    }

    fn power_chain_source(n: usize) -> String {
        let mut s = String::from("1");
        for _ in 0..n {
            s.push_str("^1");
        }
        s.push_str(";\n");
        s
    }

    /// Deeply-nested input, for every measured shape, must produce a
    /// recoverable error, not overflow the native stack. Parses 5000
    /// levels/links -- far past `MAX_RULE_DEPTH` -- on a worker thread with
    /// a generous 32 MiB stack, so the *guard* is what stops the
    /// recursion, not the stack running out.
    #[test]
    fn test_deeply_nested_input_returns_error_not_overflow_for_every_shape() {
        let sources = vec![
            nested_paren_source(5000),
            assign_chain_source(5000),
            if_else_chain_source(5000),
            cons_chain_source(5000),
            power_chain_source(5000),
        ];
        let handle = std::thread::Builder::new()
            .name("reduce-parser-depth-guard-regression".to_string())
            .stack_size(32 * 1024 * 1024)
            .spawn(move || {
                for src in sources {
                    let result = try_parse_reduce(&src);
                    assert!(
                        result.is_err(),
                        "deeply-nested input must fail with an error, not parse or crash"
                    );
                }
            })
            .expect("failed to spawn worker thread");
        handle
            .join()
            .expect("depth guard must keep the worker thread from crashing");
    }

    /// A caller relying on `MAX_RULE_DEPTH` must have the guard trip
    /// *before* the native stack overflows on a default-stack thread --
    /// otherwise a production caller (or `cargo test`'s own per-test
    /// thread) would still crash. Parses far-too-deep input, for every
    /// shape, on a worker thread with **no** `stack_size` override (the
    /// same ~2 MiB a default thread gets).
    #[test]
    fn test_cap_trips_before_overflow_on_default_stack_for_every_shape() {
        let sources = vec![
            nested_paren_source(5000),
            assign_chain_source(5000),
            if_else_chain_source(5000),
            cons_chain_source(5000),
            power_chain_source(5000),
        ];
        let handle = std::thread::spawn(move || {
            for src in sources {
                let result = try_parse_reduce(&src);
                assert!(result.is_err(), "deeply-nested input must error, not crash");
            }
        });
        handle
            .join()
            .expect("MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack");
    }

    /// Reasonable, hand-writable nesting for every shape stays well under
    /// the cap.
    #[test]
    fn test_reasonable_nesting_stays_under_the_cap_for_every_shape() {
        assert!(try_parse_reduce(&nested_paren_source(3)).is_ok());
        assert!(try_parse_reduce(&assign_chain_source(10)).is_ok());
        assert!(try_parse_reduce(&if_else_chain_source(10)).is_ok());
        assert!(try_parse_reduce(&cons_chain_source(10)).is_ok());
        assert!(try_parse_reduce(&power_chain_source(10)).is_ok());
    }
}
