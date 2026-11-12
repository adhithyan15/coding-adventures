//! # The BUILD driver passes host composites through (PR-0, item 1)
//!
//! Every package `BUILD` runs
//! `cargo run -p coding_adventures_warrant_grammar --example gen_evidence -- --check <pkg>`.
//! That example is the *only* path from an `evidence.bindings` file to a
//! checked-in harness, so whatever it drops never reaches the generator.
//!
//! Until this test existed the example built its `EvidenceOptions` with
//! `hosts: Default::default()`.  The parser read the WX-A host lines
//! (`struct`, `enum`, and on an `op` line `args=`, `ret=` and `errors=`), the
//! in-test drivers (`tests/wxa_regen.rs` and friends) passed them on, and the
//! example silently threw them away.  A package whose operation returns a host
//! `Result<(u64, usize), Leb128Error>` therefore failed generation with
//! W0704 ("neither error normalization nor errors=") or W0701 ("an args= entry
//! is not a host type") although its bindings said exactly how to adapt it.
//!
//! The test compiles the example itself into this test crate (`#[path]`), lays
//! out a miniature repository in a scratch directory -- a Warrant document under
//! `code/specs/` and a bindings file under `code/packages/rust/<pkg>/` -- and
//! drives `generate_one` exactly as `main` does for a BUILD line:
//!
//! ```text
//! <scratch>/code/specs/wxa_pkg.warrant                       (tests/fixtures/wxa.warrant)
//! <scratch>/code/packages/rust/wxa_pkg/evidence.bindings     (tests/fixtures/wxa.bindings,
//!                                                            spec/output re-pinned)
//!        │ generate_one(root, "wxa_pkg", check = false)
//!        ▼
//! <scratch>/code/packages/rust/wxa_pkg/tests/generated/warrant_evidence.rs
//! ```
//!
//! The fixture is the WX-A pilot: it has `struct` and `enum` lines, composite
//! `args=` (`opt<(u8,bool)>,list<bool>`), tuple and record `ret=` and an
//! `errors=` mapping, i.e. every host-line form the parser accepts.

#[allow(dead_code)]
#[path = "../examples/gen_evidence.rs"]
mod driver;

use std::fs;
use std::path::{Path, PathBuf};

use coding_adventures_sha256::sha256_hex;
use coding_adventures_warrant_grammar::binding::parse_evidence_bindings;
use coding_adventures_warrant_grammar::checker::check_warrant;
use coding_adventures_warrant_grammar::evidence::{compile_rust_evidence_harness, EvidenceOptions};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

const SPEC: &str = include_str!("fixtures/wxa.warrant");
const BINDINGS: &str = include_str!("fixtures/wxa.bindings");
const PKG: &str = "wxa_pkg";

/// A fresh miniature repository root for one test.
fn scratch_root(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("gen_evidence_driver").join(name);
    if root.exists() {
        fs::remove_dir_all(&root).expect("clear the previous run's scratch root");
    }
    fs::create_dir_all(root.join("code/specs")).expect("create code/specs");
    fs::create_dir_all(root.join(format!("code/packages/rust/{PKG}/tests/generated")))
        .expect("create the package's tests/generated");
    root
}

/// The fixture bindings with `spec` and `output` re-pinned to the package
/// directory, because `validate_paths` (rightly) refuses any other location.
fn package_bindings() -> String {
    BINDINGS
        .lines()
        .map(|line| {
            if line.starts_with("spec ") {
                format!("spec code/specs/{PKG}.warrant")
            } else if line.starts_with("output ") {
                format!("output code/packages/rust/{PKG}/tests/generated/warrant_evidence.rs")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn lay_out(root: &Path) {
    fs::write(root.join(format!("code/specs/{PKG}.warrant")), SPEC).expect("write spec");
    fs::write(root.join(format!("code/packages/rust/{PKG}/evidence.bindings")), package_bindings())
        .expect("write bindings");
}

/// What an in-test driver (which has always passed `hosts`) generates for the
/// same inputs: the reference the BUILD driver must now reproduce byte for byte.
fn reference_harness() -> String {
    let b = parse_evidence_bindings(&package_bindings()).expect("re-pinned bindings parse");
    let doc = SourceDocument::admit(SPEC.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");
    let checked = check_warrant(&file).expect("the fixture type-checks");
    let options = EvidenceOptions {
        package_crate: b.manifest.package.clone(),
        spec_path: b.spec.clone(),
        spec_sha256: sha256_hex(SPEC.as_bytes()),
        seed: b.seed,
        property_cases: b.property_cases,
        fuzz_iterations: b.fuzz_iterations,
        negctl_fuzz_iterations: b.negctl_fuzz_iterations,
        exhaustive_budget: b.exhaustive_budget,
        arg_types: b.arg_types.clone(),
        hosts: b.hosts.clone(),
    };
    compile_rust_evidence_harness(&file, &checked, &b.manifest, &options).expect("reference generation")
}

#[test]
fn the_fixture_really_has_every_host_line_form() {
    // Guard against the fixture quietly losing the lines this test is about.
    let b = parse_evidence_bindings(&package_bindings()).expect("parse");
    assert!(!b.hosts.types.is_empty(), "struct/enum lines");
    let decode2 = &b.hosts.ops["decode2"];
    assert!(decode2.args.is_some() && decode2.ret.is_some() && decode2.errors.is_some());
    assert!(b.hosts.ops["pick"].args.is_some(), "composite args=");
}

#[test]
fn build_driver_generates_a_bindings_file_with_ret_errors_and_composite_args() {
    let root = scratch_root("generate");
    lay_out(&root);
    driver::generate_one(&root, PKG, false)
        .unwrap_or_else(|e| panic!("the BUILD driver must accept ret=/errors=/args= host lines: {e}"));
    let written = fs::read_to_string(root.join(format!(
        "code/packages/rust/{PKG}/tests/generated/warrant_evidence.rs"
    )))
    .expect("the driver wrote the harness");
    // The errors= mapping reached the adapter (a host error variant is matched
    // by its path), and the output is exactly what the host-aware in-test
    // driver produces.
    assert!(written.contains("wxa_subject::LebErr::Overlong { .. } => V::Err(\"OVERLONG\".into())"));
    assert_eq!(written, reference_harness(), "BUILD driver and in-test driver disagree");
    // `--check` on what was just written is clean.
    driver::generate_one(&root, PKG, true).expect("--check of a fresh harness passes");
}

#[test]
fn build_driver_check_detects_drift_in_a_host_bound_harness() {
    let root = scratch_root("drift");
    lay_out(&root);
    driver::generate_one(&root, PKG, false).expect("generate");
    let out = root.join(format!("code/packages/rust/{PKG}/tests/generated/warrant_evidence.rs"));
    let mut text = fs::read_to_string(&out).expect("read");
    text.push_str("// hand edit\n");
    fs::write(&out, text).expect("tamper");
    let err = driver::generate_one(&root, PKG, true).expect_err("a hand-edited harness must fail --check");
    assert!(err.to_string().contains("generated evidence drift"), "{err}");
}
