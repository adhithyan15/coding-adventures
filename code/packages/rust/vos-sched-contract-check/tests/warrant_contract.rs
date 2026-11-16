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
//! * and, against deliberately wrong implementations, every obligation can
//!   fail (the non-vacuity check: a claim no implementation could break is not
//!   a claim).
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
const SAFETY: &str = include_str!("../../vos-sched-contract/warrant/obligations-safety.part");
const POLICY: &str = include_str!("../../vos-sched-contract/warrant/obligations-policy.part");
const CANONICAL: &str = include_str!("../../vos-sched-contract/warrant/obligations-canonical.part");
const ANY: &str = include_str!("../../vos-sched-contract/warrant/obligations-any.part");

const BITMAP_OPS: &str = include_str!("fixtures/impl-bitmap-ops.part");
const BITMAP: &str = include_str!("fixtures/impl-bitmap-models.part");
const SCAN: &str = include_str!("fixtures/impl-scan-models.part");
const ANY_IMPL: &str = include_str!("fixtures/impl-any-models.part");

/// The X1 case budget per claim (VOS30 section 8.5, VER04h section 3).
const BUDGET: u64 = 400_000;

/// Join blocks into one document. Nothing is parsed here.
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

/// The twelve obligations a production fixed-priority scheduler carries.
const PRODUCTION: [&str; 12] = [
    "insert_safe",
    "remove_safe",
    "dispatch_safe",
    "dispatch_policy",
    "canon_init",
    "canon_alpha_build",
    "canon_insert_post",
    "canon_insert_img",
    "canon_remove_post",
    "canon_remove_img",
    "canon_dispatch_post",
    "canon_dispatch_img",
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
    let (claims, examples) = check(&assemble(&[CONTRACT]));
    assert!(examples.is_empty());
    assert_eq!(claims.len(), 9, "the nine model-level lemmas: {claims:?}");
}

#[test]
fn lemmas_hold_over_their_whole_domains() {
    let outcomes = evaluate(&[CONTRACT], &LEMMAS);
    assert_all_pass(&outcomes);
    // 8 Bool binders = 256 states; with one U8 binder, 65,536 cases (16.4% of the budget).
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

#[test]
fn lemmas_can_fail() {
    // Non-vacuity of the lemmas: delete the membership test of the safety tier and
    // `safety_is_idle_or_a_member` must break (VOS01 section 3.5, "too weak").
    let weak = mutate(
        CONTRACT,
        "match o with Idle -> true | Run(t) -> t < 8u8 and mem(m, t))",
        "match o with Idle -> true | Run(t) -> t < 8u8)",
    );
    let outcomes = evaluate(&[&weak], &["safety_is_idle_or_a_member"]);
    let o = get(&outcomes, "safety_is_idle_or_a_member");
    assert!(
        !o.passed(),
        "a safety tier without membership must be caught"
    );
    // ... and a `top` that returns the minimum member breaks `top_is_the_maximum`.
    let wrong_top = mutate(
        CONTRACT,
        "if m.p7 then Run(7u8) else (if m.p6 then Run(6u8)",
        "if m.p0 then Run(0u8) else (if m.p6 then Run(6u8)",
    );
    let outcomes = evaluate(&[&wrong_top], &["top_is_the_maximum"]);
    assert!(!get(&outcomes, "top_is_the_maximum").passed());
}

// ---------------------------------------------------------------------------
// 2. The real shape: bound operations, every obligations block
// ---------------------------------------------------------------------------

#[test]
fn a_generated_shaped_document_type_checks() {
    // This is the document `vos-link gen-doc` will assemble for a bitmap
    // implementation: contract block, implementation block (operations, not
    // models), obligations blocks. Every claim of every block must type-check
    // against the operations of the fixed vocabulary.
    let (claims, examples) = check(&assemble(&[
        CONTRACT, BITMAP_OPS, SAFETY, POLICY, CANONICAL,
    ]));
    for obligation in [
        "insert_safe",
        "remove_safe",
        "dispatch_safe",
        "dispatch_policy",
        "canon_init",
        "canon_alpha_build",
        "canon_insert_post",
        "canon_insert_img",
        "canon_remove_post",
        "canon_remove_img",
        "canon_dispatch_post",
        "canon_dispatch_img",
    ] {
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
    // must be rejected: that rejection is what keeps a production crate from
    // carrying it.
    let ok = assemble(&[CONTRACT, ANY_IMPL, SAFETY, ANY]);
    check(&ok);
    let doc = assemble(&[CONTRACT, BITMAP, SAFETY, ANY]);
    let rejected = with_file(&doc, |file| check_warrant(file).is_err());
    assert!(
        rejected,
        "a bitmap Impl has no `choice` field and no 2-field constructor"
    );
}

// ---------------------------------------------------------------------------
// 3. Three implementations, evaluated over the whole domain of every claim
// ---------------------------------------------------------------------------

#[test]
fn bitmap_satisfies_safety_policy_and_canonical() {
    let obligations = evaluate(&[CONTRACT, BITMAP, SAFETY, POLICY, CANONICAL], &PRODUCTION);
    assert_all_pass(&obligations);
    assert_eq!(
        counts(&obligations),
        vec![
            ("insert_safe", 65_536, 65_536),
            ("remove_safe", 65_536, 65_536),
            ("dispatch_safe", 256, 256),
            ("dispatch_policy", 256, 256),
            ("canon_init", 2, 2),
            ("canon_alpha_build", 256, 256),
            ("canon_insert_post", 65_536, 65_536),
            ("canon_insert_img", 65_536, 65_536),
            ("canon_remove_post", 65_536, 65_536),
            ("canon_remove_img", 65_536, 65_536),
            ("canon_dispatch_post", 256, 256),
            ("canon_dispatch_img", 256, 256),
        ]
    );
    // The enumerated total VOS10 section 6 quotes for a production implementation.
    let total: u64 = obligations.iter().map(|o| o.admitted).sum();
    // six claims over (state, id), five over the state alone, and canon_init (2 cases)
    assert_eq!(total, 6 * 65_536 + 5 * 256 + 2);
    assert_eq!(total, 394_498);
}

#[test]
fn linear_scan_is_safe_and_canonical_but_not_fixed_priority() {
    let outcomes = evaluate(&[CONTRACT, SCAN, SAFETY, POLICY, CANONICAL], &PRODUCTION);
    for name in [
        "insert_safe",
        "remove_safe",
        "dispatch_safe",
        "canon_init",
        "canon_alpha_build",
        "canon_insert_post",
        "canon_insert_img",
        "canon_remove_post",
        "canon_remove_img",
        "canon_dispatch_post",
        "canon_dispatch_img",
    ] {
        let o = get(&outcomes, name);
        assert!(
            o.passed(),
            "{name} must hold for the scan: {:?}",
            o.first_failure
        );
    }
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
            // oracle and state both U8: 65,536; coverage's guard t < 8 admits 256 * 8.
            ("dispatch_sound", 65_536, 65_536),
            ("dispatch_covers_run", 65_536, 2_048),
            ("dispatch_covers_idle", 256, 256),
        ]
    );
}

#[test]
fn any_is_not_fixed_priority() {
    // Carrying the policy tier is what separates a production scheduler from -any:
    // with the oracle at 0 the register proposes thread 0 for every state that has it.
    let outcomes = evaluate(&[CONTRACT, ANY_IMPL, SAFETY, POLICY], &["dispatch_policy"]);
    assert!(!get(&outcomes, "dispatch_policy").passed());
}

#[test]
fn the_oracle_at_zero_is_canonical_so_kind_is_enforced_by_rule() {
    // VOS02 R6 says `any` cannot discharge `canonical` because `alpha` drops the oracle.
    // That argument rests on VOS02's `canon_build_alpha`, which names the fields of Impl
    // and so cannot be one fixed text for every implementation (VOS10 section 9). The
    // field-agnostic canonical block of this contract starts from build(M), where the
    // oracle is 0, and an oracle that nothing moves is canonical. So the exclusion of
    // `any` from `canonical` is a RULE of vos-link (kind any may not carry canonical),
    // and this test records the fact the rule exists for.
    let outcomes = evaluate(
        &[CONTRACT, ANY_IMPL, SAFETY, CANONICAL],
        &[
            "canon_init",
            "canon_alpha_build",
            "canon_dispatch_img",
            "canon_dispatch_post",
        ],
    );
    assert_all_pass(&outcomes);
}

// ---------------------------------------------------------------------------
// 4. Negative controls: every obligation can fail
// ---------------------------------------------------------------------------

/// The ideal fixed-priority bitmap with one operation replaced, evaluated against
/// the obligations; returns the names of the claims that fail.
fn failing_with(mutant: &str, claims: &[&str]) -> Vec<String> {
    evaluate(&[CONTRACT, mutant, SAFETY, POLICY, CANONICAL], claims)
        .into_iter()
        .filter(|o| !o.passed())
        .map(|o| o.name)
        .collect()
}

#[test]
fn control_dispatch_returns_idle_always_fails_only_policy() {
    // Always Idle, state unchanged: allowed by the safety tier, forbidden by the policy tier.
    let m = mutate(
        BITMAP,
        "model dispatch_out(s: Impl) -> Pick = if s.mask == 0u8 then Idle else Run(hibit(s.mask))",
        "model dispatch_out(s: Impl) -> Pick = Idle",
    );
    let m = mutate(
        &m,
        "model dispatch_img(s: Impl) -> Impl = (\n    if s.mask == 0u8 then s else Impl(bit_and(s.mask, bit_not(shl(1u8, to_nat(hibit(s.mask)))))))",
        "model dispatch_img(s: Impl) -> Impl = s",
    );
    assert_eq!(
        failing_with(&m, &PRODUCTION),
        vec!["dispatch_policy".to_string()]
    );
}

#[test]
fn control_dispatch_returns_a_non_member_fails_safety() {
    // Run(0) for the empty set: the thread is not ready.
    let m = mutate(
        BITMAP,
        "if s.mask == 0u8 then Idle else Run(hibit(s.mask))",
        "Run(hibit(s.mask))",
    );
    let failing = failing_with(&m, &["dispatch_safe", "dispatch_policy"]);
    assert!(
        failing.contains(&"dispatch_safe".to_string()),
        "{failing:?}"
    );
    assert!(
        failing.contains(&"dispatch_policy".to_string()),
        "{failing:?}"
    );
}

#[test]
fn control_dispatch_forgets_to_remove_fails_safety_and_canonical() {
    let m = mutate(
        BITMAP,
        "model dispatch_img(s: Impl) -> Impl = (\n    if s.mask == 0u8 then s else Impl(bit_and(s.mask, bit_not(shl(1u8, to_nat(hibit(s.mask)))))))",
        "model dispatch_img(s: Impl) -> Impl = s",
    );
    let failing = failing_with(
        &m,
        &["dispatch_safe", "dispatch_policy", "canon_dispatch_img"],
    );
    for name in ["dispatch_safe", "dispatch_policy", "canon_dispatch_img"] {
        assert!(failing.contains(&name.to_string()), "{name} in {failing:?}");
    }
}

#[test]
fn control_insert_ignores_the_range_check_fails_safety() {
    let m = mutate(
        BITMAP,
        "model insert_out(s: Impl, t: U8) -> Upd = (\n    if t >= 8u8 then OutOfRange else (if has(s, t) then Unchanged else Changed))",
        "model insert_out(s: Impl, t: U8) -> Upd = if has(s, t) then Unchanged else Changed",
    );
    // remove and dispatch are untouched: only insert_safe breaks
    let failing = failing_with(&m, &["insert_safe", "remove_safe", "dispatch_safe"]);
    assert_eq!(failing, vec!["insert_safe".to_string()]);
}

#[test]
fn control_remove_clears_the_wrong_bit_fails_safety_and_canonical() {
    let m = mutate(
        BITMAP,
        "if t >= 8u8 then s else Impl(bit_and(s.mask, bit_not(shl(1u8, to_nat(t))))))",
        "if t >= 8u8 then s else Impl(0u8))",
    );
    let failing = failing_with(&m, &["remove_safe", "canon_remove_img"]);
    assert!(failing.contains(&"remove_safe".to_string()), "{failing:?}");
    assert!(
        failing.contains(&"canon_remove_img".to_string()),
        "{failing:?}"
    );
}

#[test]
fn control_alpha_that_forgets_the_state_fails_the_canonical_alpha_claim() {
    // Shape C of VOS01 section 2.3: an `alpha` onto a single state would let any
    // implementation "refine". canon_alpha_build catches it.
    let m = mutate(
        BITMAP,
        "model alpha_w(s: Impl) -> AbsW = AbsW(s.mask)",
        "model alpha_w(s: Impl) -> AbsW = AbsW(0u8)",
    );
    let failing = failing_with(&m, &["canon_alpha_build"]);
    assert!(
        failing.contains(&"canon_alpha_build".to_string()),
        "{failing:?}"
    );
}

#[test]
fn control_a_wrong_initial_state_fails_canon_init() {
    let m = mutate(
        BITMAP,
        "model init() -> Impl = Impl(0u8)",
        "model init() -> Impl = Impl(1u8)",
    );
    assert_eq!(
        failing_with(&m, &["canon_init", "dispatch_safe", "canon_alpha_build"]),
        vec!["canon_init".to_string()]
    );
}

#[test]
fn control_a_scheduler_with_history_is_not_canonical() {
    // A round-robin scheduler keeps a rotation pointer that `alpha` forgets. Model it as
    // the oracle implementation whose dispatch moves the register: the post-state is
    // then not build(alpha(post)), which is exactly why VOS-012's round-robin scheduler
    // declares mode=representation (VOS02 section 2.7).
    let m = mutate(
        ANY_IMPL,
        "if proposal_ok(s) then Impl(bit_and(s.mask, bit_not(shl(1u8, to_nat(s.choice)))), s.choice) else s)",
        "if proposal_ok(s) then Impl(bit_and(s.mask, bit_not(shl(1u8, to_nat(s.choice)))), 1u8) else s)",
    );
    let outcomes = evaluate(
        &[CONTRACT, &m, SAFETY, CANONICAL],
        &["canon_dispatch_post", "dispatch_safe"],
    );
    assert!(!get(&outcomes, "canon_dispatch_post").passed());
    // Safety does not notice: the abstract behaviour is fine.
    assert!(get(&outcomes, "dispatch_safe").passed());
}

#[test]
fn control_any_that_never_proposes_a_thread_fails_coverage() {
    // Sound (Idle is always allowed) but not covering: thread 3 is never produced.
    let m = mutate(
        ANY_IMPL,
        "model proposal_ok(s: Impl) -> Bool = s.choice < 8u8 and has(s, s.choice)",
        "model proposal_ok(s: Impl) -> Bool = s.choice < 8u8 and has(s, s.choice) and not (s.choice == 3u8)",
    );
    let outcomes = evaluate(&[CONTRACT, &m, SAFETY, ANY], &ANY_CLAIMS);
    assert!(get(&outcomes, "dispatch_sound").passed());
    let cover = get(&outcomes, "dispatch_covers_run");
    assert!(!cover.passed());
    // thread 3 is ready in 128 of the 256 masks, so exactly those 128 cases fail
    assert_eq!(cover.failed, 128);
}

#[test]
fn control_any_that_always_proposes_fails_soundness_and_idle_coverage() {
    // Skip the validation step: the oracle can name a thread that is not ready.
    let m = mutate(
        ANY_IMPL,
        "model proposal_ok(s: Impl) -> Bool = s.choice < 8u8 and has(s, s.choice)",
        "model proposal_ok(s: Impl) -> Bool = s.choice < 8u8",
    );
    let outcomes = evaluate(&[CONTRACT, &m, SAFETY, ANY], &ANY_CLAIMS);
    assert!(!get(&outcomes, "dispatch_sound").passed());
    // The witness for Idle is the oracle value 255, which is not a thread id, so it is
    // rejected even without validation: coverage of Idle is unaffected.
    assert!(get(&outcomes, "dispatch_covers_idle").passed());
}

#[test]
fn control_any_that_answers_a_non_member_for_oracle_255_fails_idle_coverage() {
    // Oracle 255 is made to answer Run(7) for every non-empty mask, so the witness for
    // the permitted answer Idle no longer produces it.
    let m = mutate(
        ANY_IMPL,
        "model dispatch_out(s: Impl) -> Pick = if proposal_ok(s) then Run(s.choice) else Idle",
        "model dispatch_out(s: Impl) -> Pick = if proposal_ok(s) then Run(s.choice) else (if s.choice == 255u8 and not (s.mask == 0u8) then Run(7u8) else Idle)",
    );
    let outcomes = evaluate(&[CONTRACT, &m, SAFETY, ANY], &ANY_CLAIMS);
    // Run(7) is invalid for masks without bit 7: soundness catches the lie, and coverage
    // of Idle (oracle 255, every mask) fails for every non-empty mask.
    assert!(!get(&outcomes, "dispatch_sound").passed());
    assert_eq!(get(&outcomes, "dispatch_covers_idle").failed, 255);
}

// ---------------------------------------------------------------------------
// 5. The text itself
// ---------------------------------------------------------------------------

#[test]
fn parts_follow_the_vos02_marker_discipline() {
    // VOS02 section 2.3: a part may not contain a line beginning `-- vos02:`, a CR or
    // any control character but LF, and every part is ASCII.
    for (name, text) in [
        ("contract", CONTRACT),
        ("safety", SAFETY),
        ("policy", POLICY),
        ("canonical", CANONICAL),
        ("any", ANY),
    ] {
        assert!(text.is_ascii(), "{name}.part is not ASCII");
        assert!(
            text.bytes()
                .all(|b| b == b'\n' || (0x20..0x7f).contains(&b)),
            "{name}.part has a control character"
        );
        assert!(text.ends_with('\n'), "{name}.part has no final LF");
        for line in text.lines() {
            assert!(
                !line.trim_start().starts_with("-- vos02:"),
                "{name}.part forges a marker"
            );
            assert!(
                line.is_empty() || line.starts_with("  "),
                "{name}.part line not indented: {line}"
            );
            assert!(!line.ends_with(' '), "{name}.part trailing space: {line}");
        }
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

#[test]
fn the_rust_enums_and_the_warrant_variants_have_the_same_constructors() {
    use vos_sched_contract::{Pick, Upd};
    // One value of every constructor, in declaration order; Debug prints the name.
    let rust_upd: Vec<(String, usize)> = [Upd::Changed, Upd::Unchanged, Upd::OutOfRange]
        .iter()
        .map(|u| (format!("{u:?}"), 0))
        .collect();
    let rust_pick = vec![
        (format!("{:?}", Pick::Idle), 0),
        // `Run(3)` prints as `Run(3)`: the name is what precedes the payload, one field
        (
            format!("{:?}", Pick::Run(3))
                .split('(')
                .next()
                .unwrap()
                .to_string(),
            1,
        ),
    ];
    let src = assemble(&[CONTRACT]);
    with_file(&src, |file| {
        assert_eq!(variant_shape(file, "Upd"), rust_upd);
        assert_eq!(variant_shape(file, "Pick"), rust_pick);
    });
}

#[test]
fn the_configuration_constant_is_pinned_by_the_text_at_eight() {
    // The Warrant text writes N as the literal 8 (a document is per instance). A
    // scheduler whose `Sched::N` is 8 is the instance the text is about; `pre_insert`
    // and `pre_remove` are true exactly for the ids below it.
    use vos_sched_contract::Sched;
    struct Instance;
    impl Sched for Instance {
        type State = ();
        const N: u8 = 8;
        fn init() {}
        fn insert(s: (), _: u8) -> (vos_sched_contract::Upd, ()) {
            (vos_sched_contract::Upd::Unchanged, s)
        }
        fn remove(s: (), _: u8) -> (vos_sched_contract::Upd, ()) {
            (vos_sched_contract::Upd::Unchanged, s)
        }
        fn dispatch(s: ()) -> (vos_sched_contract::Pick, ()) {
            (vos_sched_contract::Pick::Idle, s)
        }
    }
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
    assert_eq!(Instance::N, 8);
    assert_eq!(<Instance as Sched>::init(), ());
}
