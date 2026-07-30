// The large `Err` variant is the crate's public `CompileError` enum; boxing it
// would churn the public API and all call sites for no behavior change.
#![allow(clippy::result_large_err)]
//! # adj-lang — surface syntax for the adjudication framework
//!
//! A small probabilistic-logic DSL whose programs lower directly to
//! a `logic-engine` [`KnowledgeBase`]. Designed to be readable by a
//! domain expert (ED physician, M&A lawyer, security researcher,
//! investigative journalist) without a Rust compiler in scope.
//!
//! Dissolves ADJ46 awkwardness items
//! [A10](../../../specs/data/adj46/AWKWARDNESS.md) (rulebook surface
//! is hand-written Rust) and A4 (joint contributions look like
//! ordinary multi-body rules) by making each clause kind a distinct
//! keyword.
//!
//! ## Pipeline
//!
//! ```text
//!  .adj source
//!     │ [GrammarLexer with adj_lang.tokens]
//!     ▼
//!  Vec<Token>
//!     │ [GrammarParser with adj_lang.grammar]
//!     ▼
//!  GrammarASTNode (generic parse tree)
//!     │ [adapter::adapt_program]
//!     ▼
//!  ast::Program (typed)
//!     │ [lower::lower]
//!     ▼
//!  LoweredProgram { kb, queries }
//! ```
//!
//! The lexer and parser are not hand-written: they're driven by
//! `code/grammars/adj_lang.tokens` and `code/grammars/adj_lang.grammar`,
//! compiled into `_lexer_grammar.rs` / `_parser_grammar.rs` by the
//! `grammar-tools` CLI. This crate is therefore conformant with the
//! rest of the repo's grammar-driven language frontends — the same
//! grammars can be reused by any other language port of the
//! adj-lang frontend.
//!
//! See [`code/grammars/adj_lang.tokens`](../../../grammars/adj_lang.tokens)
//! and [`code/grammars/adj_lang.grammar`](../../../grammars/adj_lang.grammar)
//! for the canonical source of truth.

pub mod adapter;
pub mod ast;
pub mod lower;
pub mod resolve;
pub mod statemachine;

mod _lexer_grammar;
mod _parser_grammar;

use lexer::grammar_lexer::GrammarLexer;
use logic_engine::Differential;
use parser::grammar_parser::{GrammarParseError, GrammarParser};

pub use adapter::{adapt_program, AdapterError};
pub use ast::{Annotation, Define, DefineKind, OptDir, Program, RelOp, Statement, Term as AstTerm};
pub use lower::{
    lower, ConstraintSystem, LowerError, LoweredConstraint, LoweredExit, LoweredGuard,
    LoweredProgram, LoweredRangeLookup, LoweredState, LoweredStateMachine, LoweredTransition,
};
pub use resolve::{resolve_imports, ImportError, ImportLimits, ImportProvider};
pub use statemachine::{
    run_state_machine, RunStep, StateMachineOutcome, StateMachineRun, YieldValue,
};

/// Result of compilation. Either the typed program produced by the
/// adapter, or an error from the lexer, parser, adapter, or
/// lowering stage.
#[derive(Debug)]
pub enum CompileError {
    Lex(String),
    Parse(GrammarParseError),
    Adapt(AdapterError),
    Lower(LowerError),
}

/// Recursion-depth cap for the adj-lang [`GrammarParser`] — see
/// [`GrammarParser::with_max_depth`] and
/// [`parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH`] for why the underlying
/// guard exists at all (deep recursion through `parse_rule` can overflow the
/// *native* thread stack — an uncatchable process abort — before this
/// crate's own `Result`-returning entry points ever get a chance to report
/// anything). `adj-lang` is reachable via `adj-lang-cli` on arbitrary
/// `.adj` files, a real attack surface.
///
/// # Three independent recursive shapes
///
/// Unlike most sibling grammars, adj-lang's grammar has three *independent*
/// recursion paths that must all be measured, since a single
/// `MAX_RULE_DEPTH` bounds the parser's internal rule-invocation counter for
/// any of them:
///
/// - **Paren nesting** — `factor = … | LPAREN expr RPAREN`, cascading
///   through `expr → term_expr → factor` (~3 rule-frames per real nesting
///   level).
/// - **Call nesting** — `term = IDENT [ LPAREN (term|NUMBER|VAR) {…} RPAREN
///   ]`, a direct self-recursive call (~1 rule-frame per real nesting
///   level, but each frame is heavier — more local state per call — so it
///   overflows the native stack at a *lower* rule-frame count than the
///   paren shape despite the lighter per-level rule-frame cost).
/// - **Rulebook nesting** — `rulebook_decl = "rulebook" IDENT LBRACE
///   {statement} RBRACE`, and `statement`'s own alternation includes
///   `rulebook_decl`, so `rulebook a { rulebook b { … } }` recurses
///   `statement → rulebook_decl → statement → …` once per nested block
///   (flagged by security review as a shape the first pass of this fix
///   missed).
///
/// Measured (binary search, uncapped parser, on the true default-stack
/// per-test worker thread — no `RUST_MIN_STACK` override and no explicit
/// `Builder::stack_size`, matching what `cargo test` and a production
/// caller both actually get — debug build, adversarial 5000-level input):
/// paren shape safe through 260 rule-frames, crashes at 262; rulebook shape
/// safe through 245, crashes at 250; call shape safe through 124, crashes
/// at 126 — call nesting is the *binding* (lower) floor of the three.
///
/// `MAX_RULE_DEPTH` is set to **90** — about 27% below the binding
/// 124-rule-frame floor (comparable margin to sibling crates' 25-45%
/// convention), independently confirmed not to crash a default-stack
/// thread even thousands of rule-frames past the cap for any of the three
/// shapes (see this crate's tests). Measured real-nesting headroom at 90
/// (capped parser, so no crash risk): paren nesting parses cleanly up to 28
/// levels (29 trips the cap), rulebook nesting up to 44 levels (45 trips
/// the cap), call nesting up to 86 levels (87 trips the cap) — comfortably
/// past any hand-written adj-lang program's real nesting.
const MAX_RULE_DEPTH: usize = 90;

/// Tokenize + parse + adapt: produce a typed [`Program`] from
/// source text.
pub fn parse(src: &str) -> Result<Program, CompileError> {
    let token_grammar = _lexer_grammar::token_grammar();
    let mut grammar_lexer = GrammarLexer::new(src, &token_grammar);
    let tokens = grammar_lexer
        .tokenize()
        .map_err(|e| CompileError::Lex(format!("{e}")))?;
    let parser_grammar = _parser_grammar::parser_grammar();
    let mut grammar_parser =
        GrammarParser::new(tokens, parser_grammar).with_max_depth(MAX_RULE_DEPTH);
    let ast = grammar_parser.parse().map_err(CompileError::Parse)?;
    adapt_program(&ast).map_err(CompileError::Adapt)
}

/// Top-level convenience: source text → lowered program (KB +
/// queries).
pub fn compile(src: &str) -> Result<LoweredProgram, CompileError> {
    let program = parse(src)?;
    lower(&program).map_err(CompileError::Lower)
}

/// Result of compiling a program that may `import` other files: either an
/// import-graph failure (cycle, bound, missing/unparseable file) from the
/// [`resolve`] stage, or a lowering failure from the merged program.
#[derive(Debug)]
pub enum CompileWithImportsError {
    Import(ImportError),
    Lower(LowerError),
}

/// Import-aware compile: resolve the import graph rooted at `root_id` (driven by
/// the injected [`ImportProvider`]), then lower the merged program (MYCIN-2026
/// M3). The library performs **no** filesystem I/O — the provider is the only
/// thing that reads files, so the caller controls the sandbox. See
/// [`resolve::resolve_imports`] for the graph guarantees.
pub fn compile_with_imports(
    root_id: &str,
    provider: &dyn ImportProvider,
    limits: ImportLimits,
) -> Result<LoweredProgram, CompileWithImportsError> {
    let program =
        resolve_imports(root_id, provider, limits).map_err(CompileWithImportsError::Import)?;
    lower(&program).map_err(CompileWithImportsError::Lower)
}

/// Run a **differential** over a lowered program's `? h` query lines:
/// treat the program's queries as the competing hypotheses, rank them by
/// posterior, and return the comparative [`Differential`] decision (argmax +
/// between-hypothesis margin, with a kickback when an open uncertainty
/// could flip the ranking).
///
/// This is the natural reading of a multi-`?` adj-lang program: the queries
/// *are* the differential. A program with a single `?` yields a
/// determinate, single-hypothesis result.
pub fn decide(lowered: &LoweredProgram) -> Differential {
    logic_engine::differential(&lowered.queries, &lowered.kb)
}

/// Source text → differential decision in one step (`compile` then
/// [`decide`]).
pub fn compile_and_decide(src: &str) -> Result<Differential, CompileError> {
    let lowered = compile(src)?;
    Ok(decide(&lowered))
}

#[cfg(test)]
fn nested_paren_source(n: usize) -> String {
    format!("let x = {}1{}\n", "(".repeat(n), ")".repeat(n))
}

#[cfg(test)]
fn nested_term_source(n: usize) -> String {
    format!("observe {}x{}\n", "f(".repeat(n), ")".repeat(n))
}

#[cfg(test)]
fn nested_rulebook_source(n: usize) -> String {
    let mut src = String::new();
    for i in 0..n {
        src.push_str(&format!("rulebook r{i} {{ "));
    }
    src.push_str(&"}".repeat(n));
    src.push('\n');
    src
}

/// Deeply-nested paren input must produce a recoverable error, not overflow
/// the native stack. We parse 5000 levels — far past `MAX_RULE_DEPTH` — on a
/// worker thread with a generous 32 MiB stack, so the *guard* is what stops
/// the recursion, not the stack running out.
#[test]
fn test_deeply_nested_paren_input_returns_error_not_overflow() {
    let handle = std::thread::Builder::new()
        .name("adj-lang-depth-guard-paren-regression".to_string())
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let result = parse(&nested_paren_source(5000));
            assert!(
                result.is_err(),
                "deeply-nested paren input must fail with an error, not parse or crash"
            );
        })
        .expect("failed to spawn worker thread");
    handle
        .join()
        .expect("depth guard must keep the worker thread from crashing");
}

/// Deeply-nested call input (adj-lang's second, independent recursive
/// shape) must also produce a recoverable error, not overflow the native
/// stack.
#[test]
fn test_deeply_nested_term_input_returns_error_not_overflow() {
    let handle = std::thread::Builder::new()
        .name("adj-lang-depth-guard-term-regression".to_string())
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let result = parse(&nested_term_source(5000));
            assert!(
                result.is_err(),
                "deeply-nested call input must fail with an error, not parse or crash"
            );
        })
        .expect("failed to spawn worker thread");
    handle
        .join()
        .expect("depth guard must keep the worker thread from crashing");
}

/// Paren input that nests *exactly up to* `MAX_RULE_DEPTH` still parses
/// cleanly, and one layer deeper cleanly trips the guard. These exact
/// boundary counts (28 legitimate levels) were found empirically by
/// binary-searching against increasing nesting counts at the production
/// cap — see `MAX_RULE_DEPTH`'s doc comment.
#[test]
fn test_paren_nesting_up_to_cap_still_parses() {
    assert!(
        parse(&nested_paren_source(28)).is_ok(),
        "28 levels must stay under the cap"
    );
    assert!(
        parse(&nested_paren_source(29)).is_err(),
        "one nesting level past the cap's measured limit must fail"
    );
}

/// Call input that nests *exactly up to* `MAX_RULE_DEPTH` still parses
/// cleanly, and one layer deeper cleanly trips the guard (86 legitimate
/// levels, empirically measured — see `MAX_RULE_DEPTH`'s doc comment).
#[test]
fn test_term_nesting_up_to_cap_still_parses() {
    assert!(
        parse(&nested_term_source(86)).is_ok(),
        "86 levels must stay under the cap"
    );
    assert!(
        parse(&nested_term_source(87)).is_err(),
        "one nesting level past the cap's measured limit must fail"
    );
}

/// A caller relying on `MAX_RULE_DEPTH` must have the guard trip *before*
/// the native stack overflows on a default-stack thread — otherwise a
/// production caller (e.g. `adj-lang-cli`, or `cargo test`'s own per-test
/// thread) would still crash. We parse far-too-deep paren input on a
/// worker thread with **no** `stack_size` override (the same default a
/// thread gets in this environment, unmodified by any `RUST_MIN_STACK`
/// override). A clean `Err` (not a `join()`
/// failure from a crashed thread) proves `MAX_RULE_DEPTH` sits safely below
/// the native overflow point on the default stack.
#[test]
fn test_opt_in_cap_trips_before_paren_overflow_on_default_stack() {
    let handle = std::thread::spawn(|| {
        let result = parse(&nested_paren_source(5000));
        assert!(
            result.is_err(),
            "deeply-nested paren input must error, not crash"
        );
    });
    handle
        .join()
        .expect("MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack");
}

/// Same as [`test_opt_in_cap_trips_before_paren_overflow_on_default_stack`]
/// but for the call-nesting shape — the *binding* (lower) native-stack
/// floor of the three, per `MAX_RULE_DEPTH`'s doc comment.
#[test]
fn test_opt_in_cap_trips_before_term_overflow_on_default_stack() {
    let handle = std::thread::spawn(|| {
        let result = parse(&nested_term_source(5000));
        assert!(
            result.is_err(),
            "deeply-nested call input must error, not crash"
        );
    });
    handle
        .join()
        .expect("MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack");
}

/// Deeply-nested `rulebook { rulebook { … } }` input (adj-lang's third,
/// independent recursive shape — `statement → rulebook_decl → statement`,
/// flagged by security review) must also produce a recoverable error, not
/// overflow the native stack.
#[test]
fn test_deeply_nested_rulebook_input_returns_error_not_overflow() {
    let handle = std::thread::Builder::new()
        .name("adj-lang-depth-guard-rulebook-regression".to_string())
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let result = parse(&nested_rulebook_source(5000));
            assert!(
                result.is_err(),
                "deeply-nested rulebook input must fail with an error, not parse or crash"
            );
        })
        .expect("failed to spawn worker thread");
    handle
        .join()
        .expect("depth guard must keep the worker thread from crashing");
}

/// Rulebook-nesting input that nests *exactly up to* `MAX_RULE_DEPTH` still
/// parses cleanly, and one layer deeper cleanly trips the guard (44
/// legitimate levels, empirically measured — see `MAX_RULE_DEPTH`'s doc
/// comment).
#[test]
fn test_rulebook_nesting_up_to_cap_still_parses() {
    assert!(
        parse(&nested_rulebook_source(44)).is_ok(),
        "44 levels must stay under the cap"
    );
    assert!(
        parse(&nested_rulebook_source(45)).is_err(),
        "one nesting level past the cap's measured limit must fail"
    );
}

/// Same as [`test_opt_in_cap_trips_before_paren_overflow_on_default_stack`]
/// but for the rulebook-nesting shape.
#[test]
fn test_opt_in_cap_trips_before_rulebook_overflow_on_default_stack() {
    let handle = std::thread::spawn(|| {
        let result = parse(&nested_rulebook_source(5000));
        assert!(
            result.is_err(),
            "deeply-nested rulebook input must error, not crash"
        );
    });
    handle
        .join()
        .expect("MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack");
}
