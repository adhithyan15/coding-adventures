//! # IDL Parser — building a syntax tree for IDL (Interactive Data Language).
//!
//! Turns the token stream from [`coding_adventures_idl_lexer`] into a parse
//! tree using the generic [`GrammarParser`](parser::grammar_parser::GrammarParser),
//! driven by the embedded `idl.grammar` (`src/_grammar.rs`). It hand-writes
//! no parsing logic — a sibling of `scilab-parser`/`q-parser`. See
//! `code/specs/MA12-idl-language.md` (MA-12c).
//!
//! Unlike every array-family frontend already in this repo (APL/J/Q/Scilab/
//! MATLAB), IDL's *surface* is an Algol/Fortran-family imperative grammar —
//! statements, `PRO`/`FUNCTION` definitions, `IF`/`FOR`/`WHILE`/`REPEAT`
//! blocks, an infix operator-precedence cascade with word operators
//! (`EQ`/`AND`/...) — closer in shape to this repo's `algol-parser`/
//! `dartmouth-basic-parser` than to any array-family parser (MA12 §5). So
//! `idl.grammar` is written to IDL's own shape, not forked from an array
//! sibling (contrast `scilab-parser`, forked from `matlab.grammar` at the
//! grammar-source level).
//!
//! ```text
//! IDL source
//!    |
//!    v
//! coding_adventures_idl_lexer::tokenize_idl  ->  Vec<Token>
//!    |
//!    v
//! parser::GrammarParser  (driven by the embedded idl.grammar)
//!    |
//!    v
//! GrammarASTNode  <- the tree a future idl-runtime (MA-12d) walks
//! ```
//!
//! ## The two genuinely new disambiguations (MA12 §3)
//!
//! `idl-lexer` (MA-12b) deliberately left two glyphs lexically unconditional
//! — `SLASH` is always plain division, `EQUALS` is always one plain `=` —
//! because neither ambiguity can be resolved from raw characters or a flat
//! token list (see that crate's own doc comment). This crate resolves both,
//! entirely through grammar *structure*, with no lookahead predicate of any
//! kind:
//!
//! 1. **`/BOOLEAN` keyword shorthand vs. division.** `arg` is the only
//!    production that ever references a bare, argument-leading `SLASH`
//!    (`bool_keyword_arg = SLASH NAME`). This works because `expr`'s own
//!    precedence cascade never lets `SLASH` appear except as a *binary*
//!    operator consumed after a left operand is already parsed
//!    (`multiplicative`'s own repetition) — IDL has no unary `/`. So at an
//!    argument position, `expr` can never itself succeed starting exactly on
//!    a `SLASH` token, which means a leading `SLASH` there can *only* ever be
//!    the boolean-keyword shorthand. `PLOT, x, /YLOG` hits this production;
//!    `x = a/YLOG` never even reaches `arg` at all (it's an ordinary
//!    `assignment_stmt` RHS), and `a`'s `SLASH` is consumed by
//!    `multiplicative` as ordinary division, exactly like `a + b`.
//! 2. **`=` as assignment vs. keyword-bind.** `assignment_stmt = NAME
//!    [ index_suffix ] EQUALS expr` (statement-level) and `keyword_arg = NAME
//!    EQUALS expr` (inside `arg_list`) have the identical token shape around
//!    `=`, deliberately — what tells them apart is which rule the parser is
//!    *currently inside*: `assignment_stmt` is only ever reached from
//!    `statement`, `keyword_arg` only ever from `arg` (only ever inside an
//!    argument list). No shared production reads `=` and decides later.
//!
//! See `code/grammars/idl/idl.grammar`'s own header comment for the full
//! design writeup, including the procedure-call-statement's own
//! `NAME COMMA arg_list` disambiguation (safe ordinary PEG ordered choice,
//! since `COMMA` is not used as a statement separator anywhere else in this
//! cut) and the disclosed, spec-consistent zero-argument-call scope note.

use coding_adventures_idl_lexer::{tokenize_idl, try_tokenize_idl};
use parser::grammar_parser::{GrammarASTNode, GrammarParser};
mod _grammar;

/// Recursion-depth cap for the IDL [`GrammarParser`] — see
/// [`GrammarParser::with_max_depth`] and
/// [`parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH`] for why the underlying
/// guard exists at all (deep recursion through `parse_rule` can overflow the
/// *native* thread stack — an uncatchable process abort — before this
/// crate's own `Result`-returning entry points ever get a chance to report
/// anything). `idl-lexer` (MA-12b) has no recursion at all (it only
/// tokenizes); this is the FIRST IDL crate with actual recursive descent, so
/// this cap is added fresh here, not inherited from a sibling.
///
/// # Six recursion shapes, measured independently, per MA12 §6's directive
///
/// MA12 §6 (task MA-12c) requires measuring **this** grammar's own actual
/// native-stack crash floor rather than assuming a sibling `*-parser`
/// crate's numbers transfer — the "measure, don't assume one shape's floor
/// bounds the others" methodology `apl-parser`/`j-parser`/`scilab-parser`'s
/// own `CHANGELOG.md`s document. `idl.grammar` has six structurally distinct
/// self-referential shapes:
///
/// 1. **Parenthesised nesting**, `((((…5…))))` — `group -> expr -> logical ->
///    comparison -> additive -> unary -> multiplicative -> power -> postfix
///    -> primary -> group -> …` — cycles through the entire nine-rule
///    expression cascade every nesting level.
/// 2. **Nested `IF`/`ENDIF`**, `if 1 then begin ... 5 ... endif` — `if_stmt ->
///    then_branch -> block_body -> statement_line -> statement -> if_stmt ->
///    …`. `for_stmt`/`while_stmt`/`repeat_stmt`/`begin_block` each reach
///    `block_body` the identical way (`statement -> {for_stmt|while_stmt|
///    repeat_stmt|begin_block} -> {for_body|while_body|repeat_body|
///    block_body} -> block_body -> statement_line -> statement -> …`,
///    confirmed by direct inspection of the rule graph — every one of these
///    five productions reaches `statement` through the same number of
///    wrapper frames as `if_stmt` does), so none of the other four was
///    separately measured — a provable rule-graph identity, not an assumed
///    shape resemblance, mirroring `scilab-parser`'s identical treatment of
///    `select_stmt` relative to `if_stmt`.
/// 3. **Nested function-call arguments**, `f(f(f(…5…)))` — `postfix ->
///    call_suffix -> arg_list -> arg -> expr -> … -> postfix`, interposing
///    two wrapper rule-frames (`call_suffix`, `arg_list`/`arg`) per level
///    on top of `postfix`'s own re-entry, structurally distinct from
///    parenthesised nesting (`group` interposes none).
/// 4. **Nested subscript indexing**, `a[a[a[…5…]]]` — `postfix ->
///    index_suffix -> subscript_list -> subscript -> expr -> … -> postfix`.
///    Confirmed by direct inspection to interpose the SAME number of wrapper
///    rule-frames per level as call-argument nesting (`index_suffix ->
///    subscript_list -> subscript`, three frames, vs. `call_suffix ->
///    arg_list -> arg`, also three) — not merely similar-looking, but
///    reaching `expr` through an identical frame count either way — so this
///    shape was independently measured rather than assumed identical, to
///    confirm the rule-graph symmetry actually holds at the native-stack
///    level too (see the table below: it does, within measurement
///    granularity).
/// 5. **A unary prefix chain**, `- - - … 5` (or `NOT NOT … x`) — `unary`'s
///    own `( PLUS | MINUS | "NOT" ) unary` self-reference. Chosen as its own
///    shape (not assumed to share `power`'s floor) because — unlike
///    Scilab's/MATLAB's own cascade, where `unary` sits BETWEEN
///    `multiplicative` and `power` — this grammar's `unary` sits ABOVE
///    `multiplicative` (looser, per IDL's own documented precedence table,
///    see `idl.grammar`'s header comment), so its base-case dive
///    (`unary -> multiplicative -> power -> postfix -> primary`, four
///    frames) is structurally different from every sibling `*-parser`'s own
///    unary-chain shape.
/// 6. **Nested array literals**, `[[[[…5…]]]]` — structurally distinct from
///    parenthesised nesting: `group = LPAREN expr RPAREN` wraps `expr`
///    directly (zero extra frames), but `array_literal = LBRACKET
///    [ array_elements ] RBRACKET` reaches `expr` through ONE extra
///    rule-frame (`array_elements`) — the same "one extra wrapper frame"
///    gap `maple-parser`'s own `list_literal` has relative to its `group`.
///
/// Every "flat chain of one operator" production written with EBNF `{ x }`
/// repetition (`logical`, `comparison`, `additive`, `multiplicative`,
/// `power`, `postfix`'s own suffix loop, `arg_list`, `subscript_list`,
/// `array_elements`, `params`, `statement_line`'s own `{ STMT_SEP statement
/// }`, `block_body`) costs *zero* native stack regardless of width —
/// confirmed directly by reading [`parser::grammar_parser`]'s own
/// `match_element` implementation (the `Repetition`/`SeparatedRepetition`
/// arms are plain `loop { ... }` where each iteration's `match_element` call
/// returns before the next iteration begins, so the *native* call stack
/// never grows with iteration count) — the same engine-level fact every
/// sibling `*-parser` crate's own `MAX_RULE_DEPTH` doc comment already
/// establishes, not re-measured by a throwaway probe here.
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
/// | Parenthesised nesting | 27 safe / 28 crash |
/// | Nested `IF`/`ENDIF` | 47 safe / 48 crash |
/// | Nested function-call arguments | 22 safe / 23 crash |
/// | Nested subscript indexing | 21 safe / 22 crash |
/// | Unary prefix chain | 199 safe / 200 crash |
/// | Nested array literals | 24 safe / 25 crash |
///
/// # The binding constraint is a rule-frame floor, not a nesting-count one
///
/// Exactly as every sibling `*-parser` crate's own doc comment warns, the
/// *nesting-count* floors above do not by themselves say which shape binds
/// `MAX_RULE_DEPTH` — `self.depth` counts *named-rule* invocations, and
/// different shapes cost a different number of rule-frames per nesting
/// level. Converting each measured nesting-count floor into rule-frame terms
/// (binary search over `with_max_depth` against a fixed 5000-level input —
/// 2000 for nested `IF`, since each level costs more source text — of each
/// shape, so the *cap itself* is always what triggers first; same default
/// ~2 MiB stack, same debug build):
///
/// | Shape | Nesting-count floor | Rule-frame floor |
/// |---|---|---|
/// | Parenthesised nesting | 27 safe / 28 crash | 291 safe / 292 crash |
/// | Nested `IF`/`ENDIF` | 47 safe / 48 crash | 249 safe / 250 crash |
/// | Nested function-call arguments | 22 safe / 23 crash | 266 safe / 267 crash |
/// | Nested subscript indexing | 21 safe / 22 crash | 273 safe / 274 crash |
/// | Unary prefix chain | 199 safe / 200 crash | 212 safe / 213 crash |
/// | Nested array literals | 24 safe / 25 crash | 282 safe / 283 crash |
///
/// The genuine surprise here (mirroring `scilab-parser`'s/`maple-parser`'s
/// own): the unary prefix chain tolerates by far the MOST nesting levels
/// (199) of any measured shape, yet has the LOWEST rule-frame floor (212) —
/// its persisting per-level cost is exactly ONE rule-frame (`unary` itself,
/// confirmed by the near-1:1 nesting-to-frame ratio, 212/199 ≈ 1.07), cheap
/// enough per level to reach 199 nesting levels before the native stack
/// gives out, yet its own call path evidently costs more native-stack bytes
/// per crossing than the other shapes' own higher per-level rule-frame
/// counts would suggest. Confirms, again, that "the shape tolerating the
/// fewest levels must bind" (subscript indexing, wrong here) and
/// "parenthesised nesting binds, since it does for nearly every sibling
/// crate in this repo" (also wrong — parens has a HIGHER rule-frame floor,
/// 291, than the unary chain here) both fail for this grammar too, the same
/// lesson every sibling `*-parser` crate's own doc comment already
/// documents from its own measurements.
///
/// The per-level rule-frame costs are independently consistent with the raw
/// floor ratios, a useful sanity check that the measurement behaves as the
/// rule-graph analysis predicts rather than as noise: 291/27 ≈ 10.8 for
/// parens (`group` interposes zero extra frames before cycling the
/// nine-rule expression cascade back to `primary`); 266/22 ≈ 12.1 for
/// nested calls (`call_suffix`/`arg_list`/`arg` interpose three frames on
/// top of the same cascade); 273/21 ≈ 13.0 for nested subscripts
/// (`index_suffix`/`subscript_list`/`subscript` interpose the same three);
/// 282/24 ≈ 11.75 for nested array literals (`array_literal`/
/// `array_elements` interpose two); 249/47 ≈ 5.3 for nested `IF` (`if_stmt`
/// -> `then_branch` -> `block_body` -> `statement_line` -> `statement`, five
/// frames before the next `if_stmt`); 212/199 ≈ 1.07 for the unary chain
/// (`unary`'s own one-frame self-reference).
///
/// The lowest rule-frame floor is the unary prefix chain's **212**.
/// `MAX_RULE_DEPTH` is set to **148** — about 30.2% below 212 (matching
/// `scilab-parser`'s own ~30.2% margin, and comparable to `reduce-parser`'s
/// ~28.5%, `apl-parser`'s ~26.5%, `j-parser`'s ~30%, `derive-parser`'s ~33%,
/// `maple-parser`'s ~31.2%), and therefore safely below the other five
/// rule-frame floors (291, 249, 266, 273, 282) too.
///
/// Measured real-input headroom at `148` (using the CAPPED parser, i.e.
/// [`create_idl_parser`]/[`try_parse_idl`], so no crash risk at all):
/// parenthesised nesting parses cleanly up to 13 levels (14 trips the cap);
/// a unary prefix chain parses cleanly up to 134 levels (135 trips); nested
/// `IF`s parse cleanly up to 26 levels (27 trips); nested array literals
/// parse cleanly up to 12 levels (13 trips); nested function-call arguments
/// parse cleanly up to 12 levels (13 trips); nested subscript indexing
/// parses cleanly up to 11 levels (12 trips) — all comfortably beyond
/// anything a hand-written IDL program needs, and all six independently
/// confirmed not to crash a default-stack thread even thousands of levels
/// past the cap (see this crate's tests).
const MAX_RULE_DEPTH: usize = 148;

/// Build a [`GrammarParser`] wired to the IDL grammar for the already-
/// tokenized `tokens`, with the recursion-depth guard ([`MAX_RULE_DEPTH`])
/// enabled. The ONE place that constructs a capped IDL [`GrammarParser`] —
/// [`create_idl_parser`] and [`try_parse_idl`] both funnel through this, so
/// a future change to the cap (or to how the parser is wired up) can never
/// miss one of the two call sites.
fn capped_idl_parser(tokens: Vec<lexer::token::Token>) -> GrammarParser {
    GrammarParser::new(tokens, _grammar::parser_grammar()).with_max_depth(MAX_RULE_DEPTH)
}

/// Create a [`GrammarParser`] wired to the IDL grammar and the tokens of
/// `source`, with the recursion-depth guard ([`MAX_RULE_DEPTH`]) enabled so
/// pathologically deep nesting fails cleanly instead of overflowing the
/// native stack.
pub fn create_idl_parser(source: &str) -> GrammarParser {
    capped_idl_parser(tokenize_idl(source))
}

/// Parse IDL source text into a [`GrammarASTNode`] rooted at `program`.
///
/// # Panics
///
/// Panics on a lexical or syntax error. Use [`try_parse_idl`] to handle
/// errors.
///
/// # Example
///
/// ```
/// use coding_adventures_idl_parser::parse_idl;
/// let ast = parse_idl("x = 5\n");
/// assert_eq!(ast.rule_name, "program");
/// ```
pub fn parse_idl(source: &str) -> GrammarASTNode {
    create_idl_parser(source)
        .parse()
        .unwrap_or_else(|e| panic!("IDL parse failed: {e}"))
}

/// Parse IDL source text, returning a `Result` instead of panicking.
pub fn try_parse_idl(source: &str) -> Result<GrammarASTNode, String> {
    let tokens = try_tokenize_idl(source)?;
    capped_idl_parser(tokens).parse().map_err(|e| e.to_string())
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

    /// Collect every descendant node (or self) with the given rule name.
    fn find_rules<'a>(node: &'a GrammarASTNode, name: &str) -> Vec<&'a GrammarASTNode> {
        let mut out = Vec::new();
        if node.rule_name == name {
            out.push(node);
        }
        for c in &node.children {
            if let ASTNodeOrToken::Node(n) = c {
                out.extend(find_rules(n, name));
            }
        }
        out
    }

    /// Collect every token value under a node (in order), for shape assertions.
    fn token_values(node: &GrammarASTNode) -> Vec<String> {
        let mut out = Vec::new();
        for c in &node.children {
            match c {
                ASTNodeOrToken::Token(t) => out.push(t.value.clone()),
                ASTNodeOrToken::Node(n) => out.extend(token_values(n)),
            }
        }
        out
    }

    fn parses(src: &str) -> bool {
        try_parse_idl(src).is_ok()
    }

    #[test]
    fn program_is_the_root() {
        assert_eq!(parse_idl("x = 5\n").rule_name, "program");
    }

    // --- The headline disambiguation: /BOOLEAN vs. division -------------

    #[test]
    fn slash_boolean_keyword_shorthand_inside_a_procedure_call() {
        let ast = parse_idl("PLOT, x, /YLOG\n");
        let call = find_rule(&ast, "procedure_call_stmt").expect("procedure_call_stmt");
        assert!(contains_rule(call, "bool_keyword_arg"));
        let bka = find_rule(call, "bool_keyword_arg").unwrap();
        assert_eq!(token_values(bka), vec!["/", "YLOG"]);
        // The SLASH was consumed by `bool_keyword_arg`, not by a division —
        // no `multiplicative` node anywhere contains a "/" token value.
        assert!(!find_rules(call, "multiplicative")
            .iter()
            .any(|n| token_values(n).contains(&"/".to_string())));
    }

    #[test]
    fn slash_is_ordinary_division_in_an_assignment_rhs() {
        let ast = parse_idl("x = a/YLOG\n");
        let assign = find_rule(&ast, "assignment_stmt").expect("assignment_stmt");
        let mult = find_rule(assign, "multiplicative").expect("multiplicative — division tier");
        assert_eq!(token_values(mult), vec!["a", "/", "YLOG"]);
        // No bool_keyword_arg anywhere — this statement has no argument list at all.
        assert!(!contains_rule(&ast, "bool_keyword_arg"));
    }

    #[test]
    fn slash_is_ordinary_division_as_a_positional_call_argument() {
        // PLOT, a/YLOG -- a positional argument that is itself a division,
        // not a boolean keyword (the SLASH is not argument-leading here).
        let ast = parse_idl("PLOT, a/YLOG\n");
        let call = find_rule(&ast, "procedure_call_stmt").expect("procedure_call_stmt");
        assert!(!contains_rule(call, "bool_keyword_arg"));
        let mult = find_rule(call, "multiplicative").expect("multiplicative");
        assert_eq!(token_values(mult), vec!["a", "/", "YLOG"]);
    }

    #[test]
    fn boolean_keyword_shorthand_in_a_function_call_too() {
        // MA12 §3 item 2: keyword args (and by the same production, the
        // /BOOLEAN shorthand) work identically in procedure AND function calls.
        let ast = parse_idl("y = HISTOGRAM(a, /NAN)\n");
        assert!(contains_rule(&ast, "bool_keyword_arg"));
    }

    // --- The other headline disambiguation: = assignment vs. keyword-bind -

    #[test]
    fn equals_is_keyword_bind_inside_a_call_argument_list() {
        let ast = parse_idl("PLOT, x, TITLE='flux'\n");
        let call = find_rule(&ast, "procedure_call_stmt").expect("procedure_call_stmt");
        assert!(contains_rule(call, "keyword_arg"));
        assert!(!contains_rule(call, "assignment_stmt"));
    }

    #[test]
    fn equals_is_ordinary_assignment_at_statement_level() {
        let ast = parse_idl("x = 5\n");
        assert!(contains_rule(&ast, "assignment_stmt"));
        assert!(!contains_rule(&ast, "keyword_arg"));
    }

    #[test]
    fn keyword_arg_and_positional_and_boolean_shorthand_mix_freely() {
        let ast = parse_idl("PLOT, x, y, TITLE='flux', COLOR=255, /YLOG\n");
        let call = find_rule(&ast, "procedure_call_stmt").expect("procedure_call_stmt");
        assert!(contains_rule(call, "keyword_arg"));
        assert!(contains_rule(call, "bool_keyword_arg"));
    }

    // --- Procedure-call statement vs. plain expression statement --------

    #[test]
    fn bare_name_with_no_comma_is_an_expr_statement_not_a_call() {
        // Disclosed scope note (see idl.grammar's header comment): a
        // zero-arg call is syntactically identical to a bare variable read.
        let ast = parse_idl("STOP\n");
        assert!(contains_rule(&ast, "expr_stmt"));
        assert!(!contains_rule(&ast, "procedure_call_stmt"));
    }

    #[test]
    fn function_call_as_an_expression_statement() {
        let ast = parse_idl("SIN(x)\n");
        assert!(contains_rule(&ast, "expr_stmt"));
        assert!(contains_rule(&ast, "call_suffix"));
    }

    #[test]
    fn procedure_call_with_only_positional_arguments() {
        let ast = parse_idl("PRINT, x, y\n");
        assert!(contains_rule(&ast, "procedure_call_stmt"));
    }

    // --- Assignment, including subscripted targets -----------------------

    #[test]
    fn plain_assignment() {
        assert!(parses("x = 5\n"));
    }

    #[test]
    fn subscripted_assignment_1d() {
        let ast = parse_idl("a[0] = 5\n");
        let assign = find_rule(&ast, "assignment_stmt").unwrap();
        assert!(contains_rule(assign, "index_suffix"));
    }

    #[test]
    fn subscripted_assignment_2d() {
        assert!(parses("a[0, 1] = 5\n"));
    }

    // --- Every subscript form (MA12 §4) -----------------------------------

    #[test]
    fn subscript_plain_index() {
        assert!(parses("y = a[0]\n"));
    }

    #[test]
    fn subscript_two_d() {
        assert!(parses("y = a[0, 1]\n"));
    }

    #[test]
    fn subscript_negative_from_end() {
        let ast = parse_idl("y = a[-1]\n");
        assert!(contains_rule(&ast, "index_suffix"));
        assert!(contains_rule(&ast, "unary"));
    }

    #[test]
    fn subscript_range() {
        let ast = parse_idl("y = a[0:5]\n");
        assert!(contains_rule(&ast, "range_subscript"));
    }

    #[test]
    fn subscript_strided_range() {
        let ast = parse_idl("y = a[0:10:2]\n");
        assert!(contains_rule(&ast, "range_subscript"));
    }

    #[test]
    fn subscript_whole_wildcard() {
        assert!(parses("y = a[*]\n"));
    }

    #[test]
    fn subscript_from_start_to_wildcard() {
        let ast = parse_idl("y = a[2:*]\n");
        assert!(contains_rule(&ast, "range_subscript"));
        assert!(contains_rule(&ast, "range_end"));
    }

    #[test]
    fn subscript_from_start_to_wildcard_strided() {
        assert!(parses("y = a[2:*:3]\n"));
    }

    // --- Array literals ----------------------------------------------------

    #[test]
    fn array_literal_basic() {
        let ast = parse_idl("a = [1, 2, 3]\n");
        assert!(contains_rule(&ast, "array_literal"));
    }

    #[test]
    fn array_literal_immediately_subscripted() {
        assert!(parses("y = [1, 2, 3][0]\n"));
    }

    // --- Control flow: IF/THEN/ELSE, both body forms ----------------------

    #[test]
    fn if_then_single_statement_no_terminator_needed() {
        assert!(parses("IF x GT 0 THEN y = 1\n"));
    }

    #[test]
    fn if_then_else_single_statement_forms() {
        let ast = parse_idl("IF x GT 0 THEN y = 1 ELSE y = 2\n");
        assert!(contains_rule(&ast, "if_stmt"));
    }

    #[test]
    fn if_then_block_form_with_endif() {
        assert!(parses("IF x GT 0 THEN BEGIN\n y = 1\nENDIF\n"));
    }

    #[test]
    fn if_then_block_form_with_generic_end() {
        assert!(parses("IF x GT 0 THEN BEGIN\n y = 1\nEND\n"));
    }

    #[test]
    fn if_then_else_block_form_with_endelse() {
        assert!(parses(
            "IF x GT 0 THEN BEGIN\n y = 1\nENDIF ELSE BEGIN\n y = 2\nENDELSE\n"
        ));
    }

    // --- FOR ----------------------------------------------------------------

    #[test]
    fn for_loop_two_arg_range() {
        assert!(parses("FOR i = 0, 10 DO y = i\n"));
    }

    #[test]
    fn for_loop_three_arg_range_with_step() {
        assert!(parses("FOR i = 0, 10, 2 DO y = i\n"));
    }

    #[test]
    fn for_loop_block_form() {
        assert!(parses("FOR i = 0, 10 DO BEGIN\n y = i\nENDFOR\n"));
    }

    // --- WHILE ---------------------------------------------------------------

    #[test]
    fn while_loop_single_statement() {
        assert!(parses("WHILE x GT 0 DO x = x - 1\n"));
    }

    #[test]
    fn while_loop_block_form() {
        assert!(parses("WHILE x GT 0 DO BEGIN\n x = x - 1\nENDWHILE\n"));
    }

    // --- REPEAT/UNTIL, including ENDREP-before-UNTIL order ------------------

    #[test]
    fn repeat_until_single_statement() {
        assert!(parses("REPEAT x = x - 1 UNTIL x LE 0\n"));
    }

    #[test]
    fn repeat_until_block_form_endrep_precedes_until() {
        let ast = parse_idl("REPEAT BEGIN\n x = x - 1\nENDREP UNTIL x LE 0\n");
        assert!(contains_rule(&ast, "repeat_stmt"));
    }

    #[test]
    fn repeat_until_reversed_order_is_a_syntax_error() {
        // UNTIL cannot precede ENDREP -- confirms the order is load-bearing,
        // not accidentally accepted either way.
        assert!(try_parse_idl("REPEAT BEGIN\n x = x - 1\nUNTIL x LE 0 ENDREP\n").is_err());
    }

    // --- BREAK / CONTINUE ---------------------------------------------------

    #[test]
    fn break_and_continue_inside_loops() {
        assert!(parses("WHILE x GT 0 DO BEGIN\n BREAK\nENDWHILE\n"));
        assert!(parses("FOR i = 0, 10 DO BEGIN\n CONTINUE\nENDFOR\n"));
    }

    // --- Generic BEGIN...END block ------------------------------------------

    #[test]
    fn generic_begin_end_block_as_a_statement() {
        assert!(parses("BEGIN\n x = 1\n y = 2\nEND\n"));
    }

    // --- PRO / FUNCTION definitions, including RETURN's two forms ----------

    #[test]
    fn pro_def_with_no_params() {
        let ast = parse_idl("PRO simple\n PRINT, 1\nEND\n");
        assert!(contains_rule(&ast, "pro_def"));
    }

    #[test]
    fn pro_def_with_positional_and_keyword_params() {
        let ast = parse_idl("PRO plot_it, x, y, COLOR=color\n PRINT, x\nEND\n");
        assert!(contains_rule(&ast, "pro_def"));
        assert!(contains_rule(&ast, "params"));
    }

    #[test]
    fn pro_def_return_with_no_expression() {
        let ast = parse_idl("PRO early_out, x\n IF x GT 0 THEN RETURN\n PRINT, x\nEND\n");
        assert!(contains_rule(&ast, "return_stmt"));
    }

    #[test]
    fn func_def_with_return_value() {
        let ast = parse_idl("FUNCTION square, x\n RETURN, x * x\nEND\n");
        assert!(contains_rule(&ast, "func_def"));
        let ret = find_rule(&ast, "return_stmt").unwrap();
        assert!(token_values(ret).contains(&"*".to_string()));
    }

    #[test]
    fn func_def_with_keyword_param() {
        assert!(parses(
            "FUNCTION scaled, x, FACTOR=factor\n RETURN, x * factor\nEND\n"
        ));
    }

    // --- Expression precedence cascade: one test per tier boundary --------

    #[test]
    fn logical_is_loosest() {
        let ast = parse_idl("y = a EQ b AND c EQ d\n");
        assert!(contains_rule(&ast, "logical"));
        assert!(contains_rule(&ast, "comparison"));
    }

    #[test]
    fn comparison_binds_looser_than_additive() {
        let ast = parse_idl("y = a + 1 EQ b - 1\n");
        assert!(contains_rule(&ast, "comparison"));
        assert!(contains_rule(&ast, "additive"));
    }

    #[test]
    fn all_six_comparison_operators_parse_at_the_same_tier() {
        for op in ["EQ", "NE", "LE", "LT", "GE", "GT"] {
            let src = format!("y = a {op} b\n");
            assert!(parses(&src), "`{src}` should parse");
        }
    }

    #[test]
    fn all_three_logical_operators_parse() {
        for op in ["AND", "OR", "XOR"] {
            let src = format!("y = a {op} b\n");
            assert!(parses(&src), "`{src}` should parse");
        }
    }

    #[test]
    fn unary_minus_binds_looser_than_multiplicative_and_power() {
        // -a*b == -(a*b): the unary MINUS must wrap the whole multiplicative
        // term, per IDL's own documented precedence (tier 5, same as binary
        // +/-, NOT tighter than * the way Scilab/MATLAB's own unary sits).
        let ast = parse_idl("y = -a*b\n");
        let unary = find_rule(&ast, "unary").expect("unary");
        assert_eq!(token_values(unary), vec!["-", "a", "*", "b"]);
    }

    #[test]
    fn unary_not_parses_at_the_same_tier_as_unary_minus() {
        assert!(parses("y = NOT a\n"));
        let ast = parse_idl("y = NOT a EQ b\n");
        // NOT binds tighter than EQ (tier 5 vs tier 6): `(NOT a) EQ b`.
        assert!(contains_rule(&ast, "comparison"));
        assert!(contains_rule(&ast, "unary"));
    }

    #[test]
    fn power_is_left_associative() {
        // 2^3^2 == (2^3)^2 in real IDL (left-to-right, per the official
        // precedence table), NOT right-associative like Scilab/MATLAB's `^`.
        let ast = parse_idl("y = 2^3^2\n");
        let power = find_rule(&ast, "power").expect("power");
        assert_eq!(token_values(power), vec!["2", "^", "3", "^", "2"]);
    }

    #[test]
    fn multiplicative_includes_both_matrix_product_operators() {
        assert!(parses("y = a # b\n"));
        assert!(parses("y = a ## b\n"));
    }

    #[test]
    fn grouping_parens_parse() {
        assert!(parses("y = (1 + 2) * 3\n"));
        assert!(contains_rule(&parse_idl("y = (1 + 2) * 3\n"), "group"));
    }

    #[test]
    fn syntax_error_is_reported() {
        assert!(try_parse_idl("y = 1 +\n").is_err());
        assert!(try_parse_idl("y = (1 + 2\n").is_err());
    }

    #[test]
    fn strings_single_and_double_quoted() {
        assert!(parses("s = 'hello'\n"));
        assert!(parses("s = \"hello\"\n"));
    }

    // -------------------------------------------------------------------
    // Recursion-depth guard (DoS hardening) -- exercises all six
    // independently-measured shapes documented on `MAX_RULE_DEPTH`.
    // -------------------------------------------------------------------

    fn nested_paren_source(n: usize) -> String {
        format!("y = {}5{}\n", "(".repeat(n), ")".repeat(n))
    }

    fn nested_if_source(n: usize) -> String {
        let mut s = String::new();
        for _ in 0..n {
            s.push_str("IF 1 THEN BEGIN ");
        }
        s.push_str("y = 5");
        for _ in 0..n {
            s.push_str(" ENDIF");
        }
        s.push('\n');
        s
    }

    fn nested_call_source(n: usize) -> String {
        format!("y = {}5{}\n", "f(".repeat(n), ")".repeat(n))
    }

    fn nested_subscript_source(n: usize) -> String {
        format!("y = {}5{}\n", "a[".repeat(n), "]".repeat(n))
    }

    fn unary_prefix_chain_source(n: usize) -> String {
        format!("y = {}5\n", "-".repeat(n))
    }

    fn nested_array_literal_source(n: usize) -> String {
        format!("y = {}5{}\n", "[".repeat(n), "]".repeat(n))
    }

    fn all_shape_sources(n_small: usize, n_if: usize) -> Vec<String> {
        vec![
            nested_paren_source(n_small),
            nested_if_source(n_if),
            nested_call_source(n_small),
            nested_subscript_source(n_small),
            unary_prefix_chain_source(n_small),
            nested_array_literal_source(n_small),
        ]
    }

    /// Deeply-nested input, for every measured shape, must produce a
    /// recoverable error, not overflow the native stack. Parses 5000
    /// levels -- far past `MAX_RULE_DEPTH` -- on a worker thread with a
    /// generous 32 MiB stack, so the *guard* is what stops the recursion,
    /// not the stack running out.
    #[test]
    fn test_deeply_nested_input_returns_error_not_overflow_for_every_shape() {
        let sources = all_shape_sources(5000, 2000);
        let handle = std::thread::Builder::new()
            .name("idl-parser-depth-guard-regression".to_string())
            .stack_size(32 * 1024 * 1024)
            .spawn(move || {
                for src in sources {
                    let result = try_parse_idl(&src);
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
        let sources = all_shape_sources(5000, 2000);
        let handle = std::thread::spawn(move || {
            for src in sources {
                let result = try_parse_idl(&src);
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
        assert!(try_parse_idl(&nested_paren_source(3)).is_ok());
        assert!(try_parse_idl(&nested_if_source(3)).is_ok());
        assert!(try_parse_idl(&nested_call_source(3)).is_ok());
        assert!(try_parse_idl(&nested_subscript_source(3)).is_ok());
        assert!(try_parse_idl(&unary_prefix_chain_source(5)).is_ok());
        assert!(try_parse_idl(&nested_array_literal_source(3)).is_ok());
    }

    /// Input that nests *exactly up to* `MAX_RULE_DEPTH`'s measured
    /// real-input headroom for every shape still parses cleanly, and one
    /// level deeper cleanly trips the cap. These exact boundary counts were
    /// found empirically by binary-searching `try_parse_idl` (the CAPPED
    /// public API, `MAX_RULE_DEPTH = 148`) against increasing nesting counts
    /// for each shape -- see `MAX_RULE_DEPTH`'s own doc comment ("Measured
    /// real-input headroom at `148`"). Without this test, a future change to
    /// the constant could silently move these boundaries without anyone
    /// noticing, mirroring `scilab-parser`/`maple-parser`/`j-parser`'s own
    /// per-shape boundary tests.
    #[test]
    fn test_headroom_boundary_for_every_shape() {
        assert!(try_parse_idl(&nested_paren_source(13)).is_ok());
        assert!(try_parse_idl(&nested_paren_source(14)).is_err());

        assert!(try_parse_idl(&nested_if_source(26)).is_ok());
        assert!(try_parse_idl(&nested_if_source(27)).is_err());

        assert!(try_parse_idl(&nested_call_source(12)).is_ok());
        assert!(try_parse_idl(&nested_call_source(13)).is_err());

        assert!(try_parse_idl(&nested_subscript_source(11)).is_ok());
        assert!(try_parse_idl(&nested_subscript_source(12)).is_err());

        assert!(try_parse_idl(&unary_prefix_chain_source(134)).is_ok());
        assert!(try_parse_idl(&unary_prefix_chain_source(135)).is_err());

        assert!(try_parse_idl(&nested_array_literal_source(12)).is_ok());
        assert!(try_parse_idl(&nested_array_literal_source(13)).is_err());
    }
}
