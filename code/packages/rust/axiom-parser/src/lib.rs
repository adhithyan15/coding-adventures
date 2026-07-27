//! # Axiom Parser — building a syntax tree for Axiom (the MA-13-scoped
//! consumer-view subset).
//!
//! Turns the token stream from [`coding_adventures_axiom_lexer`] into a
//! parse tree using the generic
//! [`GrammarParser`](parser::grammar_parser::GrammarParser), driven by the
//! embedded `axiom.grammar` (`src/_grammar.rs`). It hand-writes no parsing
//! logic — a sibling of `derive-parser`/`reduce-parser`/`idl-parser`. See
//! `code/specs/MA13-axiom-language.md` (MA-13a, the design-only kickoff
//! this crate implements MA-13c of).
//!
//! ## Pipeline
//!
//! ```text
//! Axiom source
//!    |
//!    v
//! coding_adventures_axiom_lexer::tokenize_axiom  ->  Vec<Token>   (MA-13b)
//!    |
//!    v
//! parser::GrammarParser  (driven by the embedded axiom.grammar)   (MA-13c, this crate)
//!    |
//!    v
//! GrammarASTNode  <- a future axiom-runtime (MA-13d) lowers this to
//!                    symbolic_ir::IRNode + its own new AxiomValue/AxiomDomain
//!                    layer (MA13 §2)
//! ```
//!
//! ## What the tree captures
//!
//! Every Axiom expression in this cut parses down to ordinary infix/postfix
//! operators over `head(args)`-shaped calls (closer in shape to this repo's
//! Reduce/Derive/Maple CAS-family grammars than to any array-family grammar,
//! MA13 §5), PLUS the three genuinely new productions MA13 §3 introduces —
//! declaration (`:`, rule `declaration`), coercion (`::`, rule `coercion`),
//! and category-membership query (`has`, rule `has_query`) — none of which
//! any prior symbolic-family grammar in this repo has ever needed. See
//! `code/grammars/axiom/axiom.grammar`'s own header comment for the full
//! precedence cascade and every grammar-design decision this crate makes
//! (most notably: `program` parses exactly ONE top-level expression, not a
//! repeated multi-statement worksheet, since `axiom.tokens` gives top-level
//! inputs no separator at all — see that file's own "WHY `program` IS A
//! SINGLE EXPRESSION" section; and the paren-optional single-argument call
//! form `f a`'s disambiguation from a bare name followed by a binary
//! operator, e.g. `f -1`, — see `postfix`'s own comment there).
//!
//! ## `:=` vs `==` — two operators where Derive/Reduce need only one
//!
//! Unlike `derive-parser`/`reduce-parser`, whose single `:=` production is
//! shared, unmodified, between plain variable assignment and function
//! definition (disambiguated later, by a runtime inspecting the parsed
//! left-hand side's shape), this crate's `assignment` (`:=`) and `define`
//! (`==`) are two ENTIRELY SEPARATE grammar productions with two entirely
//! separate confirmed left-hand-side shapes (MA13 §4): `assignment`'s
//! left-hand side is always a bare `NAME`; `define`'s is always one of two
//! fixed call/declaration shapes (`declared_define`/`undeclared_define`).
//! This is a real, structural difference from Derive/Reduce's own design,
//! not an inconsistency — see `axiom.grammar`'s own `define` rule comment
//! for exactly why the "reuse whatever the general call production already
//! parses" trick that works for Derive/Reduce does NOT transfer to Axiom's
//! own declared-function-definition form (its typed parameter list has no
//! analogue in an ordinary call's comma-list-of-expressions `arglist`).

use coding_adventures_axiom_lexer::{tokenize_axiom, try_tokenize_axiom};
use parser::grammar_parser::{GrammarASTNode, GrammarParser};
mod _grammar;

/// Recursion-depth cap for the Axiom [`GrammarParser`] — see
/// [`GrammarParser::with_max_depth`] and
/// [`parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH`] for why the underlying
/// guard exists at all (deep recursion through `parse_rule` can overflow the
/// *native* thread stack — an uncatchable process abort — before this
/// crate's own `Result`-returning entry points ever get a chance to report
/// anything). `axiom-lexer` (MA-13b) has no recursion at all (it only
/// tokenizes, see that crate's own doc comment); this is the FIRST Axiom
/// crate with actual recursive descent, so this cap is added fresh here, not
/// inherited from a sibling.
///
/// # Four recursion shapes, measured independently
///
/// Measured with the same methodology every sibling `*-parser` crate in this
/// repo uses (per this repo's own `lessons.md` — "measure, don't assume one
/// shape's floor bounds the others"): binary search, an *uncapped*
/// `GrammarParser` (`max_depth = usize::MAX`), a `std::thread::spawn` worker
/// with a fixed **2 MiB stack** (the same order of magnitude as a default
/// thread's stack), one fresh **subprocess per data point** (a real
/// native-stack overflow calls `process::abort()`, which kills the whole
/// process — an in-process loop cannot survive past the first crash to
/// report a clean number), in a **debug** build (`cargo test`'s own
/// default, since debug frames are meaningfully larger than release
/// frames). The probe harness used was a throwaway `examples/depth_probe.rs`
/// (removed before this crate shipped, per this repo's own convention —
/// `derive-parser`/`reduce-parser`/`idl-parser` do not keep a committed
/// measurement tool either) driving `__uncapped_for_depth_probe` (also
/// removed) through a small shell binary-search script; see this crate's
/// `CHANGELOG.md` for the exact recorded methodology and results.
///
/// `axiom.grammar` has four structurally distinct self-referential shapes:
///
/// 1. **Parenthesised grouping/block nesting**, `((((…5…))))` — `group ->
///    expr -> (if_expr/define/assignment/declaration/has_query all fail
///    fast on the leading LPAREN) -> comparison -> coercion -> additive ->
///    multiplicative -> unary -> power -> postfix -> atom -> group -> …` —
///    cycles through the entire cascade every nesting level, the same
///    dominant shape every prior CAS-family sibling here (`derive-parser`,
///    `reduce-parser`) measured as their own binding constraint.
/// 2. **Nested function-call arguments**, `f(f(f(…5…)))` — `postfix ->
///    call_args -> arglist -> expr -> … -> postfix`, interposing two
///    wrapper rule-frames (`call_args`, `arglist`) per level on top of
///    `postfix`'s own re-entry.
/// 3. **A unary prefix chain**, `- - - … 5` (a SPACE between every `-` is
///    load-bearing — two adjacent `-` characters lex as the `--`
///    line-comment opener, silently swallowing the rest of the line; this
///    was caught directly when an unspaced first draft of the probe made
///    every depth-guard test pass for the wrong reason — an empty token
///    stream, not the cap, see `tests`'s own `unary_prefix_chain_source`
///    comment) — `unary`'s own `MINUS unary | power` self-reference.
/// 4. **A power chain**, `1^1^1^ … ^1` — `power`'s own
///    `postfix [ (CARET|POW) unary ]` continuation (through `unary`, which
///    falls back to `power` absent a leading `-`).
///
/// Every "flat chain of one operator" production written with EBNF `{ x }`
/// repetition (`additive`, `multiplicative`, `typed_param_list`,
/// `name_list`, `arglist`, `elem_list`, `type_expr_list`) costs *zero*
/// native stack regardless of width — this is an already-established fact
/// about [`parser::grammar_parser`]'s own `Repetition` arm (a plain
/// `loop { ... }` where each iteration's `match_element` call returns
/// before the next iteration begins), confirmed directly by reading that
/// module and re-affirmed by every sibling `*-parser` crate's own
/// `MAX_RULE_DEPTH` doc comment — not re-measured by a throwaway probe here.
///
/// Nesting-count floors (parses safely up to N, crashes at N+1), and the
/// rule-frame-terms conversion (binary search over `with_max_depth` against
/// a fixed 5000-level input of each shape, so the *cap itself* — not the
/// input's own finite length — is always what triggers first):
///
/// | Shape | Nesting-count floor | Rule-frame floor |
/// |---|---|---|
/// | Parenthesised grouping | 27 safe / 28 crash | 282 safe / 283 crash |
/// | Nested function calls | 24 safe / 25 crash | 211 safe / 212 crash |
/// | Unary prefix chain | 201 safe / 202 crash | 212 safe / 213 crash |
/// | Power chain | 100 safe / 101 crash | 213 safe / 214 crash |
///
/// The lowest rule-frame floor is nested function calls' **211** — NOT
/// parenthesised grouping (282), even though grouping is the dominant
/// binding constraint for `derive-parser`/`reduce-parser`. This mirrors the
/// same "don't assume which shape binds" surprise every sibling
/// `*-parser`'s own doc comment documents (`idl-parser`, `scilab-parser`,
/// `maple-parser`, `reduce-parser`'s own cons-chain): `call_args`'s own
/// `LPAREN [arglist] RPAREN` alternative interposes an extra `arglist`
/// rule-frame beyond what plain `group`'s `LPAREN expr RPAREN` needs, and
/// that extra frame's native-stack cost evidently outweighs grouping's own
/// higher per-level *count* (~10.4 frames/level for grouping vs. ~8.8 for
/// calls) enough to make nested calls the binding shape here.
///
/// `MAX_RULE_DEPTH` is set to **140** — about 33.6% below 211 (comparable
/// margin to `derive-parser`'s own ~33%, `maple-parser`'s ~31.2%,
/// `j-parser`'s ~30%, `idl-parser`'s ~30.2%), and therefore safely below the
/// other three rule-frame floors (282, 212, 213) too.
///
/// Measured real-input headroom at `140` (using the CAPPED parser, i.e.
/// [`create_axiom_parser`]/[`try_parse_axiom`], so no crash risk at all):
/// parenthesised grouping and nested function calls each parse cleanly up
/// to 13 levels (14 trips the cap); a unary prefix chain parses cleanly up
/// to 130 levels (131 trips); a power chain parses cleanly up to 65 levels
/// (66 trips) — all comfortably beyond anything a hand-written Axiom
/// expression needs, and all four independently confirmed not to crash a
/// default-stack thread even thousands of levels past the cap (see this
/// crate's tests).
const MAX_RULE_DEPTH: usize = 140;

/// Create a [`GrammarParser`] wired to the Axiom grammar and the tokens of
/// `source`, with the recursion-depth guard ([`MAX_RULE_DEPTH`]) enabled so
/// pathologically deep nesting fails cleanly instead of overflowing the
/// native stack.
pub fn create_axiom_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_axiom(source);
    GrammarParser::new(tokens, _grammar::parser_grammar()).with_max_depth(MAX_RULE_DEPTH)
}

/// Parse Axiom source text into a [`GrammarASTNode`] rooted at `program`.
///
/// # Panics
///
/// Panics on a lexical or syntax error. Use [`try_parse_axiom`] to handle
/// errors.
///
/// # Example
///
/// ```
/// use coding_adventures_axiom_parser::parse_axiom;
/// let ast = parse_axiom("factorial 7");
/// assert_eq!(ast.rule_name, "program");
/// ```
pub fn parse_axiom(source: &str) -> GrammarASTNode {
    create_axiom_parser(source)
        .parse()
        .unwrap_or_else(|e| panic!("Axiom parse failed: {e}"))
}

/// Parse Axiom source text, returning a `Result` instead of panicking.
pub fn try_parse_axiom(source: &str) -> Result<GrammarASTNode, String> {
    let tokens = try_tokenize_axiom(source)?;
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

    /// Find the first descendant node (or self) with the given rule name.
    fn find_rule<'a>(node: &'a GrammarASTNode, name: &str) -> Option<&'a GrammarASTNode> {
        if node.rule_name == name {
            return Some(node);
        }
        for c in &node.children {
            if let ASTNodeOrToken::Node(n) = c {
                if let Some(found) = find_rule(n, name) {
                    return Some(found);
                }
            }
        }
        None
    }

    fn parses(src: &str) -> bool {
        try_parse_axiom(src).is_ok()
    }

    // --- program is a single top-level expression -------------------------

    #[test]
    fn program_is_the_root() {
        assert_eq!(parse_axiom("1").rule_name, "program");
    }

    #[test]
    fn a_bare_juxtaposed_pair_with_nothing_after_is_a_call_not_two_statements() {
        // No top-level separator exists in this cut (see axiom.grammar's own
        // header comment) -- `program = expr`, so trailing garbage after one
        // full expression is a syntax error, not a silently-accepted second
        // statement.
        assert!(try_parse_axiom("a := 1 b := 2").is_err());
    }

    // --- Literals -----------------------------------------------------------

    #[test]
    fn integer_and_float_literals_parse() {
        assert!(parses("42"));
        assert!(parses("1.5"));
    }

    #[test]
    fn string_literal_parses() {
        assert!(parses("\"hello\""));
    }

    #[test]
    fn symbol_parses() {
        assert!(parses("x"));
    }

    // --- Function calls: f(a, b) and paren-optional single-argument f a ---

    #[test]
    fn explicit_paren_call_parses() {
        let ast = parse_axiom("f(a, b)");
        assert!(contains_rule(&ast, "postfix"));
        assert!(contains_rule(&ast, "arglist"));
    }

    #[test]
    fn paren_optional_single_argument_call_parses() {
        // MA13 §4's own confirmed examples.
        let ast = parse_axiom("factorial 7");
        assert!(contains_rule(&ast, "postfix"));
        assert!(contains_rule(&ast, "call_args"));
        assert!(parses("ff z"));
    }

    #[test]
    fn paren_optional_call_with_a_string_or_list_argument_parses() {
        assert!(parses("f \"hello\""));
        assert!(parses("f [1, 2, 3]"));
    }

    #[test]
    fn call_with_no_arguments_parses() {
        assert!(parses("f()"));
    }

    #[test]
    fn nested_function_calls_parse() {
        assert!(parses("f(g(x))"));
    }

    // --- The paren-optional call form does not swallow a following binary
    // operator: `f -1` / `f +1` must read as subtraction/addition, never as
    // a call with a negative/positive-literal argument (axiom.grammar's own
    // "THE PAREN-OPTIONAL DISAMBIGUATION" comment).

    #[test]
    fn bare_name_followed_by_minus_number_is_subtraction_not_a_call() {
        let ast = parse_axiom("f - 1");
        assert!(contains_rule(&ast, "additive"));
        // No call_args node should have consumed the MINUS as part of a
        // negative-literal call argument.
        assert!(!contains_rule(&ast, "call_args"));
    }

    #[test]
    fn bare_name_followed_by_plus_number_is_addition_not_a_call() {
        let ast = parse_axiom("f + 1");
        assert!(contains_rule(&ast, "additive"));
        assert!(!contains_rule(&ast, "call_args"));
    }

    // --- Lists ----------------------------------------------------------------

    #[test]
    fn list_literal_parses() {
        let ast = parse_axiom("[a, b, c]");
        assert!(contains_rule(&ast, "list_literal"));
    }

    #[test]
    fn empty_list_literal_parses() {
        assert!(parses("[]"));
    }

    // --- Arithmetic precedence and associativity ---------------------------

    #[test]
    fn arithmetic_precedence() {
        let ast = parse_axiom("2 + 3 * 4 ^ 2");
        assert!(contains_rule(&ast, "additive"));
        assert!(contains_rule(&ast, "multiplicative"));
        assert!(contains_rule(&ast, "power"));
    }

    #[test]
    fn caret_and_double_star_are_the_same_operator() {
        assert!(parses("a ^ b"));
        assert!(parses("a ** b"));
    }

    #[test]
    fn unary_minus_binds_looser_than_power() {
        let ast = parse_axiom("-x^2");
        assert!(contains_rule(&ast, "unary"));
        assert!(contains_rule(&ast, "power"));
    }

    #[test]
    fn power_is_right_associative_by_shape() {
        assert!(parses("4^3^2"));
    }

    #[test]
    fn multiplicative_and_additive_are_left_associative_by_shape() {
        assert!(parses("1 - 2 - 3"));
        assert!(parses("1 / 2 / 3"));
    }

    #[test]
    fn grouping_parens_parse() {
        let ast = parse_axiom("(1 + 2) * 3");
        assert!(contains_rule(&ast, "group"));
    }

    // --- Equality / comparison ------------------------------------------------

    #[test]
    fn every_comparison_operator_parses() {
        for op in ["=", "~=", "<", "<=", ">", ">="] {
            let src = format!("a {op} b");
            assert!(parses(&src), "`{src}` should parse");
        }
    }

    #[test]
    fn comparison_binds_looser_than_additive() {
        let ast = parse_axiom("a + 1 = b - 1");
        assert!(contains_rule(&ast, "comparison"));
        assert!(contains_rule(&ast, "additive"));
    }

    // --- `:=` immediate assignment -------------------------------------------

    #[test]
    fn immediate_assignment_parses() {
        let ast = parse_axiom("x := 5");
        assert!(contains_rule(&ast, "assignment"));
    }

    #[test]
    fn assignment_right_associates_by_shape() {
        assert!(parses("a := b := 5"));
    }

    // --- `==` function definition: declared and undeclared forms ------------

    #[test]
    fn declared_function_definition_parses() {
        let ast = parse_axiom("power(x: Integer, n: NonNegativeInteger): Integer == x ** n");
        assert!(contains_rule(&ast, "declared_define"));
        assert!(contains_rule(&ast, "typed_param_list"));
    }

    #[test]
    fn declared_function_definition_with_no_parameters_parses() {
        assert!(parses("pi(): Float == 3.14"));
    }

    #[test]
    fn undeclared_function_definition_parses() {
        let ast = parse_axiom("f x == x * x");
        assert!(contains_rule(&ast, "undeclared_define"));
    }

    #[test]
    fn undeclared_definition_with_no_parens_is_not_confused_with_a_plain_call() {
        // `f x` alone (no `==`) is an ordinary call; `f x == e` is a
        // definition -- both must parse, to two DIFFERENT rules.
        let call_ast = parse_axiom("f x");
        assert!(contains_rule(&call_ast, "postfix"));
        assert!(!contains_rule(&call_ast, "undeclared_define"));

        let def_ast = parse_axiom("f x == x");
        assert!(contains_rule(&def_ast, "undeclared_define"));
    }

    #[test]
    fn declared_define_requires_a_return_type_annotation() {
        // MA13 §4's own row always shows the return-type annotation; this
        // grammar takes the narrower, spec-literal reading and requires it.
        assert!(try_parse_axiom("f(x: Integer) == x").is_err());
    }

    #[test]
    fn declared_define_requires_every_parameter_to_be_typed() {
        assert!(try_parse_axiom("f(x): Integer == x").is_err());
    }

    // --- `if p then e1 else e2` — `else` is MANDATORY in this cut -----------

    #[test]
    fn if_then_else_parses() {
        let ast = parse_axiom("if a > 0 then 1 else -1");
        assert!(contains_rule(&ast, "if_expr"));
    }

    #[test]
    fn if_without_else_is_rejected() {
        // MA13 §4: "missing else -- deferred" for this cut.
        assert!(try_parse_axiom("if a > 0 then 1").is_err());
    }

    #[test]
    fn if_is_usable_as_an_expression() {
        let ast = parse_axiom("x := if a > 0 then 1 else -1");
        assert!(contains_rule(&ast, "if_expr"));
        assert!(contains_rule(&ast, "assignment"));
    }

    #[test]
    fn dangling_else_attaches_to_the_nearest_if() {
        assert!(parses("if a then if b then 1 else 2 else 3"));
    }

    // --- `( e1; e2; ...; eN )` parenthesised block ---------------------------

    #[test]
    fn parenthesised_block_parses_and_shares_the_group_rule_with_plain_grouping() {
        let block_ast = parse_axiom("(a := 1; a + 1)");
        let group = find_rule(&block_ast, "group").expect("group");
        // Two `expr` children, joined by one SEMI -- a block, not plain
        // grouping (axiom.grammar's own group-vs-block child-count design).
        let expr_children = group
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "expr"))
            .count();
        assert_eq!(expr_children, 2);

        let group_ast = parse_axiom("(1 + 2)");
        let group2 = find_rule(&group_ast, "group").expect("group");
        let expr_children2 = group2
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "expr"))
            .count();
        assert_eq!(expr_children2, 1);
    }

    #[test]
    fn block_with_three_statements_parses() {
        assert!(parses("(a := 1; b := 2; a + b)"));
    }

    // --- Declaration: `a : T`, `(a, b, c) : T` -------------------------------

    #[test]
    fn plain_declaration_parses() {
        let ast = parse_axiom("a : PositiveInteger");
        assert!(contains_rule(&ast, "declaration"));
    }

    #[test]
    fn tuple_declaration_parses() {
        let ast = parse_axiom("(a, b, c) : Integer");
        assert!(contains_rule(&ast, "declaration"));
        assert!(contains_rule(&ast, "name_list"));
    }

    #[test]
    fn declaration_type_can_be_a_parameterized_domain() {
        let ast = parse_axiom("a : Fraction(Integer)");
        assert!(contains_rule(&ast, "declaration"));
        assert!(contains_rule(&ast, "type_expr"));
    }

    // --- Coercion: `e :: T` ---------------------------------------------------

    #[test]
    fn coercion_parses() {
        let ast = parse_axiom("3 :: Fraction(Integer)");
        assert!(contains_rule(&ast, "coercion"));
    }

    #[test]
    fn coercion_type_accepts_the_paren_optional_shorthand() {
        // MA13's own confirmed example: `3 :: Fraction Integer`.
        let ast = parse_axiom("3 :: Fraction Integer");
        assert!(contains_rule(&ast, "coercion"));
        let type_expr = find_rule(&ast, "type_expr").expect("type_expr");
        assert!(contains_rule(type_expr, "type_ctor_args"));
    }

    #[test]
    fn coercion_left_hand_side_can_be_a_computed_expression() {
        let ast = parse_axiom("(1 + 2) :: Float");
        assert!(contains_rule(&ast, "coercion"));
        assert!(contains_rule(&ast, "group"));
    }

    #[test]
    fn coercion_binds_tighter_than_comparison() {
        // `x :: T = y` must read as `(x :: T) = y` -- both `comparison` and
        // `coercion` nodes must appear, with coercion nested UNDER comparison.
        let ast = parse_axiom("x :: Integer = y");
        let cmp = find_rule(&ast, "comparison").expect("comparison");
        assert!(contains_rule(cmp, "coercion"));
    }

    #[test]
    fn coercion_binds_looser_than_additive() {
        // `a + b :: T` must read as `(a + b) :: T`.
        let ast = parse_axiom("a + b :: Float");
        let coerce = find_rule(&ast, "coercion").expect("coercion");
        assert!(contains_rule(coerce, "additive"));
    }

    // --- `D has C` category-membership query ---------------------------------

    #[test]
    fn has_query_true_example_parses() {
        // MA13's own worked example.
        let ast = parse_axiom("Polynomial(Integer) has Ring");
        assert!(contains_rule(&ast, "has_query"));
    }

    #[test]
    fn has_query_false_example_parses() {
        let ast = parse_axiom("List(Integer) has Ring");
        assert!(contains_rule(&ast, "has_query"));
    }

    #[test]
    fn has_query_is_not_directly_reachable_as_a_bare_arithmetic_operand() {
        // `has` is deliberately its OWN top-level `expr` alternative, not
        // folded into the general arithmetic cascade (axiom.grammar's own
        // `has_query` comment) -- `additive`'s operand is `multiplicative`,
        // never the full `expr`, so a BARE (unparenthesised) has-query
        // cannot appear as an operand of `+`.
        assert!(try_parse_axiom("1 + Integer has Ring").is_err());
    }

    #[test]
    fn has_query_is_reachable_through_explicit_parens_like_every_other_expr_form() {
        // `group = LPAREN expr { SEMI expr } RPAREN` wraps the FULL `expr`
        // (deliberately -- the same "if/assignment/block are all usable as
        // an expression" design every `expr`-shaped form in this grammar
        // gets, see `expr`'s own header comment), so a has-query wrapped in
        // explicit parens IS reachable as an arithmetic operand, the same
        // way a parenthesised assignment or `if` would be.
        let ast = parse_axiom("1 + (Integer has Ring)");
        assert!(contains_rule(&ast, "has_query"));
        assert!(contains_rule(&ast, "additive"));
    }

    #[test]
    fn a_bare_type_expr_with_no_has_falls_through_to_an_ordinary_call() {
        let ast = parse_axiom("Polynomial(Integer)");
        assert!(contains_rule(&ast, "postfix"));
        assert!(!contains_rule(&ast, "has_query"));
    }

    // --- type_expr's own paren-optional restriction (bare NAME only) --------

    #[test]
    fn type_ctor_paren_optional_argument_must_be_a_bare_name() {
        // `List Fraction(Integer)` -- combining the paren-optional shorthand
        // with a further explicitly-parenthesised argument -- is NOT
        // accepted by this grammar (axiom.grammar's own `type_expr` comment);
        // write `List(Fraction(Integer))` instead.
        assert!(try_parse_axiom("a : List Fraction(Integer)").is_err());
        assert!(parses("a : List(Fraction(Integer))"));
    }

    #[test]
    fn deeply_nested_explicit_type_constructor_parses_up_to_the_depth_cap() {
        assert!(parses("a : List(Matrix(Polynomial(Integer)))"));
    }

    // --- `--` comments are invisible to the parser (handled by the lexer) ---

    #[test]
    fn comments_are_skipped() {
        assert!(parses("-- a comment\nx := 1 -- trailing"));
    }

    // --- Syntax errors are reported, not panics ------------------------------

    #[test]
    fn syntax_error_is_reported() {
        assert!(try_parse_axiom("1 +").is_err());
        assert!(try_parse_axiom("(1 + 2").is_err());
        assert!(try_parse_axiom("has Ring").is_err());
    }

    #[test]
    #[should_panic(expected = "Axiom parse failed")]
    fn parse_axiom_panics_on_malformed_source() {
        parse_axiom("1 +");
    }

    // -------------------------------------------------------------------
    // Recursion-depth guard (DoS hardening) -- exercises all four
    // independently-measured shapes documented on `MAX_RULE_DEPTH`.
    // -------------------------------------------------------------------

    fn nested_paren_source(n: usize) -> String {
        format!("{}5{}", "(".repeat(n), ")".repeat(n))
    }

    fn nested_call_source(n: usize) -> String {
        format!("{}5{}", "f(".repeat(n), ")".repeat(n))
    }

    fn unary_prefix_chain_source(n: usize) -> String {
        // A SPACE between every `-` is load-bearing, not cosmetic: two
        // adjacent MINUS characters with no space between them (`--`) is
        // axiom.tokens' own line-comment opener (a `skip:` pattern, tried
        // BEFORE ordinary token matching at every position -- see
        // axiom-lexer's own doc comment) and would swallow the ENTIRE rest
        // of the line as a comment, silently testing "zero tokens" instead
        // of a genuine deep unary-chain -- this was caught directly: an
        // earlier, unspaced version of this helper made every depth-guard
        // test below pass for entirely the wrong reason (an empty token
        // stream fails to parse regardless of `MAX_RULE_DEPTH`).
        let mut s = "-"
            .repeat(n)
            .chars()
            .map(|c| format!("{c} "))
            .collect::<String>();
        s.push('5');
        s
    }

    fn power_chain_source(n: usize) -> String {
        let mut s = String::from("1");
        for _ in 0..n {
            s.push_str("^1");
        }
        s
    }

    fn all_shape_sources(n: usize) -> Vec<String> {
        vec![
            nested_paren_source(n),
            nested_call_source(n),
            unary_prefix_chain_source(n),
            power_chain_source(n),
        ]
    }

    /// Deeply-nested input, for every measured shape, must produce a
    /// recoverable error, not overflow the native stack. Parses 5000
    /// levels -- far past `MAX_RULE_DEPTH` -- on a worker thread with a
    /// generous 32 MiB stack, so the *guard* is what stops the recursion,
    /// not the stack running out.
    #[test]
    fn test_deeply_nested_input_returns_error_not_overflow_for_every_shape() {
        let sources = all_shape_sources(5000);
        let handle = std::thread::Builder::new()
            .name("axiom-parser-depth-guard-regression".to_string())
            .stack_size(32 * 1024 * 1024)
            .spawn(move || {
                for src in sources {
                    let result = try_parse_axiom(&src);
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
        let sources = all_shape_sources(5000);
        let handle = std::thread::spawn(move || {
            for src in sources {
                let result = try_parse_axiom(&src);
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
        assert!(try_parse_axiom(&nested_paren_source(3)).is_ok());
        assert!(try_parse_axiom(&nested_call_source(3)).is_ok());
        assert!(try_parse_axiom(&unary_prefix_chain_source(5)).is_ok());
        assert!(try_parse_axiom(&power_chain_source(5)).is_ok());
    }

    /// Input that nests *exactly up to* `MAX_RULE_DEPTH`'s measured
    /// real-input headroom for every shape still parses cleanly, and one
    /// level deeper cleanly trips the cap. These exact boundary counts were
    /// found empirically by binary-searching `try_parse_axiom` (the CAPPED
    /// public API, `MAX_RULE_DEPTH = 140`) against increasing nesting counts
    /// for each shape -- see `MAX_RULE_DEPTH`'s own doc comment ("Measured
    /// real-input headroom at `140`").
    #[test]
    fn test_headroom_boundary_for_every_shape() {
        assert!(try_parse_axiom(&nested_paren_source(13)).is_ok());
        assert!(try_parse_axiom(&nested_paren_source(14)).is_err());

        assert!(try_parse_axiom(&nested_call_source(13)).is_ok());
        assert!(try_parse_axiom(&nested_call_source(14)).is_err());

        assert!(try_parse_axiom(&unary_prefix_chain_source(130)).is_ok());
        assert!(try_parse_axiom(&unary_prefix_chain_source(131)).is_err());

        assert!(try_parse_axiom(&power_chain_source(65)).is_ok());
        assert!(try_parse_axiom(&power_chain_source(66)).is_err());
    }
}
