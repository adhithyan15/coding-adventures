//! WX-F slice 4: driver adapter generation, asserted as text. The obligations
//! themselves execute in `tests/wxf_driver_evidence.rs`; here the adapter
//! variants, pins, registrations and refusal paths are pinned down.

use coding_adventures_warrant_grammar::binding::parse_evidence_bindings;
use coding_adventures_warrant_grammar::checker::check_warrant;
use coding_adventures_warrant_grammar::codegen::DiagnosticCode as CodegenCode;
use coding_adventures_warrant_grammar::driver::validate_driver_consistency;
use coding_adventures_warrant_grammar::evidence::{compile_rust_evidence_harness, EvidenceOptions};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

const HEAD: &str = "package subject\nrevision 0\nspec code/specs/t.warrant\noutput code/packages/rust/subject/tests/generated/t.rs\nseed 1\nproperty_cases 10\nfuzz_iterations 10\nnegctl_fuzz_iterations 10\nexhaustive_budget 400000\n";

fn bindings(op_line: &str, driver_line: &str) -> String {
    format!("{HEAD}{op_line}\n{driver_line}\n")
}

fn options() -> EvidenceOptions {
    EvidenceOptions {
        package_crate: "subject".into(),
        spec_path: "code/specs/t.warrant".into(),
        spec_sha256: "0".repeat(64),
        seed: 1,
        property_cases: 10,
        fuzz_iterations: 10,
        negctl_fuzz_iterations: 10,
        exhaustive_budget: 400000,
        arg_types: std::collections::BTreeMap::new(),
        hosts: Default::default(),
    }
}

/// Parse + check + validate + generate: the full `compile_` path.
fn generate(spec: &str, bindings_text: &str) -> Result<String, CodegenCode> {
    let doc = SourceDocument::admit(spec.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");
    let checked = check_warrant(&file).expect("fixture must type-check");
    let b = parse_evidence_bindings(bindings_text).expect("bindings parse");
    validate_driver_consistency(&file, &b).expect("fixture must validate");
    compile_rust_evidence_harness(&file, &checked, &b, &options()).map_err(|e| e.code)
}

fn generates(spec: &str, op_line: &str, driver_line: &str) -> String {
    let text = bindings(op_line, driver_line);
    generate(spec, &text).expect("generation must succeed")
}

const SPEC_SHA1: &str = "warrant t v1\n  op digest_chunks(chunks: List<Bytes>) -> Bytes<20>\n  labels digest_chunks:\n    chunks public\n  claim c:\n    forall chunks: List<Bytes>. digest_chunks(chunks) == digest_chunks(chunks)\n";
const OP_SHA1: &str = "op digest_chunks driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step";
const DRIVER_SHA1: &str = "driver digest_chunks state=subject::Digest new=new() drive=chunks step=update(mut,elem:bytes) finish=sum1(ref)->bytes result=finish";

#[test]
fn adapter_calls_constructor_step_finish_in_order() {
    let out = generates(SPEC_SHA1, OP_SHA1, DRIVER_SHA1);
    let new = out.find("Digest::new()").expect("constructor");
    let update = out.find("st.update(").expect("step");
    let sum1 = out.find("st.sum1()").expect("finish");
    assert!(new < update && update < sum1, "drive order wrong");
    assert!(out.contains("for step in 0..peril"), "loop missing");
}

#[test]
fn adapter_pins_step_shape_exactly() {
    let out = generates(SPEC_SHA1, OP_SHA1, DRIVER_SHA1);
    assert!(
        out.contains("const _: fn(&mut subject::Digest, &[u8]) = subject::Digest::update;"),
        "step pin missing"
    );
}

#[test]
fn bytes_finish_pins_asref_not_exact() {
    // `sum1 -> [u8; 20]` satisfies `bytes` without spelling it: the pin is a
    // trait bound, so any `AsRef<[u8]>` return passes and a scalar fails.
    let out = generates(SPEC_SHA1, OP_SHA1, DRIVER_SHA1);
    assert!(out.contains("B: AsRef<[u8]>"), "flexible pin missing");
    assert!(out.contains("assert_sig(subject::Digest::sum1);"), "pin call missing");
}

#[test]
fn misspelled_method_surfaces_in_pins_and_calls() {
    // `shape` fails at compile time: both the pin and the call name the
    // method, so either way `rustc` reports the misspelling.
    let driver = "driver digest_chunks state=subject::Digest new=new() drive=chunks step=updat(mut,elem:bytes) finish=sum1(ref)->bytes result=finish";
    let out = generates(SPEC_SHA1, OP_SHA1, driver);
    assert!(out.contains("Digest::updat"), "pin should name it");
    assert!(out.contains("st.updat("), "call should name it");
}

#[test]
fn obligations_register_with_driver_ids() {
    let out = generates(SPEC_SHA1, OP_SHA1, DRIVER_SHA1);
    for ob in [
        "shape",
        "empty",
        "empty-element",
        "deterministic",
        "finish-observational",
        "steps-cap",
    ] {
        assert!(
            out.contains(&format!("warrant/t/driver/digest_chunks/{ob}")),
            "registration id missing for {ob}"
        );
    }
    assert!(out.contains("driver_note_obligations(\"digest_chunks\""), "note call missing");
    assert!(out.contains("driver_note_steps(\"digest_chunks\", peril);"), "steps hook missing");
    assert!(out.contains("driver_note_call();"), "call hook missing");
}

#[test]
fn collect_mode_gathers_values() {
    let spec = "warrant t v1\n  op dec_all(bs: List<Bytes>) -> List<U8>\n  labels dec_all:\n    bs public\n  claim c:\n    forall bs: List<Bytes>. dec_all(bs) == dec_all(bs)\n";
    let out = generates(
        spec,
        "op dec_all driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,drop-last-output",
        "driver dec_all state=subject::Dec new=new() drive=bs step=dec(mut,elem:bytes)->u8 result=collect",
    );
    assert!(out.contains("let mut collected: Vec<V> = Vec::new();"), "collector missing");
    assert!(out.contains("V::List(collected)"), "list shaping missing");
}

#[test]
fn both_mode_returns_a_pair() {
    let spec = "warrant t v1\n  op scan(xs: List<U8>) -> (List<U8>, Nat)\n  labels scan:\n    xs public\n  claim c:\n    forall xs: List<U8>. scan(xs) == scan(xs)\n";
    let out = generates(
        spec,
        "op scan driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step,drop-last-output",
        "driver scan state=subject::Scan new=new() drive=xs step=push(mut,elem:u8)->u8 finish=len(ref)->u32 result=both",
    );
    assert!(out.contains("V::Tup(vec![V::List(collected)"), "pair shaping missing");
}

#[test]
fn nat_drive_counts_without_elements() {
    let spec = "warrant t v1\n  op fill(n: Nat) -> Bytes\n  labels fill:\n    n public\n  claim c:\n    forall n: Nat. fill(n) == fill(n)\n";
    let out = generates(
        spec,
        "op fill driver faults=drop-last-chunk,duplicate-last-chunk,early-finish,reset-per-step",
        "driver fill state=subject::Fill new=new() drive=n step=tick(mut) finish=out(ref)->bytes result=finish",
    );
    assert!(out.contains("Nat drive is not an integer"), "count extraction missing");
    assert!(!out.contains("elems"), "nat drive must not name elements");
}

#[test]
fn ctx_value_builds_once_and_passes_by_reference() {
    let spec = "warrant t v1\n  op keyed(chunks: List<Bytes>, key: U8) -> Bytes\n  labels keyed:\n    chunks public\n    key public\n  claim c:\n    forall chunks: List<Bytes>, key: U8. keyed(chunks, key) == keyed(chunks, key)\n";
    let out = generates(
        spec,
        "op keyed driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step",
        "driver keyed state=subject::K ctx=mkctx(param.key:u8) new=new(ctx) drive=chunks step=update(mut,elem:bytes) finish=sum(ref)->bytes result=finish",
    );
    assert!(out.contains("let ctxv = mkctx("), "ctx build missing");
    assert!(out.contains("::new(&ctxv)"), "ctx passes by reference");
}

#[test]
fn seq_label_exists_only_for_fallible_drives() {
    // The `'seq` label skips aborted sequences; emitting it without a
    // fallible call would trip clippy in generated code.
    let fallible_spec = "warrant t v1\n  error E = TOO_LONG\n  op digest_try(chunks: List<Bytes>) -> Result<Bytes, E>\n  labels digest_try:\n    chunks public\n  claim c:\n    forall chunks: List<Bytes>. digest_try(chunks) == digest_try(chunks)\n";
    let out = generates(
        fallible_spec,
        "op digest_try driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step",
        "driver digest_try state=subject::D new=new() drive=chunks step=try_update(mut,elem:bytes)->result finish=digest(ref)->bytes result=finish errors=subject::E/LengthOverflow=TOO_LONG",
    );
    assert!(out.contains("'seq: for s0"), "label missing for fallible drive");
    let total = generates(SPEC_SHA1, OP_SHA1, DRIVER_SHA1);
    assert!(
        !total.contains("'seq:"),
        "spurious label on total drive"
    );
}

#[test]
fn fallible_step_maps_errors_and_stops() {
    let spec = "warrant t v1\n  error E = TOO_LONG\n  op digest_try(chunks: List<Bytes>) -> Result<Bytes, E>\n  labels digest_try:\n    chunks public\n  claim c:\n    forall chunks: List<Bytes>. digest_try(chunks) == digest_try(chunks)\n";
    let out = generates(
        spec,
        "op digest_try driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step",
        "driver digest_try state=subject::D new=new() drive=chunks step=try_update(mut,elem:bytes)->result finish=digest(ref)->bytes result=finish errors=subject::E/LengthOverflow=TOO_LONG",
    );
    assert!(out.contains("V::Err(\"TOO_LONG\".into())"), "error mapping missing");
    assert!(out.contains("V::Ok(Box::new("), "ok wrap missing");
}

#[test]
fn error_normalization_contains_subject_panics() {
    let spec = "warrant t v1\n  error E = BAD\n  op f(xs: List<Bytes>) -> Result<Bytes, E>\n  labels f:\n    xs public\n  claim c:\n    forall xs: List<Bytes>. f(xs) == f(xs)\n";
    let out = generates(
        spec,
        "op f driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step error=BAD",
        "driver f state=subject::F new=new() drive=xs step=go(mut,elem:bytes) finish=out(ref)->bytes result=finish",
    );
    assert!(out.contains("match contained(||"), "containment missing");
    assert!(out.contains("V::Err(\"BAD\".into())"), "normalization missing");
}

#[test]
fn missing_error_flips_step_errors() {
    let spec = "warrant t v1\n  error E = TOO_LONG\n  op digest_try(chunks: List<Bytes>) -> Result<Bytes, E>\n  labels digest_try:\n    chunks public\n  claim c:\n    forall chunks: List<Bytes>. digest_try(chunks) == digest_try(chunks)\n";
    let out = generates(
        spec,
        "op digest_try driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step,missing-error",
        "driver digest_try state=subject::D new=new() drive=chunks step=try_update(mut,elem:bytes)->result finish=digest(ref)->bytes result=finish errors=subject::E/LengthOverflow=TOO_LONG",
    );
    assert!(
        out.contains("Err(_) if fault == Fault::MissingError"),
        "missing arm missing"
    );
}

#[test]
fn driver_with_inout_is_refused() {
    let spec = "warrant t v1\n  op f(a: inout Bytes) -> Bytes\n  labels f:\n    a public\n  claim c:\n    forall a: Bytes. f(a) == f(a)\n";
    let text = bindings(
        "op f driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step",
        "driver f state=subject::F new=new() drive=a step=go(mut,elem:u8) finish=out(ref)->bytes result=finish",
    );
    assert_eq!(generate(spec, &text), Err(CodegenCode::W0704));
}

#[test]
fn missing_error_on_total_driver_is_refused() {
    let text = bindings(
        "op digest_chunks driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step,missing-error",
        DRIVER_SHA1,
    );
    assert_eq!(generate(SPEC_SHA1, &text), Err(CodegenCode::W0701));
}

#[test]
fn non_driver_fault_on_driver_is_refused() {
    let text = bindings(
        "op digest_chunks driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step,flip-result-bit",
        DRIVER_SHA1,
    );
    assert_eq!(generate(SPEC_SHA1, &text), Err(CodegenCode::W0701));
}

#[test]
fn hostile_strings_in_hand_built_records_are_refused() {
    let doc = SourceDocument::admit(SPEC_SHA1.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");
    let checked = check_warrant(&file).expect("fixture must type-check");
    let text = bindings(OP_SHA1, DRIVER_SHA1);
    let mut b = parse_evidence_bindings(&text).expect("bindings parse");
    // Swap hostile method text into the valid record: the line parser would
    // never produce this, so only generation's re-validation can catch it.
    b.manifest.bindings
        .get_mut("digest_chunks")
        .unwrap()
        .driver
        .as_mut()
        .unwrap()
        .step
        .method = "update(); evil(); //".to_string();
    let err = compile_rust_evidence_harness(&file, &checked, &b, &options()).expect_err("must refuse");
    assert_eq!(err.code, CodegenCode::W0701);
}
