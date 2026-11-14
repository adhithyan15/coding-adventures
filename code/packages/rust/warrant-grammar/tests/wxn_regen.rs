//! Regenerates (or checks) `tests/generated/wxn_evidence.rs`, the WX-N evidence
//! harness `tests/wxn_evidence.rs` compiles and runs.
//!
//! `WARRANT_REGEN=1 cargo test -p coding_adventures_warrant_grammar --test
//! wxn_regen` rewrites the file; without the variable the test fails when the
//! checked-in file differs from the generator's output, exactly like the BUILD
//! drift check of a real package.  The bindings come from the fixture
//! `tests/fixtures/wxn.bindings`, so the bindings-file parser is on the path.

use coding_adventures_sha256::sha256_hex;
use coding_adventures_warrant_grammar::binding::{parse_evidence_bindings, EvidenceBindingFile};
use coding_adventures_warrant_grammar::checker::check_warrant;
use coding_adventures_warrant_grammar::evidence::{compile_rust_evidence_harness, EvidenceOptions};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

pub const SPEC: &str = include_str!("fixtures/wxn.warrant");
pub const BINDINGS: &str = include_str!("fixtures/wxn.bindings");
const GENERATED: &str = "tests/generated/wxn_evidence.rs";

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
fn checked_in_wxn_harness_matches_the_fixture_and_generator() {
    let text = generate();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(GENERATED);
    if std::env::var("WARRANT_REGEN").is_ok() {
        std::fs::write(&path, text.as_bytes()).expect("write generated harness");
        return;
    }
    let on_disk = std::fs::read_to_string(&path).expect("generated harness is checked in");
    assert_eq!(
        on_disk, text,
        "tests/generated/wxn_evidence.rs is stale; rerun with WARRANT_REGEN=1"
    );
}
