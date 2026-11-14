//! VER04k sections 4.6 to 4.8 and 4.13: what the evidence generator emits for
//! `inout` parameters and `ensures` blocks, and what it refuses.  These tests
//! read the *text* of the generated harness and the diagnostics of generation;
//! `tests/wxg_evidence.rs` compiles and runs it.

use coding_adventures_sha256::sha256_hex;
use coding_adventures_warrant_grammar::binding::parse_evidence_bindings;
use coding_adventures_warrant_grammar::checker::check_warrant;
use coding_adventures_warrant_grammar::codegen::CodegenError;
use coding_adventures_warrant_grammar::evidence::{compile_rust_evidence_harness, EvidenceOptions};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

const SPEC: &str = include_str!("fixtures/wxg.warrant");
const BINDINGS: &str = include_str!("fixtures/wxg.bindings");

/// Generate the harness of `spec` with `bindings`; `Err` carries the diagnostic.
fn generate_from(spec: &str, bindings: &str) -> Result<String, String> {
    let b = parse_evidence_bindings(bindings).map_err(|e| format!("bindings: {e}"))?;
    let doc = SourceDocument::admit(spec.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).map_err(|e| format!("{e}"))?;
    let checked = check_warrant(&file).map_err(|e| format!("{e}"))?;
    let options = EvidenceOptions {
        package_crate: b.manifest.package.clone(),
        spec_path: b.spec.clone(),
        spec_sha256: sha256_hex(spec.as_bytes()),
        seed: b.seed,
        property_cases: b.property_cases,
        fuzz_iterations: b.fuzz_iterations,
        negctl_fuzz_iterations: b.negctl_fuzz_iterations,
        exhaustive_budget: b.exhaustive_budget,
        arg_types: b.arg_types.clone(),
        hosts: b.hosts.clone(),
    };
    compile_rust_evidence_harness(&file, &checked, &b.manifest, &options)
        .map_err(|e: CodegenError| format!("{e}"))
}

fn generated() -> String {
    generate_from(SPEC, BINDINGS).unwrap_or_else(|e| panic!("the fixture must generate: {e}"))
}

/// The fixture's bindings with one line replaced.
fn with_binding(from: &str, to: &str) -> String {
    assert!(BINDINGS.contains(from), "no such binding text: {from}");
    BINDINGS.replace(from, to)
}

fn refused(spec: &str, bindings: &str, code: &str, needle: &str) {
    let e = generate_from(spec, bindings).expect_err("must be refused");
    assert!(e.contains(code), "{code} expected in: {e}");
    assert!(e.contains(needle), "{needle:?} expected in: {e}");
}

#[test]
fn the_fixture_generates_a_harness_with_one_adapter_per_operation() {
    let text = generated();
    for op in ["wipe", "wipe4", "bump", "toggle", "set_on", "push_byte", "clear_nonempty", "clr", "encode"] {
        assert!(text.contains(&format!("fn op_{op}(")), "no adapter for {op}");
    }
}

#[test]
fn each_host_form_passes_the_expected_mutable_reference() {
    let text = generated();
    // slice: the slice of the copy, so a trait method resolves to `[u8]`.
    assert!(text.contains("wxg_subject::wipe(&mut h0[..])"), "slice");
    // array, int, bool, struct and vec pass the host value itself.
    assert!(text.contains("wxg_subject::wipe4(&mut h0)"), "array");
    assert!(text.contains("wxg_subject::bump(&mut h0)"), "int");
    assert!(text.contains("wxg_subject::toggle(&mut h0)"), "bool");
    assert!(text.contains("wxg_subject::set_on(&mut h0)"), "struct");
    assert!(text.contains("wxg_subject::push_byte(p0, &mut h1)"), "vec, after the plain argument");
}

#[test]
fn the_pre_state_is_copied_and_the_post_state_is_read_back_from_the_same_buffer() {
    let text = generated();
    // Pre-state isolation: the oracle's value is `args[i]`; the subject gets a copy.
    assert!(text.contains("let mut h0: Vec<u8> = args[0].bytes().to_vec();"));
    // Read-back: the value returned is built from `h0` after the call.
    assert!(text.contains("V::Bytes((h0).to_vec())"));
}

#[test]
fn every_fault_of_the_bindings_is_in_the_generated_fault_enum() {
    let text = generated();
    for (name, variant) in [
        ("stale-post", "StalePost"),
        ("skip-last-write", "SkipLastWrite"),
        ("drop-last-push", "DropLastPush"),
        ("clobber-prefix", "ClobberPrefix"),
        ("cursor-off-by-one", "CursorOffByOne"),
        ("missing-error", "MissingError"),
    ] {
        assert!(text.contains(&format!("Fault::{variant} => \"{name}\"")), "{name}");
    }
}

#[test]
fn ensures_blocks_add_a_post_suite_per_operation_and_monitor_every_call() {
    let text = generated();
    for op in ["wipe", "wipe4", "bump", "toggle", "set_on", "push_byte", "clear_nonempty", "clr", "encode"] {
        for kind in ["exhaustive", "property", "fuzz"] {
            assert!(
                text.contains(&format!("warrant/wxg/{kind}/{op}/post")),
                "no {kind} post suite for {op}"
            );
        }
        // The wrapper that every call goes through.
        assert!(text.contains(&format!("fn post_{op}(")), "no monitor for {op}");
    }
    assert!(text.contains("postcondition-violated"));
    assert!(text.contains("postcondition-unevaluable"));
}

#[test]
fn domains_are_labelled_honestly() {
    let text = generated();
    // `Bytes<4>` is a sample, `Bytes<1>` is full, a `Bytes` is a set (WX-B2 labels).
    assert!(text.contains("sample"));
    assert!(text.contains("full"));
}

#[test]
fn a_document_without_inout_or_ensures_is_unchanged() {
    // The wxh fixture predates WX-G: no wrapper, no monitor, no new fault.
    let text = include_str!("generated/wxh_evidence.rs");
    assert!(!text.contains("PostViolation"));
    assert!(!text.contains("StalePost"));
}

#[test]
fn a_host_form_this_slice_does_not_implement_is_w0704() {
    refused(
        SPEC,
        &with_binding("inout=buf:slice faults=stale-post,skip-last-write\nop wipe4", "inout=buf:returned faults=stale-post\nop wipe4"),
        "W0704",
        "not yet implemented",
    );
    for form in ["slice-of:struct:Cell", "vec-of:struct:Cell", "codec:Cell"] {
        refused(
            SPEC,
            &with_binding("inout=buf:slice faults=stale-post,skip-last-write\nop wipe4", &format!("inout=buf:{form} faults=stale-post\nop wipe4")),
            "W0704",
            "not yet implemented",
        );
    }
}

#[test]
fn a_host_form_that_cannot_carry_the_type_is_w0704() {
    // `bool` for a `Bytes` parameter; `array` for an unsized `Bytes`; `slice` for a `U8`.
    refused(SPEC, &with_binding("inout=buf:slice faults=stale-post,skip-last-write\nop wipe4", "inout=buf:bool faults=stale-post\nop wipe4"), "W0704", "wipe");
    refused(SPEC, &with_binding("inout=buf:slice faults=stale-post,skip-last-write\nop wipe4", "inout=buf:array faults=stale-post\nop wipe4"), "W0704", "wipe");
    refused(SPEC, &with_binding("inout=x:int:u8", "inout=x:slice"), "W0704", "bump");
}

#[test]
fn an_inout_parameter_with_no_host_form_or_a_stray_entry_is_refused() {
    refused(SPEC, &with_binding("op toggle export=toggle inout=flag:bool", "op toggle export=toggle"), "W0501", "flag");
    refused(SPEC, &with_binding("inout=flag:bool", "inout=nope:bool"), "W0501", "nope");
    refused(SPEC, &with_binding("op bump export=bump inout=x:int:u8", "op bump export=bump inout=x:int:u8,x:int:u8"), "W0501", "x");
}

#[test]
fn malformed_inout_options_are_binding_errors() {
    for bad in ["inout=", "inout=buf", "inout=buf:", "inout=:slice", "inout=buf:no-such-form"] {
        let text = with_binding("inout=buf:slice faults=stale-post,skip-last-write\nop wipe4", &format!("{bad} faults=stale-post\nop wipe4"));
        let e = generate_from(SPEC, &text).expect_err(bad);
        assert!(e.contains("W0501") || e.contains("bindings:"), "{bad}: {e}");
    }
}

#[test]
fn a_fault_that_does_not_apply_to_the_operation_is_w0701() {
    // 4.8 "Applies when": drop-last-push is for `vec`, cursor-off-by-one for `int:`,
    // skip-last-write for Bytes and List, clobber-prefix for `vec`.
    refused(SPEC, &with_binding("inout=buf:slice faults=stale-post,skip-last-write\nop wipe4", "inout=buf:slice faults=stale-post,drop-last-push\nop wipe4"), "W0701", "drop-last-push");
    refused(SPEC, &with_binding("inout=x:int:u8 faults=stale-post,cursor-off-by-one", "inout=x:int:u8 faults=stale-post,skip-last-write"), "W0701", "skip-last-write");
    refused(SPEC, &with_binding("inout=flag:bool faults=stale-post", "inout=flag:bool faults=stale-post,cursor-off-by-one"), "W0701", "cursor-off-by-one");
    refused(SPEC, &with_binding("inout=buf:slice faults=stale-post,missing-error", "inout=buf:slice faults=stale-post,clobber-prefix"), "W0701", "clobber-prefix");
}

#[test]
fn a_stale_post_fault_on_a_pure_operation_is_w0701() {
    // `stale-post` needs an inout parameter: a pure operation has no post-state.
    let spec = "warrant p v1\n  op f(x: U8) -> U8\n  labels f:\n    x public\n  example e:\n    f(1u8) == 1u8\n  claim c:\n    forall x: U8. f(x) == x\n";
    let bindings = "package p\nrevision 0\nspec s.warrant\noutput code/packages/rust/p/tests/generated/e.rs\nseed 1\nproperty_cases 10\nfuzz_iterations 10\nnegctl_fuzz_iterations 10\nop f export=f faults=stale-post\n";
    refused(spec, bindings, "W0701", "stale-post");
}

#[test]
fn an_example_that_contradicts_its_own_postcondition_is_w0601() {
    let spec = SPEC.replace(
        "wipe(b\"010203\") == b\"000000\"",
        "wipe(b\"010203\") == b\"010203\"",
    );
    refused(&spec, BINDINGS, "W0601", "wipe");
}

#[test]
fn an_ensures_row_that_uses_a_wxd1_form_is_refused_at_generation_not_skipped() {
    // VER04k section 4.13 item 9: the forms type-check but are not emitted yet.
    let spec = SPEC.replace(
        "    len(result) == len(buf)\n    result == zeros(len(buf))\n",
        "    result == to_bytes(replicate(0u8, len(buf)))\n",
    );
    refused(&spec, BINDINGS, "W0701", "replicate");
}

#[test]
fn an_export_path_is_accepted_only_for_an_inout_operation() {
    let ok = with_binding("op wipe export=wipe inout=", "op wipe export=wxg_subject::wipe inout=");
    parse_evidence_bindings(&ok).expect("a path is fine with inout=");
    let bad = with_binding("op wipe4 export=wipe4 inout=buf:array", "op wipe4 export=wxg_subject::wipe4");
    // No inout= on the line: a path is not an identifier.
    let e = parse_evidence_bindings(&bad).expect_err("a path needs inout=");
    assert!(e.contains("export"), "{e}");
    // More than eight segments is refused.
    let long = with_binding("op wipe export=wipe inout=", "op wipe export=a::b::c::d::e::f::g::h::i inout=");
    assert!(parse_evidence_bindings(&long).is_err());
}

/// A one-operation document whose host returns a `Result<(), ()>` or a count, to see
/// what the adapter does with the host's return value (VER04k 4.13 item 12).
fn clr_spec() -> String {
    "warrant c v1\n  error Bad = BAD\n  op clr(buf: inout Bytes) -> Result<Bytes, Bad>\n  labels clr:\n    buf public\n  example e:\n    clr(b\"01\") == Ok(b\"00\")\n  claim k:\n    forall buf: Bytes. clr(buf) == clr(buf)\n".to_string()
}

fn clr_bindings(opts: &str) -> String {
    format!("package wxg_subject\nrevision 0\nspec s.warrant\noutput code/packages/rust/c/tests/generated/e.rs\nseed 1\nproperty_cases 10\nfuzz_iterations 10\nnegctl_fuzz_iterations 10\nop clr export=clr {opts} inout=buf:slice faults=stale-post\n")
}

#[test]
fn a_result_slot_without_a_returned_component_types_the_host_return_as_unit() {
    // Ok(()) / Some(()), never Ok(_) / Some(_): a host that returns data, or an
    // Ok that carries a count, must fail to compile instead of being dropped.
    for opts in ["err_any=BAD", "error=BAD", "errors=wxg_subject::E/B=BAD"] {
        let text = generate_from(&clr_spec(), &clr_bindings(opts)).unwrap_or_else(|e| panic!("{opts}: {e}"));
        assert!(
            text.contains("Ok(()) =>") || text.contains("Some(()) =>"),
            "{opts}: the host return is not typed as ()"
        );
        assert!(!discards(&text), "{opts}: a host value is discarded");
    }
}

#[test]
fn the_fixture_discards_no_host_value_anywhere() {
    let text = generated();
    assert!(!discards(&text), "an arm `Ok(_)` / `Some(_)` discards the host's payload");
}

/// Does any generated match arm START with `Ok(_) =>` or `Some(_) =>` (the host's
/// own `Result`/`Option`; a model value `V::Ok(_)` in the prelude is not one)?
fn discards(text: &str) -> bool {
    text.lines().any(|l| {
        let t = l.trim_start();
        t.starts_with("Ok(_) =>") || t.starts_with("Some(_) =>")
    })
}
