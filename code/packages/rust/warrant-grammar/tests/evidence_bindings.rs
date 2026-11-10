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
        hosts: Default::default(),
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

const BASE: &str = "package p\nrevision 1\nspec code/specs/p.warrant\noutput code/packages/rust/p/tests/generated/e.rs\nseed 1\nproperty_cases 1\nfuzz_iterations 1\nnegctl_fuzz_iterations 1\n";

#[test]
fn exhaustive_budget_is_capped_at_two_to_the_26() {
    // L3: the parser refuses a budget above 2^26; gf256's 2^24 and 2^26 itself pass.
    let op = "op a export=a faults=invert\n";
    let at = |n: u64| parse_evidence_bindings(&format!("{BASE}exhaustive_budget {n}\n{op}"));
    assert!(at(1 << 24).is_ok());
    assert!(at(1 << 26).is_ok());
    let e = at((1 << 26) + 1).unwrap_err();
    assert!(e.contains("exhaustive_budget") && e.contains("67108864"), "{e}");
    assert!(at(u64::MAX).is_err());
}

#[test]
fn export_and_package_must_be_rust_identifiers() {
    // L4: nothing but an identifier may reach the generated source.
    let ok = format!("{BASE}op a export=a faults=invert\n");
    assert!(parse_evidence_bindings(&ok).is_ok());
    for bad in ["a;drop()", "a::b", "1a", "a-b", "a(", "a\"b"] {
        let t = format!("{BASE}op a export={bad} faults=invert\n");
        let e = parse_evidence_bindings(&t).unwrap_err();
        assert!(e.contains("not a Rust identifier"), "{bad}: {e}");
    }
    for bad in ["p;q", "p-q", "p::q", "9p"] {
        let t = ok.replace("package p\n", &format!("package {bad}\n"));
        let e = parse_evidence_bindings(&t).unwrap_err();
        assert!(e.contains("not a Rust identifier"), "{bad}: {e}");
    }
}

#[test]
fn spec_and_output_are_plain_relative_paths_pinned_to_the_package() {
    let ok = format!("{BASE}op a export=a faults=invert\n");
    let b = parse_evidence_bindings(&ok).unwrap();
    assert!(b.validate_paths("p").is_ok());
    // Wrong package for the pinned paths.
    assert!(b.validate_paths("q").is_err());
    for bad in ["../p", "p/x", "/p", "p;q", "", "p q", ".."] {
        assert!(b.validate_paths(bad).is_err(), "package arg {bad:?}");
    }
    let with = |from: &str, to: &str| parse_evidence_bindings(&ok.replace(from, to));
    for bad in ["/etc/p.warrant", "../p.warrant", "code/specs/../p.warrant", "C:/p.warrant", "code\\specs\\p.warrant"] {
        assert!(with("code/specs/p.warrant", bad).is_err(), "spec {bad:?} rejected by the parser");
    }
    for bad in ["/tmp/e.rs", "code/packages/rust/p/tests/generated/../../e.rs", "../e.rs"] {
        assert!(with("code/packages/rust/p/tests/generated/e.rs", bad).is_err(), "output {bad:?} rejected by the parser");
    }
    // Plain but not pinned: another package's spec, a deeper or non-.rs output.
    for (from, to) in [
        ("code/specs/p.warrant", "code/specs/other.warrant"),
        ("code/packages/rust/p/tests/generated/e.rs", "code/packages/rust/other/tests/generated/e.rs"),
        ("code/packages/rust/p/tests/generated/e.rs", "code/packages/rust/p/tests/generated/sub/e.rs"),
        ("code/packages/rust/p/tests/generated/e.rs", "code/packages/rust/p/src/lib.rs"),
        ("code/packages/rust/p/tests/generated/e.rs", "code/packages/rust/p/tests/generated/e.txt"),
    ] {
        let b = with(from, to).expect("plain relative path parses");
        assert!(b.validate_paths("p").is_err(), "{to} must not be pinned-valid");
    }
    // The checked-in files satisfy the pin.
    gf().validate_paths("gf256").unwrap();
    parse_evidence_bindings(CT_BINDINGS).unwrap().validate_paths("ct-compare").unwrap();
}

#[test]
fn the_generator_refuses_a_hand_built_manifest_with_a_hostile_export() {
    let mut b = gf();
    b.manifest.bindings.get_mut("add").unwrap().export = "add(); std::process::exit(0); fn x".into();
    let (code, msg) = generate(GF_SPEC, &b).unwrap_err();
    assert_eq!(code, DiagnosticCode::W0701);
    assert!(msg.contains("not a Rust identifier"), "{msg}");
    let mut b = gf();
    b.manifest.package = "gf256\"; evil".into();
    assert!(generate(GF_SPEC, &b).is_err());
}

#[test]
fn proofs_not_live_is_a_per_op_declaration() {
    let g = gf();
    assert_eq!(g.proofs_not_live.len(), 6, "every gf256 op declares its proofs not live");
    assert!(g.proofs_not_live.contains_key("power"));
    let c = parse_evidence_bindings(CT_BINDINGS).unwrap();
    assert!(c.proofs_not_live.is_empty(), "ct-compare proves its operations");
    let t = format!("{BASE}op a export=a faults=invert proofs_not_live=x\nop b export=b faults=invert\n");
    let b = parse_evidence_bindings(&t).unwrap();
    assert_eq!(b.proofs_not_live.keys().collect::<Vec<_>>(), vec!["a"]);
    // An empty reason is a malformed option, not a declaration.
    assert!(parse_evidence_bindings(&format!("{BASE}op a export=a faults=invert proofs_not_live=\n")).is_err());
}
