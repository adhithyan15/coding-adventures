//! WX-F slices 4-5: driver adapter generation, asserted as text. The obligations
//! themselves execute in `tests/wxf_driver_evidence.rs` (and the sha1 pilot
//! executes the sabotage); here the adapter variants, pins, fault plans,
//! registrations and refusal paths are pinned down.

use coding_adventures_warrant_grammar::binding::parse_evidence_bindings;
use coding_adventures_warrant_grammar::checker::check_warrant;
use coding_adventures_warrant_grammar::codegen::{
    find_all_operations_in_expr, find_operation_in_expr, DiagnosticCode as CodegenCode,
};
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

const SPEC_TWO: &str = "warrant t v1\n  op a(chunks: List<Bytes>) -> Bytes<20>\n  labels a:\n    chunks public\n  op b(chunks: List<Bytes>) -> Bytes<20>\n  labels b:\n    chunks public\n  claim ca:\n    forall chunks: List<Bytes>. a(chunks) == a(chunks)\n  claim cb:\n    forall chunks: List<Bytes>. b(chunks) == b(chunks)\n";

#[test]
fn two_drivers_namespace_their_shape_pins() {
    // Regression: `_shape_step`/`_shape_finish` collided across two driver
    // ops in one harness (the e2e fixture caught it compiling op 3).
    let text = format!(
        "{HEAD}\
         op a driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step\n\
         driver a state=subject::A new=new() drive=chunks step=feed(mut,elem:bytes) finish=out(ref)->bytes result=finish\n\
         op b driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step\n\
         driver b state=subject::B new=new() drive=chunks step=feed(mut,elem:bytes) finish=out(ref)->bytes result=finish\n"
    );
    let out = generate(SPEC_TWO, &text).expect("generation must succeed");
    assert!(out.contains("fn _shape_finish_a()"), "finish pin for a missing");
    assert!(out.contains("fn _shape_finish_b()"), "finish pin for b missing");
    assert!(!out.contains("fn _shape_finish()"), "unprefixed pin must be gone");
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

// Slice 5a: the fault plan (3.5/3.6) and its coverage control.

const SPEC_COLLECT: &str = "warrant t v1\n  op scan(data: Bytes) -> List<U8>\n  labels scan:\n    data public\n  claim c:\n    forall data: Bytes. scan(data) == scan(data)\n";
const OP_COLLECT: &str = "op scan driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,drop-last-output";
const DRIVER_COLLECT: &str = "driver scan state=subject::Dec new=new() drive=data step=read(mut,elem:u8)->u8 result=collect";

#[test]
fn plan_arms_cover_listed_faults_only() {
    let out = generates(SPEC_SHA1, OP_SHA1, DRIVER_SHA1);
    for arm in [
        "Fault::DropLastChunk",
        "Fault::ReorderChunks",
        "Fault::DuplicateLastChunk",
        "Fault::EarlyFinish",
        "Fault::ResetPerStep",
    ] {
        assert!(out.contains(arm), "plan arm missing for {arm}");
    }
    // `drop-last-output` is not listed for this finish-mode driver: no arm,
    // and the wildcard keeps the identity.
    assert!(!out.contains("Fault::DropLastOutput"), "unlisted arm emitted");
    assert!(out.contains("_ => ((0..peril).collect(), false, false)"), "wildcard missing");
}

#[test]
fn plan_rewrites_match_the_spec_table() {
    let out = generates(SPEC_SHA1, OP_SHA1, DRIVER_SHA1);
    assert!(out.contains("(0..peril.saturating_sub(1)).collect()"), "drop-last order");
    assert!(out.contains("(0..peril).rev().collect()"), "reorder order");
    assert!(out.contains("o.push(peril - 1)"), "duplicate order");
    assert!(out.contains("vec![0usize]"), "early order");
    assert!(out.contains("((0..peril).collect(), true, false)"), "reset flag");
    assert!(out.contains("for step in order"), "loop drives the plan");
    assert!(out.contains("driver_note_driven(\"digest_chunks\", peril)"), "coverage note");
}

#[test]
fn reset_arm_emitted_only_when_listed() {
    let out = generates(SPEC_SHA1, OP_SHA1, DRIVER_SHA1);
    assert!(out.contains("if reset_each"), "reset arm missing");
    // A collect-mode driver cannot list `reset-per-step` (3.6: finish/both
    // only), so its adapter has no reset arm.
    let collect = format!("{HEAD}{OP_COLLECT}\n{DRIVER_COLLECT}\n");
    let out = generate(SPEC_COLLECT, &collect).expect("collect must generate");
    // The plan tuple always names the flag; the re-creation arm must be gone.
    assert!(!out.contains("if reset_each"), "reset arm must be absent");
}

#[test]
fn drop_output_pop_only_when_collecting() {
    let out = generates(SPEC_SHA1, OP_SHA1, DRIVER_SHA1);
    assert!(!out.contains("collected.pop()"), "finish mode must not pop");
    let collect = format!("{HEAD}{OP_COLLECT}\n{DRIVER_COLLECT}\n");
    let out = generate(SPEC_COLLECT, &collect).expect("collect must generate");
    assert!(out.contains("if drop_output { collected.pop(); }"), "collect must pop");
}

#[test]
fn needy_negctl_checks_driven_coverage() {
    let out = generates(SPEC_SHA1, OP_SHA1, DRIVER_SHA1);
    assert!(
        out.contains("negctl_result_driver(caught, \"reorder-chunks\", driver_take_max_driven(\"digest_chunks\"))"),
        "needy epilogue missing"
    );
    assert!(
        out.contains("driver_take_max_driven(\"digest_chunks\");"),
        "coverage reset missing"
    );
    // Non-needy faults keep the plain control.
    assert!(
        out.contains("negctl_result(caught, \"drop-last-chunk\")"),
        "plain epilogue missing"
    );
    assert!(!out.contains("negctl_result_driver(caught, \"drop-last-chunk\""), "drop must stay plain");
}

const SPEC_REL: &str = "warrant t v1\n  op a(chunks: List<Bytes>) -> Bytes<20>\n  labels a:\n    chunks public\n  op b(chunks: List<Bytes>) -> Bytes<20>\n  labels b:\n    chunks public\n  claim r:\n    forall m: Bytes. a([m, m]) == b([m, m])\n";

#[test]
fn relational_claim_controls_every_operation_it_calls() {
    let text = format!(
        "{HEAD}\
         op a driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step\n\
         driver a state=subject::A new=new() drive=chunks step=feed(mut,elem:bytes) finish=out(ref)->bytes result=finish\n\
         op b driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step\n\
         driver b state=subject::B new=new() drive=chunks step=feed(mut,elem:bytes) finish=out(ref)->bytes result=finish\n"
    );
    let out = generate(SPEC_REL, &text).expect("relational must generate");
    // Both operations' negctls run the shared claim (fail-open vacuity: the
    // binder-dependent drive passes W0601 statically; the runtime coverage
    // control closes it).
    for op in ["a", "b"] {
        assert!(
            out.contains(&format!("negctl/exhaustive/{op}/drop-last-chunk")),
            "negctl missing for {op}"
        );
    }
    assert!(out.matches("(\"r\", neg_exh_r)").count() >= 2, "r must join both ops");
}

#[test]
fn find_all_orders_first_like_find_first() {
    let doc = SourceDocument::admit(SPEC_REL.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");
    let ops: std::collections::HashSet<&str> = ["a", "b"].into_iter().collect();
    let body = file
        .declarations
        .iter()
        .find_map(|d| match d {
            coding_adventures_warrant_grammar::parser::Declaration::Claim(c) => match &c.body {
                coding_adventures_warrant_grammar::parser::Expression::Quantified {
                    body, ..
                } => Some(body.as_ref()),
                _ => None,
            },
            _ => None,
        })
        .expect("claim");
    let all = find_all_operations_in_expr(body, &ops);
    let first = find_operation_in_expr(body, &ops);
    assert_eq!(all, vec!["a", "b"]);
    assert_eq!(first, Some("a"));
    assert_eq!(all.first().copied(), first, "first must agree");
}

const SPEC_SPLIT: &str = "warrant t v1\n  op digest_chunks(chunks: List<Bytes>) -> Bytes<20>\n  labels digest_chunks:\n    chunks public\n  claim c:\n    forall m: Bytes. digest_chunks([m]) == digest_chunks([m])\n  claim s:\n    forall m: Bytes, k: Nat. digest_chunks([take(m, k), drop(m, k)]) == digest_chunks([m])\n";

#[test]
fn split_builtins_emit_and_gate_their_region() {
    let out = generates(SPEC_SPLIT, OP_SHA1, DRIVER_SHA1);
    assert!(out.contains("V::seq_take("), "take missing");
    assert!(out.contains("V::seq_drop("), "drop missing");
    assert!(out.contains("pub fn seq_take(xs: &V, n: &V) -> V"), "region missing");
    // Harnesses that never split stay byte-identical: no region without use.
    let plain = generates(SPEC_SHA1, OP_SHA1, DRIVER_SHA1);
    assert!(!plain.contains("seq_take"), "region leaked");
    assert!(!plain.contains("seq_drop"), "region leaked");
}

#[test]
fn bytes_literal_flows_into_bytes_position() {
    // `[b"00"]` against `List<Bytes>`: the literal checks as unbounded
    // `Bytes` (inward flow, like `Int`), so pilot examples typecheck.
    let spec = "warrant t v1\n  op digest_chunks(chunks: List<Bytes>) -> Bytes<20>\n  labels digest_chunks:\n    chunks public\n  example e:\n    digest_chunks([b\"00\"]) == digest_chunks([b\"00\"])\n  claim c:\n    forall chunks: List<Bytes>. digest_chunks(chunks) == digest_chunks(chunks)\n";
    let out = generates(spec, OP_SHA1, DRIVER_SHA1);
    assert!(out.contains("V::List(vec!["), "list must emit");
}

// Slice 5b: `lengths=` hints (VER04k 3.7).

const SPEC_LEN: &str = "warrant t v1\n  op a(chunks: List<Bytes>) -> Bytes<20>\n  labels a:\n    chunks public\n  op b(chunks: List<Bytes>) -> Bytes<20>\n  labels b:\n    chunks public\n  claim r:\n    forall m: Bytes, k: Nat. a([m, m]) == b([take(m, k), drop(m, k)])\n";

fn bindings_len(a_lengths: &str, b_lengths: Option<&str>) -> String {
    let mut text = format!(
        "{HEAD}\
         op a driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step\n\
         driver a state=subject::A new=new() drive=chunks step=feed(mut,elem:bytes) finish=out(ref)->bytes result=finish lengths={a_lengths}\n"
    );
    match b_lengths {
        Some(ls) => text.push_str(&format!(
            "op b driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step\n\
             driver b state=subject::B new=new() drive=chunks step=feed(mut,elem:bytes) finish=out(ref)->bytes result=finish lengths={ls}\n"
        )),
        None => text.push_str(
            "op b driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step\n\
             driver b state=subject::B new=new() drive=chunks step=feed(mut,elem:bytes) finish=out(ref)->bytes result=finish\n",
        ),
    }
    text
}

#[test]
fn driven_list_binders_are_marked_for_rechunking() {
    // `List<Bytes>` binders of a claim calling a driven op are marked;
    // `List<U8>` binders are not (re-chunking is byte content); unmarked
    // claims keep plain `Ty::List` and no region ships.
    let head = "warrant t v1\n  op a(chunks: List<Bytes>) -> Bytes<20>\n  labels a:\n    chunks public\n";
    let bindings = format!(
        "{HEAD}\
         op a driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step\n\
         driver a state=subject::A new=new() drive=chunks step=feed(mut,elem:bytes) finish=out(ref)->bytes result=finish\n"
    );
    let pair = format!("{head}  claim r:\n    forall xs: List<Bytes>, ys: List<Bytes>. a(xs) == a(xs)\n");
    let out = generate(&pair, &bindings).expect("must generate");
    assert!(
        out.contains("CLAIM_R_TYS: &[Ty] = &[Ty::ListDriven, Ty::ListDriven];"),
        "pair not marked"
    );
    assert!(out.contains("Ty::ListDriven =>"), "rechunk region missing");
    let mixed = format!("{head}  claim r:\n    forall xs: List<Bytes>, ns: List<U8>. a(xs) == a(xs)\n");
    let out = generate(&mixed, &bindings).expect("must generate");
    assert!(
        out.contains("CLAIM_R_TYS: &[Ty] = &[Ty::ListDriven, Ty::List(&Ty::Fx(F::U8))];"),
        "non-bytes list marked"
    );
    // SPEC_SPLIT's binders are `Bytes`/`Nat` only: nothing to mark.
    let plain = generates(SPEC_SPLIT, OP_SHA1, DRIVER_SHA1);
    assert!(!plain.contains("ListDriven"), "mark leaked without list binders");
}

#[test]
fn hinted_binders_carry_the_union_sorted() {
    // Both drivers hint; the `Bytes` binder draws from the union (sorted
    // across drivers) while the `Nat` binder is untouched.
    let out = generate(SPEC_LEN, &bindings_len("10,20", Some("9,11")))
        .expect("lengths must generate");
    assert!(
        out.contains("CLAIM_R_TYS: &[Ty] = &[Ty::BytesHinted(&[9, 10, 11, 20]), Ty::Nat];"),
        "union wrong"
    );
    assert!(out.contains("fn driver_note_driven"), "lengths region missing");
}

#[test]
fn unhinted_claims_stay_plain() {
    // `b` has no hints and `r` calls both: the union is `a`'s alone.
    // (A claim calling no driven op keeps `Ty::Bytes`: SPEC_SHA1 above.)
    let out = generate(SPEC_LEN, &bindings_len("9", None)).expect("must generate");
    assert!(out.contains("Ty::BytesHinted(&[9])"), "single union wrong");
    let plain = generates(SPEC_SHA1, OP_SHA1, DRIVER_SHA1);
    assert!(!plain.contains("BytesHinted"), "region leaked without hints");
}

#[test]
fn hinted_domains_count_against_the_budget() {
    // Two `Bytes` binders with 32 hints: (347 + 64)^2 = 168,921 cases,
    // under the 400,000 budget. Three binders: 411^3 overflows it (W0706).
    let two = "warrant t v1\n  op a(chunks: List<Bytes>) -> Bytes<20>\n  labels a:\n    chunks public\n  claim r:\n    forall x: Bytes, y: Bytes. a([x, y]) == a([x, y])\n";
    let three = "warrant t v1\n  op a(chunks: List<Bytes>) -> Bytes<20>\n  labels a:\n    chunks public\n  claim r:\n    forall x: Bytes, y: Bytes, z: Bytes. a([x, y]) == a([x, y])\n";
    let hints32 = (1..=32).map(|n| n.to_string()).collect::<Vec<_>>().join(",");
    let bindings = format!(
        "{HEAD}\
         op a driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step\n\
         driver a state=subject::A new=new() drive=chunks step=feed(mut,elem:bytes) finish=out(ref)->bytes result=finish lengths={hints32}\n"
    );
    let out = generate(two, &bindings).expect("two binders fit");
    assert!(out.contains("Ty::BytesHinted(&["), "hints missing");
    let err = generate(three, &bindings).expect_err("three binders must overflow");
    assert_eq!(err, CodegenCode::W0706);
}
