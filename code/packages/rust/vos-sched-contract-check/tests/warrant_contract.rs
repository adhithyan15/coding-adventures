//! Evidence for the Warrant text of the scheduler contract (VOS-010, VOS10).
//!
//! The contract package owns Warrant text (`warrant/*.part`). `vos-link gen-doc`
//! (VOS-033) does not exist yet, so this test file plays its part in the
//! smallest possible way: it concatenates a header, the contract block, an
//! implementation block and the obligations blocks into one document (plain
//! string joining; no parsing of any kind is written here), and then uses the
//! repository's own Warrant tooling for everything else:
//!
//! * `coding_adventures_warrant_grammar` parses the document and type-checks it
//!   (`check_warrant`), exactly as it would for a generated implementation
//!   document;
//! * its ground evaluator (`codegen::Evaluator`) runs every claim over the
//!   whole domain of its binders. The generator VOS-011 will use evaluates the
//!   same claims against compiled Rust; this file evaluates them against
//!   *Warrant models* of three very different implementations, which is the
//!   most that can be done before a Rust implementation exists.
//!
//! ## What the evaluation establishes, and what it does not
//!
//! It establishes that the claim text means what VOS10 says it means:
//!
//! * the model-level lemmas hold over their full domains;
//! * an ideal fixed-priority bitmap, written independently with bit operations,
//!   satisfies the safety, policy and canonical obligations;
//! * a lowest-priority-first scan (a different state type) satisfies the safety
//!   and canonical obligations and fails the policy obligation on exactly the
//!   247 states with two or more ready threads;
//! * the most-general scheduler satisfies the safety and `any` obligations;
//! * and, against deliberately wrong implementations, every obligation, every
//!   conjunct of the compound ones and every lemma has a control that breaks it.
//!   Section 4 is a table: each row is a mutant (a textual change to one model,
//!   asserted to have changed it), the claims it can reach (those whose text
//!   names a mutated operation) and the EXACT set of those claims that must
//!   fail. A claim whose text names no mutated operation cannot fail, so it is
//!   not evaluated; that is what keeps the suite to about a minute. What a
//!   control does and does not show is stated in VOS10 section 12.
//!
//! `parts_follow_the_marker_discipline` applies the tool's own part admission
//! (`vos_manifest::part::admit_part`, VOS-033) to the real parts.  The real
//! per-implementation documents are generated and checked by `vos-link` (its tests run
//! this contract package through four fixture implementations); `assemble` below is
//! only the plain joiner that the mutation tables need, since a mutant is a textual
//! change to one block and the document it lives in is never written anywhere.
//!
//! It does **not** establish anything about Rust code: the models are written
//! in Warrant. Binding the claims to compiled code (the evidence harness) is
//! VOS-011 and VOS-012.

use std::collections::HashMap;

use coding_adventures_warrant_grammar::checker::check_warrant;
use coding_adventures_warrant_grammar::codegen::{CodegenLimits, EvalValue, Evaluator};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{
    parse_document, Declaration, Expression, ParserLimits, TypeExpr, WarrantFile,
};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

// ---------------------------------------------------------------------------
// The text under test. These are the files `vos-link gen-doc` will copy.
// ---------------------------------------------------------------------------

const CONTRACT: &str = include_str!("../../vos-sched-contract/warrant/contract.part");
/// The nine lemmas.  NOT embedded in any implementation's document (VOS02 section 2.1, VOS-D064):
/// they are evaluated here, over the contract block, and nowhere else.
const LEMMAS_PART: &str = include_str!("../../vos-sched-contract/warrant/lemmas.part");
const SAFETY: &str = include_str!("../../vos-sched-contract/warrant/obligations-safety.part");
const POLICY: &str = include_str!("../../vos-sched-contract/warrant/obligations-policy.part");
const CANONICAL: &str = include_str!("../../vos-sched-contract/warrant/obligations-canonical.part");
const ANY: &str = include_str!("../../vos-sched-contract/warrant/obligations-any.part");
const REPRESENTATION: &str =
    include_str!("../../vos-sched-contract/warrant/obligations-representation.part");

const BITMAP_OPS: &str = include_str!("fixtures/impl-bitmap-ops.part");
const BITMAP: &str = include_str!("fixtures/impl-bitmap-models.part");
const SCAN: &str = include_str!("fixtures/impl-scan-models.part");
const ANY_IMPL: &str = include_str!("fixtures/impl-any-models.part");
const RR: &str = include_str!("fixtures/impl-rr-models.part");

/// The X1 case budget per claim (VOS30 section 8.5, VER04h section 3).
const BUDGET: u64 = 400_000;

/// Join blocks into one document for evaluation. Nothing is parsed here; this is not
/// document assembly (that is `vos_manifest::document::assemble`, used by `vos-link`).
fn assemble(blocks: &[&str]) -> String {
    let mut doc = String::from("warrant vos_sched_contract_test v1\n");
    for block in blocks {
        doc.push_str(block);
        if !block.ends_with('\n') {
            doc.push('\n');
        }
    }
    doc
}

/// Run `f` over the parsed document (the parser borrows from the source).
fn with_file<R>(src: &str, f: impl FnOnce(&WarrantFile) -> R) -> R {
    let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default())
        .unwrap_or_else(|e| panic!("parse failed: {e:?}"));
    f(&file)
}

/// Parse and type-check; returns the names of the claims and examples.
fn check(src: &str) -> (Vec<String>, Vec<String>) {
    with_file(src, |file| {
        let checked = check_warrant(file).unwrap_or_else(|e| panic!("type check failed: {e:?}"));
        (checked.claims, checked.examples)
    })
}

// ---------------------------------------------------------------------------
// The claim evaluator: every binder over its whole domain.
// ---------------------------------------------------------------------------

/// What one claim did over its binders.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Outcome {
    name: String,
    /// The product of the binder domains, before refinement guards. This is the
    /// number VER04h bounds by the 400,000 budget.
    product: u64,
    /// Cases whose guards held and whose body was evaluated.
    admitted: u64,
    /// Admitted cases whose body was not `true`.
    failed: u64,
    /// The binder values of the first failing case, for the failure message.
    first_failure: Option<String>,
}

impl Outcome {
    fn passed(&self) -> bool {
        self.failed == 0
    }
}

/// The values of a complete binder type, or `None` if the type is not one the
/// evaluator here enumerates. Only `Bool` and `U8` appear in this contract
/// (VOS01 section 5.6: every binder is complete).
fn domain(ty: &TypeExpr) -> Vec<EvalValue> {
    match ty {
        TypeExpr::Named { name: "Bool", .. } => vec![EvalValue::Bool(false), EvalValue::Bool(true)],
        TypeExpr::Named { name: "U8", .. } => (0..=255u8).map(EvalValue::U8).collect(),
        other => panic!("binder type {other:?} has no complete domain here"),
    }
}

/// Evaluate every `forall` claim of `file` (or just those named in `only`).
fn run_claims(file: &WarrantFile, only: Option<&[&str]>) -> Vec<Outcome> {
    let mut evaluator = Evaluator::new(file, CodegenLimits::default());
    let mut outcomes = Vec::new();
    for decl in &file.declarations {
        let Declaration::Claim(claim) = decl else {
            continue;
        };
        if only.is_some_and(|names| !names.contains(&claim.name)) {
            continue;
        }
        let Expression::Quantified {
            is_forall: true,
            binders,
            body,
            ..
        } = &claim.body
        else {
            panic!("claim {} is not a forall claim", claim.name);
        };
        let domains: Vec<Vec<EvalValue>> = binders.iter().map(|b| domain(&b.param_type)).collect();
        let product: u64 = domains.iter().map(|d| d.len() as u64).product();
        let mut admitted = 0;
        let mut failed = 0;
        let mut first_failure = None;
        let mut index = vec![0usize; domains.len()];
        'cases: loop {
            let mut env: HashMap<&str, EvalValue> = HashMap::new();
            for (k, binder) in binders.iter().enumerate() {
                env.insert(binder.name, domains[k][index[k]].clone());
            }
            evaluator.reset_work();
            let guarded = binders.iter().all(|b| match &b.refinement {
                None => true,
                Some(guard) => {
                    evaluator.eval(guard, &env).expect("guard evaluates") == EvalValue::Bool(true)
                }
            });
            if guarded {
                admitted += 1;
                evaluator.reset_work();
                let verdict = evaluator
                    .eval(body, &env)
                    .unwrap_or_else(|e| panic!("claim {} did not evaluate: {e}", claim.name));
                if verdict != EvalValue::Bool(true) {
                    failed += 1;
                    if first_failure.is_none() {
                        let shown: Vec<String> = binders
                            .iter()
                            .map(|b| format!("{}={:?}", b.name, env[b.name]))
                            .collect();
                        first_failure = Some(shown.join(", "));
                    }
                }
            }
            // odometer step, last binder fastest
            let mut k = domains.len();
            loop {
                if k == 0 {
                    break 'cases;
                }
                k -= 1;
                index[k] += 1;
                if index[k] < domains[k].len() {
                    break;
                }
                index[k] = 0;
            }
        }
        outcomes.push(Outcome {
            name: claim.name.to_string(),
            product,
            admitted,
            failed,
            first_failure,
        });
    }
    outcomes
}

/// The nine model-level lemmas of the contract block.
const LEMMAS: [&str; 9] = [
    "closure_at_m",
    "insert_outside_pre_changes_nothing",
    "remove_outside_pre_changes_nothing",
    "insert_then_remove_restores",
    "top_is_the_maximum",
    "safety_is_idle_or_a_member",
    "safe_dispatch_only_removes_the_choice",
    "policy_implies_safety",
    "policy_admits_only_top",
];

/// The thirteen obligations a production fixed-priority scheduler carries, in
/// document order (safety, policy, canonical).
const PRODUCTION: [&str; 13] = [
    "insert_safe",
    "remove_safe",
    "dispatch_safe",
    "safety_alpha_build",
    "safety_init",
    "dispatch_policy",
    "canon_init",
    "canon_insert_post",
    "canon_insert_img",
    "canon_remove_post",
    "canon_remove_img",
    "canon_dispatch_post",
    "canon_dispatch_img",
];

/// The eight `representation` obligations (VOS02 2.7), in document order.
const REP_CLAIMS: [&str; 8] = [
    "rep_init",
    "rep_alpha",
    "insert_rep_lo",
    "insert_rep_hi",
    "remove_rep_lo",
    "remove_rep_hi",
    "dispatch_rep",
    "dispatch_rep_post",
];

/// The `any` obligations.
const ANY_CLAIMS: [&str; 3] = [
    "dispatch_sound",
    "dispatch_covers_run",
    "dispatch_covers_idle",
];

/// Type-check the assembled document, then evaluate exactly the claims in `only`
/// (each over the whole domain of its binders). Evaluating a claim costs up to
/// 65,536 ground evaluations, so each test names the claims it is about.
fn evaluate(blocks: &[&str], only: &[&str]) -> Vec<Outcome> {
    let src = assemble(blocks);
    with_file(&src, |file| {
        check_warrant(file).unwrap_or_else(|e| panic!("type check failed: {e:?}"));
        let outcomes = run_claims(file, Some(only));
        assert_eq!(outcomes.len(), only.len(), "a named claim does not exist");
        outcomes
    })
}

/// Claims of `candidates` whose own text names one of `touches`, evaluated over
/// their whole domains. The caller asserts the exact set that fails.
fn evaluate_touching(blocks: &[&str], candidates: &[&str], touches: &[&str]) -> Vec<Outcome> {
    let src = assemble(blocks);
    with_file(&src, |file| {
        check_warrant(file).unwrap_or_else(|e| panic!("type check failed: {e:?}"));
        let scope: Vec<&str> = file
            .declarations
            .iter()
            .filter_map(|d| match d {
                Declaration::Claim(c) if candidates.contains(&c.name) => {
                    let text = &src[c.span.0..c.span.1];
                    touches.iter().any(|t| text.contains(t)).then_some(c.name)
                }
                _ => None,
            })
            .collect();
        assert!(!scope.is_empty(), "the mutant reaches no claim");
        run_claims(file, Some(&scope))
    })
}

fn get<'a>(outcomes: &'a [Outcome], name: &str) -> &'a Outcome {
    outcomes
        .iter()
        .find(|o| o.name == name)
        .unwrap_or_else(|| panic!("no claim named {name}"))
}

/// `(name, product, admitted)` of every claim, for the exact-count tables.
fn counts(outcomes: &[Outcome]) -> Vec<(&str, u64, u64)> {
    outcomes
        .iter()
        .map(|o| (o.name.as_str(), o.product, o.admitted))
        .collect()
}

fn assert_all_pass(outcomes: &[Outcome]) {
    for o in outcomes {
        assert!(
            o.passed(),
            "claim {} failed {} of {} cases; first: {:?}",
            o.name,
            o.failed,
            o.admitted,
            o.first_failure
        );
        assert!(o.product <= BUDGET, "claim {} exceeds the budget", o.name);
        assert!(o.admitted > 0, "claim {} is vacuous", o.name);
    }
}

/// Replace `from` by `to` in `text`, insisting that the text changed (a mutant
/// that silently equals its original would make a negative control meaningless).
fn mutate(text: &str, from: &str, to: &str) -> String {
    assert!(text.contains(from), "mutation target not found: {from}");
    text.replacen(from, to, 1)
}

// ---------------------------------------------------------------------------
// 1. The contract block
// ---------------------------------------------------------------------------

#[test]
fn contract_block_type_checks_alone() {
    // The contract block alone holds models only: the lemmas moved to lemmas.part.
    let (claims, examples) = check(&assemble(&[CONTRACT]));
    assert!(examples.is_empty());
    assert!(
        claims.is_empty(),
        "the contract block holds no claim: {claims:?}"
    );
    let (claims, examples) = check(&assemble(&[CONTRACT, LEMMAS_PART]));
    assert!(examples.is_empty());
    assert_eq!(claims.len(), 9, "the nine model-level lemmas: {claims:?}");
}

#[test]
fn lemmas_hold_over_their_whole_domains() {
    let outcomes = evaluate(&[CONTRACT, LEMMAS_PART], &LEMMAS);
    assert_all_pass(&outcomes);
    // 8 Bool binders = 256 states; with one U8 binder, 65,536 cases (16.4% of the budget).
    // closure_at_m is a trivial lemma (inv is `true`): it is evaluated, not counted as evidence.
    assert_eq!(
        counts(&outcomes),
        vec![
            ("closure_at_m", 65_536, 65_536),
            ("insert_outside_pre_changes_nothing", 65_536, 65_536),
            ("remove_outside_pre_changes_nothing", 65_536, 65_536),
            ("insert_then_remove_restores", 65_536, 65_536),
            ("top_is_the_maximum", 65_536, 65_536),
            ("safety_is_idle_or_a_member", 65_536, 65_536),
            ("safe_dispatch_only_removes_the_choice", 65_536, 65_536),
            ("policy_implies_safety", 256, 256),
            ("policy_admits_only_top", 65_536, 65_536),
        ]
    );
}

// ---------------------------------------------------------------------------
// 2. The real shape: bound operations, every obligations block
// ---------------------------------------------------------------------------

#[test]
fn a_generated_shaped_document_type_checks() {
    // This is the document `vos-link gen-doc` will assemble for a bitmap
    // implementation: contract block, implementation block (operations, not
    // models), obligations blocks.
    let (claims, examples) = check(&assemble(&[
        CONTRACT, BITMAP_OPS, SAFETY, POLICY, CANONICAL,
    ]));
    for obligation in PRODUCTION {
        assert!(
            claims.iter().any(|c| c == obligation),
            "missing {obligation}"
        );
    }
    assert_eq!(examples, vec!["n_is_eight".to_string()]);
}

#[test]
fn the_any_block_type_checks_against_a_two_field_impl() {
    // The any block names the oracle field, so it only type-checks against an
    // Impl that has one (VOS02 section 3.3). Against the one-field bitmap Impl it
    // must be rejected, and for THAT reason: the error is about the constructor
    // or the field, not an unrelated slip.
    check(&assemble(&[CONTRACT, ANY_IMPL, SAFETY, ANY]));
    let doc = assemble(&[CONTRACT, BITMAP, SAFETY, ANY]);
    let message = with_file(&doc, |file| match check_warrant(file) {
        Ok(_) => panic!("the bitmap Impl must not satisfy the any block"),
        Err(e) => format!("{e:?}"),
    });
    assert!(
        message.contains("ArityMismatch { expected: 1, actual: 2 }"),
        "rejected for an unrelated reason: {message}"
    );
}

// ---------------------------------------------------------------------------
// 3. Three implementations, evaluated over the whole domain of every claim
// ---------------------------------------------------------------------------

#[test]
fn bitmap_satisfies_safety_policy_and_canonical() {
    let obligations = evaluate(&[CONTRACT, BITMAP, SAFETY, POLICY, CANONICAL], &PRODUCTION);
    assert_all_pass(&obligations);
    // document order: safety (5), policy (1), canonical (7)
    assert_eq!(
        counts(&obligations),
        vec![
            ("insert_safe", 65_536, 65_536),
            ("remove_safe", 65_536, 65_536),
            ("dispatch_safe", 256, 256),
            ("safety_alpha_build", 256, 256),
            ("safety_init", 2, 2),
            ("dispatch_policy", 256, 256),
            ("canon_init", 2, 2),
            ("canon_insert_post", 65_536, 65_536),
            ("canon_insert_img", 65_536, 65_536),
            ("canon_remove_post", 65_536, 65_536),
            ("canon_remove_img", 65_536, 65_536),
            ("canon_dispatch_post", 256, 256),
            ("canon_dispatch_img", 256, 256),
        ]
    );
}

#[test]
fn linear_scan_is_safe_and_canonical_but_not_fixed_priority() {
    let outcomes = evaluate(&[CONTRACT, SCAN, SAFETY, POLICY, CANONICAL], &PRODUCTION);
    let failing: Vec<&str> = outcomes
        .iter()
        .filter(|o| !o.passed())
        .map(|o| o.name.as_str())
        .collect();
    assert_eq!(failing, vec!["dispatch_policy"]);
    // The policy tier distinguishes the two: the scan returns the LOWEST member, which
    // is wrong on every state with two or more members: 256 - 1 (empty) - 8 (singletons).
    let policy = get(&outcomes, "dispatch_policy");
    assert_eq!(policy.failed, 247);
    assert_eq!(policy.admitted, 256);
}

#[test]
fn any_satisfies_safety_and_the_any_block() {
    let obligations = evaluate(
        &[CONTRACT, ANY_IMPL, SAFETY, ANY],
        &[
            "insert_safe",
            "remove_safe",
            "dispatch_safe",
            "safety_alpha_build",
            "safety_init",
            "dispatch_sound",
            "dispatch_covers_run",
            "dispatch_covers_idle",
        ],
    );
    assert_all_pass(&obligations);
    assert_eq!(
        counts(&obligations),
        vec![
            ("insert_safe", 65_536, 65_536),
            ("remove_safe", 65_536, 65_536),
            ("dispatch_safe", 256, 256),
            ("safety_alpha_build", 256, 256),
            ("safety_init", 2, 2),
            // oracle and state both U8: 65,536 each; `mem` is false for t >= 8, so coverage needs no
            // `t < 8` refinement and admits every case (VOS-D079).
            ("dispatch_sound", 65_536, 65_536),
            ("dispatch_covers_run", 65_536, 65_536),
            ("dispatch_covers_idle", 256, 256),
        ]
    );
}

#[test]
fn round_robin_satisfies_safety_and_the_representation_tier() {
    // VOS02 2.7: a scheduler whose state has history (the pointer) carries the representation
    // block instead of the canonical one.  Eight claims, 458,754 cases, and the safety tier.
    let mut names: Vec<&str> = vec![
        "insert_safe",
        "remove_safe",
        "dispatch_safe",
        "safety_alpha_build",
        "safety_init",
    ];
    names.extend(REP_CLAIMS);
    let outcomes = evaluate(&[CONTRACT, RR, SAFETY, REPRESENTATION], &names);
    assert_all_pass(&outcomes);
    assert_eq!(
        counts(&outcomes),
        vec![
            ("insert_safe", 65_536, 65_536),
            ("remove_safe", 65_536, 65_536),
            ("dispatch_safe", 256, 256),
            ("safety_alpha_build", 256, 256),
            ("safety_init", 2, 2),
            ("rep_init", 2, 2),
            ("rep_alpha", 65_536, 65_536),
            ("insert_rep_lo", 262_144, 262_144),
            ("insert_rep_hi", 262_144, 262_144),
            ("remove_rep_lo", 262_144, 262_144),
            ("remove_rep_hi", 262_144, 262_144),
            ("dispatch_rep", 65_536, 65_536),
            ("dispatch_rep_post", 65_536, 65_536),
        ]
    );
}

#[test]
fn a_round_robin_is_not_canonical() {
    // The reason the tier exists: after one dispatch the pointer has left its `build` value, so
    // the canonical commuting claim fails for the same implementation.
    let outcomes = evaluate(&[CONTRACT, RR, SAFETY, CANONICAL], &["canon_dispatch_post"]);
    assert!(!outcomes[0].passed(), "{:?}", outcomes[0]);
}

#[test]
fn any_is_not_fixed_priority() {
    let outcomes = evaluate(&[CONTRACT, ANY_IMPL, SAFETY, POLICY], &["dispatch_policy"]);
    assert!(!get(&outcomes, "dispatch_policy").passed());
}

#[test]
fn the_oracle_at_zero_is_canonical_so_kind_is_enforced_by_rule() {
    // VOS02 R6: the separation of `any` from `production` is a rule of `vos-link`, not an
    // evidence fact. Every canonical claim starts from build(M), where the oracle is 0,
    // and an oracle nothing moves is canonical (VOS10 section 14, F1).
    let outcomes = evaluate(
        &[CONTRACT, ANY_IMPL, SAFETY, CANONICAL],
        &[
            "canon_init",
            "canon_dispatch_img",
            "canon_dispatch_post",
            "safety_alpha_build",
            "safety_init",
        ],
    );
    assert_all_pass(&outcomes);
}

// ---------------------------------------------------------------------------
// 4. Negative controls: a table of mutants and the exact claims each must break
// ---------------------------------------------------------------------------

/// One wrong implementation (or wrong contract) and what it must break.
struct Mutant {
    name: &'static str,
    /// The unmutated implementation fixture.
    base: &'static str,
    /// Textual edits, each asserted to change the fixture.
    edits: &'static [(&'static str, &'static str)],
    /// Obligation blocks included in the document.
    blocks: &'static [&'static str],
    /// Operations or models the mutant changed: a claim is evaluated iff its text names one.
    touches: &'static [&'static str],
    /// The EXACT set of evaluated claims that must fail.
    fails: &'static [&'static str],
}

const OBLIGATIONS: [&str; 24] = [
    "insert_safe",
    "remove_safe",
    "dispatch_safe",
    "safety_alpha_build",
    "safety_init",
    "dispatch_policy",
    "canon_init",
    "canon_insert_post",
    "canon_insert_img",
    "canon_remove_post",
    "canon_remove_img",
    "canon_dispatch_post",
    "canon_dispatch_img",
    "dispatch_sound",
    "dispatch_covers_run",
    "dispatch_covers_idle",
    "rep_init",
    "rep_alpha",
    "insert_rep_lo",
    "insert_rep_hi",
    "remove_rep_lo",
    "remove_rep_hi",
    "dispatch_rep",
    "dispatch_rep_post",
];

const INSERT_IMG: &str =
    "model insert_img(s: Impl, t: U8) -> Impl = (\n    if t >= 8u8 then s else Impl(bit_or(s.mask, shl(1u8, to_nat(t)))))";
const REMOVE_IMG: &str =
    "model remove_img(s: Impl, t: U8) -> Impl = (\n    if t >= 8u8 then s else Impl(bit_and(s.mask, bit_not(shl(1u8, to_nat(t))))))";
const DISPATCH_IMG: &str =
    "model dispatch_img(s: Impl) -> Impl = (\n    if s.mask == 0u8 then s else Impl(bit_and(s.mask, bit_not(shl(1u8, to_nat(hibit(s.mask)))))))";
const ANY_INSERT_IMG: &str =
    "if t >= 8u8 then s else Impl(bit_or(s.mask, shl(1u8, to_nat(t))), s.choice))";
const ANY_REMOVE_IMG: &str =
    "if t >= 8u8 then s else Impl(bit_and(s.mask, bit_not(shl(1u8, to_nat(t)))), s.choice))";
const ANY_DISPATCH_IMG: &str =
    "if proposal_ok(s) then Impl(bit_and(s.mask, bit_not(shl(1u8, to_nat(s.choice)))), s.choice) else s)";
const ANY_PROPOSAL: &str =
    "model proposal_ok(s: Impl) -> Bool = s.choice < 8u8 and has(s, s.choice)";
const ANY_DISPATCH_OUT: &str =
    "model dispatch_out(s: Impl) -> Pick = if proposal_ok(s) then Run(s.choice) else Idle";

/// The blocks of a representation mutant's document.
const REP_BLOCKS: &[&str] = &["safety", "representation"];

fn mutants() -> Vec<Mutant> {
    let all = &["safety", "policy", "canonical"];
    vec![
        Mutant {
            name: "dispatch always answers Idle and keeps the state (allowed by safety, not by policy)",
            base: BITMAP,
            edits: &[
                ("model dispatch_out(s: Impl) -> Pick = if s.mask == 0u8 then Idle else Run(hibit(s.mask))", "model dispatch_out(s: Impl) -> Pick = Idle"),
                (DISPATCH_IMG, "model dispatch_img(s: Impl) -> Impl = s"),
            ],
            blocks: all,
            touches: &["dispatch_out", "dispatch_img"],
            fails: &["dispatch_policy"],
        },
        Mutant {
            name: "dispatch answers Run(0) for the empty set (a thread that is not ready)",
            base: BITMAP,
            edits: &[("if s.mask == 0u8 then Idle else Run(hibit(s.mask))", "Run(hibit(s.mask))")],
            blocks: all,
            touches: &["dispatch_out"],
            fails: &["dispatch_safe", "dispatch_policy"],
        },
        Mutant {
            name: "dispatch forgets to remove the chosen thread (dispatch_img)",
            base: BITMAP,
            edits: &[(DISPATCH_IMG, "model dispatch_img(s: Impl) -> Impl = s")],
            blocks: all,
            touches: &["dispatch_img"],
            fails: &["dispatch_safe", "dispatch_policy", "canon_dispatch_img"],
        },
        Mutant {
            name: "insert_out has no range check (insert_safe, _out conjunct)",
            base: BITMAP,
            edits: &[(
                "model insert_out(s: Impl, t: U8) -> Upd = (\n    if t >= 8u8 then OutOfRange else (if has(s, t) then Unchanged else Changed))",
                "model insert_out(s: Impl, t: U8) -> Upd = if has(s, t) then Unchanged else Changed",
            )],
            blocks: all,
            touches: &["insert_out"],
            fails: &["insert_safe"],
        },
        Mutant {
            name: "insert_img returns the state unchanged for t < 8 (stale state; insert_safe, _img conjunct)",
            base: BITMAP,
            edits: &[(INSERT_IMG, "model insert_img(s: Impl, t: U8) -> Impl = s")],
            blocks: all,
            touches: &["insert_img"],
            fails: &["insert_safe", "canon_insert_img"],
        },
        Mutant {
            name: "insert_img sets bit 0 whatever t is (wrong bit)",
            base: BITMAP,
            edits: &[(INSERT_IMG, "model insert_img(s: Impl, t: U8) -> Impl = (\n    if t >= 8u8 then s else Impl(bit_or(s.mask, 1u8)))")],
            blocks: all,
            touches: &["insert_img"],
            fails: &["insert_safe", "canon_insert_img"],
        },
        Mutant {
            name: "remove_out always answers Unchanged (remove_safe, _out conjunct)",
            base: BITMAP,
            edits: &[(
                "model remove_out(s: Impl, t: U8) -> Upd = (\n    if t >= 8u8 then OutOfRange else (if has(s, t) then Changed else Unchanged))",
                "model remove_out(s: Impl, t: U8) -> Upd = Unchanged",
            )],
            blocks: all,
            touches: &["remove_out"],
            fails: &["remove_safe"],
        },
        Mutant {
            name: "remove_img returns the state unchanged (stale state)",
            base: BITMAP,
            edits: &[(REMOVE_IMG, "model remove_img(s: Impl, t: U8) -> Impl = s")],
            blocks: all,
            touches: &["remove_img"],
            fails: &["remove_safe", "canon_remove_img"],
        },
        Mutant {
            name: "remove_img clears every bit (wrong bit)",
            base: BITMAP,
            edits: &[(REMOVE_IMG, "model remove_img(s: Impl, t: U8) -> Impl = (\n    if t >= 8u8 then s else Impl(0u8))")],
            blocks: all,
            touches: &["remove_img"],
            fails: &["remove_safe", "canon_remove_img"],
        },
        Mutant {
            name: "alpha forgets the state (shape C: alpha onto a single state)",
            base: BITMAP,
            edits: &[("model alpha_w(s: Impl) -> AbsW = AbsW(s.mask)", "model alpha_w(s: Impl) -> AbsW = AbsW(0u8)")],
            blocks: all,
            touches: &["alpha_w"],
            fails: &[
                "insert_safe", "remove_safe", "dispatch_safe", "safety_alpha_build", "dispatch_policy",
                "canon_insert_post", "canon_remove_post", "canon_dispatch_post",
            ],
        },
        Mutant {
            name: "build ignores thread 7 (the canonical remove and dispatch claims compare build with build and do not see it)",
            base: BITMAP,
            edits: &[("b2m(m.p7, 7)", "0u8")],
            blocks: all,
            touches: &["build"],
            fails: &[
                "insert_safe",
                "remove_safe",
                "dispatch_safe",
                "safety_alpha_build",
                "dispatch_policy",
                "canon_insert_post",
                "canon_insert_img",
            ],
        },
        Mutant {
            name: "the initial state is not build(empty())",
            base: BITMAP,
            edits: &[("model init() -> Impl = Impl(0u8)", "model init() -> Impl = Impl(1u8)")],
            blocks: all,
            touches: &["init("],
            fails: &["canon_init", "safety_init"],
        },
        // ---- VOS-D081: the safety tier constrains init for every implementation ----
        Mutant {
            name: "a bitmap whose initial mask is 8, safety block only (safety_init does not need canonical)",
            base: BITMAP,
            edits: &[("model init() -> Impl = Impl(0u8)", "model init() -> Impl = Impl(8u8)")],
            blocks: &["safety"],
            touches: &["init("],
            fails: &["safety_init"],
        },
        Mutant {
            name: "any starts with mask 8 and oracle 0 (dispatch still answers Idle: only the init claim sees it)",
            base: ANY_IMPL,
            edits: &[("model init() -> Impl = Impl(0u8, 0u8)", "model init() -> Impl = Impl(8u8, 0u8)")],
            blocks: &["safety", "any"],
            touches: &["init("],
            fails: &["safety_init"],
        },
        // ---- the history-bearing scheduler: the canonical claims, and only they, see it ----
        Mutant {
            name: "insert_img moves a register alpha forgets (history)",
            base: ANY_IMPL,
            edits: &[(ANY_INSERT_IMG, "if t >= 8u8 then s else Impl(bit_or(s.mask, shl(1u8, to_nat(t))), 1u8))")],
            blocks: &["safety", "canonical"],
            touches: &["insert_img"],
            fails: &["canon_insert_post", "canon_insert_img"],
        },
        Mutant {
            name: "remove_img moves a register alpha forgets (history)",
            base: ANY_IMPL,
            edits: &[(ANY_REMOVE_IMG, "if t >= 8u8 then s else Impl(bit_and(s.mask, bit_not(shl(1u8, to_nat(t)))), 1u8))")],
            blocks: &["safety", "canonical"],
            touches: &["remove_img"],
            fails: &["canon_remove_post", "canon_remove_img"],
        },
        Mutant {
            name: "dispatch_img moves a register alpha forgets (history, a round-robin pointer)",
            base: ANY_IMPL,
            edits: &[(ANY_DISPATCH_IMG, "if proposal_ok(s) then Impl(bit_and(s.mask, bit_not(shl(1u8, to_nat(s.choice)))), 1u8) else s)")],
            blocks: &["safety", "canonical"],
            touches: &["dispatch_img"],
            fails: &["canon_dispatch_post", "canon_dispatch_img"],
        },
        // ---- the most-general scheduler ----
        Mutant {
            name: "any never proposes thread 3 (sound, not covering)",
            base: ANY_IMPL,
            edits: &[(ANY_PROPOSAL, "model proposal_ok(s: Impl) -> Bool = s.choice < 8u8 and has(s, s.choice) and not (s.choice == 3u8)")],
            blocks: &["safety", "any"],
            touches: &["dispatch_out", "dispatch_img"],
            fails: &["dispatch_covers_run"],
        },
        Mutant {
            name: "any skips the validation step (proposes a thread that is not ready)",
            base: ANY_IMPL,
            edits: &[(ANY_PROPOSAL, "model proposal_ok(s: Impl) -> Bool = s.choice < 8u8")],
            blocks: &["safety", "any"],
            touches: &["dispatch_out", "dispatch_img"],
            fails: &["dispatch_safe", "dispatch_sound"],
        },
        Mutant {
            name: "any answers Run(7) for oracle 255 on a non-empty set (the witness for Idle fails)",
            base: ANY_IMPL,
            edits: &[(ANY_DISPATCH_OUT, "model dispatch_out(s: Impl) -> Pick = if proposal_ok(s) then Run(s.choice) else (if s.choice == 255u8 and not (s.mask == 0u8) then Run(7u8) else Idle)")],
            blocks: &["safety", "any"],
            touches: &["dispatch_out"],
            fails: &["dispatch_sound", "dispatch_covers_idle"],
        },
        Mutant {
            name: "any dispatch_img moves the oracle (the `.choice == c` conjunct of dispatch_sound)",
            base: ANY_IMPL,
            edits: &[(ANY_DISPATCH_IMG, "if proposal_ok(s) then Impl(bit_and(s.mask, bit_not(shl(1u8, to_nat(s.choice)))), 1u8) else s)")],
            blocks: &["safety", "any"],
            touches: &["dispatch_img"],
            fails: &["dispatch_sound"],
        },
        // ---- the representation tier (VOS02 2.7), on the round-robin model ----------------------
        // Each mutant changes one model of the RR fixture; `fails` is the EXACT set of the
        // evaluated claims (safety and representation) that must break.
        Mutant {
            name: "rr rep ignores the history code (a degenerate rep): the image is only ptr = 0, which a real round-robin leaves after a dispatch (a _post claim)",
            base: RR,
            edits: &[("model rep(m: Ready, h: U8) -> Impl = Impl(mask_of(m), h)", "model rep(m: Ready, h: U8) -> Impl = Impl(mask_of(m), 0u8)")],
            blocks: REP_BLOCKS,
            touches: &["rep("],
            fails: &["dispatch_rep_post"],
        },
        Mutant {
            name: "rr alpha cheats at ptr != 0 (forgets the mask): rep_alpha, every claim that reads alpha of an image state, and dispatch_safe (the post-state of a dispatch has ptr != 0)",
            base: RR,
            edits: &[("model alpha_w(s: Impl) -> AbsW = AbsW(s.mask)", "model alpha_w(s: Impl) -> AbsW = if s.ptr == 0u8 then AbsW(s.mask) else AbsW(0u8)")],
            blocks: REP_BLOCKS,
            touches: &["alpha_w"],
            fails: &["dispatch_safe", "rep_alpha", "insert_rep_lo", "insert_rep_hi", "remove_rep_lo", "remove_rep_hi", "dispatch_rep", "dispatch_rep_post"],
        },
        Mutant {
            name: "rr init is outside the image (pointer 9: rep reads it as 1)",
            base: RR,
            edits: &[("model init() -> Impl = Impl(0u8, 0u8)", "model init() -> Impl = Impl(0u8, 9u8)")],
            blocks: REP_BLOCKS,
            touches: &["init"],
            fails: &["rep_init"],
        },
        Mutant {
            name: "rr insert_img leaves the image (pointer 9)",
            base: RR,
            edits: &[("if t >= 8u8 then s else Impl(bit_or(s.mask, shl(1u8, to_nat(t))), s.ptr))", "if t >= 8u8 then s else Impl(bit_or(s.mask, shl(1u8, to_nat(t))), 9u8))")],
            blocks: REP_BLOCKS,
            touches: &["insert_img"],
            fails: &["insert_rep_lo", "insert_rep_hi"],
        },
        Mutant {
            name: "rr remove_img leaves the image (pointer 9)",
            base: RR,
            edits: &[("if t >= 8u8 then s else Impl(bit_and(s.mask, bit_not(shl(1u8, to_nat(t)))), s.ptr))", "if t >= 8u8 then s else Impl(bit_and(s.mask, bit_not(shl(1u8, to_nat(t)))), 9u8))")],
            blocks: REP_BLOCKS,
            touches: &["remove_img"],
            fails: &["remove_rep_lo", "remove_rep_hi"],
        },
        Mutant {
            name: "rr dispatch_img leaves the image (pointer 9)",
            base: RR,
            edits: &[("model dispatch_img(s: Impl) -> Impl = rr_next(s.mask, s.ptr)", "model dispatch_img(s: Impl) -> Impl = Impl(rr_next(s.mask, s.ptr).mask, 9u8)")],
            blocks: REP_BLOCKS,
            touches: &["dispatch_img"],
            fails: &["dispatch_rep_post"],
        },
        Mutant {
            name: "rr insert_out always answers Changed (insert_rep, _out conjunct)",
            base: RR,
            edits: &[("model insert_out(s: Impl, t: U8) -> Upd = (\n    if t >= 8u8 then OutOfRange else (if has(s, t) then Unchanged else Changed))", "model insert_out(s: Impl, t: U8) -> Upd = Changed")],
            blocks: REP_BLOCKS,
            touches: &["insert_out"],
            fails: &["insert_safe", "insert_rep_lo", "insert_rep_hi"],
        },
        Mutant {
            name: "rr remove_out always answers Unchanged (remove_rep, _out conjunct)",
            base: RR,
            edits: &[("model remove_out(s: Impl, t: U8) -> Upd = (\n    if t >= 8u8 then OutOfRange else (if has(s, t) then Changed else Unchanged))", "model remove_out(s: Impl, t: U8) -> Upd = Unchanged")],
            blocks: REP_BLOCKS,
            touches: &["remove_out"],
            fails: &["remove_safe", "remove_rep_lo", "remove_rep_hi"],
        },
        Mutant {
            name: "rr dispatch always answers Run(0) (a thread that need not be ready)",
            base: RR,
            edits: &[("model dispatch_out(s: Impl) -> Pick = rr_pick(s.mask, s.ptr)", "model dispatch_out(s: Impl) -> Pick = Run(0u8)")],
            blocks: REP_BLOCKS,
            touches: &["dispatch_out"],
            fails: &["dispatch_safe", "dispatch_rep"],
        },
        Mutant {
            name: "rr insert_img jumps out of the image for t >= 32 (right abstraction, pointer 200): a packed t = shr(k, 3) could not see it; only the every-t claims do",
            base: RR,
            edits: &[("if t >= 8u8 then s else Impl(bit_or(s.mask, shl(1u8, to_nat(t))), s.ptr))", "if t >= 32u8 then Impl(s.mask, 200u8) else (if t >= 8u8 then s else Impl(bit_or(s.mask, shl(1u8, to_nat(t))), s.ptr)))")],
            blocks: REP_BLOCKS,
            touches: &["insert_img"],
            fails: &["insert_rep_lo", "insert_rep_hi"],
        },
        Mutant {
            name: "rr remove_img jumps out of the image for t >= 32 (right abstraction, pointer 200)",
            base: RR,
            edits: &[("if t >= 8u8 then s else Impl(bit_and(s.mask, bit_not(shl(1u8, to_nat(t)))), s.ptr))", "if t >= 32u8 then Impl(s.mask, 200u8) else (if t >= 8u8 then s else Impl(bit_and(s.mask, bit_not(shl(1u8, to_nat(t)))), s.ptr)))")],
            blocks: REP_BLOCKS,
            touches: &["remove_img"],
            fails: &["remove_rep_lo", "remove_rep_hi"],
        },
        Mutant {
            name: "rr dispatch picks the highest thread and does not rotate (the pointer stays, inside the image): the tier still passes, because `_det` is not gated (VOS02 2.7)",
            base: RR,
            edits: &[
                ("model dispatch_out(s: Impl) -> Pick = rr_pick(s.mask, s.ptr)", "model dispatch_out(s: Impl) -> Pick = rr_pick(s.mask, 0u8)"),
                ("model dispatch_img(s: Impl) -> Impl = rr_next(s.mask, s.ptr)", "model dispatch_img(s: Impl) -> Impl = (\n    match rr_pick(s.mask, 0u8) with\n      Idle -> s\n    | Run(t) -> Impl(bit_and(s.mask, bit_not(shl(1u8, to_nat(t)))), s.ptr))"),
            ],
            blocks: REP_BLOCKS,
            touches: &["dispatch_out", "dispatch_img"],
            fails: &[],
        },
    ]
}

fn block_text(name: &str) -> &'static str {
    match name {
        "safety" => SAFETY,
        "policy" => POLICY,
        "canonical" => CANONICAL,
        "any" => ANY,
        "representation" => REPRESENTATION,
        other => panic!("no block {other}"),
    }
}

#[test]
fn every_mutant_breaks_exactly_the_claims_it_should() {
    // Mutants are independent, so they run on their own threads.
    let table = mutants();
    std::thread::scope(|scope| {
        let handles: Vec<_> = table
            .iter()
            .map(|m| scope.spawn(move || check_mutant(m)))
            .collect();
        for h in handles {
            h.join().expect("a mutant check panicked");
        }
    });
}

fn check_mutant(m: &Mutant) {
    let mut fixture = m.base.to_string();
    for (from, to) in m.edits {
        fixture = mutate(&fixture, from, to);
    }
    let mut blocks: Vec<&str> = vec![CONTRACT, &fixture];
    blocks.extend(m.blocks.iter().map(|b| block_text(b)));
    let outcomes = evaluate_touching(&blocks, &OBLIGATIONS, m.touches);
    let mut failing: Vec<&str> = outcomes
        .iter()
        .filter(|o| !o.passed())
        .map(|o| o.name.as_str())
        .collect();
    failing.sort_unstable();
    let mut expected = m.fails.to_vec();
    expected.sort_unstable();
    assert_eq!(failing, expected, "mutant: {}", m.name);
    // the expected failures were all evaluated (the mutant reaches them)
    for name in &expected {
        assert!(
            outcomes.iter().any(|o| o.name == *name),
            "{name} not reached"
        );
    }
}

#[test]
fn every_obligation_has_a_control() {
    // The union of the claims the table makes fail covers every obligation, so no
    // claim is left that no mutant can break. (The scan's policy failure, 247 states,
    // is the control for the policy claim on a second representation.)
    let mut covered: Vec<&str> = mutants()
        .iter()
        .flat_map(|m| m.fails.iter().copied())
        .collect();
    covered.sort_unstable();
    covered.dedup();
    for name in PRODUCTION
        .iter()
        .chain(ANY_CLAIMS.iter())
        .chain(REP_CLAIMS.iter())
    {
        assert!(covered.contains(name), "obligation {name} has no control");
    }
}

/// A mutation of the contract block that must break one lemma.
const LEMMA_MUTANTS: [(&str, &str, &str); 9] = [
    (
        "closure_at_m",
        "model inv(m: Ready) -> Bool = true",
        "model inv(m: Ready) -> Bool = m.p0",
    ),
    (
        "insert_outside_pre_changes_nothing",
        "model abs_insert(m: Ready, t: U8) -> (Upd, Ready) = (\n    if t >= 8u8 then (OutOfRange, m)",
        "model abs_insert(m: Ready, t: U8) -> (Upd, Ready) = (\n    if t >= 8u8 then (Unchanged, m)",
    ),
    (
        "remove_outside_pre_changes_nothing",
        "model abs_remove(m: Ready, t: U8) -> (Upd, Ready) = (\n    if t >= 8u8 then (OutOfRange, m)",
        "model abs_remove(m: Ready, t: U8) -> (Upd, Ready) = (\n    if t >= 8u8 then (Unchanged, m)",
    ),
    (
        "insert_then_remove_restores",
        "else (Changed, put(m, t, true))",
        "else (Changed, put(m, 0u8, true))",
    ),
    (
        "top_is_the_maximum",
        "if m.p7 then Run(7u8) else (if m.p6 then Run(6u8)",
        "if m.p0 then Run(0u8) else (if m.p6 then Run(6u8)",
    ),
    (
        "safety_is_idle_or_a_member",
        "match o with Idle -> true | Run(t) -> t < 8u8 and mem(m, t))",
        "match o with Idle -> true | Run(t) -> t < 8u8)",
    ),
    (
        "safe_dispatch_only_removes_the_choice",
        "match o with Idle -> m | Run(t) -> put(m, t, false))",
        "match o with Idle -> m | Run(t) -> empty())",
    ),
    (
        "policy_implies_safety",
        "match o with Idle -> true | Run(t) -> t < 8u8 and mem(m, t))",
        "match o with Idle -> true | Run(t) -> t < 7u8 and mem(m, t))",
    ),
    (
        "policy_admits_only_top",
        "model policy_dispatch(m: Ready, o: Pick, m2: Ready) -> Bool = o == top(m) and",
        "model policy_dispatch(m: Ready, o: Pick, m2: Ready) -> Bool = (o == top(m) or o == Idle) and",
    ),
];

#[test]
fn every_lemma_has_a_mutant_that_breaks_it() {
    let names: Vec<&str> = LEMMA_MUTANTS.iter().map(|m| m.0).collect();
    assert_eq!(names, LEMMAS.to_vec(), "one mutant per lemma, in order");
    std::thread::scope(|scope| {
        let handles: Vec<_> = LEMMA_MUTANTS
            .iter()
            .map(|(lemma, from, to)| {
                scope.spawn(move || {
                    let mutated = mutate(CONTRACT, from, to);
                    let outcomes = evaluate(&[&mutated, LEMMAS_PART], &[lemma]);
                    assert!(
                        !outcomes[0].passed(),
                        "lemma {lemma} survived its mutant ({from})"
                    );
                })
            })
            .collect();
        for h in handles {
            h.join().expect("a lemma control failed");
        }
    });
}

// ---------------------------------------------------------------------------
// 5. The text itself
// ---------------------------------------------------------------------------

#[test]
fn parts_follow_the_marker_discipline() {
    // Folded into the tool (VOS-D049): the admission rules of VOS02 section 2.3 are
    // `vos_manifest::part::admit_part` (the code `vos-link gen-doc` runs), not a copy of
    // them written here.  This test only applies it to the five real parts, and checks that
    // an obligations part holds claims and comments, nothing else.
    for (name, text) in [
        ("contract", CONTRACT),
        ("lemmas", LEMMAS_PART),
        ("safety", SAFETY),
        ("policy", POLICY),
        ("canonical", CANONICAL),
        ("any", ANY),
        ("representation", REPRESENTATION),
    ] {
        vos_manifest::part::admit_part(text.as_bytes())
            .unwrap_or_else(|e| panic!("{name}.part is not admitted: {e}"));
    }
    // The lemmas part holds exactly the nine lemmas, and none of them is an obligation.
    let lemma_claims = vos_manifest::warrant::claims_of_block(LEMMAS_PART).expect("lemmas.part");
    let lemma_names: Vec<&str> = lemma_claims.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(lemma_names, LEMMAS.to_vec());
    for (name, text) in [
        ("safety", SAFETY),
        ("policy", POLICY),
        ("canonical", CANONICAL),
        ("any", ANY),
        ("representation", REPRESENTATION),
    ] {
        let claims = vos_manifest::warrant::claims_of_block(text)
            .unwrap_or_else(|e| panic!("{name}.part: {e}"));
        assert!(!claims.is_empty(), "{name}: no claims");
    }
}

// ---------------------------------------------------------------------------
// 6. The Rust trait crate and the Warrant text agree
// ---------------------------------------------------------------------------

/// The names of a variant's constructors, with the field count of each.
fn variant_shape(file: &WarrantFile, name: &str) -> Vec<(String, usize)> {
    file.declarations
        .iter()
        .find_map(|d| match d {
            Declaration::Variant(v) if v.name == name => Some(
                v.ctors
                    .iter()
                    .map(|c| (c.name.to_string(), c.fields.len()))
                    .collect(),
            ),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no variant {name}"))
}

/// Name and payload arity of each constructor. Both matches are exhaustive with no
/// wildcard: a new variant in the Rust enum is a COMPILE ERROR here, and the test below
/// then compares it with the Warrant declaration.
fn upd_shape(u: vos_sched_contract::Upd) -> (usize, &'static str, usize) {
    use vos_sched_contract::Upd;
    match u {
        Upd::Changed => (0, "Changed", 0),
        Upd::Unchanged => (1, "Unchanged", 0),
        Upd::OutOfRange => (2, "OutOfRange", 0),
    }
}

fn pick_shape(p: vos_sched_contract::Pick) -> (usize, &'static str, usize) {
    use vos_sched_contract::Pick;
    match p {
        Pick::Idle => (0, "Idle", 0),
        Pick::Run(_) => (1, "Run", 1),
    }
}

#[test]
fn the_rust_enums_and_the_warrant_variants_have_the_same_constructors() {
    use vos_sched_contract::{Pick, Upd};
    // One value of every constructor (the `match`es above make a missing one a compile error
    // where the shape is defined, and the count check below catches a list left short).
    let upd: Vec<_> = [Upd::Changed, Upd::Unchanged, Upd::OutOfRange]
        .into_iter()
        .map(upd_shape)
        .collect();
    let pick: Vec<_> = [Pick::Idle, Pick::Run(3)]
        .into_iter()
        .map(pick_shape)
        .collect();
    let rust = |v: &[(usize, &'static str, usize)]| -> Vec<(String, usize)> {
        v.iter()
            .enumerate()
            .map(|(i, (ordinal, name, arity))| {
                assert_eq!(i, *ordinal, "declaration order");
                (name.to_string(), *arity)
            })
            .collect()
    };
    let src = assemble(&[CONTRACT]);
    with_file(&src, |file| {
        assert_eq!(variant_shape(file, "Upd"), rust(&upd));
        assert_eq!(variant_shape(file, "Pick"), rust(&pick));
    });
}

#[test]
fn pre_insert_is_exactly_the_ids_below_eight() {
    // The Warrant text writes N as the literal 8 (a document is per instance), and
    // `pre_insert` is true exactly for the ids below it. That the literal agrees with
    // `Sched::N` and with `config N 8` in contract.vos is NOT checked here: it needs the
    // `contract.vos` parser, which is `vos-manifest` (VOS-033, backlog VOS-D049).
    let src = assemble(&[
        CONTRACT,
        "  claim pre_matches_the_range:\n    forall b: Bool, t: U8. pre_insert(empty(), t) == (t < 8u8)\n",
    ]);
    with_file(&src, |file| {
        check_warrant(file).expect("type checks");
        let outcomes = run_claims(file, Some(&["pre_matches_the_range"]));
        assert!(outcomes[0].passed());
        assert_eq!(outcomes[0].admitted, 512);
    });
}
