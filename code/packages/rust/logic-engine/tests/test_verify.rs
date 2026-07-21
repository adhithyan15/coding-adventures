//! RS-4 PR-D2 — `verify`: re-execute a proof instead of believing it.
//!
//! Every earlier PR in this arc made the audit trail *richer*. None of them
//! made it *checkable*. A richer trail that nobody can re-run is still
//! testimony — the engine asserting what it did, in a format that a confidently
//! wrong system produces just as fluently.
//!
//! These tests pin the property that closes that gap: **a trail only passes
//! when the work redoes itself.** They are deliberately weighted toward the
//! failure directions, because a verifier that cannot fail is indistinguishable
//! from no verifier at all, and it is the one component whose bugs are
//! invisible — a broken checker reports success.

use logic_core::{atom, compound, var, Substitution, Term};
use logic_engine::{
    enumerate_all, verify_proof, verify_quote, BodyLiteral, DerivationOrigin, Fact, FactId,
    LogicFailure, LogicStatus, MemorySnapshots, NoSnapshots, Proof, ProofStep, Provenance,
    QuoteMiss, QuoteStatus, Rule, RuleId, TrustTier, UnverifiedReason,
};

/// The document every quoted fact in this file is grounded in. Short on
/// purpose: byte offsets stay readable, so a reader can check the tests' own
/// arithmetic by eye.
const DOC: &str = "Aspirin inhibits cyclooxygenase. Warfarin inhibits VKORC1.";

fn v(name: &str) -> Term {
    Term::Var(var(name))
}

/// A `Provenance` quoting `DOC` at a real span.
fn quoting(offset: usize, len: usize) -> Provenance {
    Provenance::new("Pharmacology 101".to_string(), None, TrustTier::Authoritative)
        .with_quote_in(DOC, offset, len)
        .expect("the fixture span must be a real slice of DOC")
}

fn snapshots() -> MemorySnapshots {
    let mut s = MemorySnapshots::new();
    s.insert(DOC.as_bytes().to_vec());
    s
}

// ---------------------------------------------------------------------------
// (1) The happy path — and what "happy" is allowed to mean.
// ---------------------------------------------------------------------------

#[test]
fn a_fact_step_re_executes_and_its_quote_is_confirmed_at_the_recorded_offset() {
    let mut kb = logic_engine::KnowledgeBase::new();
    // "Aspirin inhibits cyclooxygenase." — bytes 0..32.
    kb.add_fact(
        Fact::certain(compound(
            "inhibits",
            vec![atom("aspirin"), atom("cyclooxygenase")],
        ))
        .with_provenance(quoting(0, 32)),
    );

    let dag = enumerate_all(&compound("inhibits", vec![atom("aspirin"), v("What")]), &kb);
    let report = verify_proof(&dag.proofs[0], &kb, &snapshots());

    assert!(report.passed(), "a sound, quoted step must pass");
    assert!(
        report.fully_verified(),
        "and with a pinned snapshot it must be VERIFIED, not merely unrefuted"
    );
    assert_eq!(
        report.steps[0].quote,
        QuoteStatus::Verified {
            byte_offset: 0,
            byte_len: 32
        },
        "the span's length is surfaced so a reviewer can judge how load-bearing it is"
    );
}

#[test]
fn a_rule_step_re_unifies_its_head() {
    let mut kb = logic_engine::KnowledgeBase::new();
    kb.add_fact(Fact::certain(compound(
        "inhibits",
        vec![atom("aspirin"), atom("cyclooxygenase")],
    )));
    kb.add_rule(Rule::certain(
        compound("acts_on", vec![v("D"), v("T")]),
        vec![BodyLiteral::Pos(compound("inhibits", vec![v("D"), v("T")]))],
    ));

    let dag = enumerate_all(&compound("acts_on", vec![atom("aspirin"), v("T")]), &kb);
    let report = verify_proof(&dag.proofs[0], &kb, &NoSnapshots);

    assert_eq!(report.steps[0].kind, "FromRule");
    assert_eq!(report.steps[0].logic, LogicStatus::ReChecked);
    assert!(report.passed());
}

// ---------------------------------------------------------------------------
// (2) Unverified is not verified — the fail-open direction.
// ---------------------------------------------------------------------------

#[test]
fn an_unmigrated_quote_never_reads_as_verified() {
    let mut kb = logic_engine::KnowledgeBase::new();
    kb.add_fact(
        Fact::certain(compound("inhibits", vec![atom("warfarin"), atom("vkorc1")]))
            .with_provenance(Provenance::cited("Pharmacology 101")),
    );

    let dag = enumerate_all(&compound("inhibits", vec![atom("warfarin"), v("T")]), &kb);
    let report = verify_proof(&dag.proofs[0], &kb, &snapshots());

    assert_eq!(
        report.steps[0].quote,
        QuoteStatus::Unverified(UnverifiedReason::Unmigrated)
    );
    // It does not FAIL — the inference is sound and we are not pretending
    // otherwise — but it must not be counted as checked.
    assert!(report.passed());
    assert!(
        !report.fully_verified(),
        "an unmigrated library must not be able to report a clean bill of health"
    );
}

#[test]
fn a_quote_with_no_snapshot_is_unverified_not_searched_for() {
    let prov = Provenance::new("Pharmacology 101".to_string(), None, TrustTier::Authoritative)
        .with_quote("Aspirin inhibits cyclooxygenase.", Some(0), None);
    assert_eq!(
        verify_quote(&prov, &snapshots()),
        QuoteStatus::Unverified(UnverifiedReason::NoSnapshotPinned)
    );
}

#[test]
fn a_quote_with_no_byte_offset_is_unverified_rather_than_matched_anywhere() {
    // The text below really does occur in DOC. An unanchored verifier would
    // report Verified. That is precisely the behaviour §E.5 forbids: it would
    // confirm the words exist somewhere, not that they support this clause.
    let mut snaps = MemorySnapshots::new();
    let hash = snaps.insert(DOC.as_bytes().to_vec());
    let prov = Provenance::new("Pharmacology 101".to_string(), None, TrustTier::Authoritative)
        .with_quote("Warfarin inhibits VKORC1.", None, Some(hash));

    assert_eq!(
        verify_quote(&prov, &snaps),
        QuoteStatus::Unverified(UnverifiedReason::NoByteOffset)
    );
}

#[test]
fn a_missing_snapshot_document_is_unverified_not_a_pass() {
    let prov = quoting(0, 32);
    assert_eq!(
        verify_quote(&prov, &NoSnapshots),
        QuoteStatus::Unverified(UnverifiedReason::SnapshotUnavailable)
    );
}

// ---------------------------------------------------------------------------
// (3) QuoteMissing — the fabricated-citation direction. These MUST fail.
// ---------------------------------------------------------------------------

#[test]
fn a_quote_that_is_not_at_its_offset_is_quote_missing() {
    let mut snaps = MemorySnapshots::new();
    let hash = snaps.insert(DOC.as_bytes().to_vec());
    // Real text, real document, in-bounds range — but the wrong anchor: the
    // sentence starts at 0, not 1. Off by a single byte and the check fails,
    // which is what "anchored" has to mean to be worth anything.
    let prov = Provenance::new("Pharmacology 101".to_string(), None, TrustTier::Authoritative)
        .with_quote("Aspirin inhibits cyclooxygenase.", Some(1), Some(hash));

    assert_eq!(
        verify_quote(&prov, &snaps),
        QuoteStatus::QuoteMissing(QuoteMiss::TextDiffers {
            byte_offset: 1,
            byte_len: 32
        })
    );
}

#[test]
fn a_span_running_past_the_end_reports_out_of_bounds_and_does_not_panic() {
    let mut snaps = MemorySnapshots::new();
    let hash = snaps.insert(DOC.as_bytes().to_vec());
    let prov = Provenance::new("Pharmacology 101".to_string(), None, TrustTier::Authoritative)
        .with_quote("Warfarin inhibits VKORC1.", Some(DOC.len() - 3), Some(hash));

    assert_eq!(
        verify_quote(&prov, &snaps),
        QuoteStatus::QuoteMissing(QuoteMiss::RangeOutOfBounds {
            byte_offset: DOC.len() - 3,
            byte_len: 25,
            snapshot_len: DOC.len(),
        })
    );
}

#[test]
fn an_offset_inside_a_utf8_character_reports_a_boundary_miss_and_does_not_panic() {
    // "café" — 'é' is two bytes, so byte 4 is its interior. A naive `&doc[4..]`
    // would panic here, and a verifier that panics on hostile input is a
    // denial-of-service handed to whoever writes the trail.
    let doc = "café au lait";
    assert!(!doc.is_char_boundary(4), "fixture must straddle a character");

    let mut snaps = MemorySnapshots::new();
    let hash = snaps.insert(doc.as_bytes().to_vec());
    let prov = Provenance::new("Menu".to_string(), None, TrustTier::Authoritative)
        .with_quote(" au", Some(4), Some(hash));

    assert_eq!(
        verify_quote(&prov, &snaps),
        QuoteStatus::QuoteMissing(QuoteMiss::NotACharBoundary {
            byte_offset: 4,
            byte_len: 3
        })
    );
}

#[test]
fn a_quote_missing_verdict_fails_the_step_and_the_trace() {
    let mut kb = logic_engine::KnowledgeBase::new();
    let mut snaps = MemorySnapshots::new();
    let hash = snaps.insert(DOC.as_bytes().to_vec());
    // A sentence nobody wrote, pinned to a document that does not contain it:
    // a perfectly valid derivation resting on an invented fact.
    kb.add_fact(
        Fact::certain(compound("inhibits", vec![atom("aspirin"), atom("vkorc1")])).with_provenance(
            Provenance::new("Pharmacology 101".to_string(), None, TrustTier::Authoritative)
                .with_quote("Aspirin inhibits VKORC1.", Some(0), Some(hash)),
        ),
    );

    let dag = enumerate_all(&compound("inhibits", vec![atom("aspirin"), v("T")]), &kb);
    let report = verify_proof(&dag.proofs[0], &kb, &snaps);

    assert_eq!(report.steps[0].logic, LogicStatus::ReChecked, "the LOGIC is fine");
    assert!(
        !report.passed(),
        "but the trail is fabricated, so the trace must not pass"
    );
    assert_eq!(report.first_failure().map(|s| s.index), Some(0));
}

// ---------------------------------------------------------------------------
// (4) Negation-as-failure — re-run the absence, and never read a truncated
//     search as one.
// ---------------------------------------------------------------------------

fn negation_step(goal: Term) -> Proof {
    Proof {
        bindings: Substitution::empty(),
        steps: vec![ProofStep {
            goal: goal.clone(),
            origin: DerivationOrigin::FromNegation { goal },
            depth: 0,
        }],
        via_facts: vec![],
        via_rules: vec![],
        posterior_logit: None,
        posterior_probability: None,
    }
}

#[test]
fn an_absence_re_checks_by_re_running_the_subgoal() {
    let kb = logic_engine::KnowledgeBase::new();
    let proof = negation_step(compound("contraindicated", vec![atom("aspirin")]));
    let report = verify_proof(&proof, &kb, &NoSnapshots);

    assert_eq!(report.steps[0].logic, LogicStatus::ReChecked);
    assert_eq!(
        report.steps[0].quote,
        QuoteStatus::NotApplicable,
        "an absence has no sentence in any document — that is not 'unverified'"
    );
    assert!(report.passed());
    assert!(
        !report.fully_verified(),
        "an absence is re-checkable but grounds nothing in any document; a trail \
         made only of absences has confirmed zero bytes and must not claim the \
         strongest verdict"
    );
}

#[test]
fn an_absence_that_has_since_become_provable_fails() {
    let mut kb = logic_engine::KnowledgeBase::new();
    let goal = compound("contraindicated", vec![atom("aspirin")]);
    // The world changed under the trail: the contraindication is now known.
    kb.add_fact(Fact::certain(goal.clone()));

    let report = verify_proof(&negation_step(goal), &kb, &NoSnapshots);
    assert_eq!(
        report.steps[0].logic,
        LogicStatus::Failed(LogicFailure::NegatedGoalProvable)
    );
    assert!(!report.passed());
}

#[test]
fn a_truncated_search_is_a_failure_not_an_absence() {
    // `p(X) :- p(X)` has no base case, so the resolver hits its depth cap and
    // gives up. The proof set is empty — but for the wrong reason. Reading "I
    // stopped looking" as "there is none" is exactly the accounting failure the
    // audit trail exists to prevent, and negation is where it does damage:
    // every rule guarded by `not p(...)` would silently fire.
    let mut kb = logic_engine::KnowledgeBase::new();
    kb.add_rule(Rule::certain(
        compound("loops", vec![v("X")]),
        vec![BodyLiteral::Pos(compound("loops", vec![v("X")]))],
    ));

    let report = verify_proof(
        &negation_step(compound("loops", vec![atom("a")])),
        &kb,
        &NoSnapshots,
    );
    assert_eq!(
        report.steps[0].logic,
        LogicStatus::Failed(LogicFailure::NegationSearchTruncated)
    );
    assert!(!report.passed());
}

// ---------------------------------------------------------------------------
// (5) A trail that names clauses which do not exist.
// ---------------------------------------------------------------------------

#[test]
fn a_step_citing_a_fact_the_kb_does_not_have_fails() {
    let kb = logic_engine::KnowledgeBase::new();
    let proof = Proof {
        bindings: Substitution::empty(),
        steps: vec![ProofStep {
            goal: atom("anything"),
            origin: DerivationOrigin::FromFact(FactId(9_999)),
            depth: 0,
        }],
        via_facts: vec![FactId(9_999)],
        via_rules: vec![],
        posterior_logit: None,
        posterior_probability: None,
    };

    let report = verify_proof(&proof, &kb, &NoSnapshots);
    assert_eq!(
        report.steps[0].logic,
        LogicStatus::Failed(LogicFailure::UnknownFact(FactId(9_999)))
    );
}

#[test]
fn a_step_whose_cited_fact_does_not_unify_with_its_goal_fails() {
    let mut kb = logic_engine::KnowledgeBase::new();
    let id = kb.add_fact(Fact::certain(compound(
        "inhibits",
        vec![atom("aspirin"), atom("cyclooxygenase")],
    )));
    // The step claims that fact proved something else entirely.
    let proof = Proof {
        bindings: Substitution::empty(),
        steps: vec![ProofStep {
            goal: compound("inhibits", vec![atom("warfarin"), atom("vkorc1")]),
            origin: DerivationOrigin::FromFact(id),
            depth: 0,
        }],
        via_facts: vec![id],
        via_rules: vec![],
        posterior_logit: None,
        posterior_probability: None,
    };

    let report = verify_proof(&proof, &kb, &NoSnapshots);
    assert_eq!(
        report.steps[0].logic,
        LogicStatus::Failed(LogicFailure::GoalDoesNotUnify)
    );
}

// ---------------------------------------------------------------------------
// (6) Localization: the FIRST failure is the cause; the rest are consequences.
// ---------------------------------------------------------------------------

#[test]
fn the_report_localizes_to_the_first_failing_step_but_still_checks_the_rest() {
    let mut kb = logic_engine::KnowledgeBase::new();
    let good = kb.add_fact(
        Fact::certain(compound(
            "inhibits",
            vec![atom("aspirin"), atom("cyclooxygenase")],
        ))
        .with_provenance(quoting(0, 32)),
    );

    let mk = |goal: Term, origin| ProofStep {
        goal,
        origin,
        depth: 0,
    };
    let proof = Proof {
        bindings: Substitution::empty(),
        steps: vec![
            mk(
                compound("inhibits", vec![atom("aspirin"), atom("cyclooxygenase")]),
                DerivationOrigin::FromFact(good),
            ),
            mk(atom("nowhere"), DerivationOrigin::FromFact(FactId(4_242))),
            mk(atom("nowhere_either"), DerivationOrigin::FromFact(FactId(4_243))),
        ],
        via_facts: vec![good],
        via_rules: vec![],
        posterior_logit: None,
        posterior_probability: None,
    };

    let report = verify_proof(&proof, &kb, &snapshots());
    assert_eq!(
        report.first_failure().map(|s| s.index),
        Some(1),
        "the earliest failure is the one that localizes the error"
    );
    assert_eq!(
        report.steps.len(),
        3,
        "and checking does not stop there — a partial report would hide \
         whether the rest of the trail is sound"
    );
    assert!(report.steps[0].fully_verified());
}

// ---------------------------------------------------------------------------
// (7) A rule step must show its PREMISES, not just a head that could match.
// ---------------------------------------------------------------------------

/// The rule the next two tests forge trails against: two body literals, one of
/// them a negated guard.
fn kb_with_guarded_rule() -> (logic_engine::KnowledgeBase, RuleId) {
    let mut kb = logic_engine::KnowledgeBase::new();
    kb.add_fact(Fact::certain(compound("safe_for", vec![atom("aspirin"), atom("adult")])));
    let rid = kb.add_rule(
        Rule::certain(
            compound("may_prescribe", vec![v("D"), v("P")]),
            vec![
                BodyLiteral::Pos(compound("safe_for", vec![v("D"), v("P")])),
                BodyLiteral::Neg(compound("contraindicated", vec![v("D"), v("P")])),
            ],
        )
        .with_provenance(quoting(0, 32)),
    );
    (kb, rid)
}

fn one_step_rule_proof(rid: RuleId, extra: Vec<ProofStep>) -> Proof {
    let mut steps = vec![ProofStep {
        goal: compound("may_prescribe", vec![atom("aspirin"), atom("adult")]),
        origin: DerivationOrigin::FromRule(rid),
        depth: 0,
    }];
    steps.extend(extra);
    Proof {
        bindings: Substitution::empty(),
        steps,
        via_rules: vec![rid],
        via_facts: vec![],
        posterior_logit: None,
        posterior_probability: None,
    }
}

#[test]
fn a_rule_step_with_no_children_has_not_established_its_premises() {
    // The most dangerous forgery in the system: one step, a REAL rule id, a real
    // quoted citation, and no premises at all. Checking only that the head
    // unifies checks that the rule COULD apply — never that it did. This trail
    // would otherwise earn `fully_verified` for a conclusion nobody proved,
    // including the `not contraindicated(…)` guard the rule exists to enforce.
    let (kb, rid) = kb_with_guarded_rule();
    let report = verify_proof(&one_step_rule_proof(rid, vec![]), &kb, &snapshots());

    assert_eq!(
        report.steps[0].logic,
        LogicStatus::Failed(LogicFailure::RuleBodyNotDischarged {
            expected: 2,
            found: 0
        })
    );
    assert!(!report.passed());
}

#[test]
fn a_positive_step_cannot_stand_in_for_a_negated_guard() {
    // Right number of children, wrong kind. If the body-discharge check merely
    // COUNTED children, this would pass — and the substitution is precisely the
    // one that matters: a guard "we confirmed no contraindication" replaced by
    // some unrelated positive fact.
    let (mut kb, rid) = kb_with_guarded_rule();
    let bogus = kb.add_fact(Fact::certain(compound(
        "contraindicated",
        vec![atom("aspirin"), atom("adult")],
    )));
    let children = vec![
        ProofStep {
            goal: compound("safe_for", vec![atom("aspirin"), atom("adult")]),
            origin: DerivationOrigin::FromFact(FactId(0)),
            depth: 1,
        },
        ProofStep {
            goal: compound("contraindicated", vec![atom("aspirin"), atom("adult")]),
            origin: DerivationOrigin::FromFact(bogus),
            depth: 1,
        },
    ];
    let report = verify_proof(&one_step_rule_proof(rid, children), &kb, &snapshots());

    assert!(
        matches!(
            report.steps[0].logic,
            LogicStatus::Failed(LogicFailure::RuleBodyNotDischarged { .. })
        ),
        "a negated literal is discharged only by an established ABSENCE: {:?}",
        report.steps[0].logic
    );
}

#[test]
fn a_real_engine_rule_trail_still_passes_the_body_check() {
    // The counterweight: the discharge check must accept what the resolver
    // actually emits, or it would reject every honest trail and be worthless.
    let mut kb = logic_engine::KnowledgeBase::new();
    kb.add_fact(Fact::certain(compound(
        "safe_for",
        vec![atom("aspirin"), atom("adult")],
    )));
    kb.add_rule(Rule::certain(
        compound("may_prescribe", vec![v("D"), v("P")]),
        vec![
            BodyLiteral::Pos(compound("safe_for", vec![v("D"), v("P")])),
            BodyLiteral::Neg(compound("contraindicated", vec![v("D"), v("P")])),
        ],
    ));

    let dag = enumerate_all(
        &compound("may_prescribe", vec![atom("aspirin"), atom("adult")]),
        &kb,
    );
    assert!(!dag.proofs.is_empty(), "fixture must actually prove");
    let report = verify_proof(&dag.proofs[0], &kb, &NoSnapshots);
    assert!(
        report.passed(),
        "an honest engine-produced rule trail must survive: {:?}",
        report.first_failure()
    );
}

// ---------------------------------------------------------------------------
// (8) A goal that names no predicate cannot borrow someone else's citation.
// ---------------------------------------------------------------------------

#[test]
fn a_bare_variable_goal_is_rejected_before_any_clause_is_consulted() {
    // `Var` unifies with every fact and every rule head. A forged step carrying
    // one, plus the id of any real well-quoted fact, would re-check as sound and
    // inherit that fact's verified citation — a step that proved nothing in
    // particular, wearing someone else's evidence.
    let mut kb = logic_engine::KnowledgeBase::new();
    let id = kb.add_fact(
        Fact::certain(compound(
            "inhibits",
            vec![atom("aspirin"), atom("cyclooxygenase")],
        ))
        .with_provenance(quoting(0, 32)),
    );
    let proof = Proof {
        bindings: Substitution::empty(),
        steps: vec![ProofStep {
            goal: v("Anything"),
            origin: DerivationOrigin::FromFact(id),
            depth: 0,
        }],
        via_facts: vec![id],
        via_rules: vec![],
        posterior_logit: None,
        posterior_probability: None,
    };

    let report = verify_proof(&proof, &kb, &snapshots());
    assert_eq!(
        report.steps[0].logic,
        LogicStatus::Failed(LogicFailure::GoalIsBareVariable)
    );
    assert!(
        !matches!(report.steps[0].quote, QuoteStatus::Verified { .. }),
        "and it must not walk away with the fact's confirmed citation"
    );
}

// ---------------------------------------------------------------------------
// (9) Invisible-but-not-whitespace characters are still invisible.
// ---------------------------------------------------------------------------

#[test]
fn a_bidi_only_span_does_not_count_as_a_confirmed_quote() {
    // U+202A/U+202C are bidi controls: not Unicode White_Space, not in the
    // zero-width block, and rendered as nothing. Anchoring alone does not save
    // us here — where the snapshot is itself attacker-chosen, such a span
    // matches at its offset and would increment `quotes_verified` for something
    // a human auditor sees as blank.
    let doc = "\u{202A}\u{202C}filler";
    let mut snaps = MemorySnapshots::new();
    let hash = snaps.insert(doc.as_bytes().to_vec());
    let prov = Provenance::new("Anywhere".to_string(), None, TrustTier::Authoritative)
        .with_quote("\u{202A}\u{202C}", Some(0), Some(hash));

    assert!(
        !matches!(verify_quote(&prov, &snaps), QuoteStatus::Verified { .. }),
        "an invisible span must never read as a confirmed quote"
    );
}
