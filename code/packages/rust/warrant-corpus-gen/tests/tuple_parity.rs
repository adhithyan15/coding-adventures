//! The S10t-2 acceptance test: the Rust generator and the independent oracle agree, byte for byte,
//! on every `ir.tuple` form (VER04r section 13, slice S10t, [D20], [D35], R9 to R12).
//!
//! `python/warrant-oracle` committed, in slice S10t-1, a synthetic document that uses every tuple
//! form (the literal at arity 2 to 8, nesting, a typed literal context, `project` by name, by chain
//! and after a call, `==` and `!=`, tuple binders and tuple op parameters), every `match` pattern
//! (`ok err err_any tuple bind wild`, nested tuples, nested `Result`s, a `match` that returns a
//! tuple and a `Result`, a `match` in an op `admit`, a pure scrutinee in the expected side, a
//! subject call as scrutinee and one inside an arm) and every `ir.tuple` fault template
//! (`tuple_forms_v1.warrant` and its `.profile`), and four committed dumps of what the oracle
//! computes from them:
//!
//! | dump | schema | what this test reproduces it from |
//! |---|---|---|
//! | `.oracle-lowering.json` | `warrant-lowering-parity-v1` | [`view::oracle_view_v1`] of the lowered document |
//! | `.oracle-evaluation.json` | `warrant-evaluation-parity-v1` | the generated corpus (`corpus.json`, chunk files, `cases.tfv`) |
//! | `.oracle-faults.json` | `warrant-fault-parity-v1` | the same corpus twice: the shipped fault gate (`ir.core`) and `ir.core` + `ir.tuple` |
//! | `.oracle-refusals.json` | `warrant-refusals-parity-v1` | each corpus-error document, refused with the generator code `ir` |
//!
//! The files are read from the oracle's directory (no copy), so an edit to any of them reruns this
//! test (the build tool's repository-input row `rust/warrant-corpus-gen`).  The chain is: oracle
//! source -> committed dump == this crate's output.  The evaluation and fault dumps carry no
//! generator name or version, so the version bump of the flip (S10c) does not move them.
//!
//! The faults dump is **normative** for the composite fault JSON (`ok-as-err`, `drop-consumed`,
//! `swap-components`, the tuple-payload `missing-error`, VER04r 16.10 R11): a fault row here is the
//! oracle's byte for byte.  Its second control view (`controls_gate_on`) is what the flip turns
//! on; until then the generator reaches it through [`generate_gated`], a library parameter.
//!
//! The negative controls at the bottom show each comparison can fail.

mod support;

use coding_adventures_warrant_corpus_gen::error::{code, GenError};
use coding_adventures_warrant_corpus_gen::faults::Level;
use coding_adventures_warrant_corpus_gen::json::Json;
use coding_adventures_warrant_corpus_gen::lower::{lower_document, lower_document_omitting};
use coding_adventures_warrant_corpus_gen::view::oracle_view_v1;
use support::minijson::{parse, J};
use support::parity::*;
use support::*;

const DIR: &str = "../../../python/warrant-oracle/tests/fixtures/tuple-forms-v1";
const WARRANT: &str = include_str!(
    "../../../python/warrant-oracle/tests/fixtures/tuple-forms-v1/tuple_forms_v1.warrant"
);
const PROFILE: &str = include_str!(
    "../../../python/warrant-oracle/tests/fixtures/tuple-forms-v1/tuple_forms_v1.profile"
);
const LOWERING: &str = include_str!(
    "../../../python/warrant-oracle/tests/fixtures/tuple-forms-v1/tuple_forms_v1.oracle-lowering.json"
);
const EVALUATION: &str = include_str!(
    "../../../python/warrant-oracle/tests/fixtures/tuple-forms-v1/tuple_forms_v1.oracle-evaluation.json"
);
const FAULTS: &str = include_str!(
    "../../../python/warrant-oracle/tests/fixtures/tuple-forms-v1/tuple_forms_v1.oracle-faults.json"
);
const REFUSALS: &str = include_str!(
    "../../../python/warrant-oracle/tests/fixtures/tuple-forms-v1/tuple_forms_v1.oracle-refusals.json"
);

/// The specification's path as written into `corpus.json`.
const SPEC: &str = "tuple_forms_v1.warrant";

/// The bound the oracle's `dump-lowering` costs under by default (`MAX_IR_COST`).
const LIMIT: u64 = 2_000;

/// The gate of `controls_gate_on`: `ir.core` and `ir.tuple` evaluated, `ir.fixed` not.
const GATE_ON: [Level; 2] = [Level::Core, Level::Tuple];

/// The `omit_example` lines of the profile (the two match-shaped examples).
fn omitted() -> Vec<String> {
    PROFILE
        .lines()
        .filter_map(|l| l.strip_prefix("omit_example "))
        .map(str::to_string)
        .collect()
}

// ---- the lowering dump ---------------------------------------------------------------------------

fn lowering_text(src: &str) -> String {
    with_file(src, |f| {
        let l = lower_document_omitting(f, &omitted()).expect("lowers");
        oracle_view_v1(&l, LIMIT)
            .expect("view")
            .render()
            .expect("render")
    })
}

#[test]
fn the_tuple_forms_document_lowers_to_the_oracle_dump_byte_for_byte() {
    let text = lowering_text(WARRANT);
    assert!(text == LOWERING, "{}", first_difference(&text, LOWERING));
}

// ---- the evaluation and fault dumps ----------------------------------------------------------

/// `warrant-fault-parity-v1`: the fault rows and the controls under both gates (the oracle's
/// `parity_dumps.faults_view`).
fn faults_view(src: &str, profile: &str) -> Json {
    let shipped = generated(src, SPEC, profile, None).expect("generates (shipped gate)");
    let on = generated(src, SPEC, profile, Some(&GATE_ON)).expect("generates (ir.tuple evaluated)");
    let s = parse(&shipped.corpus_json).expect("parses");
    let g = parse(&on.corpus_json).expect("parses");
    assert_eq!(
        s.get("faults"),
        g.get("faults"),
        "the fault rows never depend on the gate"
    );
    Json::obj([
        ("schema", Json::str("warrant-fault-parity-v1")),
        ("module", to_json(s.get("module"))),
        ("faults", to_json(s.get("faults"))),
        ("controls_shipped", to_json(s.get("controls"))),
        ("controls_gate_on", to_json(g.get("controls"))),
        (
            "accepted_blind_faults",
            to_json(s.get("accepted_blind_faults")),
        ),
        ("levels_shipped", to_json(s.get("levels"))),
        ("levels_gate_on", to_json(g.get("levels"))),
    ])
}

fn evaluation_text(src: &str, profile: &str) -> String {
    evaluation_view(&generated(src, SPEC, profile, None).expect("generates"))
        .render()
        .expect("render")
}

#[test]
fn the_tuple_forms_corpus_matches_the_oracle_evaluation_dump_byte_for_byte() {
    let text = evaluation_text(WARRANT, PROFILE);
    assert!(text == EVALUATION, "{}", first_difference(&text, EVALUATION));
}

#[test]
fn the_fault_rows_and_controls_match_the_oracle_fault_dump_byte_for_byte() {
    let text = faults_view(WARRANT, PROFILE).render().expect("render");
    assert!(text == FAULTS, "{}", first_difference(&text, FAULTS));
}

// ---- the refusals dump ------------------------------------------------------------------------

/// The contract boilerplate (`assume`, `labels`, `input`, `bound`) for the two `Bool` operations
/// `f` and `g` every refusal document declares, so the S2 checker reaches the `match` and not a
/// missing contract.
const CONTRACTS: &str = "  assume platform:\n    none\n  assume environment:\n    none\n  labels f:\n    a secret\n  labels g:\n    a secret\n  input f.a:\n    hostile\n    ensures no_panic, terminates\n  input g.a:\n    hostile\n    ensures no_panic, terminates\n  bound f:\n    steps <= 64\n    heap <= 0\n    depth <= 8\n  bound g:\n    steps <= 64\n    heap <= 0\n    depth <= 8\n";

fn refused(source: &str) -> GenError {
    let src = format!("{source}{CONTRACTS}");
    with_file(&src, |f| {
        lower_document(f).expect_err("the document is refused")
    })
}

fn refusals() -> Vec<J> {
    let dump = parse(REFUSALS).expect("the refusals dump parses");
    assert_eq!(dump.get("schema").s(), "warrant-refusals-parity-v1");
    dump.get("refusals").a().to_vec()
}

#[test]
fn every_corpus_error_document_is_refused_with_the_generator_code_ir_and_the_oracles_sentence() {
    let mut corpus_errors = 0;
    for r in refusals() {
        let (name, source) = (r.get("name").s(), r.get("source").s());
        let e = refused(source);
        match r.get("generator_code").s() {
            "ir" => {
                assert_eq!(r.get("oracle_code").s(), "corpus", "{name}");
                assert_eq!(e.code, code::IR, "{name}: {e}");
                // the generator names the same corpus error as the oracle, in the oracle's order
                assert_eq!(e.message, r.get("message").s(), "{name}");
                corpus_errors += 1;
            }
            "" => {
                // not a corpus error: the oracle refuses with `type` or `level`; the generator
                // refuses too (its checker or its typer), and never accepts
                assert_ne!(r.get("oracle_code").s(), "corpus", "{name}");
                assert_ne!(e.code, code::IR, "{name}: {e}");
            }
            other => panic!("{name}: an unknown generator code {other}"),
        }
    }
    // 3 that fall through, 6 with an unreachable arm, 2 duplicate binds and 5 shadowing binds
    assert_eq!(corpus_errors, 16);
}

#[test]
fn the_oracle_orders_unreachable_before_fall_through_and_the_generator_agrees() {
    // R12: `Ok(_) -> .. | Ok((a, b)) -> ..` is both non-exhaustive and has an unreachable arm
    let both = refusals()
        .into_iter()
        .find(|r| r.get("name").s() == "fall-through-partial-tuple")
        .expect("the dump has the document");
    assert_eq!(both.get("message").s(), "a match arm that no value can reach");
    assert_eq!(refused(both.get("source").s()).message, both.get("message").s());
}

// ---- the dumps are canonical ------------------------------------------------------------------

#[test]
fn the_dumps_are_canonical_json_of_their_schemas() {
    for (name, d, schema) in [
        ("lowering", LOWERING, "warrant-lowering-parity-v1"),
        ("evaluation", EVALUATION, "warrant-evaluation-parity-v1"),
        ("faults", FAULTS, "warrant-fault-parity-v1"),
        ("refusals", REFUSALS, "warrant-refusals-parity-v1"),
    ] {
        assert!(!d.contains('\r'), "{name}");
        assert!(d.ends_with("}\n") && !d.ends_with("\n\n"), "{name}");
        assert!(d.is_ascii(), "{name}");
        assert!(d.contains(&format!("\"schema\": \"{schema}\"")), "{name}");
    }
    assert!(DIR.ends_with("tuple-forms-v1"));
}

#[test]
fn the_document_really_has_every_tuple_form_and_every_match_pattern() {
    // not a vacuous parity: the lowered document is full of the new nodes
    let text = lowering_text(WARRANT);
    for needle in [
        r#""tuple""#,
        r#""project""#,
        r#""match""#,
        r#""err_any""#,
        r#""wild""#,
        r#""bind""#,
    ] {
        assert!(text.matches(needle).count() > 10, "{needle}");
    }
    let eval = evaluation_text(WARRANT, PROFILE);
    assert!(eval.contains(r#""d": "tuple""#));
    assert!(eval.contains("L2 T"), "the TFV tuple form");
    assert!(eval.contains("warrant-stream-v2"));
}

// ---- negative controls ------------------------------------------------------------------------

#[test]
fn a_mis_lowered_tuple_component_loses_lowering_parity() {
    // swap the two projections of `swap`: a one-node difference in one model
    let mutated = WARRANT.replacen("= (t.1, t.0)", "= (t.0, t.1)", 1);
    assert_ne!(mutated, WARRANT);
    assert_ne!(lowering_text(&mutated), LOWERING);
}

#[test]
fn a_mis_lowered_pattern_loses_lowering_parity() {
    let mutated = WARRANT.replacen("Ok((a, b)) -> a and b", "Ok((a, b)) -> a or b", 1);
    assert_ne!(mutated, WARRANT);
    assert_ne!(lowering_text(&mutated), LOWERING);
}

#[test]
fn a_changed_model_changes_the_evaluation_dump() {
    // `!=` evaluated as `==`: every digest of c_tup_ne moves
    let mutated = WARRANT.replacen(
        "model tup_ne(a: (Bool,Bool), b: (Bool,Bool)) -> Bool = a != b",
        "model tup_ne(a: (Bool,Bool), b: (Bool,Bool)) -> Bool = a == b",
        1,
    );
    assert_ne!(mutated, WARRANT);
    let text = evaluation_text(&mutated, PROFILE);
    assert_ne!(text, EVALUATION);
    // and nothing else moved but that suite's expected digests (the inputs do not depend on it)
    let before = parse(EVALUATION).unwrap();
    let after = parse(&text).unwrap();
    let (b, a) = (
        before.get("suites").get("c_tup_ne"),
        after.get("suites").get("c_tup_ne"),
    );
    assert_ne!(b.get("expected_sha256"), a.get("expected_sha256"));
    assert_eq!(b.get("inputs_sha256"), a.get("inputs_sha256"));
}

#[test]
fn a_dropped_arm_of_the_scrutinee_value_changes_the_call_count() {
    // an arm that no longer calls the subject: `expected_calls` of c_arm_calls moves
    let mutated = WARRANT.replacen(
        "match r with Ok(x) -> op_inc(x) == x + 1u8 | Err(_) -> true",
        "match r with Ok(x) -> x == x + 1u8 | Err(_) -> true",
        1,
    );
    assert_ne!(mutated, WARRANT);
    // the claim now calls no operation: refused (W0701), never silently emitted
    let e = generated(&mutated, SPEC, PROFILE, None).expect_err("a claim must call an op");
    assert_eq!(e.code, code::W0701, "{e}");
}

#[test]
fn a_changed_profile_changes_the_fault_dump() {
    // dropping a fault line drops its row and its control
    let mutated = PROFILE.replacen("fault op_k ok-as-err\n", "", 1);
    assert_ne!(mutated, PROFILE);
    let text = faults_view(WARRANT, &mutated).render().unwrap();
    assert_ne!(text, FAULTS);
    assert!(!text.contains("op_k/ok-as-err"));
    assert!(FAULTS.contains("op_k/ok-as-err"));
}

#[test]
fn the_gate_changes_exactly_the_witnesses_of_the_gated_faults() {
    // not a vacuous second view: the shipped gate defers, the gate that evaluates ir.tuple finds
    // witnesses, and a fault that also needs ir.fixed stays deferred
    let shipped = generated(WARRANT, SPEC, PROFILE, None).unwrap();
    let on = generated(WARRANT, SPEC, PROFILE, Some(&GATE_ON)).unwrap();
    assert_ne!(shipped.corpus_json, on.corpus_json);
    assert!(shipped.corpus_json.contains(r#""deferred": "ir.tuple""#));
    assert!(!on.corpus_json.contains(r#""deferred": "ir.tuple""#));
    assert!(on.corpus_json.contains(r#""deferred": "ir.fixed""#));
    // the shipped gate is the default of `generate`
    let again = generated(WARRANT, SPEC, PROFILE, Some(&[Level::Core])).unwrap();
    assert_eq!(shipped, again);
}

#[test]
fn a_document_that_fell_through_would_not_be_accepted_by_a_weakened_refusal_check() {
    // each of the 16 documents is refused; making one a valid match (adding the missing arm)
    // generates, so the refusal is the pattern check and not some other rule
    let fixed = "warrant refusals v1\n  error E = A | B | C\n  op f(a: Bool) -> Result<(Bool,Bool),E>\n  op g(a: Bool) -> Result<Bool,E>\n  model m(r: Result<Bool,E>) -> Bool = match r with Ok(_) -> true | Err(_) -> false\n  claim c:\n    forall a: Bool. g(a) == Ok(m(Ok(a)))\n";
    let src = format!("{fixed}{CONTRACTS}");
    with_file(&src, |f| {
        lower_document(f).expect("a total match lowers");
    });
    let broken = src.replace("Ok(_) -> true | Err(_) -> false", "Ok(_) -> true");
    assert_ne!(broken, src);
    let e = with_file(&broken, |f| lower_document(f).expect_err("falls through"));
    assert_eq!(e.code, code::IR);
}

// ---- the lowering-time guard rule treats `match` like `if` (CV2-F1) ----------------------------

fn claim_doc(claim_body: &str) -> String {
    format!(
        "warrant refusals v1\n  error E = A | B\n  op f(a: Bool) -> Result<Bool,E>\n  op g(a: Bool) -> Result<Bool,E>\n  claim c:\n    forall a: Bool, r: Result<Bool,E>. {claim_body}\n{CONTRACTS}"
    )
}

#[test]
fn a_match_whose_arm_calls_the_subject_needs_a_pure_scrutinee_at_lowering() {
    // a subject call as the scrutinee AND in an arm: the scrutinee is an impure guard
    let impure = claim_doc("match g(a) with Ok(x) -> f(x) == Ok(x) | Err(_) -> true");
    let e = with_file(&impure, |f| lower_document(f).expect_err("refused"));
    assert_eq!(e.code, code::W0701, "{e}");
    assert!(e.message.contains("impure guard"), "{e}");
    // a pure scrutinee with the call in an arm: accepted
    let pure = claim_doc("match r with Ok(x) -> f(x) == Ok(x) | Err(_) -> true");
    with_file(&pure, |f| {
        lower_document(f).expect("a pure scrutinee guards the arm");
    });
    // no arm calls: the scrutinee may call (nothing is guarded)
    let scrut = claim_doc("match g(a) with Ok(x) -> x | Err(_) -> true");
    with_file(&scrut, |f| {
        lower_document(f).expect("a call in the scrutinee alone guards nothing");
    });
}
