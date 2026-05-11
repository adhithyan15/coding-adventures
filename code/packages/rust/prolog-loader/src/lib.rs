//! # prolog-loader — Prolog source text → logic-engine KnowledgeBase.
//!
//! Closes the Rust Prolog text → engine pipeline. Takes a string of
//! Prolog source, runs it through `prolog-lexer` + `prolog-parser`,
//! and walks the resulting `Vec<ProgramItem>` to build a
//! `logic_engine::KnowledgeBase`. Returns the KB and the file's
//! top-level queries.
//!
//! ## Negation-as-failure recognition
//!
//! The parser produces a compound term `'\+'(G)` for `\+ G`. The
//! loader recognizes this pattern and lowers it to
//! `BodyLiteral::Neg(G)`. Every other body goal becomes
//! `BodyLiteral::Pos(_)`.

use logic_core::Term;
use logic_engine::{BodyLiteral, Fact, KnowledgeBase, Rule};
use parser::grammar_parser::GrammarParseError;
use prolog_parser::{collect_clauses_and_queries, ProgramItem};

/// Top-level result of loading a Prolog source string.
#[derive(Debug)]
pub struct LoadedProgram {
    /// The KnowledgeBase populated with every fact and rule from the
    /// source. Ready to call `logic_engine::search` on.
    pub kb: KnowledgeBase,
    /// One conjunction body per `?-` query in the source, in source
    /// order. Each entry is the list of goals that the user wants the
    /// engine to prove.
    pub queries: Vec<Vec<Term>>,
}

/// Errors the loader can produce.
#[derive(Debug)]
pub enum LoaderError {
    /// Parsing the source failed.
    ParseFailed(GrammarParseError),
    /// A rule was declared `head :- .` with no body literals. The
    /// grammar should make this unreachable, but the loader reports
    /// it explicitly rather than silently ignoring.
    EmptyConjunctionBody,
}

impl std::fmt::Display for LoaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoaderError::ParseFailed(e) => write!(f, "parse failed: {}", e.message),
            LoaderError::EmptyConjunctionBody => write!(f, "rule has an empty body"),
        }
    }
}

impl std::error::Error for LoaderError {}

/// Parse a Prolog source string and build a knowledge base. Returns
/// the KB plus the file's top-level queries.
pub fn load_source(src: &str) -> Result<LoadedProgram, LoaderError> {
    let ast = prolog_parser::try_parse_iso_prolog(src).map_err(LoaderError::ParseFailed)?;
    let items = collect_clauses_and_queries(&ast);
    load_program_items(items)
}

/// Lower a pre-parsed `Vec<ProgramItem>` into a `LoadedProgram`.
pub fn load_program_items(items: Vec<ProgramItem>) -> Result<LoadedProgram, LoaderError> {
    let mut kb = KnowledgeBase::new();
    let mut queries: Vec<Vec<Term>> = Vec::new();

    for item in items {
        match item {
            ProgramItem::Fact(t) => {
                kb.add_fact(Fact::certain(t));
            }
            ProgramItem::Rule { head, body } => {
                if body.is_empty() {
                    return Err(LoaderError::EmptyConjunctionBody);
                }
                let body_literals: Vec<BodyLiteral> =
                    body.into_iter().map(naf_or_pos).collect();
                kb.add_rule(Rule::certain(head, body_literals));
            }
            ProgramItem::Query(goals) => {
                queries.push(goals);
            }
        }
    }

    Ok(LoadedProgram { kb, queries })
}

/// Translate a Prolog body goal into a `BodyLiteral`. Recognizes the
/// `'\+'(G)` compound shape as negation-as-failure; every other goal
/// becomes a positive subgoal.
fn naf_or_pos(goal: Term) -> BodyLiteral {
    if let Term::Compound { functor, args } = &goal {
        if functor == "\\+" && args.len() == 1 {
            return BodyLiteral::Neg(args[0].clone());
        }
    }
    BodyLiteral::Pos(goal)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use logic_core::{atom, compound, var, Term};
    use logic_engine::{search, SearchMode, SearchResult};

    #[test]
    fn empty_source_produces_empty_kb_and_no_queries() {
        let LoadedProgram { kb, queries } = load_source("").unwrap();
        assert!(kb.is_all_certain()); // vacuously
        assert!(queries.is_empty());
    }

    #[test]
    fn bare_atom_fact_is_loaded() {
        let LoadedProgram { kb, queries } = load_source("homer.").unwrap();
        assert!(queries.is_empty());
        let r = search(&atom("homer"), &kb, SearchMode::FindFirst);
        assert!(matches!(r, SearchResult::FindFirstResult(Some(_))));
    }

    #[test]
    fn compound_fact_is_loaded() {
        let src = "father(homer, bart).";
        let LoadedProgram { kb, .. } = load_source(src).unwrap();
        let r = search(
            &compound("father", vec![atom("homer"), atom("bart")]),
            &kb,
            SearchMode::FindFirst,
        );
        assert!(matches!(r, SearchResult::FindFirstResult(Some(_))));
    }

    #[test]
    fn rule_with_single_body_goal_loads_and_resolves() {
        let src = "\
            father(homer, bart).\n\
            parent(X, Y) :- father(X, Y).\n\
        ";
        let LoadedProgram { kb, .. } = load_source(src).unwrap();
        // parent(homer, bart) should succeed via the rule.
        let r = search(
            &compound("parent", vec![atom("homer"), atom("bart")]),
            &kb,
            SearchMode::FindFirst,
        );
        assert!(matches!(r, SearchResult::FindFirstResult(Some(_))));
    }

    #[test]
    fn rule_with_conjunction_body_loads_and_resolves() {
        let src = "\
            father(homer, bart).\n\
            father(grandpa, homer).\n\
            parent(X, Y) :- father(X, Y).\n\
            gp(X, Z) :- parent(X, Y), parent(Y, Z).\n\
        ";
        let LoadedProgram { kb, .. } = load_source(src).unwrap();
        let who = var("Who");
        let r = search(
            &compound("gp", vec![atom("grandpa"), Term::Var(who.clone())]),
            &kb,
            SearchMode::FindFirst,
        );
        match r {
            SearchResult::FindFirstResult(Some(subst)) => {
                assert_eq!(subst.walk_var(&who), atom("bart"));
            }
            other => panic!("expected gp(grandpa, bart) to succeed, got {:?}", other),
        }
    }

    #[test]
    fn multiple_queries_are_returned_in_source_order() {
        let src = "\
            a.\n\
            b.\n\
            ?- a.\n\
            ?- b.\n\
        ";
        let LoadedProgram { queries, .. } = load_source(src).unwrap();
        assert_eq!(queries.len(), 2);
        assert_eq!(queries[0], vec![atom("a")]);
        assert_eq!(queries[1], vec![atom("b")]);
    }

    #[test]
    fn family_relations_end_to_end() {
        // The "Simpsons family" worked example used by the LP layer.
        let src = "\
            father(homer, bart).\n\
            father(homer, lisa).\n\
            father(grandpa_abe, homer).\n\
            mother(marge, bart).\n\
            mother(marge, lisa).\n\
            parent(X, Y) :- father(X, Y).\n\
            parent(X, Y) :- mother(X, Y).\n\
            grandparent(X, Z) :- parent(X, Y), parent(Y, Z).\n\
            ?- grandparent(grandpa_abe, Who).\n\
        ";
        let LoadedProgram { kb, queries } = load_source(src).unwrap();
        assert_eq!(queries.len(), 1);

        // Run the (single-goal) query — grandparent(grandpa_abe, Who).
        let goal = &queries[0][0];
        let r = search(goal, &kb, SearchMode::FindFirst);
        assert!(matches!(r, SearchResult::FindFirstResult(Some(_))));
    }

    #[test]
    fn naf_compound_lowers_to_neg_body_literal() {
        // We can't construct NAF through the current parser (the
        // grammar doesn't recognize `\+ G` as a goal); test the
        // loader's lowering by passing a hand-built ProgramItem with
        // a `\+`-compound body.
        let head = atom("q");
        let negation_compound = compound("\\+", vec![atom("p")]);
        let items = vec![
            ProgramItem::Fact(atom("p")),
            ProgramItem::Rule {
                head: head.clone(),
                body: vec![negation_compound],
            },
        ];
        let LoadedProgram { kb, .. } = load_program_items(items).unwrap();
        // q should fail because p is provable (NAF semantics).
        let r = search(&head, &kb, SearchMode::FindFirst);
        assert!(matches!(r, SearchResult::FindFirstResult(None)));
    }

    #[test]
    fn invalid_source_returns_parse_error() {
        // Missing closing paren.
        let res = load_source("father(homer, bart.");
        assert!(matches!(res, Err(LoaderError::ParseFailed(_))));
    }
}
