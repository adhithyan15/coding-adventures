//! Integration tests for the PREP01 engine's one-pass traversal.
//!
//! These use a deliberately minimal test dialect rather than a real language.
//! That is *not* the acceptance proof for slice 1 — MacroOct is, precisely
//! because a dialect written in the same file as the tests can be shaped to
//! agree with whatever the engine does. What these tests are for is the part
//! MacroOct cannot easily reach: the resource bounds and the malformed-input
//! paths, each of which needs a hostile program to trigger it.

use coding_adventures_source_preprocessor::{
    bounds::Bounds,
    diag::PpError,
    dialect::{Dialect, Directive},
    fs::{IncludeRequest, MemoryFs},
    preprocess,
    source_map::FileId,
};
use lexer::token::{Token, TokenType};
use std::cell::Cell;

fn tok(value: &str, line: usize) -> Token {
    Token {
        type_: TokenType::Name,
        value: value.to_string(),
        line,
        column: 1,
        type_name: None,
        flags: None,
        cv: None,
    }
}

/// Turn a compact program description into tokens, one line per `&str`.
fn program(lines: &[&str]) -> Vec<Token> {
    let mut out = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        for word in line.split_whitespace() {
            out.push(tok(word, i + 1));
        }
    }
    out
}

/// A tiny non-C dialect: `@if`/`@else`/`@end`/`@include`/`@define`.
///
/// `evals` counts condition evaluations, which is how the "a skipped group is
/// not evaluated" tests prove a negative.
#[derive(Default)]
struct TestDialect {
    evals: Cell<u32>,
}

impl Dialect for TestDialect {
    fn classify(&self, line: &[Token]) -> Option<Result<Directive, PpError>> {
        let head = line.first()?;
        match head.value.as_str() {
            "@if" => Some(Ok(Directive::If(line[1..].to_vec()))),
            "@else" => Some(Ok(Directive::Else)),
            "@end" => Some(Ok(Directive::EndIf)),
            "@include" => {
                let spelling = line.get(1).map(|t| t.value.clone()).unwrap_or_default();
                Some(Ok(Directive::Include(IncludeRequest {
                    spelling,
                    from: None,
                    system: false,
                })))
            }
            "@define" => Some(Ok(Directive::Define {
                name: line.get(1).map(|t| t.value.clone()).unwrap_or_default(),
                // Object-like only in the test dialect; function-like macros
                // are exercised through MacroOct and the macros module's own
                // suite.
                params: None,
                body: line.get(2..).unwrap_or(&[]).to_vec(),
            })),
            _ => None,
        }
    }

    fn eval_condition(&self, tokens: &[Token]) -> Result<bool, PpError> {
        self.evals.set(self.evals.get() + 1);
        // A numeric literal is its own truth value; anything else -- an
        // identifier that no macro expanded away -- is UNDEFINED and therefore
        // false, as in C and in MacroOct.
        //
        // An earlier version here was `value != "0"`, which made every
        // identifier truthy. That is not merely unrealistic: it would let the
        // "macros are expanded in conditions" tests pass whether or not
        // expansion actually happened, since the unexpanded name is truthy
        // too. The toy dialect has to model undefined-is-false or the property
        // it is used to test becomes unfalsifiable.
        Ok(tokens
            .first()
            .and_then(|t| t.value.parse::<i64>().ok())
            .map(|n| n != 0)
            .unwrap_or(false))
    }

    fn lex(&self, text: &str, _file: FileId) -> Result<Vec<Token>, PpError> {
        Ok(program(&text.lines().collect::<Vec<_>>()))
    }
}

/// Preprocess `lines`, returning the surviving token spellings.
fn run(lines: &[&str], fs: &mut MemoryFs, bounds: Bounds) -> Result<Vec<String>, PpError> {
    run_with(lines, fs, bounds, &TestDialect::default())
}

fn run_with(
    lines: &[&str],
    fs: &mut MemoryFs,
    bounds: Bounds,
    dialect: &TestDialect,
) -> Result<Vec<String>, PpError> {
    let file = fs.insert("<main>", "");
    let out = preprocess(program(lines), file, dialect, fs, bounds)?;
    out.map
        .check_len(out.tokens.len())
        .expect("map must describe the stream it is returned with");
    Ok(out.tokens.into_iter().map(|t| t.value).collect())
}

// ===========================================================================
// Conditionals
// ===========================================================================

#[test]
fn source_without_directives_passes_through_unchanged() {
    let mut fs = MemoryFs::new();
    let out = run(&["fn main ( ) {", "out ( 1 , 200 ) ;", "}"], &mut fs, Bounds::default()).unwrap();
    assert_eq!(out.join(" "), "fn main ( ) { out ( 1 , 200 ) ; }");
}

#[test]
fn a_true_condition_keeps_its_branch_and_a_false_one_drops_it() {
    let mut fs = MemoryFs::new();
    assert_eq!(run(&["@if 1", "kept", "@end"], &mut fs, Bounds::default()).unwrap(), ["kept"]);

    let mut fs = MemoryFs::new();
    assert!(run(&["@if 0", "dropped", "@end"], &mut fs, Bounds::default()).unwrap().is_empty());
}

#[test]
fn else_selects_exactly_one_branch() {
    let mut fs = MemoryFs::new();
    assert_eq!(
        run(&["@if 1", "a", "@else", "b", "@end"], &mut fs, Bounds::default()).unwrap(),
        ["a"]
    );

    let mut fs = MemoryFs::new();
    assert_eq!(
        run(&["@if 0", "a", "@else", "b", "@end"], &mut fs, Bounds::default()).unwrap(),
        ["b"]
    );
}

#[test]
fn conditionals_nest() {
    let mut fs = MemoryFs::new();
    let out = run(
        &["@if 1", "outer", "@if 0", "inner_no", "@else", "inner_yes", "@end", "after", "@end"],
        &mut fs,
        Bounds::default(),
    )
    .unwrap();
    assert_eq!(out, ["outer", "inner_yes", "after"]);
}

#[test]
fn a_nested_conditional_inside_a_skipped_group_is_never_evaluated() {
    // The security-relevant one. If a skipped group still evaluated its
    // nested conditions, every expansion bomb would be reachable from inside
    // `@if 0` -- which is exactly where hostile input would hide it, and where
    // a human reviewer stops reading.
    let dialect = TestDialect::default();
    let mut fs = MemoryFs::new();
    let out = run_with(
        &["@if 0", "@if 1", "buried", "@end", "@end"],
        &mut fs,
        Bounds::default(),
        &dialect,
    )
    .unwrap();

    assert!(out.is_empty());
    assert_eq!(
        dialect.evals.get(),
        1,
        "only the outer condition may be evaluated; the buried one must not be"
    );
}

#[test]
fn an_include_inside_a_skipped_group_is_never_resolved() {
    let mut fs = MemoryFs::new();
    // Deliberately NOT inserted into the filesystem: if the engine tried to
    // resolve it, it would fail, and this test would catch that.
    let out = run(&["@if 0", "@include missing.oct", "@end"], &mut fs, Bounds::default()).unwrap();
    assert!(out.is_empty());
}

// ===========================================================================
// Malformed input — none of these may panic
// ===========================================================================

#[test]
fn an_unterminated_conditional_is_reported_with_its_opening_line() {
    let mut fs = MemoryFs::new();
    let e = run(&["@if 1", "body"], &mut fs, Bounds::default()).unwrap_err();
    assert!(e.to_string().contains("never closed"), "{e}");
    assert_eq!(e.position().map(|p| p.line), Some(1), "should point at the @if, not at EOF");
}

#[test]
fn else_without_an_open_conditional_is_refused() {
    let mut fs = MemoryFs::new();
    let e = run(&["@else", "x"], &mut fs, Bounds::default()).unwrap_err();
    assert!(e.to_string().contains("without an open conditional"), "{e}");
}

#[test]
fn a_conditional_closed_without_being_opened_is_refused() {
    let mut fs = MemoryFs::new();
    let e = run(&["@end"], &mut fs, Bounds::default()).unwrap_err();
    assert!(e.to_string().contains("without being opened"), "{e}");
}

#[test]
fn a_macro_definition_takes_effect_on_later_lines() {
    // Slice 1 refused `@define`; slice 2 implements it. The refusal test that
    // stood here is now obsolete, and this replaces it rather than deleting
    // the coverage.
    let mut fs = MemoryFs::new();
    let out = run(&["@define ANSWER 42", "value = ANSWER ;"], &mut fs, Bounds::default()).unwrap();
    assert_eq!(out.join(" "), "value = 42 ;");
}

#[test]
fn a_definition_does_not_apply_to_lines_before_it() {
    // Definitions are positional, not file-scoped. A program that used a name
    // before defining it must see the name.
    let mut fs = MemoryFs::new();
    let out = run(
        &["before = ANSWER ;", "@define ANSWER 42", "after = ANSWER ;"],
        &mut fs,
        Bounds::default(),
    )
    .unwrap();
    assert_eq!(out.join(" "), "before = ANSWER ; after = 42 ;");
}

#[test]
fn a_definition_inside_a_skipped_group_never_takes_effect() {
    // The guard that matters most in this slice. Before macros existed the
    // skipped-group check on `@define` only avoided a spurious error; now it
    // decides whether a definition the program explicitly skipped gets
    // installed anyway.
    let mut fs = MemoryFs::new();
    let out = run(
        &["@if 0", "@define ANSWER 999", "@end", "value = ANSWER ;"],
        &mut fs,
        Bounds::default(),
    )
    .unwrap();
    assert_eq!(out.join(" "), "value = ANSWER ;", "a skipped @define must not define");
}

#[test]
fn a_macro_is_expanded_inside_a_controlling_expression() {
    // VM-068. Without expansion here, `LED_PORT` evaluates as an undefined
    // name (0), the `@else` branch is taken, and the program compiles — to the
    // wrong thing. Silent wrong-branch selection, not an error.
    //
    // This is PREP01 §7's own worked example, so the spec's canonical
    // illustration was broken until the engine expanded conditions.
    let mut fs = MemoryFs::new();
    let out = run(
        &["@define LED_PORT 1", "@if LED_PORT", "lit", "@else", "dark", "@end"],
        &mut fs,
        Bounds::default(),
    )
    .unwrap();
    assert_eq!(out, ["lit"], "the defined value must drive the branch");
}

#[test]
fn an_undefined_name_in_a_condition_is_still_falsey() {
    // The other half: expansion must not make an UNDEFINED name suddenly
    // truthy. Only a defined macro changes the outcome.
    let mut fs = MemoryFs::new();
    let out = run(
        &["@if NEVER_DEFINED", "lit", "@else", "dark", "@end"],
        &mut fs,
        Bounds::default(),
    )
    .unwrap();
    assert_eq!(out, ["dark"]);
}

#[test]
fn a_macro_from_an_included_file_drives_a_later_condition() {
    // Inclusion and conditional selection interleaving, end to end — the
    // dependency that makes the one-pass design necessary rather than tidy.
    let mut fs = MemoryFs::new();
    fs.insert("ports.oct", "@define LED_PORT 1");
    let out = run(
        &["@include ports.oct", "@if LED_PORT", "lit", "@else", "dark", "@end"],
        &mut fs,
        Bounds::default(),
    )
    .unwrap();
    assert_eq!(out, ["lit"]);
}

#[test]
fn a_condition_expanded_from_a_macro_is_still_depth_bounded() {
    // A macro body can introduce grouping the raw text did not have, so the
    // pre-expansion scan alone does not bound what the dialect finally sees.
    let mut fs = MemoryFs::new();
    let deep = format!("@define DEEP {}1{}", "( ".repeat(40), " )".repeat(40));
    let bounds = Bounds { condition_depth: 8, ..Bounds::default() };
    let e = run(&[&deep, "@if DEEP", "x", "@end"], &mut fs, bounds)
        .expect_err("grouping introduced BY a macro must still be bounded");
    assert!(e.to_string().contains("nested deeper"), "{e}");
}

#[test]
fn a_macro_defined_in_an_included_file_is_visible_afterwards() {
    // Inclusion and definition interleave -- this is the dependency that makes
    // the one-pass design necessary rather than merely tidy.
    let mut fs = MemoryFs::new();
    fs.insert("defs.oct", "@define ANSWER 42");
    let out = run(&["@include defs.oct", "value = ANSWER ;"], &mut fs, Bounds::default()).unwrap();
    assert_eq!(out.join(" "), "value = 42 ;");
}

// ===========================================================================
// Inclusion
// ===========================================================================

#[test]
fn an_included_file_contributes_its_tokens() {
    let mut fs = MemoryFs::new();
    fs.insert("ports.oct", "static LED = 1 ;");
    let out = run(&["@include ports.oct", "main"], &mut fs, Bounds::default()).unwrap();
    assert_eq!(out, ["static", "LED", "=", "1", ";", "main"]);
}

#[test]
fn includes_nest() {
    let mut fs = MemoryFs::new();
    fs.insert("a.oct", "@include b.oct\nfrom_a");
    fs.insert("b.oct", "from_b");
    let out = run(&["@include a.oct"], &mut fs, Bounds::default()).unwrap();
    assert_eq!(out, ["from_b", "from_a"]);
}

#[test]
fn a_direct_include_cycle_is_caught() {
    let mut fs = MemoryFs::new();
    fs.insert("loop.oct", "@include loop.oct");
    let e = run(&["@include loop.oct"], &mut fs, Bounds::default()).unwrap_err();
    assert!(e.to_string().contains("cycle"), "{e}");
}

#[test]
fn an_indirect_include_cycle_is_caught() {
    let mut fs = MemoryFs::new();
    fs.insert("a.oct", "@include b.oct");
    fs.insert("b.oct", "@include a.oct");
    let e = run(&["@include a.oct"], &mut fs, Bounds::default()).unwrap_err();
    assert!(e.to_string().contains("cycle"), "{e}");
}

#[test]
fn the_same_file_may_be_included_twice_when_not_nested() {
    // Cycle detection is STACK-based, not global dedup. Rejecting a second
    // sibling include would break any language with a shared header, so this
    // must be allowed -- which is exactly why the fan-out bound below exists.
    let mut fs = MemoryFs::new();
    fs.insert("shared.oct", "shared");
    let out = run(
        &["@include shared.oct", "@include shared.oct"],
        &mut fs,
        Bounds::default(),
    )
    .unwrap();
    assert_eq!(out, ["shared", "shared"]);
}

// ===========================================================================
// Bounds
// ===========================================================================

#[test]
fn include_depth_is_bounded() {
    let mut fs = MemoryFs::new();
    // A chain deeper than the bound, with distinct names so cycle detection
    // is not what stops it.
    for i in 0..30 {
        fs.insert(format!("d{i}.oct"), format!("@include d{}.oct", i + 1));
    }
    fs.insert("d30.oct", "bottom");

    let bounds = Bounds { include_depth: 5, ..Bounds::default() };
    let e = run(&["@include d0.oct"], &mut fs, bounds).unwrap_err();
    assert!(e.to_string().contains("nested deeper"), "{e}");
}

#[test]
fn a_shallow_acyclic_fan_out_is_stopped_by_the_total_inclusion_bound() {
    // The DAG bomb. Depth 3 and never a repeat on the active stack, so
    // neither the depth bound nor cycle detection fires -- only the cumulative
    // total does. This is the test that justifies that counter existing.
    let mut fs = MemoryFs::new();
    for i in 0..8 {
        let body: String = (0..8).map(|j| format!("@include leaf{i}_{j}.oct\n")).collect();
        fs.insert(format!("mid{i}.oct"), body);
        for j in 0..8 {
            fs.insert(format!("leaf{i}_{j}.oct"), "x");
        }
    }
    let top: Vec<String> = (0..8).map(|i| format!("@include mid{i}.oct")).collect();
    let top_refs: Vec<&str> = top.iter().map(String::as_str).collect();

    let bounds = Bounds { total_inclusions: 20, ..Bounds::default() };
    let e = run(&top_refs, &mut fs, bounds).unwrap_err();
    assert!(e.to_string().contains("total inclusions"), "{e}");
}

#[test]
fn conditional_nesting_is_bounded() {
    let mut fs = MemoryFs::new();
    let mut lines: Vec<String> = (0..50).map(|_| "@if 1".to_string()).collect();
    lines.push("body".into());
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();

    let bounds = Bounds { conditional_depth: 10, ..Bounds::default() };
    let e = run(&refs, &mut fs, bounds).unwrap_err();
    assert!(e.to_string().contains("nesting deeper"), "{e}");
}

#[test]
fn a_controlling_expression_cannot_nest_past_the_bound() {
    // Guards the dialect from a million nested parens: the engine pre-scans
    // BEFORE eval_condition, so a dialect that parses recursively cannot be
    // handed something that would abort the process.
    let mut fs = MemoryFs::new();
    let deep = format!("@if {}1{}", "( ".repeat(40), " )".repeat(40));
    let bounds = Bounds { condition_depth: 8, ..Bounds::default() };
    let e = run(&[&deep, "@end"], &mut fs, bounds).unwrap_err();
    assert!(e.to_string().contains("nested deeper"), "{e}");
}

#[test]
fn the_pre_scan_runs_before_the_dialect_sees_the_expression() {
    // Not just "it errors" -- it must error WITHOUT the dialect being called,
    // otherwise a recursive-descent dialect would already have blown the stack.
    let dialect = TestDialect::default();
    let mut fs = MemoryFs::new();
    let deep = format!("@if {}1{}", "( ".repeat(40), " )".repeat(40));
    let bounds = Bounds { condition_depth: 8, ..Bounds::default() };

    let _ = run_with(&[&deep, "@end"], &mut fs, bounds, &dialect);
    assert_eq!(dialect.evals.get(), 0, "the dialect must never see an over-deep expression");
}

#[test]
fn the_token_budget_is_enforced() {
    let mut fs = MemoryFs::new();
    let bounds = Bounds { tokens_produced: 3, ..Bounds::default() };
    let e = run(&["a b c d e f"], &mut fs, bounds).unwrap_err();
    assert!(e.to_string().contains("more than 3 tokens"), "{e}");
}

#[test]
fn tokens_inside_a_skipped_group_are_still_charged() {
    // The bound must count tokens PRODUCED, not tokens emitted.
    //
    // A security review measured what counting only emitted tokens costs:
    // ~4 MiB of source inside `@if 0` expanded to a 279 MiB working set while
    // the token counter read ZERO, because every token was allocated, retained
    // by its frame, and then discarded without ever being emitted. An attacker
    // does not need their tokens to survive in order to spend your memory on
    // them.
    //
    // So: a budget of 4 must be exhausted by a skipped group, even though the
    // program emits nothing at all.
    let mut fs = MemoryFs::new();
    let bounds = Bounds { tokens_produced: 4, ..Bounds::default() };
    let e = run(&["@if 0", "a b c d e f g h", "@end"], &mut fs, bounds)
        .expect_err("tokens in a skipped group must be charged");
    assert!(e.to_string().contains("more than 4 tokens"), "{e}");
}

#[test]
fn a_budget_cannot_be_widened_past_the_defaults() {
    // `Bounds`'s fields are public, so a caller CAN write `u64::MAX`. The
    // engine clamps on entry, so asking for more than the default silently
    // yields the default rather than honouring the request.
    let mut fs = MemoryFs::new();
    let greedy = Bounds { tokens_produced: u64::MAX, fuel: u64::MAX, ..Bounds::default() };
    // Still succeeds on a small program — clamping is not refusal.
    assert_eq!(run(&["kept"], &mut fs, greedy).unwrap(), ["kept"]);

    // And a genuinely tighter budget is still honoured exactly.
    let mut fs = MemoryFs::new();
    let tight = Bounds { tokens_produced: 2, ..Bounds::default() };
    assert!(run(&["a b c d"], &mut fs, tight).is_err());
}

#[test]
fn a_macro_definition_inside_a_skipped_group_is_inert() {
    // `@define` is refused outright in slice 1, but a skipped group must not
    // reach the refusal -- and more importantly, when slice 2 replaces that
    // arm with real expansion, an unguarded version would make every expansion
    // bomb reachable from inside `@if 0`.
    let mut fs = MemoryFs::new();
    let out = run(&["@if 0", "@define X 1", "@end", "after"], &mut fs, Bounds::default())
        .expect("a skipped @define must not be refused");
    assert_eq!(out, ["after"]);
}

#[test]
fn the_fuel_budget_is_enforced() {
    let mut fs = MemoryFs::new();
    let bounds = Bounds { fuel: 2, ..Bounds::default() };
    let e = run(&["a b c", "d e f", "g h i"], &mut fs, bounds).unwrap_err();
    assert!(e.to_string().contains("budget"), "{e}");
}

#[test]
fn the_total_source_byte_budget_is_enforced() {
    let mut fs = MemoryFs::new();
    fs.insert("big.oct", "x ".repeat(5000));
    let bounds = Bounds { total_source_bytes: 100, ..Bounds::default() };
    let e = run(&["@include big.oct"], &mut fs, bounds).unwrap_err();
    assert!(e.to_string().contains("total source bytes"), "{e}");
}

// ===========================================================================
// Source map
// ===========================================================================

#[test]
fn every_surviving_token_has_provenance() {
    let mut fs = MemoryFs::new();
    fs.insert("inc.oct", "from_include");
    let file = fs.insert("<main>", "");
    let out = preprocess(
        program(&["@if 1", "kept", "@include inc.oct", "@end"]),
        file,
        &TestDialect::default(),
        &mut fs,
        Bounds::default(),
    )
    .unwrap();

    assert_eq!(out.map.len(), out.tokens.len());
    assert!(out.map.check_len(out.tokens.len()).is_ok());
}

#[test]
fn a_token_from_an_included_file_is_attributed_to_that_file() {
    // The whole point of the side table: `line 1` is ambiguous across files,
    // and the map is what disambiguates it.
    let mut fs = MemoryFs::new();
    // Two lines, so the included file has a line 2 of its own.
    let inc = fs.insert("inc.oct", "inc_line1\ninc_line2");
    let main = fs.insert("<main>", "");

    let out = preprocess(
        program(&["@include inc.oct", "main_line2"]),
        main,
        &TestDialect::default(),
        &mut fs,
        Bounds::default(),
    )
    .unwrap();

    let values: Vec<&str> = out.tokens.iter().map(|t| t.value.as_str()).collect();
    assert_eq!(values, ["inc_line1", "inc_line2", "main_line2"]);

    assert_eq!(out.map.locus(0).unwrap().position.file, inc);
    assert_eq!(out.map.locus(1).unwrap().position.file, inc);
    assert_eq!(out.map.locus(2).unwrap().position.file, main);

    // The point of the side table, made concrete: tokens 1 and 2 are adjacent
    // in the output and BOTH report line 2. Only the file distinguishes them,
    // and a bare line number -- which is all `Token` carries -- could not.
    assert_eq!(out.map.locus(1).unwrap().position.line, 2);
    assert_eq!(out.map.locus(2).unwrap().position.line, 2);
    assert_ne!(
        out.map.locus(1).unwrap().position.file,
        out.map.locus(2).unwrap().position.file
    );
}
