//! Regenerates (or checks) `tests/generated/wxg_evidence.rs`, the WX-G evidence
//! harness `tests/wxg_evidence.rs` and `tests/wxg_vulnerable.rs` compile and run.
//!
//! `WARRANT_REGEN=1 cargo test -p coding_adventures_warrant_grammar --test
//! wxg_regen` rewrites the file; without the variable the test fails when the
//! checked-in file differs from the generator's output, exactly like the BUILD
//! drift check of a real package.

use coding_adventures_sha256::sha256_hex;
use coding_adventures_warrant_grammar::binding::{parse_evidence_bindings, EvidenceBindingFile};
use coding_adventures_warrant_grammar::checker::check_warrant;
use coding_adventures_warrant_grammar::evidence::{compile_rust_evidence_harness, EvidenceOptions};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

pub const SPEC: &str = include_str!("fixtures/wxg.warrant");
pub const BINDINGS: &str = include_str!("fixtures/wxg.bindings");
const GENERATED: &str = "tests/generated/wxg_evidence.rs";

pub fn bindings() -> EvidenceBindingFile {
    parse_evidence_bindings(BINDINGS).expect("the fixture bindings parse")
}

pub fn options(b: &EvidenceBindingFile, spec: &str) -> EvidenceOptions {
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
        hosts: b.hosts.clone(),
    }
}

pub fn generate() -> String {
    let doc = SourceDocument::admit(SPEC.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");
    let checked = check_warrant(&file).expect("the fixture must type-check");
    let b = bindings();
    compile_rust_evidence_harness(&file, &checked, &b.manifest, &options(&b, SPEC))
        .expect("generation must succeed")
}

#[test]
fn checked_in_wxg_harness_matches_the_fixture_and_generator() {
    let text = generate();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(GENERATED);
    if std::env::var("WARRANT_REGEN").is_ok() {
        std::fs::write(&path, text.as_bytes()).expect("write generated harness");
        return;
    }
    let on_disk = std::fs::read_to_string(&path).expect("generated harness is checked in");
    assert_eq!(
        on_disk, text,
        "tests/generated/wxg_evidence.rs is stale; rerun with WARRANT_REGEN=1"
    );
}
