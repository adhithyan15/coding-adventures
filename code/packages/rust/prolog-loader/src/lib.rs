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
use logic_engine::{search, BodyLiteral, Fact, KnowledgeBase, Rule, SearchMode, SearchResult};
use parser::grammar_parser::GrammarParseError;
use prolog_parser::{collect_clauses_and_queries, ProgramItem};

pub mod problog;
pub use problog::ProblogProgram;

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
    /// A ProbLog probabilistic clause `p :: ...` had a probability
    /// outside `[0, 1]`. The grammar accepts any numeric literal so
    /// the loader is responsible for the range check.
    ProbabilityOutOfRange { value: f64 },
}

impl std::fmt::Display for LoaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoaderError::ParseFailed(e) => write!(f, "parse failed: {}", e.message),
            LoaderError::EmptyConjunctionBody => write!(f, "rule has an empty body"),
            LoaderError::ProbabilityOutOfRange { value } => {
                write!(f, "probability {value} is outside [0, 1]")
            }
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
            ProgramItem::ProbabilisticFact { term, probability } => {
                check_probability(probability)?;
                kb.add_fact(Fact::with_probability(term, probability));
            }
            ProgramItem::ProbabilisticRule {
                head,
                body,
                probability,
            } => {
                check_probability(probability)?;
                if body.is_empty() {
                    return Err(LoaderError::EmptyConjunctionBody);
                }
                let body_literals: Vec<BodyLiteral> =
                    body.into_iter().map(naf_or_pos).collect();
                kb.add_rule(Rule::with_probability(head, body_literals, probability));
            }
        }
    }

    Ok(LoadedProgram { kb, queries })
}

// ---------------------------------------------------------------------------
// End-to-end runner — text → KB → answers
// ---------------------------------------------------------------------------

/// One query and the engine's answer for it. Kept as a separate type
/// so callers can pattern-match on the engine result and pretty-print
/// it however they like — the loader stays neutral about output format.
#[derive(Debug, Clone)]
pub struct QueryRun {
    /// The original goal list as it appeared in the source (e.g.,
    /// `[parent(X, Y), parent(Y, Z)]` for `?- parent(X, Y), parent(Y, Z).`).
    pub goals: Vec<Term>,
    /// The single term the engine actually searched for. For a
    /// one-goal query this equals `goals[0]`. For a conjunction it
    /// equals the synthetic head atom (`__query_N`) of the rule
    /// added to the KB at execute time — see
    /// [`run_all_queries`] for the rewrite.
    pub searched: Term,
    /// What the engine returned.
    pub result: SearchResult,
}

impl QueryRun {
    /// True iff the engine found at least one proof. Works for both
    /// `FindFirstResult(Some)` and any `EnumerateAllResult` whose DAG
    /// has at least one derivation (`probability > 0.0`).
    pub fn succeeded(&self) -> bool {
        match &self.result {
            SearchResult::FindFirstResult(opt) => opt.is_some(),
            SearchResult::EnumerateAllResult { probability, .. } => *probability > 0.0,
        }
    }

    /// Probability of the query under the engine's WMC, when an
    /// `EnumerateAllResult` is available. Returns `1.0` if the engine
    /// short-circuited via `FindFirst` and succeeded, `0.0` if it
    /// failed. The 1/0 mapping matches LP19's "every certain-clause
    /// proof is fully confident" rule.
    pub fn probability(&self) -> f64 {
        match &self.result {
            SearchResult::FindFirstResult(opt) => {
                if opt.is_some() {
                    1.0
                } else {
                    0.0
                }
            }
            SearchResult::EnumerateAllResult { probability, .. } => *probability,
        }
    }
}

/// Run every query in `loaded.queries` against `loaded.kb`. The
/// `mode` argument is forwarded to `logic_engine::search`; deployments
/// that don't care should pass `SearchMode::AutoDetect`, which uses
/// the cheap `FindFirst` path on certain-only KBs and falls back to
/// `EnumerateAll` + WMC when any clause is probabilistic.
///
/// ## How multi-goal queries become single-term searches
///
/// `logic_engine::search` takes one term and matches it against KB
/// clauses — it has no built-in handling for the `,/2` conjunction
/// operator. The runner translates each multi-goal query
/// `?- g1, g2, ..., gn.` into a *synthetic rule* added to the KB:
///
/// ```text
///     __query_N :- g1, g2, ..., gn.
/// ```
///
/// then searches the atom `__query_N`. This is the canonical Prolog
/// rewrite (the same trick a top-level uses internally) and routes
/// conjunction handling through the engine's existing body-literal
/// machinery. Single-goal queries skip the rewrite and search their
/// goal directly.
///
/// The synthetic rules accumulate in the KB across queries; each has
/// a unique `__query_N` head so they don't shadow each other. The
/// name `__query_N` starts with `_` which the ISO-Prolog grammar
/// tokenises as a **variable**, not an atom — meaning user source
/// code cannot define a clashing predicate of the same name.
pub fn run_all_queries(loaded: &mut LoadedProgram, mode: SearchMode) -> Vec<QueryRun> {
    let mut runs = Vec::with_capacity(loaded.queries.len());
    for (i, goals) in loaded.queries.clone().into_iter().enumerate() {
        let searched = match goals.len() {
            0 => Term::Atom("true".to_string()),
            1 => goals[0].clone(),
            _ => {
                // Mint a fresh synthetic head and install the rule.
                let head_name = format!("__query_{i}");
                let head = Term::Atom(head_name);
                let body: Vec<BodyLiteral> = goals.iter().cloned().map(naf_or_pos).collect();
                loaded.kb.add_rule(Rule::certain(head.clone(), body));
                head
            }
        };
        let result = search(&searched, &loaded.kb, mode);
        runs.push(QueryRun {
            goals,
            searched,
            result,
        });
    }
    runs
}

/// One-call end-to-end: parse a Prolog source string, build the KB,
/// and run every `?-` query the file contains. Returns the KB (so
/// callers can ask follow-up questions) and a [`QueryRun`] per query.
///
/// This is the canonical entry point for end-to-end tests and for any
/// caller that wants "give me a string of Prolog and the answers in
/// one call." Errors surface as [`LoaderError`] just like
/// [`load_source`].
///
/// The returned `KnowledgeBase` includes the synthetic
/// `__query_N` rules that the runner installed for multi-goal queries
/// (see [`run_all_queries`]). Callers who want to ask follow-up
/// questions can ignore those — they don't collide with user predicates.
pub fn execute(src: &str, mode: SearchMode) -> Result<(KnowledgeBase, Vec<QueryRun>), LoaderError> {
    let mut loaded = load_source(src)?;
    let runs = run_all_queries(&mut loaded, mode);
    Ok((loaded.kb, runs))
}

/// Range-check a parsed probability literal. `[0, 1]` is the
/// inclusive Bernoulli range; NaN and out-of-range values surface as
/// `LoaderError::ProbabilityOutOfRange` so the source pinpoints the
/// problem rather than the engine seeing a nonsense weight later.
fn check_probability(p: f64) -> Result<(), LoaderError> {
    if p.is_nan() || !(0.0..=1.0).contains(&p) {
        return Err(LoaderError::ProbabilityOutOfRange { value: p });
    }
    Ok(())
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
