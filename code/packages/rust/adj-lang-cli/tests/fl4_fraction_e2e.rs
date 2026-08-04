//! End-to-end tests for ADJ-FORMULA-LIBRARIES FL-4 — the `fraction.adj`
//! elementary library — driven through the built CLI binary against the SHIPPED
//! stdlib. Each proves the FL-4 invariant: the library COMPOSES the cited
//! `arithmetic.adj` primitives (it re-derives no arithmetic), computes the exact
//! value on the CPU, and carries BOTH its own citation and the primitives' as
//! corroborations — write-once-use-many all the way down.

use std::path::{Path, PathBuf};
use std::process::Command;

fn stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib")
        .canonicalize()
        .expect("shipped adj-formula-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_fl4frac_{tag}_{}", std::process::id()));
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
    place(dir, "arithmetic/fraction.adj");
}

// ---------------------------------------------------------------------------
// fraction_of — a fractional part of a whole, composing product.
// ---------------------------------------------------------------------------

#[test]
fn fraction_of_composes_product_and_carries_both_citations() {
    let dir = scratch("fraction_of");
    with_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fraction.adj\"\n\
         observe fraction(0.25)\n\
         observe whole(12)\n\
         ? fraction_of(fraction, whole)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 0.25 * 12 = 3, via the product primitive on the CPU.
    assert!(
        s.contains("\"name\":\"fraction_of\"") && s.contains("\"value\":3"),
        "fraction_of(0.25, 12) = 3: {s}"
    );
    // BOTH citations: fraction's own definition (primary) AND the product
    // primitive it composed (corroboration).
    assert!(
        s.contains("mathworld.wolfram.com/Fraction.html"),
        "primary cites the fraction definition: {s}"
    );
    assert!(
        s.contains("mathworld.wolfram.com/Product.html"),
        "corroboration cites the product primitive it composed: {s}"
    );
}

// ---------------------------------------------------------------------------
// reciprocal — the fraction flipped, composing quotient with swapped args.
// ---------------------------------------------------------------------------

#[test]
fn reciprocal_flips_the_fraction_via_quotient() {
    let dir = scratch("reciprocal");
    with_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fraction.adj\"\n\
         observe numerator(4)\n\
         observe denominator(5)\n\
         ? reciprocal(numerator, denominator)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // reciprocal of 4/5 is 5/4 = 1.25, via quotient(denominator, numerator).
    assert!(
        s.contains("\"name\":\"reciprocal\"") && s.contains("\"value\":1.25"),
        "reciprocal(4, 5) = 1.25: {s}"
    );
    // The exact rational is preserved as 5/4 — no f64 round-trip.
    assert!(
        s.contains("\"num\":\"5\"") && s.contains("\"den\":\"4\""),
        "the exact value is 5/4, not a lossy decimal: {s}"
    );
    assert!(
        s.contains("mathworld.wolfram.com/Reciprocal.html"),
        "primary cites the reciprocal definition: {s}"
    );
    assert!(
        s.contains("mathworld.wolfram.com/Quotient.html"),
        "corroboration cites the quotient primitive it composed: {s}"
    );
}

// ---------------------------------------------------------------------------
// mixed_number — a whole plus a proper fraction, composing sum and quotient.
// ---------------------------------------------------------------------------

#[test]
fn mixed_number_composes_sum_and_quotient() {
    let dir = scratch("mixed");
    with_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fraction.adj\"\n\
         observe whole_number(2)\n\
         observe numerator(1)\n\
         observe denominator(2)\n\
         ? mixed_number(whole_number, numerator, denominator)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // two and a half = 2 + 1/2 = 2.5, composing sum and quotient.
    assert!(
        s.contains("\"name\":\"mixed_number\"") && s.contains("\"value\":2.5"),
        "mixed_number(2, 1, 2) = 2.5: {s}"
    );
    assert!(
        s.contains("mathworld.wolfram.com/MixedFraction.html"),
        "primary cites the mixed-number definition: {s}"
    );
    // Both composed primitives appear as corroborations.
    assert!(
        s.contains("mathworld.wolfram.com/Sum.html"),
        "corroboration cites the sum primitive: {s}"
    );
    assert!(
        s.contains("mathworld.wolfram.com/Quotient.html"),
        "corroboration cites the quotient primitive: {s}"
    );
}

// ---------------------------------------------------------------------------
// The compute trace bottoms out at the observed slots — no model arithmetic.
// ---------------------------------------------------------------------------

#[test]
fn the_derivation_tree_names_the_composed_op_and_its_leaves() {
    let dir = scratch("tree");
    with_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fraction.adj\"\n\
         observe fraction(0.5)\n\
         observe whole(10)\n\
         ? fraction_of(fraction, whole)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    // 0.5 * 10 = 5, and the trace shows the multiplication over the two observed
    // leaves — the answer is reconstructable operand by operand, not asserted.
    assert!(s.contains("\"value\":5"), "fraction_of(0.5, 10) = 5: {s}");
    assert!(
        s.contains("\"node\":\"op\"") && s.contains("\"op\":\"*\""),
        "the derivation names the product op: {s}"
    );
    assert!(
        s.contains("\"slot\":\"fraction\"") && s.contains("\"slot\":\"whole\""),
        "the leaves name the observed slots the value was built from: {s}"
    );
}
