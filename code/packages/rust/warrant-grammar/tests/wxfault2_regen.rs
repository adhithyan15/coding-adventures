//! Regenerates (or checks) the compiled WX-FAULT2 integration fixture.

use coding_adventures_sha256::sha256_hex;
use coding_adventures_warrant_grammar::binding::{parse_evidence_bindings, EvidenceBindingFile};
use coding_adventures_warrant_grammar::checker::check_warrant;
use coding_adventures_warrant_grammar::evidence::{compile_rust_evidence_harness, EvidenceOptions};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

const SPEC: &str = include_str!("fixtures/wxfault2.warrant");
const BINDINGS: &str = include_str!("fixtures/wxfault2.bindings");
const GENERATED: &str = "tests/generated/wxfault2_evidence.rs";

fn generate() -> String {
    let source = SourceDocument::admit(SPEC.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&source, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &source, ParserLimits::default()).expect("parse");
    let checked = check_warrant(&file).expect("fixture checks");
    let bindings: EvidenceBindingFile =
        parse_evidence_bindings(BINDINGS).expect("bindings parse");
    let options = EvidenceOptions {
        package_crate: bindings.manifest.package.clone(),
        spec_path: bindings.spec.clone(),
        spec_sha256: sha256_hex(SPEC.as_bytes()),
        seed: bindings.seed,
        property_cases: bindings.property_cases,
        fuzz_iterations: bindings.fuzz_iterations,
        negctl_fuzz_iterations: bindings.negctl_fuzz_iterations,
        exhaustive_budget: bindings.exhaustive_budget,
        arg_types: bindings.arg_types.clone(),
        hosts: bindings.hosts.clone(),
    };
    compile_rust_evidence_harness(&file, &checked, &bindings.manifest, &options)
        .expect("generation succeeds")
}

#[test]
fn checked_in_wxfault2_harness_matches_fixture_and_generator() {
    let text = generate();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(GENERATED);
    if std::env::var("WARRANT_REGEN").is_ok() {
        std::fs::write(&path, text.as_bytes()).expect("write generated harness");
        return;
    }
    let on_disk = std::fs::read_to_string(&path).expect("generated harness is checked in");
    assert_eq!(on_disk, text, "wxfault2 generated harness is stale; rerun with WARRANT_REGEN=1");
}
