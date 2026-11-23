//! VER04k 3.3, 3.4 rules 5/7 and 3.11 (consistency negatives): a driver
//! record typed against its operation. Each positive is a shape the rules
//! accept; each negative names the rule that refuses it with W0711.

use coding_adventures_warrant_grammar::binding::parse_evidence_bindings;
use coding_adventures_warrant_grammar::checker::{check_warrant, CheckerErrorKind};
use coding_adventures_warrant_grammar::codegen::DiagnosticCode as CodegenCode;
use coding_adventures_warrant_grammar::driver::{validate_driver_consistency, ArgSource};
use coding_adventures_warrant_grammar::evidence::{compile_rust_evidence_harness, EvidenceOptions};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

const HEAD: &str = "package subject\nrevision 0\nspec code/specs/t.warrant\noutput code/packages/rust/subject/tests/generated/t.rs\nseed 1\nproperty_cases 10\nfuzz_iterations 10\nnegctl_fuzz_iterations 10\nexhaustive_budget 400000\n";

fn bindings(op_line: &str, driver_line: &str) -> String {
    format!("{HEAD}{op_line}\n{driver_line}\n")
}

fn bindings_extra(op_line: &str, driver_line: &str, extra: &str) -> String {
    format!("{HEAD}{extra}\n{op_line}\n{driver_line}\n")
}

/// Parse + check `spec` (a bad fixture fails the test, never W0711), then
/// run driver consistency: `Ok(())` or the `W0711` message.
fn check(spec: &str, bindings_text: &str) -> Result<(), String> {
    let doc = SourceDocument::admit(spec.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");
    check_warrant(&file).expect("fixture must type-check");
    let b = parse_evidence_bindings(bindings_text).expect("bindings parse");
    validate_driver_consistency(&file, &b).map_err(|e| {
        assert_eq!(e.code.to_string(), "W0711");
        match e.kind {
            CheckerErrorKind::DriverInconsistency(m) => m,
            other => panic!("wrong kind: {other:?}"),
        }
    })
}

fn ok(spec: &str, op_line: &str, driver_line: &str) {
    let text = bindings(op_line, driver_line);
    assert_eq!(check(spec, &text), Ok(()), "{spec}\n{text}");
}

fn bad(spec: &str, op_line: &str, driver_line: &str, needle: &str) {
    let text = bindings(op_line, driver_line);
    let err = check(spec, &text).expect_err("must be W0711");
    assert!(
        err.contains(needle),
        "missing `{needle}` in: {err}\n{spec}\n{text}"
    );
}

const SPEC_SHA1: &str = "warrant t v1\n  op digest_chunks(chunks: List<Bytes>) -> Bytes<20>\n  labels digest_chunks:\n    chunks public\n  claim c:\n    forall chunks: List<Bytes>. digest_chunks(chunks) == digest_chunks(chunks)\n";
const OP_SHA1: &str = "op digest_chunks driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step";
const DRIVER_SHA1: &str = "driver digest_chunks state=subject::Digest new=new() drive=chunks step=update(mut,elem:bytes) finish=sum1(ref)->bytes result=finish";

#[test]
fn sha1_shaped_finish_driver_is_consistent() {
    ok(SPEC_SHA1, OP_SHA1, DRIVER_SHA1);
}

#[test]
fn lengths_allowed_on_bytes_elements() {
    ok(SPEC_SHA1, OP_SHA1, &format!("{DRIVER_SHA1} lengths=55,56"));
}

#[test]
fn collect_driver_is_consistent() {
    let spec = "warrant t v1\n  op dec_all(bs: List<Bytes>) -> List<U8>\n  labels dec_all:\n    bs public\n  claim c:\n    forall bs: List<Bytes>. dec_all(bs) == dec_all(bs)\n";
    ok(
        spec,
        "op dec_all driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,drop-last-output",
        "driver dec_all state=subject::Dec new=new() drive=bs step=dec(mut,elem:bytes)->u8 result=collect",
    );
}

#[test]
fn both_driver_is_consistent() {
    let spec = "warrant t v1\n  op scan(xs: List<U8>) -> (List<U8>, Nat)\n  labels scan:\n    xs public\n  claim c:\n    forall xs: List<U8>. scan(xs) == scan(xs)\n";
    ok(
        spec,
        "op scan driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step,drop-last-output",
        "driver scan state=subject::Scan new=new() drive=xs step=push(mut,elem:u8)->u8 finish=len(ref)->u32 result=both",
    );
}

#[test]
fn nat_count_driver_is_consistent() {
    let spec = "warrant t v1\n  op fill(n: Nat) -> Bytes\n  labels fill:\n    n public\n  claim c:\n    forall n: Nat. fill(n) == fill(n)\n";
    ok(
        spec,
        "op fill driver faults=drop-last-chunk,duplicate-last-chunk,early-finish,reset-per-step",
        "driver fill state=subject::Fill new=new() drive=n step=tick(mut) finish=out(ref)->bytes result=finish",
    );
}

#[test]
fn fallible_driver_with_errors_map_is_consistent() {
    let spec = "warrant t v1\n  error E = TOO_LONG\n  op digest_try(chunks: List<Bytes>) -> Result<Bytes, E>\n  labels digest_try:\n    chunks public\n  claim c:\n    forall chunks: List<Bytes>. digest_try(chunks) == digest_try(chunks)\n";
    ok(
        spec,
        "op digest_try driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step",
        "driver digest_try state=subject::D new=new() drive=chunks step=try_update(mut,elem:bytes)->result finish=digest(ref)->bytes result=finish errors=subject::E/LengthOverflow=TOO_LONG",
    );
}

#[test]
fn tuple_element_path_is_consistent() {
    let spec = "warrant t v1\n  op encode_bits(bits: List<(Bool, U8)>) -> Bytes\n  labels encode_bits:\n    bits public\n  claim c:\n    forall bits: List<(Bool, U8)>. encode_bits(bits) == encode_bits(bits)\n";
    ok(
        spec,
        "op encode_bits driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step",
        "driver encode_bits state=subject::Enc new=new() drive=bits step=write_bit(mut,elem.0:bool,elem.1:u8) finish=finish(own)->bytes result=finish",
    );
}

#[test]
fn record_element_path_is_consistent() {
    let spec = "warrant t v1\n  record Rec(a: U8, b: Bool)\n  op pack(rs: List<Rec>) -> Bytes\n  labels pack:\n    rs public\n  claim c:\n    forall rs: List<Rec>. pack(rs) == pack(rs)\n";
    ok(
        spec,
        "op pack driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step",
        "driver pack state=subject::Pack new=new() drive=rs step=push(mut,elem.0:u8,elem.1:bool) finish=seal(ref)->bytes result=finish",
    );
}

#[test]
fn second_parameter_consumed_by_new_is_consistent() {
    let spec = "warrant t v1\n  op keyed(chunks: List<Bytes>, key: U8) -> Bytes\n  labels keyed:\n    chunks public\n    key public\n  claim c:\n    forall chunks: List<Bytes>, key: U8. keyed(chunks, key) == keyed(chunks, key)\n";
    ok(
        spec,
        "op keyed driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step",
        "driver keyed state=subject::K new=new(param.key:u8) drive=chunks step=update(mut,elem:bytes) finish=sum(ref)->bytes result=finish",
    );
}

#[test]
fn secret_labels_with_driver_opt_in_is_consistent() {
    let spec = "warrant t v1\n  op digest_chunks(chunks: List<Bytes>) -> Bytes<20>\n  labels digest_chunks:\n    chunks secret\n  claim c:\n    forall chunks: List<Bytes>. digest_chunks(chunks) == digest_chunks(chunks)\n";
    ok(
        spec,
        "op digest_chunks driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step proofs_not_live=driver",
        DRIVER_SHA1,
    );
}

#[test]
fn bound_row_with_driver_opt_in_is_consistent() {
    let spec = "warrant t v1\n  op digest_chunks(chunks: List<Bytes>) -> Bytes<20>\n  labels digest_chunks:\n    chunks public\n  bound digest_chunks:\n    steps <= 10\n  claim c:\n    forall chunks: List<Bytes>. digest_chunks(chunks) == digest_chunks(chunks)\n";
    ok(
        spec,
        "op digest_chunks driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step proofs_not_live=driver",
        DRIVER_SHA1,
    );
}

#[test]
fn export_op_is_untouched() {
    let text = format!("{HEAD}op digest_chunks export=ct_digest faults=x\n");
    assert_eq!(check(SPEC_SHA1, &text), Ok(()));
}

#[test]
fn phantom_named_host_is_w0711() {
    let spec = "warrant t v1\n  record Rec(a: U8, b: Bool)\n  op pack(rs: List<Rec>) -> Bytes\n  labels pack:\n    rs public\n  claim c:\n    forall rs: List<Rec>. pack(rs) == pack(rs)\n";
    bad(
        spec,
        "op pack driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step",
        "driver pack state=subject::Pack new=new() drive=rs step=push(mut,elem:Rec) finish=seal(ref)->bytes result=finish",
        "no `struct`/`enum` line declares",
    );
}

#[test]
fn declared_struct_host_is_consistent() {
    let spec = "warrant t v1\n  record Rec(a: U8, b: Bool)\n  op pack(rs: List<Rec>) -> Bytes\n  labels pack:\n    rs public\n  claim c:\n    forall rs: List<Rec>. pack(rs) == pack(rs)\n";
    let text = bindings_extra(
        "op pack driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step",
        "driver pack state=subject::Pack new=new() drive=rs step=push(mut,elem:Rec) finish=seal(ref)->bytes result=finish",
        "struct Rec path=subject::Rec fields=a:u8,b:bool",
    );
    assert_eq!(check(spec, &text), Ok(()), "{spec}\n{text}");
}

#[test]
fn hand_built_deep_path_is_w0711() {
    let doc = SourceDocument::admit(SPEC_SHA1.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");
    check_warrant(&file).expect("fixture must type-check");
    let text = bindings(OP_SHA1, DRIVER_SHA1);
    let mut b = parse_evidence_bindings(&text).expect("bindings parse");
    b.manifest.bindings
        .get_mut("digest_chunks")
        .expect("bound")
        .driver
        .as_mut()
        .expect("driver")
        .step
        .args[0]
        .from = ArgSource::ElemPath(vec![0; 100]);
    let err = validate_driver_consistency(&file, &b).expect_err("must be W0711");
    assert_eq!(err.code.to_string(), "W0711");
    let CheckerErrorKind::DriverInconsistency(m) = err.kind else {
        panic!("wrong kind: {:?}", err.kind);
    };
    assert!(m.contains("nests 100 deep"), "missing depth in: {m}");
}

// --- Consistency negatives (VER04k 3.11): driven parameter, rule 1 ---

#[test]
fn driven_parameter_of_type_u8_is_w0711() {
    let spec = "warrant t v1\n  op f(x: U8) -> Bytes\n  labels f:\n    x public\n";
    bad(
        spec,
        "op f driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step",
        "driver f state=subject::S new=new() drive=x step=tick(mut,elem:u8) finish=out(ref)->bytes result=finish",
        "neither `List<T>`, `Bytes` nor `Nat`",
    );
}

#[test]
fn unknown_drive_parameter_is_w0711() {
    bad(
        SPEC_SHA1,
        OP_SHA1,
        "driver digest_chunks state=subject::Digest new=new() drive=nope step=update(mut,elem:bytes) finish=sum1(ref)->bytes result=finish",
        "has no such parameter",
    );
}

#[test]
fn elem_on_nat_driver_is_w0711() {
    let spec = "warrant t v1\n  op fill(n: Nat) -> Bytes\n  labels fill:\n    n public\n  claim c:\n    forall n: Nat. fill(n) == fill(n)\n";
    bad(
        spec,
        "op fill driver faults=drop-last-chunk,duplicate-last-chunk,early-finish,reset-per-step",
        "driver fill state=subject::Fill new=new() drive=n step=tick(mut,elem:u8) finish=out(ref)->bytes result=finish",
        "with no element",
    );
}

// --- Element paths and host types, rule 2 ---

#[test]
fn elem_2_on_a_pair_is_w0711() {
    let spec = "warrant t v1\n  op encode_bits(bits: List<(Bool, U8)>) -> Bytes\n  labels encode_bits:\n    bits public\n  claim c:\n    forall bits: List<(Bool, U8)>. encode_bits(bits) == encode_bits(bits)\n";
    bad(
        spec,
        "op encode_bits driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step",
        "driver encode_bits state=subject::Enc new=new() drive=bits step=write_bit(mut,elem.2:u8) finish=finish(own)->bytes result=finish",
        "component 2 of a 2-tuple",
    );
}

#[test]
fn elem_path_into_scalar_is_w0711() {
    let spec = "warrant t v1\n  op scan(xs: List<U8>) -> Bytes\n  labels scan:\n    xs public\n  claim c:\n    forall xs: List<U8>. scan(xs) == scan(xs)\n";
    bad(
        spec,
        "op scan driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step,drop-last-output",
        "driver scan state=subject::S new=new() drive=xs step=push(mut,elem.0:u8) finish=out(ref)->bytes result=finish",
        "no components",
    );
}

#[test]
fn elem_path_into_variant_is_w0711() {
    let spec = "warrant t v1\n  variant V = A(x: U8) | B\n  op scan(vs: List<V>) -> Bytes\n  labels scan:\n    vs public\n";
    bad(
        spec,
        "op scan driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step,drop-last-output",
        "driver scan state=subject::S new=new() drive=vs step=push(mut,elem.0:u8) finish=out(ref)->bytes result=finish",
        "not a record",
    );
}

#[test]
fn bool_host_for_bytes_element_is_w0711() {
    bad(
        SPEC_SHA1,
        OP_SHA1,
        "driver digest_chunks state=subject::Digest new=new() drive=chunks step=update(mut,elem:bool) finish=sum1(ref)->bytes result=finish",
        "cannot carry",
    );
}

#[test]
fn param_source_naming_unknown_param_is_w0711() {
    bad(
        SPEC_SHA1,
        OP_SHA1,
        "driver digest_chunks state=subject::Digest new=new(param.nope:u8) drive=chunks step=update(mut,elem:bytes) finish=sum1(ref)->bytes result=finish",
        "not a parameter",
    );
}

#[test]
fn ctx_source_without_ctx_is_w0711() {
    bad(
        SPEC_SHA1,
        OP_SHA1,
        "driver digest_chunks state=subject::Digest new=new(ctx) drive=chunks step=update(mut,elem:bytes) finish=sum1(ref)->bytes result=finish",
        "no `ctx=`",
    );
}

// --- Parameter consumption, rule 3 ---

#[test]
fn unconsumed_second_parameter_is_w0711() {
    let spec = "warrant t v1\n  op keyed(chunks: List<Bytes>, key: U8) -> Bytes\n  labels keyed:\n    chunks public\n    key public\n  claim c:\n    forall chunks: List<Bytes>, key: U8. keyed(chunks, key) == keyed(chunks, key)\n";
    bad(
        spec,
        "op keyed driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step",
        "driver keyed state=subject::K new=new() drive=chunks step=update(mut,elem:bytes) finish=sum(ref)->bytes result=finish",
        "consumed by neither",
    );
}

// --- Result modes and fallibility, rule 4 ---

#[test]
fn collect_on_bytes20_result_is_w0711() {
    bad(
        SPEC_SHA1,
        OP_SHA1,
        "driver digest_chunks state=subject::Digest new=new() drive=chunks step=dec(mut,elem:bytes)->u8 result=collect",
        "returns `collect` but the operation returns",
    );
}

#[test]
fn collect_with_unit_step_is_w0711() {
    let spec = "warrant t v1\n  op dec_all(bs: List<Bytes>) -> List<U8>\n  labels dec_all:\n    bs public\n  claim c:\n    forall bs: List<Bytes>. dec_all(bs) == dec_all(bs)\n";
    bad(
        spec,
        "op dec_all driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,drop-last-output",
        "driver dec_all state=subject::Dec new=new() drive=bs step=dec(mut,elem:bytes) result=collect",
        "emits no value",
    );
}

#[test]
fn finish_mode_without_finish_is_w0711() {
    bad(
        SPEC_SHA1,
        OP_SHA1,
        "driver digest_chunks state=subject::Digest new=new() drive=chunks step=update(mut,elem:bytes) result=finish",
        "no `finish=`",
    );
}

#[test]
fn both_on_non_tuple_is_w0711() {
    bad(
        SPEC_SHA1,
        OP_SHA1,
        "driver digest_chunks state=subject::Digest new=new() drive=chunks step=dec(mut,elem:bytes)->u8 finish=sum1(ref)->bytes result=both",
        "returns `both` but the operation returns",
    );
}

#[test]
fn fallible_step_on_non_result_is_w0711() {
    bad(
        SPEC_SHA1,
        OP_SHA1,
        "driver digest_chunks state=subject::Digest new=new() drive=chunks step=try_update(mut,elem:bytes)->result finish=sum1(ref)->bytes result=finish",
        "fallible but the operation does not return `Result`",
    );
}

#[test]
fn fallible_driver_without_errors_map_is_w0711() {
    let spec = "warrant t v1\n  error E = TOO_LONG\n  op digest_try(chunks: List<Bytes>) -> Result<Bytes, E>\n  labels digest_try:\n    chunks public\n  claim c:\n    forall chunks: List<Bytes>. digest_try(chunks) == digest_try(chunks)\n";
    bad(
        spec,
        "op digest_try driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step",
        "driver digest_try state=subject::D new=new() drive=chunks step=try_update(mut,elem:bytes)->result finish=digest(ref)->bytes result=finish",
        "has no `errors=` map",
    );
}

#[test]
fn errors_map_naming_unknown_case_is_w0711() {
    let spec = "warrant t v1\n  error E = TOO_LONG\n  op digest_try(chunks: List<Bytes>) -> Result<Bytes, E>\n  labels digest_try:\n    chunks public\n  claim c:\n    forall chunks: List<Bytes>. digest_try(chunks) == digest_try(chunks)\n";
    bad(
        spec,
        "op digest_try driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step",
        "driver digest_try state=subject::D new=new() drive=chunks step=try_update(mut,elem:bytes)->result finish=digest(ref)->bytes result=finish errors=subject::E/LengthOverflow=NOPE",
        "not a case of",
    );
}

#[test]
fn errors_map_without_result_is_w0711() {
    bad(
        SPEC_SHA1,
        OP_SHA1,
        "driver digest_chunks state=subject::Digest new=new() drive=chunks step=update(mut,elem:bytes) finish=sum1(ref)->bytes result=finish errors=subject::E/V=TOO_LONG",
        "maps host errors but the operation does not return `Result`",
    );
}

// --- Static parameters and document version, rule 5 ---

#[test]
fn static_params_on_driven_op_are_w0711() {
    let spec = "warrant t v1\n  op f<N: Nat>(x: Bytes<N>) -> Bool\n  labels f:\n    x public\n";
    bad(
        spec,
        "op f driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step",
        "driver f state=subject::S new=new() drive=x step=tick(mut,elem:u8) finish=out(ref)->bool result=finish",
        "static parameters",
    );
}

#[test]
fn driver_on_v0_document_is_w0711() {
    let spec = "warrant t\n  op digest_chunks(chunks: List<Bytes>) -> Bytes\n  labels digest_chunks:\n    chunks public\n";
    bad(spec, OP_SHA1, DRIVER_SHA1, "needs a v1 document");
}

// --- Obligations opt-in, 3.4 rule 5; lengths, 3.4 rule 7 ---

#[test]
fn secret_without_opt_in_is_w0711() {
    let spec = "warrant t v1\n  op digest_chunks(chunks: List<Bytes>) -> Bytes<20>\n  labels digest_chunks:\n    chunks secret\n  claim c:\n    forall chunks: List<Bytes>. digest_chunks(chunks) == digest_chunks(chunks)\n";
    bad(spec, OP_SHA1, DRIVER_SHA1, "proofs_not_live=driver");
}

#[test]
fn bound_row_without_opt_in_is_w0711() {
    let spec = "warrant t v1\n  op digest_chunks(chunks: List<Bytes>) -> Bytes<20>\n  labels digest_chunks:\n    chunks public\n  bound digest_chunks:\n    steps <= 10\n";
    bad(spec, OP_SHA1, DRIVER_SHA1, "proofs_not_live=driver");
}

#[test]
fn wrong_opt_in_reason_is_w0711() {
    let spec = "warrant t v1\n  op digest_chunks(chunks: List<Bytes>) -> Bytes<20>\n  labels digest_chunks:\n    chunks secret\n  claim c:\n    forall chunks: List<Bytes>. digest_chunks(chunks) == digest_chunks(chunks)\n";
    bad(
        spec,
        "op digest_chunks driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step proofs_not_live=frontend-refuses-lib",
        DRIVER_SHA1,
        "proofs_not_live=driver",
    );
}

#[test]
fn lengths_on_non_bytes_elements_is_w0711() {
    let spec = "warrant t v1\n  op scan(xs: List<U8>) -> Bytes\n  labels scan:\n    xs public\n  claim c:\n    forall xs: List<U8>. scan(xs) == scan(xs)\n";
    bad(
        spec,
        "op scan driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step,drop-last-output",
        "driver scan state=subject::S new=new() drive=xs step=push(mut,elem:u8) finish=out(ref)->bytes result=finish lengths=1,2",
        "needs `Bytes` elements",
    );
}

// --- End to end: the code survives the evidence boundary ---

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
fn bad_driver_reports_w0711_through_compile() {
    let text = bindings(
        OP_SHA1,
        "driver digest_chunks state=subject::Digest new=new() drive=chunks step=update(mut,elem:bool) finish=sum1(ref)->bytes result=finish",
    );
    assert_eq!(compile_code(SPEC_SHA1, &text), CodegenCode::W0711);
}

#[test]
fn good_driver_still_refuses_generation_with_w0704() {
    let text = bindings(OP_SHA1, DRIVER_SHA1);
    assert_eq!(compile_code(SPEC_SHA1, &text), CodegenCode::W0704);
}
