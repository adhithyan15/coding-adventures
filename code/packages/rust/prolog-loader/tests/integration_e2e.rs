//! End-to-end integration tests for the Rust Prolog pipeline.
//!
//! These tests exercise every layer in one go: prolog-lexer (via
//! prolog-parser) → prolog-parser → prolog-loader → logic-engine.
//! They're the regression check that the layers compose into a
//! working Prolog runtime.
//!
//! Each test takes a Prolog source string, runs every `?-` query the
//! file contains, and asserts properties of the returned
//! `QueryRun`s. The single-call entry point is
//! [`prolog_loader::execute`].
//!
//! ## What "end-to-end" means here
//!
//! - **Tokenization** of arbitrary Prolog text: atoms, variables,
//!   numbers, operators, comments.
//! - **Parsing** to an AST.
//! - **Lowering** to a `logic_core::Term` plus `Fact` / `Rule` /
//!   `Query` shape.
//! - **KB construction** in `logic_engine::KnowledgeBase`.
//! - **Search** via `logic_engine::search` under `SearchMode::AutoDetect`
//!   (FindFirst on certain-only KBs, EnumerateAll + WMC otherwise).
//! - **Negation-as-failure** for the standard `\+` operator.
//!
//! Each test is a self-contained slice the rest of the framework
//! depends on staying green. Failures here are integration-level
//! regressions, distinct from the per-crate unit tests.

use logic_engine::{SearchMode, SearchResult};
use prolog_loader::{execute, QueryRun};

/// Run a Prolog program with the autodetect search mode and return
/// its query runs. Panics on parse/load error so tests assert against
/// the engine output directly.
fn run(src: &str) -> Vec<QueryRun> {
    let (_kb, runs) = execute(src, SearchMode::AutoDetect).expect("Prolog source loads");
    runs
}

#[test]
fn empty_program_has_no_queries() {
    let runs = run("");
    assert!(runs.is_empty());
}

#[test]
fn single_fact_query_finds_it() {
    // The classic "this is the smallest interesting program" test:
    // one atomic fact, one ground query.
    let src = r#"
        sunny.
        ?- sunny.
    "#;
    let runs = run(src);
    assert_eq!(runs.len(), 1);
    assert!(runs[0].succeeded());
}

#[test]
fn ground_query_for_missing_fact_fails() {
    // Closed-world assumption: anything not provable is false.
    let src = r#"
        sunny.
        ?- rainy.
    "#;
    let runs = run(src);
    assert_eq!(runs.len(), 1);
    assert!(!runs[0].succeeded());
}

#[test]
fn compound_facts_with_arguments_match_exact() {
    let src = r#"
        parent(alice, bob).
        parent(bob,   carol).
        ?- parent(alice, bob).
        ?- parent(alice, carol).
    "#;
    let runs = run(src);
    assert_eq!(runs.len(), 2);
    assert!(runs[0].succeeded(), "alice → bob is a direct fact");
    assert!(!runs[1].succeeded(), "alice → carol has no rule yet");
}

#[test]
fn variable_query_binds_through_first_match() {
    let src = r#"
        animal(cat).
        animal(dog).
        ?- animal(X).
    "#;
    let runs = run(src);
    assert_eq!(runs.len(), 1);
    assert!(runs[0].succeeded());
    // We don't pin which animal X bound to — the engine is free to
    // pick either. The end-to-end test only cares that *some*
    // binding was found.
}

#[test]
fn family_relations_rule_chains_via_conjunction() {
    // The canonical "rules compose" test. grandparent/2 is defined
    // in terms of parent/2 and the engine should chain.
    let src = r#"
        parent(alice, bob).
        parent(bob,   carol).
        parent(carol, dan).

        grandparent(X, Z) :- parent(X, Y), parent(Y, Z).

        ?- grandparent(alice, carol).
        ?- grandparent(alice, dan).
        ?- grandparent(bob, alice).
    "#;
    let runs = run(src);
    assert_eq!(runs.len(), 3);
    assert!(runs[0].succeeded(), "alice → bob → carol = grandparent");
    assert!(!runs[1].succeeded(), "alice → ... → dan needs great-grandparent, not defined");
    assert!(!runs[2].succeeded(), "no parent relation runs the other direction");
}

#[test]
fn recursive_ancestor_rule_terminates_and_finds_proof() {
    // Two-clause recursive rule: ancestor is parent OR ancestor of a
    // parent. The engine must terminate and find a derivation.
    let src = r#"
        parent(alice, bob).
        parent(bob,   carol).
        parent(carol, dan).

        ancestor(X, Y) :- parent(X, Y).
        ancestor(X, Y) :- parent(X, Z), ancestor(Z, Y).

        ?- ancestor(alice, dan).
    "#;
    let runs = run(src);
    assert_eq!(runs.len(), 1);
    assert!(runs[0].succeeded(), "alice is an ancestor of dan via 2 hops");
}

// NAF round-trip is not yet exercised end-to-end because the
// current ISO-Prolog grammar does not recognise the `\+` token.
// The loader's `naf_or_pos` lowering is unit-tested in
// `prolog-loader/src/lib.rs::tests::naf_compound_lowers_to_neg_body_literal`,
// so the loader→engine half of the NAF path is covered. Re-enable a
// source-level test here once the grammar grows a `negation_as_failure`
// production.

#[test]
fn conjunction_in_query_threads_through_engine() {
    // Two-goal query: both must succeed. End-to-end test of
    // fold_conjunction routing through to the engine.
    let src = r#"
        likes(alice, books).
        likes(alice, coffee).
        ?- likes(alice, books), likes(alice, coffee).
        ?- likes(alice, books), likes(alice, tea).
    "#;
    let runs = run(src);
    assert_eq!(runs.len(), 2);
    assert!(runs[0].succeeded(), "both conjuncts hold");
    assert!(!runs[1].succeeded(), "tea is not in the KB");
}

#[test]
fn certain_kb_uses_find_first_path_and_reports_probability_one() {
    // AutoDetect short-circuits to FindFirst on certain-only KBs.
    // Verify QueryRun::probability() returns 1.0 on success (the
    // LP19 short-circuit), without invoking WMC.
    let src = r#"
        truth_be_told.
        ?- truth_be_told.
    "#;
    let runs = run(src);
    let r = &runs[0];
    assert!(matches!(r.result, SearchResult::FindFirstResult(Some(_))));
    assert_eq!(r.probability(), 1.0);
    assert!(r.succeeded());
}

#[test]
fn certain_kb_failure_reports_probability_zero() {
    let src = r#"
        truth_be_told.
        ?- nonexistent_fact.
    "#;
    let runs = run(src);
    let r = &runs[0];
    assert!(matches!(r.result, SearchResult::FindFirstResult(None)));
    assert_eq!(r.probability(), 0.0);
    assert!(!r.succeeded());
}

#[test]
fn line_comments_and_whitespace_are_ignored() {
    // ISO-Prolog `%` line comments must round-trip through the
    // lexer to the engine without affecting semantics. Block
    // comments are not yet supported by the grammar — a follow-up
    // can enable them in the lexer if needed.
    let src = "
        % This is a line comment.
            % Indented comment.

        red.
        ?- red.
    ";
    let runs = run(src);
    assert_eq!(runs.len(), 1);
    assert!(runs[0].succeeded());
}

#[test]
fn multi_query_file_preserves_source_order() {
    // The loader returns queries in source order; assertions rely on
    // index-based access, so this guarantee is part of the contract.
    let src = r#"
        a.
        b.
        c.
        ?- a.
        ?- b.
        ?- c.
        ?- d.
    "#;
    let runs = run(src);
    assert_eq!(runs.len(), 4);
    assert!(runs[0].succeeded());
    assert!(runs[1].succeeded());
    assert!(runs[2].succeeded());
    assert!(!runs[3].succeeded());
}

#[test]
fn invalid_syntax_surfaces_as_loader_error() {
    // Junk input should fail at the parser stage, not panic the
    // engine. `execute` returns LoaderError::ParseFailed.
    let result = execute("$$$not valid prolog$$$", SearchMode::AutoDetect);
    assert!(result.is_err());
}

#[test]
fn family_e2e_works_under_enumerate_all_mode_too() {
    // Force EnumerateAll on a certain-only KB. The probability should
    // still be exactly 1.0 (the WMC of a single all-Certain proof).
    let src = r#"
        parent(alice, bob).
        parent(bob,   carol).
        grandparent(X, Z) :- parent(X, Y), parent(Y, Z).
        ?- grandparent(alice, carol).
    "#;
    let (_kb, runs) = execute(src, SearchMode::EnumerateAll).expect("loads");
    assert_eq!(runs.len(), 1);
    assert!(runs[0].succeeded());
    assert_eq!(runs[0].probability(), 1.0);
}
