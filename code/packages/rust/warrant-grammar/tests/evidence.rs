//! VER04h: the evidence generator is tested by *compiling and running* what it
//! emits against the real `coding_adventures_ct_compare` crate (a generator
//! tested only by matching text can emit code that never compiled).

use std::collections::BTreeMap;

use coding_adventures_sha256::sha256_hex;
use coding_adventures_warrant_grammar::binding::{BindingManifest, ErrorNormalization, OpBinding};
use coding_adventures_warrant_grammar::checker::check_warrant;
use coding_adventures_warrant_grammar::codegen::{CodegenErrorKind, DiagnosticCode};
use coding_adventures_warrant_grammar::evidence::{
    compile_rust_evidence_harness, generate_evidence_harness, EvidenceOptions,
};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits, WarrantFile};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};
use coding_adventures_test_bootstrap::Verdict;

/// The checked-in generated target, compiled here as a module so a generator
/// bug that emits uncompilable code fails this test binary's build.
#[allow(dead_code, unused_variables, unused_parens, clippy::all)]
mod harness {
    include!("../../ct-compare/tests/generated/warrant_evidence.rs");
}
use harness::Fault;

const SPEC: &str = include_str!("../../../../specs/ct-compare.warrant");
const CHECKED_IN: &str = include_str!("../../ct-compare/tests/generated/warrant_evidence.rs");

fn with_parsed<R>(src: &str, f: impl FnOnce(&WarrantFile) -> R) -> R {
    let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");
    f(&file)
}

fn bind(export: &str, widths: &[&str], fresh: bool, norm: Option<&str>) -> OpBinding {
    OpBinding {
        export: export.into(),
        static_params: widths.iter().map(|s| s.to_string()).collect(),
        fresh,
        error_normalization: norm.map(|c| ErrorNormalization {
            kind: "panic_to_error".into(),
            error_case: c.into(),
        }),
        negative_controls: vec!["x".into()],
    }
}

fn ct_bindings() -> BindingManifest {
    let mut b = BTreeMap::new();
    b.insert("eq".into(), bind("ct_eq", &[], false, None));
    b.insert("eq_fixed".into(), bind("ct_eq_fixed", &["N"], false, None));
    b.insert(
        "select_bytes".into(),
        bind("ct_select_bytes", &[], true, Some("CT_SELECT_LENGTH_MISMATCH")),
    );
    b.insert("eq_u64".into(), bind("ct_eq_u64", &[], false, None));
    BindingManifest {
        package: "coding_adventures_ct_compare".into(),
        revision: "0.1.0".into(),
        bindings: b,
    }
}

fn options(spec: &str) -> EvidenceOptions {
    EvidenceOptions {
        package_crate: "coding_adventures_ct_compare".into(),
        spec_path: "code/specs/ct-compare.warrant".into(),
        spec_sha256: sha256_hex(spec.as_bytes()),
        seed: 0x5741_5252_414e_5421,
        property_cases: 4096,
        fuzz_iterations: 5000,
        negctl_fuzz_iterations: 1000,
    }
}

fn generate_src(src: &str, bindings: &BindingManifest) -> Result<String, (DiagnosticCode, String)> {
    with_parsed(src, |file| {
        generate_evidence_harness(file, bindings, &options(src))
            .map_err(|e| (e.code, format!("{e}")))
    })
}

fn is_fail(v: &Verdict, id: &str) -> bool {
    matches!(v, Verdict::Fail { failure_id, .. } if *failure_id == id)
}

#[test]
fn checked_in_harness_matches_the_spec_and_generator() {
    let generated = with_parsed(SPEC, |file| {
        let checked = check_warrant(file).expect("spec must type-check");
        compile_rust_evidence_harness(file, &checked, &ct_bindings(), &options(SPEC))
            .expect("generation must succeed")
    });
    assert_eq!(
        generated, CHECKED_IN,
        "tests/generated/warrant_evidence.rs is stale; run the gen_evidence example"
    );
    assert!(CHECKED_IN.contains(&sha256_hex(SPEC.as_bytes())));
}

#[test]
fn every_warrant_claim_and_example_gets_every_suite_kind() {
    for claim in [
        "eq_agrees",
        "eq_fixed_agrees_w0",
        "eq_fixed_agrees_w16",
        "select_picks",
        "select_rejects_mismatch",
        "eq_u64_agrees",
    ] {
        for kind in ["exhaustive", "property", "fuzz"] {
            assert!(
                CHECKED_IN.contains(&format!("\"warrant/ct_compare/{kind}/{claim}\"")),
                "{kind}/{claim} missing"
            );
        }
    }
    assert!(CHECKED_IN.contains("\"warrant/ct_compare/example/u64_diff\""));
    assert!(CHECKED_IN.contains("negctl/exhaustive/select_bytes/swap-choice"));
}

#[test]
fn generated_suites_pass_against_the_real_crate() {
    let (v, s) = harness::exh_eq_fixed_agrees_w4(Fault::None);
    assert_eq!(v, Verdict::Pass);
    assert_eq!(s.cases, 256);
    let (v, s) = harness::exh_select_rejects_mismatch(Fault::None);
    assert_eq!(v, Verdict::Pass);
    assert!(s.cases > 0 && s.discarded > 0);
    let (v, s) = harness::prop_eq_agrees(Fault::None);
    assert_eq!(v, Verdict::Pass);
    assert_eq!(s.cases, 4096);
    // The relation bias must actually reach the equal branch.
    assert!(s.equal > 500, "equal branch hit only {} times", s.equal);
    let (v, s) = harness::fuz_eq_u64_agrees(Fault::None, 300);
    assert_eq!(v, Verdict::Pass);
    assert!(s.iterations > 0 && s.admitted > 0);
    assert!(harness::example_select_left_fresh(Fault::None));
    assert!(harness::example_eq_empty(Fault::None));
}

#[test]
fn every_fault_is_detected_with_a_registered_failure_id() {
    let cases: [(&str, Verdict, &str); 8] = [
        ("invert/exh", harness::exh_eq_agrees(Fault::Invert).0, "property-counterexample"),
        ("last/exh", harness::exh_eq_fixed_agrees_w2(Fault::IgnoreLastByte).0, "property-counterexample"),
        ("high/exh", harness::exh_eq_u64_agrees(Fault::IgnoreHighBit).0, "property-counterexample"),
        ("swap/exh", harness::exh_select_picks(Fault::SwapChoice).0, "property-counterexample"),
        ("miss/exh", harness::exh_select_rejects_mismatch(Fault::MissingError).0, "property-counterexample"),
        ("invert/prop", harness::prop_eq_agrees(Fault::Invert).0, "property-counterexample"),
        ("invert/fuzz", harness::fuz_eq_agrees(Fault::Invert, 300).0, "fuzz-crash"),
        ("last/prop", harness::prop_eq_agrees(Fault::IgnoreLastByte).0, "property-counterexample"),
    ];
    for (name, verdict, id) in cases {
        assert!(is_fail(&verdict, id), "{name}: {verdict:?}");
    }
    assert!(!harness::example_eq_empty(Fault::Invert));
}

#[test]
fn fault_names_round_trip() {
    for f in [
        Fault::None,
        Fault::Invert,
        Fault::IgnoreLastByte,
        Fault::IgnoreHighBit,
        Fault::SwapChoice,
        Fault::MissingError,
    ] {
        assert_eq!(Fault::parse(f.name()), Some(f));
    }
    assert_eq!(Fault::parse("bogus"), None);
}

#[test]
fn generator_refuses_what_it_cannot_translate() {
    let head = "warrant t\n  op eq(a: Bytes, b: Bytes) -> Bool\n  assume platform:\n    none\n  assume environment:\n    none\n  bound eq:\n    steps <= 10\n";
    let mut b = BTreeMap::new();
    b.insert("eq".to_string(), bind("ct_eq", &[], false, None));
    let bindings = BindingManifest {
        package: "p".into(),
        revision: "0".into(),
        bindings: b,
    };

    // No claims at all.
    let (code, msg) = generate_src(&format!("{head}  example e:\n    eq(b\"\", b\"\") == true\n"), &bindings).unwrap_err();
    assert_eq!(code, DiagnosticCode::W0701, "{msg}");

    // Existential claim.
    let src = format!("{head}  claim c:\n    exists a: Bytes. eq(a, a) == true\n");
    assert_eq!(generate_src(&src, &bindings).unwrap_err().0, DiagnosticCode::W0701);

    // A claim over three Bytes binders blows the exhaustive budget (W0706).
    let src = format!(
        "{head}  claim c:\n    forall a: Bytes, b: Bytes, c: Bytes. eq(a, b) == eq(b, a)\n"
    );
    assert_eq!(generate_src(&src, &bindings).unwrap_err().0, DiagnosticCode::W0706);

    // Unknown call inside a claim.
    let src = format!("{head}  claim c:\n    forall a: Bytes. eq(a, a) == nosuch(a)\n");
    assert_eq!(generate_src(&src, &bindings).unwrap_err().0, DiagnosticCode::W0701);

    // Unsupported binder type.
    let src = format!("{head}  claim c:\n    forall a: Int. eq(b\"\", b\"\") == true\n");
    assert_eq!(generate_src(&src, &bindings).unwrap_err().0, DiagnosticCode::W0701);

    // Match / list forms are not translated.
    let src = format!("{head}  claim c:\n    forall a: Bytes. eq(a, a) == [true]\n");
    assert_eq!(generate_src(&src, &bindings).unwrap_err().0, DiagnosticCode::W0701);

    // A claim that calls no operation.
    let src = format!("{head}  claim c:\n    forall a: Bytes. a == a\n");
    assert_eq!(generate_src(&src, &bindings).unwrap_err().0, DiagnosticCode::W0701);
}

#[test]
fn generator_refuses_unadaptable_signatures_and_missing_normalization() {
    // Three-Bytes operation has no adapter shape.
    let src = "warrant t\n  op tri(a: Bytes, b: Bytes, c: Bytes) -> Bool\n  assume platform:\n    none\n  assume environment:\n    none\n  bound tri:\n    steps <= 10\n  claim c:\n    forall a: Bytes. tri(a, a, a) == true\n";
    let mut b = BTreeMap::new();
    b.insert("tri".to_string(), bind("tri", &[], false, None));
    let bindings = BindingManifest { package: "p".into(), revision: "0".into(), bindings: b };
    assert_eq!(generate_src(src, &bindings).unwrap_err().0, DiagnosticCode::W0701);

    // select shape without error normalization is a binding error.
    let src = "warrant t\n  error E = BAD\n  op sel(a: Bytes, b: Bytes, c: Bool) -> Result<Bytes, E> fresh\n  assume platform:\n    none\n  assume environment:\n    none\n  bound sel:\n    steps <= 10\n  claim c:\n    forall a: Bytes, b: Bytes | len(a) == len(b), c: Bool. sel(a, b, c) == Ok(a)\n";
    let mut b = BTreeMap::new();
    b.insert("sel".to_string(), bind("sel", &[], true, None));
    let bindings = BindingManifest { package: "p".into(), revision: "0".into(), bindings: b };
    let (code, msg) = generate_src(src, &bindings).unwrap_err();
    assert_eq!(code, DiagnosticCode::W0704, "{msg}");
    let _ = CodegenErrorKind::DivisionByZero;
}

#[test]
fn missing_binding_is_rejected_by_validation() {
    with_parsed(SPEC, |file| {
        let checked = check_warrant(file).unwrap();
        let mut b = ct_bindings();
        b.bindings.remove("eq_u64");
        let err = compile_rust_evidence_harness(file, &checked, &b, &options(SPEC)).unwrap_err();
        assert_eq!(err.code, DiagnosticCode::W0704);
    });
}

#[test]
fn model_calling_an_operation_is_rejected() {
    let src = "warrant t\n  model m(a: Bytes) -> Bool = eq(a, a)\n  op eq(a: Bytes, b: Bytes) -> Bool\n  assume platform:\n    none\n  assume environment:\n    none\n  bound eq:\n    steps <= 10\n  claim c:\n    forall a: Bytes. eq(a, a) == m(a)\n";
    let mut b = BTreeMap::new();
    b.insert("eq".to_string(), bind("ct_eq", &[], false, None));
    let bindings = BindingManifest { package: "p".into(), revision: "0".into(), bindings: b };
    assert_eq!(generate_src(src, &bindings).unwrap_err().0, DiagnosticCode::W0701);
}

// ---- audit A1/A2: the exhaustive budget must not wrap or count only elements ----

fn eq_head() -> (&'static str, BindingManifest) {
    let head = "warrant t\n  op eq(a: Bytes, b: Bytes) -> Bool\n  assume platform:\n    none\n  assume environment:\n    none\n  bound eq:\n    steps <= 10\n";
    let mut b = BTreeMap::new();
    b.insert("eq".to_string(), bind("ct_eq", &[], false, None));
    (head, BindingManifest { package: "p".into(), revision: "0".into(), bindings: b })
}

#[test]
fn a_wrapping_domain_product_is_over_budget_not_zero() {
    // Eight U8 binders: 256^8 = 2^64, which wraps to 0 in u64 and used to slip
    // under the budget (the enumeration would then never finish).
    let (head, bindings) = eq_head();
    let binders: Vec<String> = (0..8).map(|i| format!("x{i}: U8")).collect();
    let src = format!("{head}  claim c:\n    forall {}. eq(b\"\", b\"\") == true\n", binders.join(", "));
    let (code, msg) = generate_src(&src, &bindings).unwrap_err();
    assert_eq!(code, DiagnosticCode::W0706, "{msg}");
    // A product that wraps to a *non-zero* value is refused as well.
    let binders: Vec<String> = (0..9).map(|i| format!("x{i}: U8")).collect();
    let src = format!("{head}  claim c:\n    forall {}. eq(b\"\", b\"\") == true\n", binders.join(", "));
    assert_eq!(generate_src(&src, &bindings).unwrap_err().0, DiagnosticCode::W0706);
}

#[test]
fn the_prelude_budget_check_is_checked_arithmetic_too() {
    use harness::{domain, domain_bytes, domain_size, within_budget, Ty};
    assert!(!within_budget(&[Ty::U8; 8]), "2^64 wraps to 0 and must not pass");
    assert!(!within_budget(&[Ty::U8; 9]));
    assert!(!within_budget(&[Ty::BytesN(4097)]));
    assert!(!within_budget(&[Ty::BytesN(usize::MAX)]));
    assert!(within_budget(&[Ty::BytesN(4096)]), "the cap itself is allowed");
    assert!(within_budget(&[Ty::U8, Ty::Bool]));
    // The runner refuses before enumerating (it would otherwise never return).
    let (v, s) = harness::exhaustive(&[Ty::U8; 8], |_| true, |_, _| true, Fault::None);
    assert!(is_fail(&v, "corpus-malformed"), "{v:?}");
    assert_eq!(s.cases, 0);
    // The closed forms must describe what `domain` really enumerates, and the
    // byte bound must dominate the real payload.
    for ty in [Ty::Bool, Ty::U8, Ty::Nat, Ty::U64, Ty::Bytes, Ty::BytesN(0), Ty::BytesN(1), Ty::BytesN(2), Ty::BytesN(3), Ty::BytesN(4), Ty::BytesN(17)] {
        let d = domain(ty);
        assert_eq!(domain_size(ty), Some(d.len() as u64), "{ty:?}");
        let payload: usize = d
            .iter()
            .map(|v| match v {
                harness::V::Bytes(b) => b.len(),
                _ => 0,
            })
            .sum();
        assert!(domain_bytes(ty).unwrap() >= payload as u64, "{ty:?}");
    }
}

#[test]
fn oversized_fixed_widths_are_refused_with_w0707() {
    // Bytes<100000> would materialise ~30 GB of domain.
    let (head, bindings) = eq_head();
    let src = format!("{head}  claim c:\n    forall a: Bytes<100000>. eq(a, a) == true\n");
    let (code, msg) = generate_src(&src, &bindings).unwrap_err();
    assert_eq!(code, DiagnosticCode::W0707, "{msg}");
    // Just over the cap is refused, the cap itself is accepted.
    let src = format!("{head}  claim c:\n    forall a: Bytes<4097>. eq(a, a) == true\n");
    assert_eq!(generate_src(&src, &bindings).unwrap_err().0, DiagnosticCode::W0707);
    let src = format!("{head}  claim c:\n    forall a: Bytes<4096>. eq(a, a) == true\n");
    assert!(generate_src(&src, &bindings).is_ok());

    // A huge const-generic width in a call is refused too (one match arm and one
    // monomorphisation per width).
    let head = "warrant t\n  op eq_fixed<N: Nat>(a: Bytes<N>, b: Bytes<N>) -> Bool\n  assume platform:\n    none\n  assume environment:\n    none\n  bound eq_fixed:\n    steps <= 10\n";
    let mut b = BTreeMap::new();
    b.insert("eq_fixed".to_string(), bind("ct_eq_fixed", &["N"], false, None));
    let bindings = BindingManifest { package: "p".into(), revision: "0".into(), bindings: b };
    let src = format!("{head}  claim c:\n    forall a: Bytes<2>, b: Bytes<2>. eq_fixed<100000>(a, b) == true\n");
    assert_eq!(generate_src(&src, &bindings).unwrap_err().0, DiagnosticCode::W0707);
}

// ---- audit A3: a filter without a fault must not produce a green run ----------

#[test]
fn filter_without_fault_is_an_error_and_an_empty_filter_match_fails() {
    use harness::{require_tests, resolve_sabotage};
    assert_eq!(resolve_sabotage(None, None), Ok(Fault::None));
    let e = resolve_sabotage(None, Some("/exhaustive/")).unwrap_err();
    assert!(e.contains("WARRANT_EVIDENCE_FILTER") && e.contains("WARRANT_EVIDENCE_FAULT"), "{e}");
    assert_eq!(resolve_sabotage(Some("invert"), Some("/exhaustive/")), Ok(Fault::Invert));
    assert_eq!(resolve_sabotage(Some("invert"), None), Ok(Fault::Invert));
    assert!(resolve_sabotage(Some("nonsense"), None).unwrap_err().contains("unknown"));
    // A filter that selects zero tests proves nothing.
    assert!(require_tests(0, Some("/no-such-suite/")).is_err());
    assert!(require_tests(3, Some("/exhaustive/")).is_ok());
    assert!(require_tests(0, None).is_ok(), "an unfiltered empty suite is the generator's concern");
}
