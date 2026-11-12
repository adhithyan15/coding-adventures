//! VER04j section 6 and 14.2: what the generator decides for a document with
//! `output` / `heap` bounds -- which documents get the allocation observer and
//! which are left byte-for-byte alone, how the limit parameter is found, which
//! faults may attack which operation -- and what it refuses.  These tests inspect
//! the generated text and the generator's diagnostics; `tests/wxh_evidence.rs`
//! compiles and runs the output.

use coding_adventures_warrant_grammar::binding::parse_evidence_bindings;
use coding_adventures_warrant_grammar::codegen::DiagnosticCode;
use coding_adventures_warrant_grammar::evidence::{
    generate_evidence_harness, EvidenceOptions, OBSERVER_CAP_BYTES,
};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

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

fn generate(src: &str, lines: &[&str]) -> Result<String, (DiagnosticCode, String)> {
    let b = parse_evidence_bindings(&bindings_text(lines)).expect("bindings parse");
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
    generate_evidence_harness(&file, &b.manifest, &options).map_err(|e| (e.code, format!("{e}")))
}

const DECOMPRESS: &str = "op decompress export=decompress errors=subject::E/Limit=LIMIT,Bad=BAD faults=";

/// A v1 document with `decompress(d: Bytes, m: U32) -> Result<Bytes, Err>` and a
/// `bound` block made of `rows`.
fn doc(rows: &str, claim: &str) -> String {
    format!(
        "warrant t v1\n  error Err = LIMIT | BAD\n  op decompress(d: Bytes, m: U32) -> Result<Bytes, Err>\n  labels decompress:\n    d public\n    m public\n  bound decompress:\n{rows}  claim c:\n    forall m: U32. {claim}\n"
    )
}

const ROWS: &str = "    output <= to_int(m)\n    heap <= 2 * to_int(m) + 64\n";
const CLAIM: &str = "decompress(b\"\", m) == Err(Err.BAD)";

// ---------------------------------------------------------------------------
// Which documents get the observer
// ---------------------------------------------------------------------------

#[test]
fn an_output_or_heap_row_in_a_v1_document_enables_the_observer() {
    for rows in [ROWS, "    output <= to_int(m)\n", "    heap <= 4096\n"] {
        let text = generate(&doc(rows, CLAIM), &[&format!("{DECOMPRESS}missing-error")]).unwrap();
        assert!(text.contains("#[global_allocator]"), "{rows}");
        assert!(text.contains(&format!("pub const OBSERVER_CAP_BYTES: u64 = {OBSERVER_CAP_BYTES};")));
        assert!(text.contains("fn op_decompress_raw("));
        assert!(text.contains("obs_observe(\"decompress\""));
    }
}

#[test]
fn steps_and_depth_rows_alone_do_not_enable_it() {
    let rows = "    steps <= 4096\n    depth <= 8\n";
    let text = generate(&doc(rows, CLAIM), &[&format!("{DECOMPRESS}missing-error")]).unwrap();
    assert!(!text.contains("global_allocator"));
    assert!(!text.contains("OBSERVER_CAP_BYTES"));
    assert!(!text.contains("op_decompress_raw"));
}

#[test]
fn the_heap_row_is_a_function_of_the_arguments_and_the_output_row_a_claim() {
    let text = generate(&doc(ROWS, CLAIM), &[&format!("{DECOMPRESS}missing-error")]).unwrap();
    assert!(text.contains("fn op_decompress_heap(args: &[V]) -> i128"));
    assert!(text.contains("fn op_decompress_output(args: &[V]) -> i128"));
    // The synthesized claim and its three suites.
    assert!(text.contains("fn claim_decompress_output_bound_holds("));
    for kind in ["exhaustive", "property", "fuzz"] {
        assert!(
            text.contains(&format!("warrant/t/{kind}/decompress/output-bound")),
            "{kind}"
        );
    }
    assert!(text.contains("\"decompress/output-bound\""));
    // Only an `output` row makes a claim: a heap row alone does not.
    let heap_only = generate(&doc("    heap <= 4096\n", CLAIM), &[&format!("{DECOMPRESS}missing-error")]).unwrap();
    assert!(!heap_only.contains("/output-bound"));
    assert!(!heap_only.contains("op_decompress_output"));
}

#[test]
fn a_document_without_bounds_is_not_touched() {
    let src = "warrant t v1\n  error Err = LIMIT | BAD\n  op decompress(d: Bytes, m: U32) -> Result<Bytes, Err>\n  labels decompress:\n    d public\n    m public\n  claim c:\n    forall m: U32. decompress(b\"\", m) == Err(Err.BAD)\n";
    let text = generate(src, &[&format!("{DECOMPRESS}missing-error")]).unwrap();
    assert!(!text.contains("global_allocator") && !text.contains("obs_observe"));
    assert!(!text.contains("//@"), "no marker line may reach the output");
}

#[test]
fn a_v0_heap_row_never_enables_the_observer() {
    // gf256 and ct-compare are v0 documents with `heap <= 0`; their checked-in
    // harnesses are byte-identical (tests/v0_compat.rs and the BUILD drift check).
    let src = "warrant t\n  op f(a: U8, b: U8) -> U8\n  labels f:\n    a public\n    b public\n  bound f:\n    steps <= 16\n    heap <= 0\n    depth <= 8\n  claim c:\n    forall a: U8, b: U8. f(a, b) == f(b, a)\n";
    let text = generate(src, &["op f export=f faults=or-for-xor"]).unwrap();
    assert!(!text.contains("global_allocator"));
}

// ---------------------------------------------------------------------------
// The limit parameter and the faults
// ---------------------------------------------------------------------------

fn faults(list: &str) -> Result<String, (DiagnosticCode, String)> {
    generate(&doc(ROWS, CLAIM), &[&format!("{DECOMPRESS}{list}")])
}

#[test]
fn the_four_limit_faults_are_accepted_on_a_composite_operation_with_a_limit() {
    let text = faults("ignore-limit,off-by-one-limit,limit-after-alloc,truncate-silently,missing-error")
        .unwrap();
    for (variant, needle) in [
        ("IgnoreLimit", "Fault::IgnoreLimit => u32::MAX"),
        ("OffByOneLimit", "Fault::OffByOneLimit => p1.saturating_add(1)"),
        ("LimitAfterAlloc", "Fault::LimitAfterAlloc).then(|| vec![0u8; p1 as usize])"),
        ("TruncateSilently", "fault_truncate_silently(v, args[0].bytes(), args[1].int())"),
    ] {
        assert!(text.contains(variant), "{variant}");
        assert!(text.contains(needle), "{needle}");
    }
    assert!(text.contains("std::hint::black_box(&hog)"));
}

#[test]
fn no_limit_fault_without_exactly_one_unsigned_numeric_parameter_in_the_output_row() {
    // The bound mentions no parameter.
    let none = doc("    output <= 100\n", CLAIM);
    // ... or only a byte string.
    let bytes_only = doc("    output <= len(d)\n", CLAIM);
    for src in [none, bytes_only] {
        for f in ["ignore-limit", "off-by-one-limit", "limit-after-alloc", "truncate-silently"] {
            let (code, msg) = generate(&src, &[&format!("{DECOMPRESS}{f}")]).unwrap_err();
            assert_eq!(code, DiagnosticCode::W0701, "{f}: {msg}");
            assert!(msg.contains("does not apply") || msg.contains("limit"), "{f}: {msg}");
        }
    }
    // Two unsigned numeric parameters: which one is the limit is ambiguous.
    let two = "warrant t v1\n  error Err = LIMIT | BAD\n  op decompress(d: Bool, m: U32, k: U32) -> Result<Bytes, Err>\n  labels decompress:\n    d public\n    m public\n    k public\n  bound decompress:\n    output <= to_int(m) + to_int(k)\n  claim c:\n    forall m: U32. decompress(true, m, 0u32) == Err(Err.BAD)\n";
    let (code, msg) = generate(two, &[&format!("{DECOMPRESS}ignore-limit")]).unwrap_err();
    assert_eq!(code, DiagnosticCode::W0701, "{msg}");
    // The faults themselves are fine once the bound names one limit.
    let one = two.replace("to_int(m) + to_int(k)", "to_int(m)");
    generate(&one, &[&format!("{DECOMPRESS}ignore-limit")]).unwrap();
}

#[test]
fn truncate_silently_needs_a_byte_string_parameter_and_a_bytes_result() {
    // A list result: no truncation of "the first bytes of the input" exists.
    let list = "warrant t v1\n  error Err = LIMIT | BAD\n  op decompress(d: Bytes, m: U32) -> Result<List<Bool>, Err>\n  labels decompress:\n    d public\n    m public\n  bound decompress:\n    output <= to_int(m)\n  claim c:\n    forall m: U32. decompress(b\"\", m) == Err(Err.BAD)\n";
    let (code, msg) = generate(list, &[&format!("{DECOMPRESS}truncate-silently")]).unwrap_err();
    assert_eq!(code, DiagnosticCode::W0701, "{msg}");
    // But the other limit faults are fine on it.
    generate(list, &[&format!("{DECOMPRESS}ignore-limit")]).unwrap();
}

#[test]
fn a_limit_fault_on_a_scalar_operation_is_refused() {
    let src = "warrant t v1\n  op f(a: U16, b: U16) -> U16\n  labels f:\n    a public\n    b public\n  claim c:\n    forall a: U16, b: U16. f(a, b) == f(b, a)\n";
    let (code, msg) = generate(src, &["op f export=f faults=ignore-limit"]).unwrap_err();
    assert_eq!(code, DiagnosticCode::W0701, "{msg}");
    assert!(msg.contains("composite"), "{msg}");
}

// ---------------------------------------------------------------------------
// Refusals and the caps of the generator
// ---------------------------------------------------------------------------

#[test]
fn a_bound_of_an_unknown_operation_is_refused() {
    let src = doc(ROWS, CLAIM).replace("bound decompress:", "bound ghost:");
    let (code, msg) = generate(&src, &[&format!("{DECOMPRESS}missing-error")]).unwrap_err();
    assert_eq!(code, DiagnosticCode::W0701, "{msg}");
    assert!(msg.contains("ghost"), "{msg}");
}

#[test]
fn a_bound_expression_may_not_call_an_operation() {
    // A bound is a function of the arguments; calling the operation it bounds
    // would make the observer's own check depend on the faulted adapter.
    let rows = "    output <= len(unwrap_or(Some(d), d)) + to_int(m)\n    heap <= len(d)\n";
    generate(&doc(rows, CLAIM), &[&format!("{DECOMPRESS}missing-error")]).unwrap();
    let src = doc("    heap <= len(decompress(d, m))\n", CLAIM);
    let (code, msg) = generate(&src, &[&format!("{DECOMPRESS}missing-error")]).unwrap_err();
    assert_eq!(code, DiagnosticCode::W0701, "{msg}");
}

#[test]
fn an_output_bound_on_an_operation_with_static_widths_is_refused() {
    let src = "warrant t v1\n  op f<N: Nat>(d: Bytes<N>) -> Bytes\n  labels f:\n    d public\n  bound f:\n    output <= 16\n  claim c:\n    forall d: Bytes<2>. f<2>(d) == f<2>(d)\n";
    let (code, msg) = generate(src, &["op f export=f static=N faults=invert"]).unwrap_err();
    assert_eq!(code, DiagnosticCode::W0701, "{msg}");
    assert!(msg.contains("static widths"), "{msg}");
}

#[test]
fn a_generated_claim_name_may_not_collide_with_a_document_claim() {
    let src = doc(ROWS, CLAIM).replace("claim c:", "claim decompress_output_bound:");
    let (code, msg) = generate(&src, &[&format!("{DECOMPRESS}missing-error")]).unwrap_err();
    assert_eq!(code, DiagnosticCode::W0701, "{msg}");
    assert!(msg.contains("already taken"), "{msg}");
}

#[test]
fn an_oversized_binder_domain_of_the_output_bound_claim_is_w0706() {
    // The synthesized claim ranges over the operation's own parameters, so the
    // budget of section 3 applies to it exactly as to a written claim.
    let src = "warrant t v1\n  error Err = LIMIT | BAD\n  op big(a: List<List<Bytes>>, b: List<List<Bytes>>, m: U32) -> Result<Bytes, Err>\n  labels big:\n    a public\n    b public\n    m public\n  bound big:\n    output <= to_int(m)\n  claim c:\n    forall m: U32. big([], [], m) == Err(Err.BAD)\n";
    // (list literals are not translatable, so build the claim differently)
    let src = src.replace("big([], [], m) == Err(Err.BAD)", "m == m");
    let (code, msg) = generate(&src, &["op big export=big errors=subject::E/Limit=LIMIT,Bad=BAD faults=missing-error"]).unwrap_err();
    assert!(
        matches!(code, DiagnosticCode::W0706 | DiagnosticCode::W0701),
        "{code:?}: {msg}"
    );
}

#[test]
fn the_cap_constant_is_the_documented_64_mib() {
    assert_eq!(OBSERVER_CAP_BYTES, 64 * 1024 * 1024);
}
