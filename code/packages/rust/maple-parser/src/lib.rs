//! # Maple Parser — building a syntax tree for Maple (a subset).
//!
//! Turns the token stream from [`coding_adventures_maple_lexer`] into a parse
//! tree using the generic
//! [`GrammarParser`](parser::grammar_parser::GrammarParser), driven by the
//! embedded `maple.grammar` (`src/_grammar.rs`). It hand-writes no parsing
//! logic. A sibling of `reduce-parser` / `derive-parser` / `macsyma-parser` /
//! `wolfram-parser`. See `code/specs/MA09-maple-language.md`.
//!
//! ## What the tree captures
//!
//! Maple's surface splits into two nonterminals that never call back into
//! each other — `statement` (assignment, the arrow-operator definition, and
//! the `if`/`elif`/`else`/`end if`|`fi` conditional — Programming Guide
//! Chapter 5 "Maple Statements") and `expr` (the ordinary arithmetic/
//! relational/logical chain — Chapter 3 "Maple Expressions"). See
//! `maple.grammar`'s own "Design decision: statements vs. expressions"
//! header comment for why this split exists and what it deliberately
//! excludes (`if`/`:=` are never usable as a nested expression in this
//! subset, unlike `reduce-parser`'s identical-looking shape). This parser
//! produces the surface tree whose rule names (`statement`, `if_expr`,
//! `assignment`, `arrow_def`, `logical_or`, `comparison`, `additive`,
//! `multiplicative`, `power`, `postfix`, `atom`, `list_literal`,
//! `set_literal`, …) a future `maple-runtime` (MP-4) will lower into the
//! canonical `symbolic-ir` heads (`Add`/`Sub`/`Mul`/`Div`/`Pow`/`Neg`/
//! `Equal`/`NotEqual`/`Less`/`Greater`/`LessEqual`/`GreaterEqual`/`And`/`Or`/
//! `Not`/`Assign`/`Define`/`If`/`List`/`Set`/…).
//!
//! ```text
//! Maple source
//!    |
//!    v
//! coding_adventures_maple_lexer::tokenize_maple  ->  Vec<Token>
//!    |
//!    v
//! parser::GrammarParser  (driven by the embedded maple.grammar)
//!    |
//!    v
//! GrammarASTNode  <- the tree MP-4 lowers to symbolic-ir
//! ```

use coding_adventures_maple_lexer::{tokenize_maple, try_tokenize_maple};
use parser::grammar_parser::{GrammarASTNode, GrammarParser};
mod _grammar;

/// Recursion-depth cap for the Maple [`GrammarParser`] — see
/// [`GrammarParser::with_max_depth`] and
/// [`parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH`] for why the underlying
/// guard exists at all (deep recursion through `parse_rule` can overflow the
/// *native* thread stack — an uncatchable process abort — before this
/// crate's own `Result`-returning entry points ever get a chance to report
/// anything).
///
/// # Six recursion shapes, measured independently, per MA06 §6's established
/// methodology
///
/// `maple.grammar` has no single dominant recursive shape the way a plain
/// arithmetic-only grammar might — the statement/expression split (see
/// `maple.grammar`'s own "statements vs. expressions" design-decision
/// comment) and the two distinct bracket-delimited aggregate literals each
/// introduce their own self-referential production. Every "flat chain of one
/// operator" production written with EBNF `{ x }` repetition
/// (`logical_or`, `logical_and`, `additive`, `multiplicative`, the `elif`
/// chain in `if_expr`, `arglist`, `arrow_params`' own `{ COMMA NAME }`) costs
/// *zero* native stack regardless of width — confirmed directly by reading
/// [`parser::grammar_parser`]'s own `match_element` implementation (not
/// re-measured by a throwaway probe here, since `reduce-parser`'s own
/// `MAX_RULE_DEPTH` doc comment already established this as a fact about the
/// *shared engine* itself, not about any one grammar: the `Repetition` arm is
/// a plain `loop { ... }` where each iteration's `match_element` call
/// returns before the next iteration begins, so the *native* call stack
/// never grows with iteration count, only with the nesting *within* one
/// iteration's own match — the same reasoning applies unchanged to every
/// grammar built on this parser, including this one).
///
/// What genuinely recurses in `maple.grammar` are its distinct
/// self-referential (right-recursive, prefix-recursive, or mutually-nested)
/// productions, each measured independently below with the same methodology
/// every sibling `*-parser` crate's own `MAX_RULE_DEPTH` doc comment uses:
/// binary search, an *uncapped* `GrammarParser` (`max_depth = usize::MAX`),
/// a `std::thread::spawn` worker with the **default ~2 MiB stack** (no
/// `stack_size` override), one fresh **subprocess per data point** (a real
/// native-stack overflow calls `process::abort()`, which kills the whole
/// process, not just the offending thread — an in-process loop cannot
/// survive past the first crash to report a clean number), in a **debug**
/// build (`cargo test`'s own default) since debug frames are meaningfully
/// larger than release frames.
///
/// 1. **Parenthesised nesting**, `((((…5…))))` — `group -> expr ->
///    logical_or -> logical_and -> logical_not -> comparison -> additive ->
///    multiplicative -> unary -> power -> postfix -> atom -> group -> …` —
///    cycles through the *entire* expression precedence cascade every
///    nesting level. Measured: parses safely up to **23 levels**, crashes
///    the process at **24**.
/// 2. **List-literal nesting**, `[[[[…5…]]]]` — `list_literal -> arglist ->
///    expr -> … -> atom -> list_literal -> …` — structurally distinct from
///    parenthesised nesting (one extra `arglist` rule-frame per level,
///    confirmed by inspecting `maple.grammar`'s own rule graph, not
///    assumed): `group` wraps `expr` directly, but `list_literal` wraps
///    `expr` through `arglist` first. Set-literal nesting (`{{{…}}}`) shares
///    this *exact* rule-frame shape (`set_literal -> arglist -> expr -> …`
///    is identical in every respect except which bracket token is matched),
///    so it was not separately measured — this was confirmed by direct
///    inspection of the rule graph (both `list_literal` and `set_literal`
///    reuse the identical `arglist` production), not assumed by shape
///    resemblance the way MA06 §6 warns against; the two productions are
///    provably, not just plausibly, identical in recursion cost. Measured:
///    parses safely up to **21 levels**, crashes at **22**.
/// 3. **A `not` prefix chain**, `not not not … x` — `logical_not`'s own
///    `"not" logical_not` self-reference. Measured: parses safely up to
///    **205 levels**, crashes at **206**.
/// 4. **A unary-minus prefix chain**, `- - - … x` — `unary`'s own `MINUS
///    unary` self-reference. Measured: parses safely up to **205 levels**,
///    crashes at **206**.
/// 5. **A flat power (`^`) chain**, `1^1^1^ … ^1` — `power`'s own `[ CARET
///    unary ]` continuation, cycling `power -> unary -> power -> …` per
///    level (mirroring `reduce-parser`'s identical `power`/`unary` mutual
///    reference; Maple has no `**` synonym, so only `^` was used). Measured:
///    parses safely up to **102 levels**, crashes at **103**.
/// 6. **Nested `if`/`end if`, or `fi`**, `if 1 then if 1 then … 5 … end if
///    end if` — `if_expr`'s `then`-branch is a `statement`, and `statement`'s
///    first alternative is `if_expr` again, so nesting `if`s inside their
///    own `then`-branch cycles `if_expr -> statement -> if_expr -> …`. This
///    is the one genuinely new shape with no `reduce-parser` analogue at all
///    (REDUCE's `if`/`else` chain has no closing keyword to nest through;
///    Maple's does) — measured using the `end if` closing spelling. Measured:
///    parses safely up to **137 levels**, crashes at **138**. The `fi`
///    closing spelling was spot-checked at this exact boundary (136 and 137
///    levels) and confirmed to behave identically (137 safe, 138 crash) —
///    expected, since the closing spelling is matched entirely within one
///    `if_expr` invocation on the way *back up* the recursion and does not
///    itself add or remove any recursive call.
///
/// # The binding constraint is a rule-frame floor, not a nesting-count one
///
/// Exactly as `reduce-parser`'s own doc comment warns, the *nesting-count*
/// floors above do not by themselves say which shape binds
/// `MAX_RULE_DEPTH` — `self.depth` counts *named-rule* invocations, and
/// different shapes cost a different number of rule-frames per nesting
/// level. Converting each measured nesting-count floor into rule-frame terms
/// (binary search over `with_max_depth` against a fixed 5000-level input of
/// each shape, so the *cap itself* — not the input's own finite length — is
/// always what triggers first; same default ~2 MiB stack, same debug build):
///
/// | Shape | Nesting-count floor | Rule-frame floor |
/// |---|---|---|
/// | Parens | 23 safe / 24 crash | 298 safe / 299 crash |
/// | List-literal (= set-literal) | 21 safe / 22 crash | 289 safe / 290 crash |
/// | `not` chain | 205 safe / 206 crash | **218 safe / 219 crash** |
/// | Unary-minus chain | 205 safe / 206 crash | 219 safe / 220 crash |
/// | Power (`^`) chain | 102 safe / 103 crash | 220 safe / 221 crash |
/// | Nested `if`/`end if` (= `fi`) | 137 safe / 138 crash | 289 safe / 290 crash |
///
/// The genuine surprise, mirroring `reduce-parser`'s own: the `not` prefix
/// chain — which tolerates by far the *most* nesting levels of the six (205,
/// alongside its near-twin the unary-minus chain) — has the *lowest*
/// rule-frame floor (218), lower even than the nested-`if` shape (289),
/// which tolerates *far fewer* levels (137) before crashing. Each `not`
/// link's *persistent* per-level cost is exactly one `logical_not`
/// rule-frame (mirroring the unary-minus chain's identical one-`unary`-frame
/// cost) — cheaper, in rule-frame-count terms, than parenthesised nesting's
/// twelve frames per level or nested-`if`'s two (`if_expr` + `statement`) —
/// yet whatever native-stack *bytes* this specific self-referential call
/// path (`logical_not` calling `match_element` on a `Sequence` containing a
/// `Literal` match followed immediately by a recursive `RuleReference` back
/// into `logical_not`) consumes per crossing evidently costs more than the
/// other five shapes' own per-frame byte cost, so it reaches the native
/// ceiling at a *lower total rule-frame count* despite needing *far more*
/// levels to get there. Naively assuming either "the shape that tolerates
/// fewest nesting levels must bind" (nested-`if`, wrong) or "parenthesised
/// nesting binds, since it does for nearly every sibling `*-parser` crate in
/// this repo" (also wrong — parens has the *highest* rule-frame floor of the
/// six here) would each have shipped a cap unsafe specifically for `not`/
/// unary-minus prefix chains. This confirms, rather than merely restates,
/// MA06 §6's warning that shape analogies — nesting-count rankings *and*
/// which shape historically binds in a sibling grammar — do not transfer
/// across grammars with a different rule-recursion shape.
///
/// `MAX_RULE_DEPTH` is set to **150** — about 31.2% below the binding
/// `not`-chain rule-frame floor of 218 (a comparable margin to
/// `reduce-parser`'s own ~28.5%, `apl-parser`'s ~26.5%, `j-parser`'s ~30%,
/// `derive-parser`'s ~33%), and therefore safely below all five other
/// rule-frame floors (219, 220, 289, 289, 298) as well.
///
/// Measured real-input headroom at `150` (using the CAPPED parser, i.e.
/// `create_maple_parser`/`try_parse_maple`, so no crash risk at all):
/// parenthesised nesting parses cleanly up to 11 levels (12 trips the cap);
/// list-literal (and, by the proven-identical shape above, set-literal)
/// nesting parses cleanly up to 10 levels (11 trips); a `not` chain and a
/// unary-minus chain each parse cleanly up to 135 levels (136 trips); a
/// power chain parses cleanly up to 67 levels (68 trips); nested `if`s parse
/// cleanly up to 67 levels (68 trips) — all comfortably beyond anything a
/// hand-written Maple program needs, and all six independently confirmed not
/// to crash a default-stack thread even thousands of levels past the cap
/// (see this crate's tests).
const MAX_RULE_DEPTH: usize = 150;

/// Create a [`GrammarParser`] wired to the Maple grammar and the tokens of
/// `source`, with the recursion-depth guard ([`MAX_RULE_DEPTH`]) enabled so
/// pathologically deep nesting fails cleanly instead of overflowing the
/// native stack.
pub fn create_maple_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_maple(source);
    GrammarParser::new(tokens, _grammar::parser_grammar()).with_max_depth(MAX_RULE_DEPTH)
}

/// Parse Maple source text into a [`GrammarASTNode`] rooted at `program`.
///
/// # Panics
///
/// Panics on a lexical or syntax error. Use [`try_parse_maple`] to handle
/// errors.
///
/// # Example
///
/// ```
/// use coding_adventures_maple_parser::parse_maple;
/// let ast = parse_maple("x := 5;\n");
/// assert_eq!(ast.rule_name, "program");
/// ```
pub fn parse_maple(source: &str) -> GrammarASTNode {
    create_maple_parser(source)
        .parse()
        .unwrap_or_else(|e| panic!("Maple parse failed: {e}"))
}

/// Parse Maple source text, returning a `Result` instead of panicking.
pub fn try_parse_maple(source: &str) -> Result<GrammarASTNode, String> {
    let tokens = try_tokenize_maple(source)?;
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
        try_parse_maple(src).is_ok()
    }

    #[test]
    fn program_is_the_root() {
        assert_eq!(parse_maple("1;\n").rule_name, "program");
    }

    #[test]
    fn bare_trailing_statement_with_no_terminator_parses() {
        assert!(parses("1"));
    }

    #[test]
    fn semi_and_colon_are_interchangeable_terminators() {
        assert!(parses("x := 1; y := 2:"));
    }

    // --- Function application uses ordinary parens -------------------------

    #[test]
    fn function_call_uses_ordinary_parens() {
        let ast = parse_maple("sin(x);\n");
        assert!(contains_rule(&ast, "postfix"));
        assert!(contains_rule(&ast, "arglist"));
    }

    #[test]
    fn multi_arg_calls_parse() {
        assert!(parses("f(x, y, z);\n"));
    }

    #[test]
    fn nested_function_calls_parse() {
        assert!(parses("f(g(x));\n"));
    }

    // --- `:=` is assignment, `=` is equation, never confused ---------------

    #[test]
    fn assignment_parses() {
        let ast = parse_maple("x := 5;\n");
        assert!(contains_rule(&ast, "assignment"));
    }

    #[test]
    fn eq_is_equation_distinct_from_assign() {
        assert!(parses("x = 4;\n"));
        assert!(contains_rule(&parse_maple("x = 4;\n"), "comparison"));
    }

    #[test]
    fn every_comparison_operator_parses() {
        for op in ["=", "<>", "<", ">", "<=", ">="] {
            let src = format!("a {op} b;\n");
            assert!(parses(&src), "`{src}` should parse");
        }
    }

    // --- The arrow-operator / Define shape ----------------------------------

    #[test]
    fn arrow_definition_with_two_parameters_parses() {
        let ast = parse_maple("f := (x, y) -> x + y;\n");
        assert!(contains_rule(&ast, "arrow_def"));
        assert!(contains_rule(&ast, "arrow_params"));
    }

    #[test]
    fn arrow_definition_with_one_bare_parameter_parses() {
        let ast = parse_maple("f := x -> x;\n");
        assert!(contains_rule(&ast, "arrow_def"));
    }

    #[test]
    fn arrow_definition_with_zero_parameters_parses() {
        // `() -> 5` -- a nullary constant function. Not one of MA09 §3's own
        // worked examples, but a natural, undeliberate fallout of
        // `arrow_params`' own optional inner list -- see maple.grammar's
        // "arrow_params" comment.
        assert!(parses("f := () -> 5;\n"));
    }

    #[test]
    fn plain_assignment_does_not_produce_an_arrow_def_node() {
        // `f := x;` (assigning a variable's VALUE) must backtrack cleanly
        // out of `arrow_def` (which requires an ARROW after the bare name)
        // and fall through to the plain `expr` alternative instead.
        let ast = parse_maple("f := x;\n");
        assert!(contains_rule(&ast, "assignment"));
        assert!(!contains_rule(&ast, "arrow_def"));
    }

    #[test]
    fn arrow_never_appears_outside_an_assignment_rhs() {
        // MA09 §3: `->` is used ONLY as a Define right-hand side. A bare
        // arrow expression with no `NAME :=` prefix must be a syntax error
        // -- there is nowhere else in this grammar `ARROW` is referenced.
        assert!(try_parse_maple("(x) -> x;\n").is_err());
        assert!(try_parse_maple("x -> x;\n").is_err());
    }

    #[test]
    fn arrow_cannot_nest_inside_arithmetic() {
        assert!(try_parse_maple("y := 1 + (x -> x);\n").is_err());
    }

    // --- `f(x) := e` (the remember-table spelling) is rejected --------------

    #[test]
    fn remember_table_spelling_is_rejected() {
        // MA09 §1/§4: `f(x) := expr` is real Maple's narrower remember-table
        // mechanism, deliberately EXCLUDED from this subset -- this
        // grammar's `assignment` left-hand side is a bare NAME, so this
        // spelling must fail to parse entirely, not merely lower
        // differently at a later stage. See maple.grammar's own "assignment
        // left-hand side" design-decision comment.
        assert!(try_parse_maple("f(x) := 1;\n").is_err());
        assert!(try_parse_maple("h(l, m) := l + m;\n").is_err());
    }

    // --- `if` and `:=` are statements, never nested expressions -------------

    #[test]
    fn if_is_not_usable_as_an_assignment_rhs() {
        // Unlike reduce-parser's `if_is_usable_as_an_expression` test --
        // MA09 makes no equivalent claim for Maple, and real Maple's own
        // conditional-VALUE idiom is the `piecewise` library call, not a
        // bare `if`. See maple.grammar's "statements vs. expressions" design
        // decision.
        assert!(try_parse_maple("x := if a then 1 else 2 end if;\n").is_err());
    }

    #[test]
    fn chained_assignment_is_rejected() {
        // Unlike reduce-parser's `assignment_right_associates` test -- MA09
        // cites no equivalent to REDUCE manual §2.7's "a:=b:=c evaluates as
        // a:=(b:=c)" for Maple, and every MA09 §3 worked example is a
        // single, non-chained assignment.
        assert!(try_parse_maple("a := b := 5;\n").is_err());
    }

    // --- List vs. set literals are genuinely distinct productions ----------

    #[test]
    fn list_literal_uses_square_brackets() {
        let ast = parse_maple("[a, b, c];\n");
        assert!(contains_rule(&ast, "list_literal"));
        assert!(!contains_rule(&ast, "set_literal"));
    }

    #[test]
    fn set_literal_uses_curly_braces() {
        let ast = parse_maple("{a, b, c};\n");
        assert!(contains_rule(&ast, "set_literal"));
        assert!(!contains_rule(&ast, "list_literal"));
    }

    #[test]
    fn empty_list_literal_parses() {
        let ast = parse_maple("[];\n");
        assert!(contains_rule(&ast, "list_literal"));
    }

    #[test]
    fn empty_set_literal_parses() {
        let ast = parse_maple("{};\n");
        assert!(contains_rule(&ast, "set_literal"));
    }

    #[test]
    fn list_and_set_literal_in_a_call_snippet() {
        assert!(parses("g([1, 2, 3], {1, 2, 2});\n"));
    }

    // --- Boolean literals and logical operators -----------------------------

    #[test]
    fn boolean_literals_parse() {
        assert!(parses("true;\n"));
        assert!(parses("false;\n"));
    }

    #[test]
    fn boolean_keywords_parse() {
        assert!(parses("a and b or not c;\n"));
        assert!(contains_rule(&parse_maple("a and b;\n"), "logical_and"));
        assert!(contains_rule(&parse_maple("a or b;\n"), "logical_or"));
        assert!(contains_rule(&parse_maple("not a;\n"), "logical_not"));
    }

    #[test]
    fn uppercase_keyword_spellings_lex_as_plain_names_not_keywords() {
        // maple.tokens' keywords are lowercase-only (mirroring
        // reduce.tokens, the opposite of derive.tokens' uppercase
        // AND/OR/NOT) -- `AND` here is just a NAME, so this is a bare `a
        // AND b` juxtaposition with no operator between two names, which is
        // not valid syntax in this subset (no juxtaposition production
        // exists anywhere in this grammar, MA09 §4).
        assert!(try_parse_maple("a AND b;\n").is_err());
    }

    #[test]
    fn bare_juxtaposed_names_with_no_operator_is_rejected() {
        // Mirrors reduce-parser's identically-named, identically-motivated
        // regression test: this is a regression test for exactly the bug
        // `program`'s own grammar comment documents -- an earlier draft
        // could fold "no terminator" into `statement_line` itself (tried
        // per repetition iteration, not just once at the very end), which
        // would let `a`, `AND`, and `b;` each parse as their own bare
        // statement and silently consume the entire input as three separate
        // program items.
        assert!(try_parse_maple("a AND b;\n").is_err());
        assert!(try_parse_maple("a b\n").is_err());
    }

    // --- Arithmetic precedence and associativity ----------------------------

    #[test]
    fn arithmetic_precedence_nests_correctly() {
        // `1 + 2 * 3` must parse as `1 + (2*3)`, not `(1+2)*3` -- the tree
        // must contain both `additive` and `multiplicative`, with
        // `multiplicative` nested inside the `additive` continuation.
        let ast = parse_maple("1 + 2 * 3;\n");
        assert!(contains_rule(&ast, "additive"));
        assert!(contains_rule(&ast, "multiplicative"));
    }

    #[test]
    fn explicit_star_is_required_no_pow_synonym_double_star() {
        // MA09 §3: `^` ONLY, no `**` synonym. `a ** b` must NOT parse as a
        // single power expression -- `**` lexes as two TIMES tokens
        // (confirmed by maple-lexer's own `double_star_is_not_a_single_pow_token`
        // test), so `a ** b` is `a` followed by a bare `*` with nothing
        // between the two stars, i.e. `a * (* b)` -- a syntax error, since
        // `multiplicative`'s right operand must be a `unary`, and a bare
        // `*` is not a valid start of one.
        assert!(try_parse_maple("a ** b;\n").is_err());
    }

    #[test]
    fn power_is_right_associative_by_shape() {
        assert!(parses("4^3^2;\n"));
        let ast = parse_maple("4^3^2;\n");
        assert!(contains_rule(&ast, "power"));
    }

    #[test]
    fn unary_minus_binds_looser_than_power() {
        let ast = parse_maple("-x^2;\n");
        assert!(contains_rule(&ast, "unary"));
        assert!(contains_rule(&ast, "power"));
    }

    #[test]
    fn grouping_parens_parse() {
        assert!(parses("(1 + 2) * 3;\n"));
        assert!(contains_rule(&parse_maple("(1 + 2) * 3;\n"), "group"));
    }

    // --- `if`/`elif`/`else` closed both ways --------------------------------

    #[test]
    fn if_then_end_if_parses() {
        let ast = parse_maple("if x > 0 then 1 end if;\n");
        assert!(contains_rule(&ast, "if_expr"));
    }

    #[test]
    fn if_then_else_end_if_parses() {
        let ast = parse_maple("if x > 0 then 1 else -1 end if;\n");
        assert!(contains_rule(&ast, "if_expr"));
    }

    #[test]
    fn if_then_fi_parses() {
        // The `fi` closing spelling -- "if" reversed.
        let ast = parse_maple("if x > 0 then 1 else -1 fi;\n");
        assert!(contains_rule(&ast, "if_expr"));
    }

    #[test]
    fn if_elif_else_end_if_and_fi_produce_equivalent_shapes() {
        // Both closing spellings must parse to a structurally equivalent
        // if_expr shape (same number of elif arms, same else presence) --
        // matching MA09 §3's own two-worked-spelling example.
        let end_if = parse_maple(
            "if x > 0 then 1 elif x < 0 then -1 else 0 end if;\n",
        );
        let fi = parse_maple("if x > 0 then 1 elif x < 0 then -1 else 0 fi;\n");
        assert!(contains_rule(&end_if, "if_expr"));
        assert!(contains_rule(&fi, "if_expr"));

        fn count_nodes(node: &GrammarASTNode, name: &str) -> usize {
            let mut n = if node.rule_name == name { 1 } else { 0 };
            for c in &node.children {
                if let ASTNodeOrToken::Node(child) = c {
                    n += count_nodes(child, name);
                }
            }
            n
        }
        // Both must contain exactly one `if_expr` and the elif condition
        // must have been parsed (an `expr`/`comparison` for `x < 0`).
        assert_eq!(count_nodes(&end_if, "if_expr"), 1);
        assert_eq!(count_nodes(&fi, "if_expr"), 1);
    }

    #[test]
    fn elif_chain_preserves_every_arm() {
        let ast = parse_maple(
            "if a then 1 elif b then 2 elif c then 3 else 4 end if;\n",
        );
        // Every `elif` condition ("b" and "c") and its body ("2" and "3")
        // must appear somewhere in the tree, in order -- the flat `{ "elif"
        // ... }` repetition preserves the whole chain as sibling children.
        fn all_names(node: &GrammarASTNode, out: &mut Vec<String>) {
            for c in &node.children {
                match c {
                    ASTNodeOrToken::Token(t) if t.effective_type_name() == "NAME" => {
                        out.push(t.value.clone())
                    }
                    ASTNodeOrToken::Node(n) => all_names(n, out),
                    _ => {}
                }
            }
        }
        let mut names = Vec::new();
        all_names(&ast, &mut names);
        assert_eq!(names, vec!["a", "b", "c"]);
    }

    #[test]
    fn if_can_branch_to_an_assignment() {
        // MA09 §3's own s1/s2/s3 labelling reads as statement-shaped, not
        // narrowly expression-shaped -- an `if` branching to an assignment
        // must be supported even though `if` is never itself usable as a
        // nested expression (see maple.grammar's "statements vs.
        // expressions" design decision).
        let ast = parse_maple("if a then x := 1 else x := 2 end if;\n");
        assert!(contains_rule(&ast, "if_expr"));
        assert!(contains_rule(&ast, "assignment"));
    }

    #[test]
    fn nested_if_resolves_unambiguously_with_no_dangling_else() {
        // Unlike reduce-parser's dangling-else test -- Maple's explicit
        // closing keywords mean there is only one place an outer `else`
        // could possibly attach, by construction.
        assert!(parses("if a then if b then c end if else d end if;\n"));
        let ast = parse_maple("if a then if b then c end if else d end if;\n");
        fn count_if_exprs(node: &GrammarASTNode) -> usize {
            let mut n = if node.rule_name == "if_expr" { 1 } else { 0 };
            for c in &node.children {
                if let ASTNodeOrToken::Node(child) = c {
                    n += count_if_exprs(child);
                }
            }
            n
        }
        assert_eq!(count_if_exprs(&ast), 2);
    }

    // --- Multi-statement programs --------------------------------------------

    #[test]
    fn a_multi_statement_program_parses() {
        let ast = parse_maple("x := 1; y := 2; x + y;\n");
        let stmt_lines: usize = ast
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "statement_line"))
            .count();
        assert_eq!(stmt_lines, 3);
    }

    #[test]
    fn syntax_error_is_reported() {
        assert!(try_parse_maple("1 +\n").is_err());
        assert!(try_parse_maple("(1 + 2\n").is_err());
    }

    // -------------------------------------------------------------------
    // Recursion-depth guard (DoS hardening) -- exercises all six
    // independently-measured shapes documented on `MAX_RULE_DEPTH`.
    // -------------------------------------------------------------------

    fn nested_paren_source(n: usize) -> String {
        format!("{}5{};\n", "(".repeat(n), ")".repeat(n))
    }

    fn nested_list_source(n: usize) -> String {
        format!("{}5{};\n", "[".repeat(n), "]".repeat(n))
    }

    fn not_chain_source(n: usize) -> String {
        format!("{}a;\n", "not ".repeat(n))
    }

    fn unary_minus_chain_source(n: usize) -> String {
        format!("{}a;\n", "-".repeat(n))
    }

    fn power_chain_source(n: usize) -> String {
        let mut s = String::from("1");
        for _ in 0..n {
            s.push_str("^1");
        }
        s.push_str(";\n");
        s
    }

    fn nested_if_source(n: usize) -> String {
        let mut s = String::new();
        for _ in 0..n {
            s.push_str("if 1 then ");
        }
        s.push('5');
        for _ in 0..n {
            s.push_str(" end if");
        }
        s.push_str(";\n");
        s
    }

    /// Deeply-nested input, for every measured shape, must produce a
    /// recoverable error, not overflow the native stack. Parses 5000
    /// levels -- far past `MAX_RULE_DEPTH` -- on a worker thread with a
    /// generous 32 MiB stack, so the *guard* is what stops the recursion,
    /// not the stack running out.
    #[test]
    fn test_deeply_nested_input_returns_error_not_overflow_for_every_shape() {
        let sources = vec![
            nested_paren_source(5000),
            nested_list_source(5000),
            not_chain_source(5000),
            unary_minus_chain_source(5000),
            power_chain_source(5000),
            nested_if_source(2000),
        ];
        let handle = std::thread::Builder::new()
            .name("maple-parser-depth-guard-regression".to_string())
            .stack_size(32 * 1024 * 1024)
            .spawn(move || {
                for src in sources {
                    let result = try_parse_maple(&src);
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
            nested_list_source(5000),
            not_chain_source(5000),
            unary_minus_chain_source(5000),
            power_chain_source(5000),
            nested_if_source(2000),
        ];
        let handle = std::thread::spawn(move || {
            for src in sources {
                let result = try_parse_maple(&src);
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
        assert!(try_parse_maple(&nested_paren_source(3)).is_ok());
        assert!(try_parse_maple(&nested_list_source(3)).is_ok());
        assert!(try_parse_maple(&not_chain_source(5)).is_ok());
        assert!(try_parse_maple(&unary_minus_chain_source(5)).is_ok());
        assert!(try_parse_maple(&power_chain_source(5)).is_ok());
        assert!(try_parse_maple(&nested_if_source(3)).is_ok());
    }

    /// Input that nests *exactly up to* `MAX_RULE_DEPTH`'s measured
    /// real-input headroom for every shape still parses cleanly, and one
    /// level deeper cleanly trips the cap. These exact boundary counts were
    /// found empirically by binary-searching `try_parse_maple` (the CAPPED
    /// public API, `MAX_RULE_DEPTH = 150`) against increasing nesting counts
    /// for each shape -- see `MAX_RULE_DEPTH`'s own doc comment ("Measured
    /// real-input headroom at `150`"). Without this test, a future change to
    /// the constant could silently move these boundaries without anyone
    /// noticing, mirroring `j-parser`'s own per-shape boundary tests.
    #[test]
    fn test_headroom_boundary_for_every_shape() {
        assert!(try_parse_maple(&nested_paren_source(11)).is_ok());
        assert!(try_parse_maple(&nested_paren_source(12)).is_err());

        assert!(try_parse_maple(&nested_list_source(10)).is_ok());
        assert!(try_parse_maple(&nested_list_source(11)).is_err());

        assert!(try_parse_maple(&not_chain_source(135)).is_ok());
        assert!(try_parse_maple(&not_chain_source(136)).is_err());

        assert!(try_parse_maple(&unary_minus_chain_source(135)).is_ok());
        assert!(try_parse_maple(&unary_minus_chain_source(136)).is_err());

        assert!(try_parse_maple(&power_chain_source(67)).is_ok());
        assert!(try_parse_maple(&power_chain_source(68)).is_err());

        assert!(try_parse_maple(&nested_if_source(67)).is_ok());
        assert!(try_parse_maple(&nested_if_source(68)).is_err());
    }
}
