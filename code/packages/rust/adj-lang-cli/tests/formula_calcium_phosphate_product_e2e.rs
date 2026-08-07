//! End-to-end tests for the `clinical/calcium-phosphate-product.adj` library — the definition of
//! the calcium phosphate product (CPP = serum calcium × serum phosphate) and its two exact
//! rearrangements — driven through the built CLI binary against the SHIPPED stdlib. The same
//! invariant as every other formula library: a consumer states NO arithmetic; it imports the
//! grounded library, binds the measured labs with `observe`, and the engine applies the cited
//! relation on the CPU, computing the EXACT value and rendering the relation's citation and trust
//! tier in the `derived` section (the auditable answer). The three formulas INVERT around the
//! worked case Ca = 9 mg/dL, PO4 = 4 mg/dL: 9 × 4 = 36, 36 ÷ 4 = 9, and 36 ÷ 9 = 4. The three
//! asserted values (36, 9, 4) are chosen so none is a colon-anchored prefix of another rendered
//! value. This is the renal/bone-mineral product-cousin of the shipped rate-pressure-product.adj
//! (RPP = HR × SBP), pairing two serum minerals the same multiplicative way.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped calcium-phosphate-product library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_cpp_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/calcium-phosphate-product.adj")
        .canonicalize()
        .expect("shipped calcium-phosphate-product.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_cpp_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(program: &Path) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .arg(program)
        .output()
        .expect("run adj-lang-cli");
    (out.status.success(), String::from_utf8(out.stdout).unwrap())
}

/// Copy the shipped library next to a consumer that imports it, so the CLI's
/// sandbox-checked relative import resolves.
fn place_lib(dir: &Path) {
    let lib = std::fs::read_to_string(shipped_cpp_lib()).unwrap();
    std::fs::write(dir.join("calcium-phosphate-product.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// calcium_phosphate_product — the definition: serum calcium times serum phosphate.
// ---------------------------------------------------------------------------

#[test]
fn imports_calcium_phosphate_product_library_and_computes_it_with_citation() {
    let dir = scratch("cpp");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"calcium-phosphate-product.adj\"\n\
         observe serum_calcium(9)\n\
         observe serum_phosphate(4)\n\
         ? calcium_phosphate_product(serum_calcium, serum_phosphate)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied relation's result: 9 × 4 = 36.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"calcium_phosphate_product\"") && s.contains("\"value\":36"),
        "calcium_phosphate_product(9, 4) = 36: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// serum_calcium — the same relation solved for calcium: CPP ÷ PO4.
// ---------------------------------------------------------------------------

#[test]
fn computes_serum_calcium_from_product_and_phosphate_with_citation() {
    let dir = scratch("ca");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"calcium-phosphate-product.adj\"\n\
         observe calcium_phosphate_product(36)\n\
         observe serum_phosphate(4)\n\
         ? serum_calcium(calcium_phosphate_product, serum_phosphate)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 36 ÷ 4 = 9, computed on the CPU.
    assert!(
        s.contains("\"name\":\"serum_calcium\"") && s.contains("\"value\":9"),
        "serum_calcium(36, 4) = 9: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "serum_calcium carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// serum_phosphate — the same relation solved for phosphate: CPP ÷ Ca, the third reading of the one
// definition.
// ---------------------------------------------------------------------------

#[test]
fn computes_serum_phosphate_from_product_and_calcium_with_citation() {
    let dir = scratch("po4");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"calcium-phosphate-product.adj\"\n\
         observe calcium_phosphate_product(36)\n\
         observe serum_calcium(9)\n\
         ? serum_phosphate(calcium_phosphate_product, serum_calcium)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 36 ÷ 9 = 4, computed on the CPU.
    assert!(
        s.contains("\"name\":\"serum_phosphate\"") && s.contains("\"value\":4"),
        "serum_phosphate(36, 9) = 4: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "serum_phosphate carries its cited provenance: {s}"
    );
}
