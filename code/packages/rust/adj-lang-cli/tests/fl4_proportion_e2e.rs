//! End-to-end tests for ADJ-FORMULA-LIBRARIES FL-4 — the `proportion.adj`
//! elementary library — driven through the built CLI binary against the SHIPPED
//! stdlib. Each proves the FL-4 invariant: the library COMPOSES the cited
//! `arithmetic.adj` primitives (it re-derives no arithmetic), computes the exact
//! value on the CPU, and carries BOTH its own citation and the primitives' as
//! corroborations — write-once-use-many all the way down. The `fourth_proportional`
//! formula is two levels deep: `quotient(product(b, c), a)` — the rule of three.

use std::path::{Path, PathBuf};
use std::process::Command;

fn stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib")
        .canonicalize()
        .expect("shipped adj-formula-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_fl4prop_{tag}_{}", std::process::id()));
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
    place(dir, "arithmetic/proportion.adj");
}

// ---------------------------------------------------------------------------
// fourth_proportional — the rule of three, composing product THEN quotient.
// ---------------------------------------------------------------------------

#[test]
fn fourth_proportional_composes_product_and_quotient_and_carries_all_citations() {
    let dir = scratch("fourth");
    with_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"proportion.adj\"\n\
         observe first_term(2)\n\
         observe second_term(3)\n\
         observe third_term(4)\n\
         ? fourth_proportional(first_term, second_term, third_term)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 2/3 = 4/x → x = (3 * 4) / 2 = 6, via product then quotient on the CPU.
    assert!(
        s.contains("\"name\":\"fourth_proportional\"") && s.contains("\"value\":6"),
        "fourth_proportional(2, 3, 4) = 6: {s}"
    );
    // Exact integer 6/1 — no f64 round-trip.
    assert!(s.contains("\"num\":\"6\"") && s.contains("\"den\":\"1\""), "exact value 6/1: {s}");
    // The primary cites the cross-multiplication rule of three.
    assert!(
        s.contains("en.wikipedia.org/wiki/Cross-multiplication"),
        "primary cites the rule-of-three source: {s}"
    );
    // BOTH composed primitives appear as corroborations.
    assert!(
        s.contains("mathworld.wolfram.com/Product.html"),
        "corroboration cites the product primitive it composed: {s}"
    );
    assert!(
        s.contains("mathworld.wolfram.com/Quotient.html"),
        "corroboration cites the quotient primitive it composed: {s}"
    );
}

// ---------------------------------------------------------------------------
// The compute trace names both composed ops over the observed slots.
// ---------------------------------------------------------------------------

#[test]
fn the_derivation_tree_names_the_quotient_over_the_product() {
    let dir = scratch("tree");
    with_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"proportion.adj\"\n\
         observe first_term(2)\n\
         observe second_term(3)\n\
         observe third_term(4)\n\
         ? fourth_proportional(first_term, second_term, third_term)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    // The outer op is the division (12 / 2 = 6); the inner op is the product
    // (3 * 4 = 12) — the answer is reconstructable operand by operand.
    assert!(
        s.contains("\"node\":\"op\"") && s.contains("\"op\":\"/\"") && s.contains("\"op\":\"*\""),
        "the derivation names the quotient over the product: {s}"
    );
    assert!(
        s.contains("\"slot\":\"first_term\"")
            && s.contains("\"slot\":\"second_term\"")
            && s.contains("\"slot\":\"third_term\""),
        "the leaves name the observed slots the value was built from: {s}"
    );
}

// ---------------------------------------------------------------------------
// Honest edge case — a degenerate proportion 0/b = c/x has no finite fourth
// proportional; the engine REFUSES (DivisionByZero), never fabricates a value.
// ---------------------------------------------------------------------------

#[test]
fn a_zero_first_term_is_refused_not_fabricated() {
    let dir = scratch("zero");
    with_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"proportion.adj\"\n\
         observe first_term(0)\n\
         observe second_term(3)\n\
         observe third_term(4)\n\
         ? fourth_proportional(first_term, second_term, third_term)\n",
    )
    .unwrap();

    let (_ok, s) = run(&dir.join("case.adj"));
    // Dividing by the zero first term is refused: the engine reports
    // DivisionByZero rather than inventing a fourth proportional.
    assert!(s.contains("DivisionByZero"), "a zero first term must be refused, not fabricated: {s}");
    assert!(!s.contains("\"value\":0"), "no fabricated value for a degenerate proportion: {s}");
}
