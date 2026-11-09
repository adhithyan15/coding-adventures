//! VER04h: the evidence generator is data-driven per package.  These tests pin
//! the `evidence.bindings` reader, the scalar (`U8`-in, `U8`-out) adapter shape
//! used by gf256, the closed fault registry, and the rules the generator
//! enforces (budget, refinements, fault applicability).  The gf256 harness is
//! also regenerated and compared byte for byte, so drift fails `cargo test`
//! and not only `BUILD`.

use coding_adventures_sha256::sha256_hex;
use coding_adventures_warrant_grammar::binding::{parse_evidence_bindings, EvidenceBindingFile};
use coding_adventures_warrant_grammar::codegen::{CodegenErrorKind, DiagnosticCode};
use coding_adventures_warrant_grammar::evidence::{generate_evidence_harness, EvidenceOptions};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

const GF_SPEC: &str = include_str!("../../../../specs/gf256.warrant");
const GF_BINDINGS: &str = include_str!("../../gf256/evidence.bindings");
const GF_CHECKED_IN: &str = include_str!("../../gf256/tests/generated/warrant_evidence.rs");
const CT_BINDINGS: &str = include_str!("../../ct-compare/evidence.bindings");
const CT_CHECKED_IN: &str = include_str!("../../ct-compare/tests/generated/warrant_evidence.rs");

fn options(b: &EvidenceBindingFile, spec: &str) -> EvidenceOptions {
    EvidenceOptions {
        package_crate: b.manifest.package.clone(),
        spec_path: b.spec.clone(),
        spec_sha256: sha256_hex(spec.as_bytes()),
        seed: b.seed,
        property_cases: b.property_cases,
        fuzz_iterations: b.fuzz_iterations,
        negctl_fuzz_iterations: b.negctl_fuzz_iterations,
        exhaustive_budget: b.exhaustive_budget,
        arg_types: b.arg_types.clone(),
    }
}

/// Generate from `spec` with the bindings in `b`, returning the error code and
/// message on refusal.
fn generate(spec: &str, b: &EvidenceBindingFile) -> Result<String, (DiagnosticCode, String)> {
    let doc = SourceDocument::admit(spec.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");
    generate_evidence_harness(&file, &b.manifest, &options(b, spec)).map_err(|e| {
        let msg = match &e.kind {
            CodegenErrorKind::UnsupportedTarget(m) | CodegenErrorKind::MissingBinding(m) => m.clone(),
            other => format!("{other:?}"),
        };
        (e.code, msg)
    })
}

fn gf() -> EvidenceBindingFile {
    parse_evidence_bindings(GF_BINDINGS).expect("gf256 bindings")
}

#[test]
fn checked_in_bindings_files_parse() {
    let g = gf();
    assert_eq!(g.manifest.package, "gf256");
    assert_eq!(g.exhaustive_budget, 1 << 24);
    assert_eq!(g.manifest.bindings.len(), 6);
    assert_eq!(g.arg_types["power"], vec!["u8".to_string(), "u32".to_string()]);
    let divide = &g.manifest.bindings["divide"];
    assert_eq!(divide.error_normalization.as_ref().unwrap().error_case, "DIVIDE_BY_ZERO");

    let c = parse_evidence_bindings(CT_BINDINGS).expect("ct-compare bindings");
    assert_eq!(c.exhaustive_budget, 400_000, "ct-compare keeps the default budget");
    assert!(c.manifest.bindings["select_bytes"].fresh);
    assert_eq!(c.manifest.bindings["eq_fixed"].static_params, vec!["N".to_string()]);
}

#[test]
fn gf256_harness_is_current() {
    let b = gf();
    assert_eq!(generate(GF_SPEC, &b).expect("generates"), GF_CHECKED_IN);
}

#[test]
fn ct_compare_harness_is_byte_identical_through_the_bindings_file() {
    let b = parse_evidence_bindings(CT_BINDINGS).unwrap();
    let spec = include_str!("../../../../specs/ct-compare.warrant");
    assert_eq!(generate(spec, &b).expect("generates"), CT_CHECKED_IN);
}

#[test]
fn fault_enum_lists_only_the_faults_a_package_uses() {
    // ct-compare uses exactly the five original faults and none of gf256's.
    assert!(CT_CHECKED_IN.contains("MissingError,\n}"));
    assert!(!CT_CHECKED_IN.contains("OrForXor"));
    assert!(!CT_CHECKED_IN.contains("fn fault_for"), "scalar scoping is gated");
    // gf256 uses none of ct-compare's byte-slice faults except missing-error.
    assert!(GF_CHECKED_IN.contains("MissingError"));
    assert!(GF_CHECKED_IN.contains("GeneratorShift"));
    assert!(!GF_CHECKED_IN.contains("IgnoreLastByte"));
    assert!(GF_CHECKED_IN.contains("fn fault_for"));
}

#[test]
fn bindings_reader_rejects_typos_instead_of_weakening_evidence() {
    let base = "package p\nrevision 1\nspec s\noutput o\nseed 1\nproperty_cases 1\nfuzz_iterations 1\nnegctl_fuzz_iterations 1\n";
    let ok = format!("{base}op a export=a faults=invert\n");
    assert!(parse_evidence_bindings(&ok).is_ok());
    let cases: [(&str, String); 8] = [
        ("unknown key", format!("{ok}sead 3\n")),
        ("unknown op option", format!("{base}op a export=a faults=invert colour=red\n")),
        ("no faults", format!("{base}op a export=a\n")),
        ("no export", format!("{base}op a faults=invert\n")),
        ("duplicate op", format!("{ok}op a export=a faults=invert\n")),
        ("bad number", ok.replace("seed 1", "seed one")),
        ("repeated key", format!("{ok}seed 2\n")),
        ("no ops", base.to_string()),
    ];
    for (what, text) in cases {
        assert!(parse_evidence_bindings(&text).is_err(), "{what} must be rejected");
    }
    // A required key that is missing is named.
    let missing = ok.replace("output o\n", "");
    assert!(parse_evidence_bindings(&missing).unwrap_err().contains("output"));
}

#[test]
fn exhaustive_budget_is_enforced_per_package() {
    // gf256's three-binder claims need 2^24; the default 400000 refuses them.
    let mut b = gf();
    b.exhaustive_budget = 400_000;
    let (code, msg) = generate(GF_SPEC, &b).unwrap_err();
    assert_eq!(code, DiagnosticCode::W0706);
    assert!(msg.contains("exceeds budget 400000"), "{msg}");
}

#[test]
fn unknown_scalar_fault_is_refused() {
    let mut b = gf();
    b.manifest.bindings.get_mut("add").unwrap().negative_controls = vec!["bit-rot".into()];
    let (code, msg) = generate(GF_SPEC, &b).unwrap_err();
    assert_eq!(code, DiagnosticCode::W0701);
    assert!(msg.contains("bit-rot"), "{msg}");
}

#[test]
fn missing_error_fault_needs_a_fallible_operation() {
    let mut b = gf();
    b.manifest.bindings.get_mut("add").unwrap().negative_controls = vec!["missing-error".into()];
    let (code, msg) = generate(GF_SPEC, &b).unwrap_err();
    assert_eq!(code, DiagnosticCode::W0701);
    assert!(msg.contains("fallible"), "{msg}");
}

#[test]
fn argument_types_must_match_the_parameter_list() {
    let mut b = gf();
    b.arg_types.insert("power".into(), vec!["u8".into()]);
    let (code, msg) = generate(GF_SPEC, &b).unwrap_err();
    assert_eq!(code, DiagnosticCode::W0701);
    assert!(msg.contains("arg_types"), "{msg}");

    let mut b = gf();
    b.arg_types.insert("power".into(), vec!["u8".into(), "i128".into()]);
    let (_, msg) = generate(GF_SPEC, &b).unwrap_err();
    assert!(msg.contains("i128"), "{msg}");
}

#[test]
fn a_refinement_may_not_call_an_operation() {
    // An admission predicate evaluated by a faulted adapter would change which
    // cases a negative control admits, so the generator refuses it.
    let spec = GF_SPEC.replace(
        "claim divide_self_is_one:\n    forall a: U8 | a != 0u8. divide(a, a) == Ok(1u8)",
        "claim divide_self_is_one:\n    forall a: U8 | multiply(a, 1u8) != 0u8. divide(a, a) == Ok(1u8)",
    );
    assert_ne!(spec, GF_SPEC, "the replaced claim must exist");
    let (code, msg) = generate(&spec, &gf()).unwrap_err();
    assert_eq!(code, DiagnosticCode::W0701);
    assert!(msg.contains("refinement"), "{msg}");
}

#[test]
fn scalar_adapter_has_one_arm_per_declared_fault() {
    // The adapter of `power` converts its u32 exponent, scopes faults to the
    // attacked operation and lists exactly its three faults.
    let start = GF_CHECKED_IN.find("fn op_power").expect("adapter");
    let body = &GF_CHECKED_IN[start..start + 1400];
    assert!(body.contains("u32::try_from(args[1].u64v())"), "{body}");
    assert!(body.contains("fault_for(\"power\", fault)"), "{body}");
    assert!(body.contains("Fault::AltPoly => gf256::Field::new(0x11B).power(p0, p1)"), "{body}");
    assert!(body.contains("Fault::GeneratorShift"), "{body}");
    assert!(body.contains("Fault::ExpMod256"), "{body}");
    assert!(!body.contains("Fault::OrForXor"), "{body}");
}
