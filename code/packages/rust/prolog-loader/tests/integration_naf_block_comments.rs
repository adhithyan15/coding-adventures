//! Source-level integration tests for the two ISO-Prolog grammar
//! features added in this PR:
//!
//! - **Negation-as-failure (`\+`)** in queries and in rule bodies.
//! - **Block comments (`/* ... */`)**, including multi-line.
//!
//! Both round-trip through the regenerated lexer + parser + loader +
//! engine. Failures here mean a grammar-level regression that the
//! per-crate unit tests in `prolog-parser` / `prolog-lexer` /
//! `prolog-loader` wouldn't catch.

use logic_core::Term;
use logic_engine::{search, BodyLiteral, KnowledgeBase, Rule, SearchMode, SearchResult};
use prolog_loader::{load_source, LoadedProgram};

/// Run one Prolog source string, executing every `?-` query under
/// `AutoDetect`. Returns the success/fail vector.
///
/// **Always wraps each query in a synthetic rule** even when it has a
/// single goal. This is the canonical Prolog top-level rewrite —
/// without it, a single-goal `\+ G` would hit the engine as a raw
/// `\+(G)` compound (no fact / rule head matches) rather than as a
/// `BodyLiteral::Neg(G)` literal. Wrapping always lets the loader's
/// `naf_or_pos` lowering kick in.
fn run_queries(src: &str) -> Vec<bool> {
    let LoadedProgram { mut kb, queries } = load_source(src).expect("loads");
    let mut outcomes = Vec::new();
    for (i, goals) in queries.into_iter().enumerate() {
        let head = Term::Atom(format!("__query_{i}"));
        let body: Vec<BodyLiteral> = goals
            .iter()
            .cloned()
            .map(|g| {
                // Inline `naf_or_pos`: `\+(G)` → BodyLiteral::Neg(G);
                // everything else → BodyLiteral::Pos.
                if let Term::Compound { functor, args } = &g {
                    if functor == "\\+" && args.len() == 1 {
                        return BodyLiteral::Neg(args[0].clone());
                    }
                }
                BodyLiteral::Pos(g)
            })
            .collect();
        kb.add_rule(Rule::certain(head.clone(), body));
        let r = search(&head, &kb, SearchMode::AutoDetect);
        let succeeded = match r {
            SearchResult::FindFirstResult(opt) => opt.is_some(),
            SearchResult::EnumerateAllResult { probability, .. } => probability > 0.0,
        };
        outcomes.push(succeeded);
    }
    outcomes
}

// Silence an unused-import warning when KnowledgeBase is referenced
// only through the LoadedProgram type alias above.
#[allow(dead_code)]
fn _force_use_kb_type() {
    let _ = KnowledgeBase::new();
}

#[test]
fn naf_in_query_succeeds_when_inner_goal_is_unprovable() {
    let src = r#"
        vegan(alice).
        ?- \+ vegan(bob).
        ?- \+ vegan(alice).
    "#;
    let s = run_queries(src);
    assert_eq!(s, vec![true, false]);
}

#[test]
fn naf_in_rule_body_round_trips() {
    let src = r#"
        vegan(alice).
        not_vegan(X) :- \+ vegan(X).
        ?- not_vegan(bob).
        ?- not_vegan(alice).
    "#;
    let s = run_queries(src);
    assert_eq!(s, vec![true, false]);
}

#[test]
fn naf_chained_with_conjunction_in_query() {
    let src = r#"
        adult(alice).
        adult(bob).
        vegan(alice).
        ?- adult(bob), \+ vegan(bob).
    "#;
    let s = run_queries(src);
    assert_eq!(s, vec![true]);
}

#[test]
fn naf_without_space_before_atom() {
    // ISO Prolog allows tight juxtaposition: `\+vegan(bob)` must
    // tokenise the same as `\+ vegan(bob)`.
    let src = r#"
        vegan(alice).
        ?- \+vegan(bob).
    "#;
    let s = run_queries(src);
    assert_eq!(s, vec![true]);
}

#[test]
fn single_line_block_comment_is_ignored() {
    let src = r#"
        /* a single-line block comment */ red.
        ?- red.
    "#;
    let s = run_queries(src);
    assert_eq!(s, vec![true]);
}

#[test]
fn multi_line_block_comment_is_ignored() {
    let src = "
        /* this comment
           spans several lines
           and contains * and / characters */
        red.
        ?- red.
    ";
    let s = run_queries(src);
    assert_eq!(s, vec![true]);
}

#[test]
fn block_comment_inside_a_clause_works() {
    // The comment is mid-clause; the lexer must skip it transparently.
    let src = "
        parent /* :: relation :: */ (alice, bob).
        ?- parent(alice, bob).
    ";
    let s = run_queries(src);
    assert_eq!(s, vec![true]);
}

#[test]
fn line_and_block_comments_coexist() {
    let src = r#"
        % line one
        /* block one */
        % line two
        /* block
           two */
        green.
        ?- green.
    "#;
    let s = run_queries(src);
    assert_eq!(s, vec![true]);
}

#[test]
fn block_comment_terminator_is_first_star_slash() {
    // The non-greedy regex must stop at the FIRST `*/`, not the last
    // one — otherwise everything between two unrelated comments
    // would be swallowed.
    let src = r#"
        /* first */ alpha. /* second */
        ?- alpha.
    "#;
    let s = run_queries(src);
    assert_eq!(s, vec![true]);
}
