//! VER04j section 14.4 (WX-N): what the generator decides for `Bytes<N>` across
//! the host boundary -- which bindings it accepts, what it refuses (a width that
//! does not match, a host array for a dynamic `Bytes`, an opaque-error option
//! beside a named-error one), and the text it emits for the conversions.
//! `tests/wxn_evidence.rs` compiles and runs the output.

use coding_adventures_warrant_grammar::binding::{parse_evidence_bindings, parse_host_ty, HostTy};
use coding_adventures_warrant_grammar::codegen::DiagnosticCode;
use coding_adventures_warrant_grammar::evidence::{generate_evidence_harness, EvidenceOptions};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

const FIXTURE: &str = include_str!("fixtures/wxn.warrant");

fn bindings_text(lines: &[&str]) -> String {
    let mut s = String::from(
        "package subject\nrevision 0\nspec code/specs/t.warrant\noutput code/packages/rust/x/tests/generated/t.rs\nseed 1\nproperty_cases 10\nfuzz_iterations 10\nnegctl_fuzz_iterations 10\nexhaustive_budget 400000\n",
    );
    for l in lines {
        s.push_str(l);
        s.push('\n');
    }
    s
}

fn generate(src: &str, lines: &[&str]) -> Result<String, (Option<DiagnosticCode>, String)> {
    let b = parse_evidence_bindings(&bindings_text(lines)).map_err(|e| (None, format!("bindings: {e}")))?;
    let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");
    let options = EvidenceOptions {
        package_crate: "subject".into(),
        spec_path: "code/specs/t.warrant".into(),
        spec_sha256: "0".repeat(64),
        seed: 1,
        property_cases: 10,
        fuzz_iterations: 10,
        negctl_fuzz_iterations: 10,
        exhaustive_budget: 400_000,
        arg_types: b.arg_types.clone(),
        hosts: b.hosts.clone(),
    };
    generate_evidence_harness(&file, &b.manifest, &options).map_err(|e| (Some(e.code), format!("{e}")))
}

/// A one-operation document: `f(a: Bytes<4>) -> Bytes<4>`.
fn doc(param: &str, ret: &str) -> String {
    format!(
        "warrant t v1\n  op f(a: {param}) -> {ret}\n  labels f:\n    a public\n  claim c:\n    forall a: {param}. f(a) == f(a)\n"
    )
}

// ---------------------------------------------------------------------------
// The host type `bytes<N>`
// ---------------------------------------------------------------------------

#[test]
fn bytes_n_is_a_host_type_with_a_literal_width() {
    assert_eq!(parse_host_ty("bytes<32>"), Ok(HostTy::BytesN(32)));
    assert_eq!(parse_host_ty("bytes<0>"), Ok(HostTy::BytesN(0)));
    assert_eq!(parse_host_ty("bytes"), Ok(HostTy::Bytes));
    // It nests like any other host type.
    assert_eq!(
        parse_host_ty("opt<bytes<4>>"),
        Ok(HostTy::Opt(Box::new(HostTy::BytesN(4))))
    );
    assert_eq!(
        parse_host_ty("(u8,bytes<2>)"),
        Ok(HostTy::Tuple(vec![HostTy::Int("u8".into()), HostTy::BytesN(2)]))
    );
}

#[test]
fn a_malformed_or_oversized_width_is_not_a_host_type() {
    for bad in ["bytes<>", "bytes<x>", "bytes<-1>", "bytes<4097>", "bytes<1,2>", "bytes<04>", "bytes<4"] {
        assert!(parse_host_ty(bad).is_err(), "{bad} must be refused");
    }
    // 4096 is the generator's MAX_FIXED_WIDTH and is accepted.
    assert_eq!(parse_host_ty("bytes<4096>"), Ok(HostTy::BytesN(4096)));
}

// ---------------------------------------------------------------------------
// Accepted bindings and the emitted conversions
// ---------------------------------------------------------------------------

#[test]
fn the_fixture_generates() {
    let b = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/wxn.bindings"),
    )
    .unwrap();
    let lines: Vec<&str> = b.lines().skip_while(|l| !l.starts_with("op ")).collect();
    let text = generate(FIXTURE, &lines).expect("the fixture generates");
    assert!(!text.contains("//@"), "no marker line may reach the output");
}

#[test]
fn a_parameter_is_converted_with_a_length_check_that_fails_closed() {
    let text = generate(&doc("Bytes<4>", "Bytes<4>"), &["op f export=f args=bytes<4> ret=bytes<4> faults=flip-result-bit"]).unwrap();
    // The model value is narrowed to the array by try_from; a wrong length is
    // the harness's Unreachable signal, never a truncation or a padding.
    assert!(text.contains("<[u8; 4]>::try_from("), "{text}");
    assert!(text.contains("std::panic::panic_any(Unreachable)"));
    // A top-level parameter is passed by reference, as `bytes` is.
    assert!(text.contains("let p0 = &"), "{text}");
}

#[test]
fn a_bytes_n_result_is_length_checked_whatever_the_host_type() {
    for ret in ["ret=bytes<4>", ""] {
        let line = format!("op f export=f args=bytes<4> {ret} faults=flip-result-bit");
        let text = generate(&doc("Bytes<4>", "Bytes<4>"), &[&line]).unwrap();
        assert!(text.contains("!= 4"), "result length check missing for `{ret}`:\n{text}");
        assert!(text.contains("is not 4 bytes") || text.contains("must be 4 bytes"), "{text}");
    }
}

// ---------------------------------------------------------------------------
// Refusals
// ---------------------------------------------------------------------------

#[test]
fn a_host_width_that_differs_from_the_warrant_width_is_refused() {
    let r = generate(&doc("Bytes<4>", "Bytes<4>"), &["op f export=f args=bytes<5> ret=bytes<4> faults=flip-result-bit"]);
    let (code, msg) = r.expect_err("bytes<5> is not Bytes<4>");
    assert_eq!(code, Some(DiagnosticCode::W0701), "{msg}");
    let r = generate(&doc("Bytes<4>", "Bytes<4>"), &["op f export=f args=bytes<4> ret=bytes<3> faults=flip-result-bit"]);
    assert_eq!(r.expect_err("result width").0, Some(DiagnosticCode::W0701));
}

#[test]
fn a_host_array_for_a_dynamic_bytes_is_refused() {
    let r = generate(&doc("Bytes", "Bytes"), &["op f export=f args=bytes<4> ret=bytes faults=flip-result-bit"]);
    assert_eq!(r.expect_err("Bytes is not an array").0, Some(DiagnosticCode::W0701));
    let r = generate(&doc("Bytes", "Bytes"), &["op f export=f args=bytes ret=bytes<4> faults=flip-result-bit"]);
    assert_eq!(r.expect_err("Bytes is not an array").0, Some(DiagnosticCode::W0701));
}

#[test]
fn a_named_width_is_still_refused_on_a_composite_operation() {
    let src = "warrant t v1\n  op f<N: Nat>(a: Bytes<N>) -> Bytes<4>\n  labels f:\n    a public\n  claim c:\n    forall a: Bytes<4>. f<4>(a) == f<4>(a)\n";
    let r = generate(src, &["op f export=f static=N args=bytes<4> ret=bytes<4> faults=flip-result-bit"]);
    assert_eq!(r.expect_err("static widths").0, Some(DiagnosticCode::W0701));
}

// ---------------------------------------------------------------------------
// err_any=
// ---------------------------------------------------------------------------

fn result_doc() -> String {
    "warrant t v1\n  error E = LOW | HIGH\n  op f(a: Bytes<2>) -> Result<Bytes<2>, E>\n  labels f:\n    a public\n  claim c:\n    forall a: Bytes<2>. f(a) == f(a)\n".to_string()
}

#[test]
fn err_any_maps_every_host_error_to_one_case() {
    let text = generate(&result_doc(), &["op f export=f args=bytes<2> ret=bytes<2> err_any=LOW faults=missing-error"]).unwrap();
    assert!(text.contains("Err(_) => V::Err(\"LOW\".into())"), "{text}");
}

#[test]
fn err_any_needs_a_declared_case_and_excludes_the_other_error_options() {
    let r = generate(&result_doc(), &["op f export=f args=bytes<2> ret=bytes<2> err_any=NOPE faults=missing-error"]);
    assert_eq!(r.expect_err("unknown case").0, Some(DiagnosticCode::W0701));
    for other in ["error=LOW", "errors=subject::E/A=LOW"] {
        let line = format!("op f export=f err_any=LOW {other} faults=missing-error");
        assert!(generate(&result_doc(), &[&line]).is_err(), "{other}");
    }
}
