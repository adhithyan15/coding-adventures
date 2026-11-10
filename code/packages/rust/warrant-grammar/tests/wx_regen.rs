//! Regenerates (or checks) `tests/generated/wx_evidence.rs`, the v1 evidence
//! harness the WX-B/WX-C tests compile and run (`tests/wx_evidence.rs`).
//!
//! `WARRANT_REGEN=1 cargo test -p coding_adventures_warrant_grammar --test
//! wx_regen` rewrites the file; without the variable the test fails when the
//! checked-in file differs from the generator's output, exactly like the
//! BUILD drift check of a real package.

use std::collections::BTreeMap;

use coding_adventures_sha256::sha256_hex;
use coding_adventures_warrant_grammar::binding::{BindingManifest, ErrorNormalization, OpBinding};
use coding_adventures_warrant_grammar::checker::check_warrant;
use coding_adventures_warrant_grammar::evidence::{compile_rust_evidence_harness, EvidenceOptions};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

pub const SPEC: &str = include_str!("fixtures/wx_fixed.warrant");
const GENERATED: &str = "tests/generated/wx_evidence.rs";

fn op(export: &str, error: Option<&str>, faults: &[&str]) -> OpBinding {
    OpBinding {
        export: export.into(),
        static_params: Vec::new(),
        fresh: false,
        error_normalization: error.map(|c| ErrorNormalization {
            kind: "panic_to_error".into(),
            error_case: c.into(),
        }),
        negative_controls: faults.iter().map(|s| s.to_string()).collect(),
    }
}

/// The binding of every operation of the fixture: which in-test export
/// implements it and which faults attack it.
pub fn bindings() -> BindingManifest {
    let mut b = BTreeMap::new();
    b.insert(
        "add32".to_string(),
        op(
            "add32",
            None,
            &["saturate-add", "trap-as-zero-add", "narrow-width"],
        ),
    );
    b.insert(
        "sub8".to_string(),
        op("sub8", None, &["saturate-sub", "trap-as-zero-sub"]),
    );
    b.insert(
        "mul16".to_string(),
        op(
            "mul16",
            None,
            &["saturate-mul", "trap-as-zero-mul", "narrow-width"],
        ),
    );
    b.insert("lt_i32".to_string(), op("lt_i32", None, &["sign-confuse"]));
    b.insert(
        "try_u8".to_string(),
        op("try_u8", None, &["off-by-one-range"]),
    );
    b.insert(
        "shl16".to_string(),
        op("shl16", None, &["shift-mask", "shift-ub"]),
    );
    b.insert(
        "sar8".to_string(),
        op("sar8", None, &["sar-as-shr", "shift-mask"]),
    );
    b.insert(
        "rotl32".to_string(),
        op("rotl32", None, &["rotate-as-shift-left"]),
    );
    b.insert(
        "rotr16".to_string(),
        op("rotr16", None, &["rotate-as-shift-right"]),
    );
    b.insert(
        "popcnt8".to_string(),
        op("popcnt8", None, &["popcount-off"]),
    );
    b.insert(
        "narrow8".to_string(),
        op("narrow8", Some("OUT_OF_RANGE"), &["missing-error"]),
    );
    BindingManifest {
        package: "wx_subject".into(),
        revision: "0.0.0".into(),
        bindings: b,
    }
}

pub fn options(spec: &str, budget: u64) -> EvidenceOptions {
    EvidenceOptions {
        package_crate: "wx_subject".into(),
        spec_path: "code/packages/rust/warrant-grammar/tests/fixtures/wx_fixed.warrant".into(),
        spec_sha256: sha256_hex(spec.as_bytes()),
        seed: 0x5758_5f46_4958_4544,
        property_cases: 4096,
        fuzz_iterations: 1500,
        negctl_fuzz_iterations: 1500,
        exhaustive_budget: budget,
        arg_types: BTreeMap::new(),
        hosts: Default::default(),
    }
}

pub fn generate(budget: u64) -> String {
    let doc = SourceDocument::admit(SPEC.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");
    let checked = check_warrant(&file).expect("the fixture must type-check");
    compile_rust_evidence_harness(&file, &checked, &bindings(), &options(SPEC, budget))
        .expect("generation must succeed")
}

#[test]
fn checked_in_v1_harness_matches_the_fixture_and_generator() {
    let text = generate(400_000);
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(GENERATED);
    if std::env::var("WARRANT_REGEN").is_ok() {
        std::fs::write(&path, text.as_bytes()).expect("write generated harness");
        return;
    }
    let on_disk = std::fs::read_to_string(&path).expect("generated harness is checked in");
    assert_eq!(
        on_disk, text,
        "tests/generated/wx_evidence.rs is stale; rerun with WARRANT_REGEN=1"
    );
}
