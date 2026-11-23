//! VER04k 3.4 rule 4, 3.5 and 3.6 (fault set): `faults=` must list every
//! applicable driver fault (W0502) and none that does not apply (W0701),
//! and a driver needing a two-element exercise must get one from some claim
//! or example (W0601).

use coding_adventures_warrant_grammar::binding::parse_evidence_bindings;
use coding_adventures_warrant_grammar::checker::{check_warrant, CheckerErrorKind};
use coding_adventures_warrant_grammar::codegen::DiagnosticCode as CodegenCode;
use coding_adventures_warrant_grammar::driver::validate_driver_consistency;
use coding_adventures_warrant_grammar::evidence::{compile_rust_evidence_harness, EvidenceOptions};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

const HEAD: &str = "package subject\nrevision 0\nspec code/specs/t.warrant\noutput code/packages/rust/subject/tests/generated/t.rs\nseed 1\nproperty_cases 10\nfuzz_iterations 10\nnegctl_fuzz_iterations 10\nexhaustive_budget 400000\n";

const FINISH5: &str =
    "drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step";
const COLLECT5: &str =
    "drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,drop-last-output";
const BOTH6: &str = "drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step,drop-last-output";
const NAT4: &str = "drop-last-chunk,duplicate-last-chunk,early-finish,reset-per-step";

fn bindings(op_line: &str, driver_line: &str) -> String {
    format!("{HEAD}{op_line}\n{driver_line}\n")
}

/// Parse + check `spec` (a bad fixture fails the test), then run driver
/// consistency: `Ok(())` or the `(code, message)` pair.
fn check(spec: &str, bindings_text: &str) -> Result<(), (String, String)> {
    let doc = SourceDocument::admit(spec.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");
    check_warrant(&file).expect("fixture must type-check");
    let b = parse_evidence_bindings(bindings_text).expect("bindings parse");
    validate_driver_consistency(&file, &b).map_err(|e| {
        let code = e.code.to_string();
        match e.kind {
            CheckerErrorKind::DriverFaultSet(m) => (code, m),
            other => panic!("wrong kind: {other:?}"),
        }
    })
}

fn ok(spec: &str, op_line: &str, driver_line: &str) {
    let text = bindings(op_line, driver_line);
    assert_eq!(check(spec, &text), Ok(()), "{spec}\n{text}");
}

fn bad(spec: &str, op_line: &str, driver_line: &str, code: &str, needle: &str) {
    let text = bindings(op_line, driver_line);
    let (got_code, msg) = check(spec, &text).expect_err("must fail");
    assert_eq!(got_code, code, "{spec}\n{text}");
    assert!(msg.contains(needle), "missing `{needle}` in: {msg}\n{spec}\n{text}");
}

const SPEC_SHA1: &str = "warrant t v1\n  op digest_chunks(chunks: List<Bytes>) -> Bytes<20>\n  labels digest_chunks:\n    chunks public\n  claim c:\n    forall chunks: List<Bytes>. digest_chunks(chunks) == digest_chunks(chunks)\n";
const DRIVER_SHA1: &str = "driver digest_chunks state=subject::Digest new=new() drive=chunks step=update(mut,elem:bytes) finish=sum1(ref)->bytes result=finish";

fn op_sha1(faults: &str) -> String {
    format!("op digest_chunks driver faults={faults}")
}

// --- W0502: every applicable fault listed ---

#[test]
fn full_finish_set_passes() {
    ok(SPEC_SHA1, &op_sha1(FINISH5), DRIVER_SHA1);
}

#[test]
fn missing_reorder_chunks_is_w0502() {
    let faults = "drop-last-chunk,duplicate-last-chunk,early-finish,reset-per-step";
    bad(
        SPEC_SHA1,
        &op_sha1(faults),
        DRIVER_SHA1,
        "W0502",
        "reorder-chunks",
    );
}

#[test]
fn missing_several_faults_lists_all() {
    bad(
        SPEC_SHA1,
        &op_sha1("drop-last-chunk"),
        DRIVER_SHA1,
        "W0502",
        "missing driver faults: reorder-chunks, duplicate-last-chunk, early-finish, reset-per-step",
    );
}

#[test]
fn collect_mode_missing_drop_last_output_is_w0502() {
    let spec = "warrant t v1\n  op dec_all(bs: List<Bytes>) -> List<U8>\n  labels dec_all:\n    bs public\n  claim c:\n    forall bs: List<Bytes>. dec_all(bs) == dec_all(bs)\n";
    let op = format!(
        "op dec_all driver faults={}",
        "drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish"
    );
    bad(
        spec,
        &op,
        "driver dec_all state=subject::Dec new=new() drive=bs step=dec(mut,elem:bytes)->u8 result=collect",
        "W0502",
        "drop-last-output",
    );
}

#[test]
fn collect_mode_does_not_require_reset() {
    let spec = "warrant t v1\n  op dec_all(bs: List<Bytes>) -> List<U8>\n  labels dec_all:\n    bs public\n  claim c:\n    forall bs: List<Bytes>. dec_all(bs) == dec_all(bs)\n";
    ok(
        spec,
        &format!("op dec_all driver faults={COLLECT5}"),
        "driver dec_all state=subject::Dec new=new() drive=bs step=dec(mut,elem:bytes)->u8 result=collect",
    );
}

#[test]
fn nat_driver_does_not_require_reorder() {
    let spec = "warrant t v1\n  op fill(n: Nat) -> Bytes\n  labels fill:\n    n public\n  claim c:\n    forall n: Nat. fill(n) == fill(n)\n";
    ok(
        spec,
        &format!("op fill driver faults={NAT4}"),
        "driver fill state=subject::Fill new=new() drive=n step=tick(mut) finish=out(ref)->bytes result=finish",
    );
}

#[test]
fn bytes1_driver_needs_only_drop_and_duplicate() {
    let spec = "warrant t v1\n  op h1(m: Bytes<1>) -> Bytes\n  labels h1:\n    m public\n  claim c:\n    forall m: Bytes<1>. h1(m) == h1(m)\n";
    ok(
        spec,
        "op h1 driver faults=drop-last-chunk,duplicate-last-chunk",
        "driver h1 state=subject::H new=new() drive=m step=absorb(mut,elem:u8) finish=out(ref)->bytes result=finish",
    );
}

#[test]
fn both_mode_requires_all_six() {
    let spec = "warrant t v1\n  op scan(xs: List<U8>) -> (List<U8>, Nat)\n  labels scan:\n    xs public\n  claim c:\n    forall xs: List<U8>. scan(xs) == scan(xs)\n";
    ok(
        spec,
        &format!("op scan driver faults={BOTH6}"),
        "driver scan state=subject::Scan new=new() drive=xs step=push(mut,elem:u8)->u8 finish=len(ref)->u32 result=both",
    );
}

// --- W0701: nothing inapplicable listed ---

#[test]
fn drop_last_output_on_finish_mode_is_w0701() {
    bad(
        SPEC_SHA1,
        &op_sha1(BOTH6),
        DRIVER_SHA1,
        "W0701",
        "drop-last-output",
    );
}

#[test]
fn reset_on_collect_mode_is_w0701() {
    let spec = "warrant t v1\n  op dec_all(bs: List<Bytes>) -> List<U8>\n  labels dec_all:\n    bs public\n  claim c:\n    forall bs: List<Bytes>. dec_all(bs) == dec_all(bs)\n";
    let op = format!("op dec_all driver faults={BOTH6}");
    bad(
        spec,
        &op,
        "driver dec_all state=subject::Dec new=new() drive=bs step=dec(mut,elem:bytes)->u8 result=collect",
        "W0701",
        "reset-per-step",
    );
}

#[test]
fn reorder_on_nat_driver_is_w0701() {
    let spec = "warrant t v1\n  op fill(n: Nat) -> Bytes\n  labels fill:\n    n public\n  claim c:\n    forall n: Nat. fill(n) == fill(n)\n";
    let op = format!("op fill driver faults={NAT4},reorder-chunks");
    bad(
        spec,
        &op,
        "driver fill state=subject::Fill new=new() drive=n step=tick(mut) finish=out(ref)->bytes result=finish",
        "W0701",
        "reorder-chunks",
    );
}

#[test]
fn reorder_on_bytes1_is_w0701() {
    let spec = "warrant t v1\n  op h1(m: Bytes<1>) -> Bytes\n  labels h1:\n    m public\n  claim c:\n    forall m: Bytes<1>. h1(m) == h1(m)\n";
    bad(
        spec,
        "op h1 driver faults=drop-last-chunk,duplicate-last-chunk,reorder-chunks",
        "driver h1 state=subject::H new=new() drive=m step=absorb(mut,elem:u8) finish=out(ref)->bytes result=finish",
        "W0701",
        "reorder-chunks",
    );
}

#[test]
fn non_driver_faults_are_not_judged_here() {
    // Slice boundary: `missing-error` on a non-Result op is for the existing
    // per-fault machinery; the driver fault-set check ignores faults outside
    // the six and still passes on a complete driver set.
    let op = op_sha1(&format!("{FINISH5},missing-error"));
    ok(SPEC_SHA1, &op, DRIVER_SHA1);
}

// --- W0601: a two-element exercise from some claim or example ---

const SPEC_NOCLAIM: &str = "warrant t v1\n  op digest_chunks(chunks: List<Bytes>) -> Bytes<20>\n  labels digest_chunks:\n    chunks public\n";

#[test]
fn driver_no_claim_calls_is_w0601() {
    bad(
        SPEC_NOCLAIM,
        &op_sha1(FINISH5),
        DRIVER_SHA1,
        "W0601",
        "driver not exercised",
    );
}

#[test]
fn claim_calling_with_empty_list_is_w0601() {
    let spec = "warrant t v1\n  op digest_chunks(chunks: List<Bytes>) -> Bytes<20>\n  labels digest_chunks:\n    chunks public\n  claim c:\n    digest_chunks([]) == digest_chunks([])\n";
    bad(spec, &op_sha1(FINISH5), DRIVER_SHA1, "W0601", "not exercised");
}

#[test]
fn claim_calling_with_singleton_is_w0601() {
    let spec = "warrant t v1\n  op digest_chunks(chunks: List<Bytes>) -> Bytes<20>\n  labels digest_chunks:\n    chunks public\n  claim c:\n    forall m: Bytes. digest_chunks([m]) == digest_chunks([m])\n";
    bad(spec, &op_sha1(FINISH5), DRIVER_SHA1, "W0601", "not exercised");
}

#[test]
fn claim_calling_with_two_empty_elements_is_w0601() {
    let spec = "warrant t v1\n  op digest_chunks(chunks: List<Bytes>) -> Bytes<20>\n  labels digest_chunks:\n    chunks public\n  claim c:\n    forall x: Bytes. digest_chunks([b\"\", x]) == digest_chunks([b\"\", x])\n";
    bad(spec, &op_sha1(FINISH5), DRIVER_SHA1, "W0601", "not exercised");
}

#[test]
fn example_with_two_element_call_exercises() {
    let spec = "warrant t v1\n  model id2(x: Bytes) -> Bytes = x\n  op digest_chunks(chunks: List<Bytes>) -> Bytes<20>\n  labels digest_chunks:\n    chunks public\n  example e:\n    digest_chunks([id2(b\"00\"), id2(b\"01\")]) == digest_chunks([id2(b\"00\"), id2(b\"01\")])\n";
    ok(spec, &op_sha1(FINISH5), DRIVER_SHA1);
}

#[test]
fn model_call_elements_exercise() {
    let spec = "warrant t v1\n  model id2(x: Bytes) -> Bytes = x\n  op digest_chunks(chunks: List<Bytes>) -> Bytes<20>\n  labels digest_chunks:\n    chunks public\n  claim c:\n    forall m: Bytes. digest_chunks([id2(m), m]) == digest_chunks([id2(m), m])\n";
    ok(spec, &op_sha1(FINISH5), DRIVER_SHA1);
}

#[test]
fn conditional_with_big_branch_exercises() {
    let spec = "warrant t v1\n  op digest_chunks(chunks: List<Bytes>) -> Bytes<20>\n  labels digest_chunks:\n    chunks public\n  claim c:\n    forall c: Bool, m: Bytes, n: Bytes. (if c then digest_chunks([m, n]) else digest_chunks([])) == digest_chunks([])\n";
    ok(spec, &op_sha1(FINISH5), DRIVER_SHA1);
}

#[test]
fn grouped_two_element_call_exercises() {
    let spec = "warrant t v1\n  op digest_chunks(chunks: List<Bytes>) -> Bytes<20>\n  labels digest_chunks:\n    chunks public\n  claim c:\n    forall m: Bytes, n: Bytes. digest_chunks(([m, n])) == digest_chunks([m, n])\n";
    ok(spec, &op_sha1(FINISH5), DRIVER_SHA1);
}

#[test]
fn nat_literal_two_exercises_nat_driver() {
    let spec = "warrant t v1\n  op fill(n: Nat) -> Bytes\n  labels fill:\n    n public\n  example e:\n    fill(5) == fill(5)\n";
    ok(
        spec,
        &format!("op fill driver faults={NAT4}"),
        "driver fill state=subject::Fill new=new() drive=n step=tick(mut) finish=out(ref)->bytes result=finish",
    );
}

#[test]
fn nat_literal_one_does_not_exercise() {
    let spec = "warrant t v1\n  op fill(n: Nat) -> Bytes\n  labels fill:\n    n public\n  example e:\n    fill(1) == fill(1)\n";
    bad(
        spec,
        &format!("op fill driver faults={NAT4}"),
        "driver fill state=subject::Fill new=new() drive=n step=tick(mut) finish=out(ref)->bytes result=finish",
        "W0601",
        "not exercised",
    );
}

#[test]
fn bytes_literal_length_two_exercises() {
    let spec = "warrant t v1\n  op h(m: Bytes) -> Bytes\n  labels h:\n    m public\n  example e:\n    h(b\"0011\") == h(b\"0011\")\n";
    ok(
        spec,
        &format!("op h driver faults={FINISH5}"),
        "driver h state=subject::H new=new() drive=m step=absorb(mut,elem:u8) finish=out(ref)->bytes result=finish",
    );
}

#[test]
fn bytes_literal_length_one_does_not_exercise() {
    let spec = "warrant t v1\n  op h(m: Bytes) -> Bytes\n  labels h:\n    m public\n  example e:\n    h(b\"00\") == h(b\"00\")\n";
    bad(
        spec,
        &format!("op h driver faults={FINISH5}"),
        "driver h state=subject::H new=new() drive=m step=absorb(mut,elem:u8) finish=out(ref)->bytes result=finish",
        "W0601",
        "not exercised",
    );
}

#[test]
fn driven_index_selects_the_right_argument() {
    // `drive=b`: a big first argument must not count; only `b` matters.
    let spec = "warrant t v1\n  op two(a: List<Bytes>, b: List<Bytes>) -> Bytes\n  labels two:\n    a public\n    b public\n  claim c:\n    forall a: List<Bytes>. two(a, []) == two(a, [])\n";
    bad(
        spec,
        "op two driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step",
        "driver two state=subject::T new=new(param.a:list<bytes>) drive=b step=absorb(mut,elem:bytes) finish=out(ref)->bytes result=finish",
        "W0601",
        "not exercised",
    );
}

#[test]
fn match_arm_call_exercises() {
    let spec = "warrant t v1\n  op digest_chunks(chunks: List<Bytes>) -> Bytes<20>\n  labels digest_chunks:\n    chunks public\n  claim c:\n    forall c: Bool, m: Bytes, n: Bytes. (match c with true -> digest_chunks([m, n]) | false -> digest_chunks([])) == digest_chunks([])\n";
    ok(spec, &op_sha1(FINISH5), DRIVER_SHA1);
}

// --- End to end: fault codes survive the evidence boundary ---

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

fn compile_code(spec: &str, bindings_text: &str) -> CodegenCode {
    let doc = SourceDocument::admit(spec.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");
    let checked = check_warrant(&file).expect("fixture must type-check");
    let b = parse_evidence_bindings(bindings_text).expect("bindings parse");
    compile_rust_evidence_harness(&file, &checked, &b, &options())
        .expect_err("generation cannot succeed")
        .code
}

#[test]
fn missing_fault_reports_w0502_through_compile() {
    let text = bindings(
        &op_sha1("drop-last-chunk"),
        DRIVER_SHA1,
    );
    assert_eq!(compile_code(SPEC_SHA1, &text), CodegenCode::W0502);
}

#[test]
fn inapplicable_fault_reports_w0701_through_compile() {
    let text = bindings(&op_sha1(BOTH6), DRIVER_SHA1);
    assert_eq!(compile_code(SPEC_SHA1, &text), CodegenCode::W0701);
}

#[test]
fn unexercised_driver_reports_w0601_through_compile() {
    let text = bindings(&op_sha1(FINISH5), DRIVER_SHA1);
    assert_eq!(compile_code(SPEC_NOCLAIM, &text), CodegenCode::W0601);
}
