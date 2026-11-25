//! # Scilab Parser — building a syntax tree for Scilab (a subset).
//!
//! Turns the token stream from [`coding_adventures_scilab_lexer`] into a
//! parse tree using the generic
//! [`GrammarParser`](parser::grammar_parser::GrammarParser), driven by the
//! embedded `scilab.grammar` (`src/_grammar.rs`). It hand-writes no parsing
//! logic — a sibling of `maple-parser`/`matlab-parser`/`j-parser`. See
//! `code/specs/MA10-scilab-language.md` (MA-10c).
//!
//! `scilab.grammar` is **forked from** `code/grammars/matlab/matlab.grammar`
//! at the grammar-**source** level (copied, then diverged) — this crate does
//! **not** depend on `matlab-parser` at build time (MA10 §5). The *shape* of
//! MATLAB's grammar (matrix literals, ranges, the operator-precedence
//! cascade, indexing) is a legitimate inheritance; three things genuinely are
//! not MATLAB (MA10 §3 "Parser differences"), each documented at its own
//! production in `scilab.grammar`:
//!
//! 1. `stmt_sep` — one new, reused production for the optional `then`/`do`
//!    linker keyword at six header sites (`if`/`elseif`/`select`/`case`/
//!    `while`/`for`), each individually replaceable by a bare comma or
//!    newline instead.
//! 2. `endfunction` is `func_def`'s own distinct closing keyword, never
//!    unified with the generic `end` every other block construct closes
//!    with (MA10 §1 finding 7).
//! 3. `$` (`DOLLAR`) replaces MATLAB's context-sensitive `end`-as-last-index
//!    trick with an ordinary, always-unambiguous `primary` alternative —
//!    `scilab-parser` needs no pre-parse retagging hook at all, unlike
//!    `matlab-parser`.
//!
//! ```text
//! Scilab source
//!    |
//!    v
//! coding_adventures_scilab_lexer::tokenize_scilab  ->  Vec<Token>
//!    |
//!    v
//! parser::GrammarParser  (driven by the embedded scilab.grammar)
//!    |
//!    v
//! GrammarASTNode  <- the tree a future scilab-runtime (MA-10d) walks
//! ```

use coding_adventures_scilab_lexer::{tokenize_scilab, try_tokenize_scilab};
use parser::grammar_parser::{GrammarASTNode, GrammarParser};
mod _grammar;

/// Recursion-depth cap for the Scilab [`GrammarParser`] — see
/// [`GrammarParser::with_max_depth`] and
/// [`parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH`] for why the underlying
/// guard exists at all (deep recursion through `parse_rule` can overflow the
/// *native* thread stack — an uncatchable process abort — before this
/// crate's own `Result`-returning entry points ever get a chance to report
/// anything).
///
/// # Seven recursion shapes, measured independently, per MA10 §6's directive
///
/// MA10 §6 (task MA-10c) requires measuring **this** grammar's own actual
/// native-stack crash floor rather than assuming a sibling `*-parser`
/// crate's numbers transfer — following the "measure, don't assume one
/// shape's floor bounds the others" methodology `apl-parser`/`j-parser`'s
/// own `CHANGELOG.md`s document, and the same six/five/four-shape survey
/// `maple-parser`'s own `MAX_RULE_DEPTH` doc comment performs for Maple.
/// `scilab.grammar` has (at least) seven structurally distinct
/// self-referential shapes. The first four were measured when this constant
/// was first introduced; shapes 5–7 close a gap a later security review
/// flagged — `scilab.grammar`'s own header comment names `assignment` and
/// `power` as the ONLY two "optional self-recursion" (`[ ... self ]`)
/// productions in the whole file, yet only `power` had been measured, and
/// two more genuinely distinct self-referential shapes (function/cell-index
/// argument nesting, and the unary prefix chain this doc comment already
/// *acknowledged* but never actually measured) were still open:
///
/// 1. **Parenthesised nesting**, `((((…5…))))` — `group -> expr ->
///    assignment -> logical_or -> logical_and -> bit_or -> bit_and ->
///    comparison -> colon_expr -> additive -> multiplicative -> unary ->
///    power -> postfix -> primary -> group -> …` — cycles through the
///    *entire* expression precedence cascade every nesting level (fifteen
///    rule-frames: `group`, `expr`, `assignment`, `logical_or`,
///    `logical_and`, `bit_or`, `bit_and`, `comparison`, `colon_expr`,
///    `additive`, `multiplicative`, `unary`, `power`, `postfix`, `primary`,
///    before the next `group`).
/// 2. **A flat right-recursive dyadic chain**: the power operator, `1^1^1^
///    … ^1` — chosen as the representative "flat chain" shape over a unary
///    prefix chain (`- - - x` / `~ ~ ~ x`) because `power`'s own `[ ( CARET |
///    ELEM_POW ) unary ]` continuation is the one production in this
///    grammar that is genuinely *right-recursive through a dyadic operator*
///    (cycling `power -> unary -> power -> …` once per `^`), matching the
///    task brief's own suggested representative shape and mirroring
///    `reduce-parser`/`maple-parser`'s identical choice of the power chain
///    for this exact role — a prefix chain measures a different thing
///    (repeated *unary* rule-frames, no dyadic operand on the right) and is
///    not this grammar's most natural "flat chain" exemplar. (Shape 7 below
///    measures that prefix chain too, for parity with `maple-parser`/
///    `reduce-parser`, which measure both.)
/// 3. **Deeply nested `if`/`end`**, `if 1 then if 1 then … 5 … end end` —
///    `if_stmt`'s `block_body` is `{ statement_line }`, `statement_line`
///    contains `statement`, and `statement`'s first alternative is
///    `func_def | if_stmt | select_stmt | …`, so nesting an `if` inside its
///    own body cycles `if_stmt -> block_body -> statement_line -> statement
///    -> if_stmt -> …`. `select_stmt` shares the identical
///    `statement -> if_stmt`-shaped reachability (`select_stmt`'s own
///    `case_clause`'s `block_body` reaches `statement` exactly the same
///    way), so nested `select`/`end` was not separately measured — confirmed
///    by direct inspection of the rule graph (both `if_stmt`'s and
///    `case_clause`'s bodies are the identical `block_body` production), not
///    assumed by shape resemblance.
///
///    **Known, disclosed limitation**: `while_stmt`/`for_stmt`/`func_def`
///    each also contain `block_body` directly (not through an extra
///    `case_clause`-shaped wrapper the way `select_stmt` does) and so form
///    the *same* `statement -> {while_stmt|for_stmt|func_def} -> block_body
///    -> statement_line -> statement -> …` cycle nested `if` does — but,
///    unlike `select_stmt`, this was not independently confirmed by a
///    dedicated measurement, only by this same rule-graph argument. Given
///    this file's own data shows near-identical per-level rule-frame cost
///    does NOT reliably predict a shape's true native-stack floor (chained
///    assignment's 179 vs. the unary-prefix chain's 219, despite both
///    costing one persisting rule-frame per level — see shape 7 and the
///    table below), treat this identity claim with appropriately less
///    confidence than the measured shapes. Practical risk is assessed as
///    low: these three are structurally closer to nested-`if` (rule-frame
///    floor 268, comfortably above `MAX_RULE_DEPTH`) than to the two shapes
///    that turned out to diverge, and 125 sits 143+ units below 268 — but a
///    future audit should actually measure `while 1 do while 1 do … end
///    end`, the analogous `for`, and nested `function … endfunction` rather
///    than rely on this argument alone.
/// 4. **Matrix-literal nesting**, `[[[[…5…]]]]` — structurally DISTINCT
///    from parenthesised nesting, confirmed by direct inspection of the rule
///    graph: `group = LPAREN expr RPAREN` wraps `expr` directly (zero extra
///    frames), but `matrix_literal = LBRACKET [ matrix_rows ] RBRACKET`
///    reaches `expr` through TWO extra rule-frames (`matrix_rows` then
///    `matrix_row`) before an inner `expr` is reached — the same
///    "one extra rule-frame per level" gap `maple-parser`'s own
///    `list_literal` (via `arglist`) has relative to its `group`, doubled
///    here since `matrix_literal` interposes two wrapper rules
///    (`matrix_rows`, `matrix_row`) rather than one.
/// 5. **Chained assignment**, `x=x=x=…=x` — `assignment`'s own `[ EQ
///    assignment ]` continuation, the OTHER "optional self-recursion"
///    production `scilab.grammar`'s header comment names alongside `power`.
///    Unlike every shape above, `assignment`'s persisting per-level cost is
///    exactly ONE rule-frame (`assignment` itself): its `logical_or`
///    left-hand side dives through the entire twelve-frame cascade down to
///    `primary` and back on every level, but that dive is a transient
///    sibling call that fully returns *before* `[ EQ assignment ]` recurses,
///    so it never accumulates — the same "primary returns before the next
///    call, so it doesn't persist" shape `postfix`'s own suffix-repetition
///    loop has (see shape 6). This is why chained assignment tolerates far
///    more nesting than parens, nested-`if`, or matrix literals (see the
///    nesting-count table below) despite being genuinely self-recursive.
/// 6. **Nested function-call arguments**, `f(f(f(…5…)))` — `postfix`'s own
///    suffix-repetition loop calls `primary` (matches the `NAME`, returns
///    immediately — transient, does not persist) and then, on seeing
///    `LPAREN`, `call_suffix -> arg_list -> arg -> expr -> …` down the full
///    twelve-frame cascade and back into a NEW `postfix` invocation.
///    Structurally distinct from every shape above — it re-enters `postfix`
///    through a *suffix* repetition, not through `primary`'s own alternation
///    the way `group`/`matrix_literal` do — and, like matrix-literal
///    nesting, interposes THREE wrapper rule-frames per level
///    (`call_suffix`, `arg_list`, `arg`) instead of `group`'s one, so its
///    per-level rule-frame cost (see the ratio check below) lands close to
///    matrix-literal nesting's own. `cell_suffix = LBRACE [ arg_list ]
///    RBRACE` reaches `arg_list` the identical way (`A{A{A{…}}}` would cycle
///    through the exact same `postfix -> cell_suffix -> arg_list -> arg ->
///    expr -> … -> postfix` path), so nested cell-indexing was not
///    separately measured — the same "identical reachability, confirmed by
///    direct inspection, not assumed by shape resemblance" reasoning shape 3
///    above already applies to `select_stmt`/`if_stmt`.
/// 7. **A unary prefix chain**, `- - - … 5` (or `~ ~ ~ … 5`) — `unary`'s own
///    `( PLUS | MINUS | TILDE ) unary` self-reference, ACKNOWLEDGED but never
///    actually measured when this doc comment first justified picking the
///    power chain as "the" representative flat-chain shape (see shape 2).
///    `maple-parser`/`reduce-parser` measure both their power chain AND a
///    unary/`not` prefix chain separately, and found the two can have very
///    different rule-frame floors despite superficially similar "flat
///    chain" shapes; measuring it here closes that parity gap. Like chained
///    assignment (shape 5), `unary`'s persisting per-level cost is exactly
///    ONE rule-frame (`unary` itself) — its base case, `power = postfix [
///    ( CARET | ELEM_POW ) unary ]`, is a much SHALLOWER transient dive
///    (`power -> postfix -> primary`, three frames) than assignment's own
///    twelve-frame `logical_or` dive, which is why it tolerates even more
///    nesting than chained assignment (202 vs. 162 — see the nesting-count
///    table below) yet still binds at a meaningfully lower rule-frame floor
///    than parens/nested-`if`/matrix-literal nesting (see the rule-frame
///    table below).
///
/// Every "flat chain of one operator" production written with EBNF `{ x }`
/// repetition (`logical_or`, `logical_and`, `bit_or`, `bit_and`,
/// `comparison`, `additive`, `multiplicative`, `{ elseif_clause }`,
/// `{ case_clause }`, `arg_list`, `name_list`, `matrix_rows`) costs *zero*
/// native stack regardless of width — confirmed directly by reading
/// [`parser::grammar_parser`]'s own `match_element` implementation (the
/// `Repetition` arm is a plain `loop { ... }` where each iteration's
/// `match_element` call returns before the next iteration begins, so the
/// *native* call stack never grows with iteration count), the same
/// engine-level fact every sibling `*-parser` crate's own `MAX_RULE_DEPTH`
/// doc comment already establishes — not re-measured by a throwaway probe
/// here, since it is a fact about the shared engine, not about this one
/// grammar.
///
/// Measured with the same methodology every sibling `*-parser` crate uses:
/// binary search, an *uncapped* `GrammarParser` (`max_depth = usize::MAX`,
/// [`GrammarParser::new`]'s own default), a `std::thread::spawn` worker with
/// the **default ~2 MiB stack** (no `stack_size` override), one fresh
/// **subprocess per data point** (a real native-stack overflow calls
/// `process::abort()`, which kills the whole process, not just the
/// offending thread — an in-process loop cannot survive past the first
/// crash to report a clean number), in a **debug** build (`cargo test`'s own
/// default, since debug frames are meaningfully larger than release frames).
///
/// Nesting-count floors (parses safely up to N, crashes at N+1):
///
/// | Shape | Nesting-count floor |
/// |---|---|
/// | Parenthesised nesting | 18 safe / 19 crash |
/// | Power (`^`) chain | 101 safe / 102 crash |
/// | Nested `if`/`end` | 62 safe / 63 crash |
/// | Matrix-literal nesting | 15 safe / 16 crash |
/// | Chained assignment | 162 safe / 163 crash |
/// | Nested function-call arguments | 16 safe / 17 crash |
/// | Unary prefix chain | 202 safe / 203 crash |
///
/// # The binding constraint is a rule-frame floor, not a nesting-count one
///
/// Exactly as every sibling `*-parser` crate's own doc comment warns, the
/// *nesting-count* floors above do not by themselves say which shape binds
/// `MAX_RULE_DEPTH` — `self.depth` counts *named-rule* invocations, and
/// different shapes cost a different number of rule-frames per nesting
/// level. Converting each measured nesting-count floor into rule-frame terms
/// (binary search over `with_max_depth` against a fixed 5000-level input —
/// 2000 for nested `if`, since each level costs more source text — of each
/// shape, so the *cap itself* — not the input's own finite length — is
/// always what triggers first; same default ~2 MiB stack, same debug build):
///
/// | Shape | Nesting-count floor | Rule-frame floor |
/// |---|---|---|
/// | Parenthesised nesting | 18 safe / 19 crash | 295 safe / 296 crash |
/// | Power (`^`) chain | 101 safe / 102 crash | 220 safe / 221 crash |
/// | Nested `if`/`end` | 62 safe / 63 crash | 268 safe / 269 crash |
/// | Matrix-literal nesting | 15 safe / 16 crash | 289 safe / 290 crash |
/// | Chained assignment | 162 safe / 163 crash | **179 safe / 180 crash** |
/// | Nested function-call arguments | 16 safe / 17 crash | 277 safe / 278 crash |
/// | Unary prefix chain | 202 safe / 203 crash | 219 safe / 220 crash |
///
/// The lowest rule-frame floor is chained assignment's **179** — genuinely
/// the binding shape here (this changed when shapes 5–7 were added; before
/// then, the power chain's 220 was the lowest of the four measured, and
/// `MAX_RULE_DEPTH` was 150). It is not the shape with the fewest tolerated
/// nesting levels (nested function calls, at 16/17), not the shape that
/// binds for most sibling `*-parser` crates in this repo (parenthesised
/// nesting), and not even the power chain — the previous holder of "lowest
/// floor despite tolerating the most levels" among the original four.
/// Chained assignment's *persistent* per-level cost is exactly ONE
/// rule-frame (`assignment` itself — see shape 5's own reasoning above) —
/// cheaper in rule-frame-*count* terms than every other shape here,
/// including the power chain's two (`power`, `unary`) — yet it reaches the
/// native ceiling at a *lower total rule-frame count* (179) than the power
/// chain (220) despite tolerating *more* nesting levels to get there (162
/// vs. 101). The unary prefix chain (shape 7) has the identical
/// one-rule-frame-per-level persistent cost and tolerates even *more*
/// nesting (202) than chained assignment, yet its own rule-frame floor
/// (219) sits close to the power chain's (220) rather than near chained
/// assignment's (179) — so the two "cheapest per-frame-count" shapes here
/// do NOT share a rule-frame floor either, despite an identical persisting
/// frame-per-level count. This reinforces, more sharply than the original
/// four-shape survey could, the standing warning every sibling `*-parser`
/// crate's own doc comment gives: neither "the shape that tolerates the
/// fewest nesting levels must bind" (nested function calls, wrong here) nor
/// "parenthesised nesting binds, since it does for nearly every sibling
/// crate in this repo" (also wrong) nor even "the shape with the fewest
/// rule-frames per level must have the highest floor" (wrong — chained
/// assignment and the unary prefix chain tie on rule-frames-per-level yet
/// differ by 40 in their own rule-frame floors) holds in general; each
/// grammar's own recursive shapes must be measured, not inferred by
/// analogy, and not even inferred from a same-grammar sibling shape that
/// merely *looks* similar. The per-level rule-frame costs are independently
/// consistent with the raw floor ratios (295 / 18 ≈ 16.4 for parens, close
/// to its own fifteen-frame-per-level count; 220 / 101 ≈ 2.18 for power,
/// close to its own two-frame-per-level count; 268 / 62 ≈ 4.32 for nested
/// `if`, close to its own four-frame-per-level count; 289 / 15 ≈ 19.3 for
/// matrix literals, close to its own seventeen-frame-per-level count; 179 /
/// 162 ≈ 1.10 for chained assignment, close to its own one-frame-per-level
/// count; 277 / 16 ≈ 17.3 for nested function calls, close to matrix
/// literals' own seventeen-frame-per-level count — expected, since both
/// interpose three wrapper rule-frames per level relative to `group`'s one;
/// 219 / 202 ≈ 1.08 for the unary prefix chain, close to its own
/// one-frame-per-level count — small discrepancies are the constant
/// top-level wrapping overhead, e.g. `program`/`statement_line`, paid once
/// regardless of nesting depth), a useful sanity check that the measurement
/// is behaving as the rule-graph analysis predicts, not an artifact of
/// measurement noise.
///
/// `MAX_RULE_DEPTH` is set to **125** — about 30.2% below the binding
/// chained-assignment rule-frame floor of 179 (a comparable margin to
/// `reduce-parser`'s own ~28.5%, `apl-parser`'s ~26.5%, `j-parser`'s ~30%,
/// `derive-parser`'s ~33%, `maple-parser`'s ~31.2%), and therefore safely
/// below the other six rule-frame floors (219, 220, 268, 277, 289, 295) as
/// well. This is DOWN from the original **150**, which sat only ~16.2%
/// below 179 — comfortably safe in absolute terms (150 < 179), but not
/// enough margin to match this repo's own 25–33% convention once chained
/// assignment's lower floor was known, so the constant moved rather than
/// leaving a thinner safety margin than every sibling crate uses.
///
/// Measured real-input headroom at `125` (using the CAPPED parser, i.e.
/// [`create_scilab_parser`]/[`try_parse_scilab`], so no crash risk at all):
/// parenthesised nesting parses cleanly up to 7 levels (8 trips the cap); a
/// power chain parses cleanly up to 53 levels (54 trips); nested `if`s parse
/// cleanly up to 27 levels (28 trips); matrix-literal nesting parses cleanly
/// up to 6 levels (7 trips); chained assignment parses cleanly up to 108
/// levels (109 trips); nested function-call arguments parse cleanly up to 6
/// levels (7 trips); a unary prefix chain parses cleanly up to 107 levels
/// (108 trips) — all comfortably beyond anything a hand-written Scilab
/// program needs, and all seven independently confirmed not to crash a
/// default-stack thread even thousands of levels past the cap (see this
/// crate's tests).
const MAX_RULE_DEPTH: usize = 125;

/// Build a [`GrammarParser`] wired to the Scilab grammar for the already-
/// tokenized `tokens`, with the recursion-depth guard ([`MAX_RULE_DEPTH`])
/// enabled. The ONE place that constructs a capped Scilab [`GrammarParser`] --
/// [`create_scilab_parser`] and [`try_parse_scilab`] both funnel through
/// this, so a future change to the cap (or to how the parser is wired up)
/// can never miss one of the two call sites.
fn capped_scilab_parser(tokens: Vec<lexer::token::Token>) -> GrammarParser {
    GrammarParser::new(tokens, _grammar::parser_grammar()).with_max_depth(MAX_RULE_DEPTH)
}

/// Create a [`GrammarParser`] wired to the Scilab grammar and the tokens of
/// `source`, with the recursion-depth guard ([`MAX_RULE_DEPTH`]) enabled so
/// pathologically deep nesting fails cleanly instead of overflowing the
/// native stack.
pub fn create_scilab_parser(source: &str) -> GrammarParser {
    capped_scilab_parser(tokenize_scilab(source))
}

/// Parse Scilab source text into a [`GrammarASTNode`] rooted at `program`.
///
/// # Panics
///
/// Panics on a lexical or syntax error. Use [`try_parse_scilab`] to handle
/// errors.
///
/// # Example
///
/// ```
/// use coding_adventures_scilab_parser::parse_scilab;
/// let ast = parse_scilab("x = 5\n");
/// assert_eq!(ast.rule_name, "program");
/// ```
pub fn parse_scilab(source: &str) -> GrammarASTNode {
    create_scilab_parser(source)
        .parse()
        .unwrap_or_else(|e| panic!("Scilab parse failed: {e}"))
}

/// Parse Scilab source text, returning a `Result` instead of panicking.
pub fn try_parse_scilab(source: &str) -> Result<GrammarASTNode, String> {
    let tokens = try_tokenize_scilab(source)?;
    capped_scilab_parser(tokens)
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
        try_parse_scilab(src).is_ok()
    }

    #[test]
    fn program_is_the_root() {
        assert_eq!(parse_scilab("x = 5\n").rule_name, "program");
    }

    // --- Assignment, terminators --------------------------------------

    #[test]
    fn assignment_with_semicolon_suppresses_nothing_structurally() {
        // The grammar keeps whichever terminator was used (`;` vs newline/
        // comma) in the tree; a future runtime decides display suppression.
        assert!(parses("x = 5;\n"));
        assert!(parses("x = 5\n"));
        assert!(parses("x = 5,\n"));
    }

    #[test]
    fn bare_trailing_statement_with_no_terminator_parses() {
        assert!(parses("x = 5"));
    }

    // --- if/elseif/else/end, with and without `then` -------------------

    #[test]
    fn if_end_with_newline_separator_no_then() {
        let ast = parse_scilab("if x\n y = 1\nend\n");
        assert!(contains_rule(&ast, "if_stmt"));
    }

    #[test]
    fn if_end_with_comma_separator_no_then() {
        let ast = parse_scilab("if x, y = 1, end\n");
        assert!(contains_rule(&ast, "if_stmt"));
    }

    #[test]
    fn if_end_with_then_linker() {
        let ast = parse_scilab("if x then y = 1, end\n");
        assert!(contains_rule(&ast, "if_stmt"));
    }

    #[test]
    fn if_then_with_no_trailing_separator_before_body() {
        // MA10's own reading: `then` REPLACES the comma/newline, it does not
        // require one in addition -- `then y = 1` with nothing at all
        // between the keyword and the first body statement must parse.
        assert!(parses("if x then y = 1\nend\n"));
    }

    #[test]
    fn elseif_and_else_with_and_without_then() {
        let ast = parse_scilab("if x then y = 1\nelseif z, y = 2\nelse y = 3\nend\n");
        assert!(contains_rule(&ast, "elseif_clause"));
        assert!(contains_rule(&ast, "else_clause"));
    }

    #[test]
    fn bare_end_does_not_close_a_function() {
        // `endfunction` is required to close a function -- a generic `end`
        // must NOT close it (MA10 §1 finding 7 / §3). Here a bare `end`
        // closes the `if`, leaving the function itself unclosed, so the
        // whole thing must fail to parse.
        assert!(try_parse_scilab("function y = f(x)\n if x then y = 1\n end\nend\n").is_err());
    }

    // --- select/case/else/end, with and without `then` -----------------

    #[test]
    fn select_case_else_end_with_then() {
        let ast =
            parse_scilab("select x\n case 1 then y = 1\n case 2 then y = 2\n else y = 0\n end\n");
        assert!(contains_rule(&ast, "select_stmt"));
        assert!(contains_rule(&ast, "case_clause"));
        assert!(contains_rule(&ast, "else_clause"));
    }

    #[test]
    fn select_case_else_end_with_comma_no_then() {
        let ast = parse_scilab("select x, case 1, y = 1, case 2, y = 2, else y = 0, end\n");
        assert!(contains_rule(&ast, "select_stmt"));
        assert!(contains_rule(&ast, "case_clause"));
    }

    #[test]
    fn select_header_itself_accepts_bare_newline_separator() {
        assert!(parses("select x\ncase 1 then y = 1\nend\n"));
    }

    #[test]
    fn switch_and_otherwise_are_not_valid_scilab_constructs() {
        // Scilab has neither spelling (MA10 §1 finding 4) -- `switch`/
        // `otherwise` are ordinary NAMEs to the lexer, so this is just two
        // bare names juxtaposed with no operator, a syntax error.
        assert!(try_parse_scilab("switch x\n otherwise y = 1\n end\n").is_err());
    }

    // --- while/end and for/end, with and without `do` ------------------

    #[test]
    fn while_end_with_do_linker() {
        let ast = parse_scilab("while x do y = 1, end\n");
        assert!(contains_rule(&ast, "while_stmt"));
    }

    #[test]
    fn while_end_with_comma_no_do() {
        assert!(parses("while x, y = 1, end\n"));
    }

    #[test]
    fn while_end_with_newline_no_do() {
        assert!(parses("while x\n y = 1\nend\n"));
    }

    #[test]
    fn for_end_with_do_linker() {
        let ast = parse_scilab("for i = 1:10 do y = i, end\n");
        assert!(contains_rule(&ast, "for_stmt"));
    }

    #[test]
    fn for_end_with_newline_no_do() {
        assert!(parses("for i = 1:10\n y = i\nend\n"));
    }

    #[test]
    fn break_and_continue_parse_inside_loops() {
        assert!(parses("while x do break, end\n"));
        assert!(parses("for i = 1:10 do continue, end\n"));
    }

    // --- function ... endfunction ---------------------------------------

    #[test]
    fn function_with_single_return_and_params() {
        let ast = parse_scilab("function y = f(x)\n y = x * 2\nendfunction\n");
        assert!(contains_rule(&ast, "func_def"));
        assert!(contains_rule(&ast, "func_returns"));
    }

    #[test]
    fn function_with_multiple_returns() {
        let ast = parse_scilab("function [a, b] = f(x)\n a = x\n b = x\nendfunction\n");
        assert!(contains_rule(&ast, "func_def"));
    }

    #[test]
    fn function_with_no_return_value() {
        assert!(parses("function f(x)\n disp(x)\nendfunction\n"));
    }

    #[test]
    fn function_with_no_parameters_at_all() {
        // Parens themselves are optional in this grammar -- unchanged
        // inheritance from matlab.grammar's own `func_def` shape.
        assert!(parses("function y = f()\n y = 1\nendfunction\n"));
    }

    #[test]
    fn endfunction_is_required_not_generic_end() {
        assert!(try_parse_scilab("function y = f(x)\n y = x\nend\n").is_err());
        assert!(parses("function y = f(x)\n y = x\nendfunction\n"));
    }

    // --- Precedence cascade: one test per tier boundary -----------------

    #[test]
    fn additive_and_multiplicative_precedence() {
        // a + b * c -> a + (b*c)
        let ast = parse_scilab("r = a + b * c\n");
        assert!(contains_rule(&ast, "additive"));
        assert!(contains_rule(&ast, "multiplicative"));
    }

    #[test]
    fn colon_binds_looser_than_additive() {
        // a:b+1 -> a:(b+1)
        let ast = parse_scilab("r = a:b+1\n");
        assert!(contains_rule(&ast, "colon_expr"));
        assert!(contains_rule(&ast, "additive"));
    }

    #[test]
    fn comparison_binds_looser_than_colon() {
        let ast = parse_scilab("r = a:b == c:d\n");
        assert!(contains_rule(&ast, "comparison"));
        assert!(contains_rule(&ast, "colon_expr"));
    }

    #[test]
    fn bit_and_binds_looser_than_comparison() {
        let ast = parse_scilab("r = a == b & c == d\n");
        assert!(contains_rule(&ast, "bit_and"));
        assert!(contains_rule(&ast, "comparison"));
    }

    #[test]
    fn bit_or_binds_looser_than_bit_and() {
        let ast = parse_scilab("r = a & b | c & d\n");
        assert!(contains_rule(&ast, "bit_or"));
        assert!(contains_rule(&ast, "bit_and"));
    }

    #[test]
    fn logical_and_binds_looser_than_bit_or() {
        let ast = parse_scilab("r = a | b && c | d\n");
        assert!(contains_rule(&ast, "logical_and"));
        assert!(contains_rule(&ast, "bit_or"));
    }

    #[test]
    fn logical_or_is_loosest_of_all() {
        let ast = parse_scilab("r = a && b || c && d\n");
        assert!(contains_rule(&ast, "logical_or"));
        assert!(contains_rule(&ast, "logical_and"));
    }

    #[test]
    fn unary_binds_looser_than_power() {
        // -x^2 -> -(x^2)
        let ast = parse_scilab("r = -x^2\n");
        assert!(contains_rule(&ast, "unary"));
        assert!(contains_rule(&ast, "power"));
    }

    #[test]
    fn power_is_right_associative_by_shape() {
        assert!(parses("r = 2^3^2\n"));
        let ast = parse_scilab("r = 2^3^2\n");
        assert!(contains_rule(&ast, "power"));
    }

    #[test]
    fn postfix_transpose_binds_tightest_of_the_operators() {
        let ast = parse_scilab("r = A' * B\n");
        assert!(contains_rule(&ast, "postfix"));
        assert!(contains_rule(&ast, "transpose_suffix"));
    }

    #[test]
    fn grouping_parens_parse() {
        assert!(parses("r = (1 + 2) * 3\n"));
        assert!(contains_rule(&parse_scilab("r = (1 + 2) * 3\n"), "group"));
    }

    #[test]
    fn assignment_is_right_associative_and_loosest() {
        // `=` binds looser than everything in the expression cascade.
        let ast = parse_scilab("x = a + b\n");
        assert!(contains_rule(&ast, "assignment"));
        assert!(contains_rule(&ast, "additive"));
    }

    // --- `$` last-index atom ---------------------------------------------

    #[test]
    fn dollar_bare_index() {
        let ast = parse_scilab("r = A($)\n");
        assert!(contains_rule(&ast, "primary"));
    }

    #[test]
    fn dollar_minus_one_composes_with_arithmetic() {
        // A($-1) -- $ must behave as an ordinary primary/atom so it can be
        // the left operand of `additive`'s MINUS.
        let ast = parse_scilab("r = A($-1)\n");
        assert!(contains_rule(&ast, "additive"));
    }

    #[test]
    fn bare_dollar_is_a_legal_expression_on_its_own() {
        // Not just reachable inside an index -- `$` is an ordinary primary,
        // so a bare `$` statement parses too.
        assert!(parses("$\n"));
    }

    #[test]
    fn dollar_composes_with_transpose() {
        // Mirrors scilab-lexer's own `last_index_dollar_is_a_value_before_transpose`
        // lexer test -- `A($)'` must parse as an index followed by a transpose.
        assert!(parses("r = A($)'\n"));
    }

    // --- `PERCENT_CONST` as a primary --------------------------------------

    #[test]
    fn percent_const_used_as_a_primary() {
        let ast = parse_scilab("r = %pi * 2\n");
        assert!(contains_rule(&ast, "primary"));
        assert!(contains_rule(&ast, "multiplicative"));
    }

    #[test]
    fn every_percent_constant_parses_as_a_bare_expression() {
        for name in ["%pi", "%e", "%i", "%inf", "%nan", "%eps", "%t", "%f"] {
            assert!(parses(&format!("{name}\n")), "{name} should parse");
        }
    }

    // --- Both not-equal spellings, same tier -------------------------------

    #[test]
    fn both_not_equal_spellings_reach_the_comparison_tier() {
        let tilde_eq = parse_scilab("r = a ~= b\n");
        let angle_eq = parse_scilab("r = a <> b\n");
        assert!(contains_rule(&tilde_eq, "comparison"));
        assert!(contains_rule(&angle_eq, "comparison"));
    }

    #[test]
    fn all_comparison_operators_parse() {
        for op in ["==", "~=", "<>", "<", ">", "<=", ">="] {
            let src = format!("r = a {op} b\n");
            assert!(parses(&src), "`{src}` should parse");
        }
    }

    // --- Matrix/cell literals and ranges (inherited, confirm correct) ----

    #[test]
    fn matrix_literal_rows_and_columns() {
        let ast = parse_scilab("A = [1 2; 3 4]\n");
        assert!(contains_rule(&ast, "matrix_literal"));
        assert!(contains_rule(&ast, "matrix_rows"));
    }

    #[test]
    fn matrix_literal_with_explicit_commas() {
        assert!(parses("A = [1, 2, 3]\n"));
    }

    #[test]
    fn cell_literal_parses() {
        let ast = parse_scilab("c = {1, 2, 3}\n");
        assert!(contains_rule(&ast, "cell_literal"));
    }

    #[test]
    fn range_two_and_three_argument_forms() {
        assert!(parses("r = 1:10\n"));
        assert!(parses("r = 1:2:10\n"));
    }

    #[test]
    fn whole_dimension_colon_in_index() {
        assert!(parses("r = A(:, 2)\n"));
    }

    #[test]
    fn elementwise_operators_parse() {
        assert!(parses("r = A .* B ./ C .^ D .\\ E\n"));
        assert!(parses("r = A.'\n"));
    }

    #[test]
    fn syntax_error_is_reported() {
        assert!(try_parse_scilab("r = 1 +\n").is_err());
        assert!(try_parse_scilab("r = (1 + 2\n").is_err());
    }

    // -------------------------------------------------------------------
    // Recursion-depth guard (DoS hardening) -- exercises all seven
    // independently-measured shapes documented on `MAX_RULE_DEPTH`.
    // -------------------------------------------------------------------

    fn nested_paren_source(n: usize) -> String {
        format!("r = {}5{}\n", "(".repeat(n), ")".repeat(n))
    }

    fn power_chain_source(n: usize) -> String {
        let mut s = String::from("r = 1");
        for _ in 0..n {
            s.push_str("^1");
        }
        s.push('\n');
        s
    }

    fn nested_if_source(n: usize) -> String {
        let mut s = String::new();
        for _ in 0..n {
            s.push_str("if 1 then ");
        }
        s.push('5');
        for _ in 0..n {
            s.push_str(" end");
        }
        s.push('\n');
        s
    }

    fn nested_matrix_source(n: usize) -> String {
        format!("r = {}5{}\n", "[".repeat(n), "]".repeat(n))
    }

    /// Chained assignment, `r=r=r=...=r` -- `assignment`'s own `[ EQ
    /// assignment ]` self-reference (shape 5 on `MAX_RULE_DEPTH`'s doc
    /// comment).
    fn assignment_chain_source(n: usize) -> String {
        let mut s = String::from("r");
        for _ in 0..n {
            s.push_str("=r");
        }
        s.push('\n');
        s
    }

    /// Nested function-call arguments, `r = f(f(f(...5...)))` -- `postfix ->
    /// call_suffix -> arg_list -> arg -> expr -> ... -> postfix` (shape 6).
    /// `cell_suffix` (`A{...}`) has the identical `arg_list`-mediated shape
    /// and is not separately exercised here (see that shape's own reasoning
    /// on `MAX_RULE_DEPTH`'s doc comment).
    fn nested_call_source(n: usize) -> String {
        format!("r = {}5{}\n", "f(".repeat(n), ")".repeat(n))
    }

    /// A unary prefix chain, `r = ---...-5` -- `unary`'s own `( PLUS | MINUS
    /// | TILDE ) unary` self-reference (shape 7).
    fn unary_prefix_chain_source(n: usize) -> String {
        format!("r = {}5\n", "-".repeat(n))
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
            power_chain_source(5000),
            nested_if_source(2000),
            nested_matrix_source(5000),
            assignment_chain_source(5000),
            nested_call_source(5000),
            unary_prefix_chain_source(5000),
        ];
        let handle = std::thread::Builder::new()
            .name("scilab-parser-depth-guard-regression".to_string())
            .stack_size(32 * 1024 * 1024)
            .spawn(move || {
                for src in sources {
                    let result = try_parse_scilab(&src);
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
            power_chain_source(5000),
            nested_if_source(2000),
            nested_matrix_source(5000),
            assignment_chain_source(5000),
            nested_call_source(5000),
            unary_prefix_chain_source(5000),
        ];
        let handle = std::thread::spawn(move || {
            for src in sources {
                let result = try_parse_scilab(&src);
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
        assert!(try_parse_scilab(&nested_paren_source(3)).is_ok());
        assert!(try_parse_scilab(&power_chain_source(5)).is_ok());
        assert!(try_parse_scilab(&nested_if_source(3)).is_ok());
        assert!(try_parse_scilab(&nested_matrix_source(3)).is_ok());
        assert!(try_parse_scilab(&assignment_chain_source(5)).is_ok());
        assert!(try_parse_scilab(&nested_call_source(3)).is_ok());
        assert!(try_parse_scilab(&unary_prefix_chain_source(5)).is_ok());
    }

    /// Input that nests *exactly up to* `MAX_RULE_DEPTH`'s measured
    /// real-input headroom for every shape still parses cleanly, and one
    /// level deeper cleanly trips the cap. These exact boundary counts were
    /// found empirically by binary-searching `try_parse_scilab` (the CAPPED
    /// public API, `MAX_RULE_DEPTH = 125`) against increasing nesting counts
    /// for each shape -- see `MAX_RULE_DEPTH`'s own doc comment ("Measured
    /// real-input headroom at `125`"). Without this test, a future change to
    /// the constant could silently move these boundaries without anyone
    /// noticing, mirroring `maple-parser`/`j-parser`'s own per-shape
    /// boundary tests.
    #[test]
    fn test_headroom_boundary_for_every_shape() {
        assert!(try_parse_scilab(&nested_paren_source(7)).is_ok());
        assert!(try_parse_scilab(&nested_paren_source(8)).is_err());

        assert!(try_parse_scilab(&power_chain_source(53)).is_ok());
        assert!(try_parse_scilab(&power_chain_source(54)).is_err());

        assert!(try_parse_scilab(&nested_if_source(27)).is_ok());
        assert!(try_parse_scilab(&nested_if_source(28)).is_err());

        assert!(try_parse_scilab(&nested_matrix_source(6)).is_ok());
        assert!(try_parse_scilab(&nested_matrix_source(7)).is_err());

        assert!(try_parse_scilab(&assignment_chain_source(108)).is_ok());
        assert!(try_parse_scilab(&assignment_chain_source(109)).is_err());

        assert!(try_parse_scilab(&nested_call_source(6)).is_ok());
        assert!(try_parse_scilab(&nested_call_source(7)).is_err());

        assert!(try_parse_scilab(&unary_prefix_chain_source(107)).is_ok());
        assert!(try_parse_scilab(&unary_prefix_chain_source(108)).is_err());
    }
}
