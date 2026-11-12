//! Regenerates (or checks) `tests/generated/wxv_evidence.rs` and the table copies
//! under `tests/generated/tables/`, the WX-E evidence `tests/wxe_evidence.rs`
//! compiles and runs.
//!
//! `WARRANT_REGEN=1 cargo test -p coding_adventures_warrant_grammar --test
//! wxv_regen` rewrites the files; without the variable the test fails when a
//! checked-in file differs from the generator's output, like the BUILD drift
//! check of a real package.  The tables are read from the real fixtures root
//! (`code/specs/fixtures`), so the loader, the pin check and the materialisation
//! are on the path.

use std::path::{Path, PathBuf};

use coding_adventures_sha256::sha256_hex;
use coding_adventures_warrant_grammar::binding::{parse_evidence_bindings, EvidenceBindingFile};
use coding_adventures_warrant_grammar::checker::check_warrant;
use coding_adventures_warrant_grammar::evidence::{
    compile_rust_evidence_harness_with_tables, load_document_tables, EvidenceOptions, TableSet,
};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};
use coding_adventures_warrant_grammar::vectors::LoadedTable;

pub const SPEC: &str = include_str!("fixtures/wxv.warrant");
pub const BINDINGS: &str = include_str!("fixtures/wxv.bindings");
const GENERATED: &str = "tests/generated/wxv_evidence.rs";
pub const TABLE_DIR: &str = "tests/generated/tables";

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

/// The real fixtures root: `code/specs/fixtures` of this checkout.
pub fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../specs/fixtures")
}

pub fn generate_with(tables: &[LoadedTable]) -> String {
    let doc = SourceDocument::admit(SPEC.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");
    let checked = check_warrant(&file).expect("the fixture must type-check");
    let b = bindings();
    let set = TableSet {
        dir: TABLE_DIR,
        tables,
    };
    compile_rust_evidence_harness_with_tables(&file, &checked, &b.manifest, &options(&b, SPEC), &set)
        .expect("generation must succeed")
}

pub fn load() -> Vec<LoadedTable> {
    let doc = SourceDocument::admit(SPEC.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");
    load_document_tables(&file, &fixtures_root()).expect("the shipped tables load")
}

#[test]
fn checked_in_wxv_harness_and_table_copies_match_the_fixtures_and_generator() {
    let tables = load();
    let text = generate_with(&tables);
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let regen = std::env::var("WARRANT_REGEN").is_ok();
    let set = TableSet {
        dir: TABLE_DIR,
        tables: &tables,
    };
    if regen {
        std::fs::create_dir_all(root.join(TABLE_DIR)).expect("tables dir");
        std::fs::write(root.join(GENERATED), text.as_bytes()).expect("write generated harness");
        for (rel, bytes) in set.files() {
            std::fs::write(root.join(rel), bytes).expect("write table copy");
        }
        return;
    }
    let on_disk = std::fs::read_to_string(root.join(GENERATED)).expect("harness is checked in");
    assert_eq!(on_disk, text, "{GENERATED} is stale; rerun with WARRANT_REGEN=1");
    for (rel, bytes) in set.files() {
        let copy = std::fs::read(root.join(&rel)).unwrap_or_else(|_| panic!("{rel} is checked in"));
        assert_eq!(copy, bytes, "{rel} is stale; rerun with WARRANT_REGEN=1");
    }
}
