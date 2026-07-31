//! End-to-end tests for ADJ-FORMULA-LIBRARIES FL-4 — the `percent-of.adj`
//! elementary library — driven through the built CLI binary against the SHIPPED
//! stdlib. Proves the FL-4 invariant: the library COMPOSES the cited
//! `arithmetic.adj` `product` primitive (it re-derives no arithmetic), computes
//! the exact value on the CPU, and carries BOTH its own citation and the
//! primitive's as corroboration. `percent_of(whole, rate) = (whole·rate)/100` —
//! the inverse of `percent`.

use std::path::{Path, PathBuf};
use std::process::Command;

fn stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib")
        .canonicalize()
        .expect("shipped adj-formula-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_fl4po_{tag}_{}", std::process::id()));
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

/// Copy a shipped `.adj` file (by relative path under the stdlib) into `dir`
/// under its basename, so a consumer's relative `import` resolves.
fn place(dir: &Path, rel: &str) {
    let src = stdlib().join(rel);
    let name = Path::new(rel).file_name().unwrap();
    std::fs::copy(&src, dir.join(name)).unwrap_or_else(|e| panic!("copy {rel}: {e}"));
}

fn with_lib(dir: &Path) {
    place(dir, "arithmetic/arithmetic.adj");
    place(dir, "arithmetic/percent-of.adj");
}

// ---------------------------------------------------------------------------
// percent_of — (whole·rate)/100, composing the cited product primitive.
// ---------------------------------------------------------------------------

#[test]
fn percent_of_composes_product_and_carries_both_citations() {
    let dir = scratch("po");
    with_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"percent-of.adj\"\n\
         observe whole(50)\n\
         observe rate(20)\n\
         ? percent_of(whole, rate)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 20% of 50 → (50 * 20) / 100 = 10, via the product primitive on the CPU.
    assert!(
        s.contains("\"name\":\"percent_of\"") && s.contains("\"value\":10"),
        "percent_of(50, 20) = 10: {s}"
    );
    // Exact integer 10/1 — no f64 round-trip.
    assert!(s.contains("\"num\":\"10\"") && s.contains("\"den\":\"1\""), "exact value 10/1: {s}");
    // The primary cites the percent definition.
    assert!(
        s.contains("mathworld.wolfram.com/Percent.html"),
        "primary cites the percent definition: {s}"
    );
    // The composed primitive appears as corroboration.
    assert!(
        s.contains("mathworld.wolfram.com/Product.html"),
        "corroboration cites the product primitive it composed: {s}"
    );
}

// ---------------------------------------------------------------------------
// The compute trace names the division over the product.
// ---------------------------------------------------------------------------

#[test]
fn the_derivation_tree_names_the_division_over_the_product() {
    let dir = scratch("tree");
    with_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"percent-of.adj\"\n\
         observe whole(50)\n\
         observe rate(20)\n\
         ? percent_of(whole, rate)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    // Outer op is the /100 division; the inner op is the product (50*20 = 1000).
    assert!(
        s.contains("\"node\":\"op\"") && s.contains("\"op\":\"/\"") && s.contains("\"op\":\"*\""),
        "the derivation names the division over the product: {s}"
    );
    assert!(
        s.contains("\"slot\":\"whole\"") && s.contains("\"slot\":\"rate\""),
        "the leaves name the observed slots the value was built from: {s}"
    );
}

// ---------------------------------------------------------------------------
// Honest edge case — 0% of anything is 0; the engine COMPUTES it, never invents.
// ---------------------------------------------------------------------------

#[test]
fn zero_percent_of_anything_is_zero() {
    let dir = scratch("zero");
    with_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"percent-of.adj\"\n\
         observe whole(50)\n\
         observe rate(0)\n\
         ? percent_of(whole, rate)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    assert!(
        s.contains("\"name\":\"percent_of\"") && s.contains("\"value\":0"),
        "percent_of(50, 0) = 0: {s}"
    );
}
