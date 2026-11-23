//! WX-F slice 4: the driver-obligation evidence harness, checked in and
//! drift-tested. `tests/wxf_driver_evidence.rs` compiles and runs what this
//! generates against a mode-switched subject.

use std::collections::BTreeMap;

use coding_adventures_warrant_grammar::binding::parse_evidence_bindings;
use coding_adventures_warrant_grammar::checker::check_warrant;
use coding_adventures_warrant_grammar::evidence::{compile_rust_evidence_harness, EvidenceOptions};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

const SPEC: &str = include_str!("fixtures/wxf_driver.warrant");
const GENERATED: &str = "tests/generated/wxf_driver_evidence.rs";

const BINDINGS: &str = "package wxf_subject\nrevision 0\nspec code/packages/rust/warrant-grammar/tests/fixtures/wxf_driver.warrant\noutput code/packages/rust/warrant-grammar/tests/generated/wxf_driver_evidence.rs\nseed 1\nproperty_cases 32\nfuzz_iterations 32\nnegctl_fuzz_iterations 32\nexhaustive_budget 400000\nop digest_chunks driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step\ndriver digest_chunks state=Digest new=new() drive=chunks step=update(mut,elem:bytes) finish=sum1(ref)->bytes result=finish\nop scan2 driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step,drop-last-output\ndriver scan2 state=Scan new=new(param.key:u8,elem:bytes) drive=chunks step=push(mut,elem:bytes)->u8 finish=total(ref)->u32 result=both\nop try_hash driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step\ndriver try_hash state=Digest new=new() drive=chunks step=try_update(mut,elem:bytes)->result finish=sum1(ref)->bytes result=finish errors=E/LengthOverflow=TOO_LONG\n";

pub fn options() -> EvidenceOptions {
    EvidenceOptions {
        package_crate: "wxf_subject".into(),
        spec_path: "code/packages/rust/warrant-grammar/tests/fixtures/wxf_driver.warrant".into(),
        spec_sha256: "0".repeat(64),
        seed: 0x5758_465f_4452_4956,
        property_cases: 32,
        fuzz_iterations: 32,
        negctl_fuzz_iterations: 32,
        exhaustive_budget: 400000,
        arg_types: BTreeMap::new(),
        hosts: Default::default(),
    }
}

pub fn generate() -> String {
    let doc = SourceDocument::admit(SPEC.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");
    let checked = check_warrant(&file).expect("the fixture must type-check");
    let b = parse_evidence_bindings(BINDINGS).expect("bindings parse");
    compile_rust_evidence_harness(&file, &checked, &b, &options()).expect("generation must succeed")
}

#[test]
fn checked_in_driver_harness_matches_the_fixture_and_generator() {
    let text = generate();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(GENERATED);
    if std::env::var("WARRANT_REGEN").is_ok() {
        std::fs::write(&path, text.as_bytes()).expect("write generated harness");
        return;
    }
    let on_disk = std::fs::read_to_string(&path).expect("generated harness is checked in");
    assert_eq!(
        on_disk, text,
        "tests/generated/wxf_driver_evidence.rs is stale; rerun with WARRANT_REGEN=1"
    );
}
