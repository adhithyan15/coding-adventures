//! End-to-end tests for ADJ-FORMULA-LIBRARIES FL-4 — the `simple-interest.adj`
//! elementary library — driven through the built CLI binary against the SHIPPED
//! stdlib. Proves the FL-4 invariant: the library COMPOSES the cited
//! `arithmetic.adj` `product` primitive (twice) — it re-derives no arithmetic,
//! computes the exact value on the CPU, and carries BOTH its own citation and the
//! primitive's as corroboration. `simple_interest(P, R, T) = (P·R·T)/100`.

use std::path::{Path, PathBuf};
use std::process::Command;

fn stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib")
        .canonicalize()
        .expect("shipped adj-formula-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_fl4si_{tag}_{}", std::process::id()));
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
    place(dir, "arithmetic/simple-interest.adj");
}

// ---------------------------------------------------------------------------
// simple_interest — (P·R·T)/100, composing the cited product primitive twice.
// ---------------------------------------------------------------------------

#[test]
fn simple_interest_composes_product_twice_and_carries_both_citations() {
    let dir = scratch("si");
    with_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"simple-interest.adj\"\n\
         observe principal(1000)\n\
         observe rate(5)\n\
         observe time(2)\n\
         ? simple_interest(principal, rate, time)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // $1000 at 5%/yr for 2 yr → (1000 * 5 * 2) / 100 = 100, via product then product.
    assert!(
        s.contains("\"name\":\"simple_interest\"") && s.contains("\"value\":100"),
        "simple_interest(1000, 5, 2) = 100: {s}"
    );
    // Exact integer 100/1 — no f64 round-trip.
    assert!(s.contains("\"num\":\"100\"") && s.contains("\"den\":\"1\""), "exact value 100/1: {s}");
    // The primary cites the simple-interest definition.
    assert!(
        s.contains("cuemath.com/commercial-math/simple-interest"),
        "primary cites the simple-interest source: {s}"
    );
    // The composed primitive appears as corroboration.
    assert!(
        s.contains("mathworld.wolfram.com/Product.html"),
        "corroboration cites the product primitive it composed: {s}"
    );
}

// ---------------------------------------------------------------------------
// The compute trace names the division over the nested products.
// ---------------------------------------------------------------------------

#[test]
fn the_derivation_tree_names_the_division_over_the_products() {
    let dir = scratch("tree");
    with_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"simple-interest.adj\"\n\
         observe principal(1000)\n\
         observe rate(5)\n\
         observe time(2)\n\
         ? simple_interest(principal, rate, time)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    // Outer op is the /100 division; the inner ops are the two products
    // (1000*5 = 5000, *2 = 10000) — reconstructable operand by operand.
    assert!(
        s.contains("\"node\":\"op\"") && s.contains("\"op\":\"/\"") && s.contains("\"op\":\"*\""),
        "the derivation names the division over the products: {s}"
    );
    assert!(
        s.contains("\"slot\":\"principal\"")
            && s.contains("\"slot\":\"rate\"")
            && s.contains("\"slot\":\"time\""),
        "the leaves name the observed slots the value was built from: {s}"
    );
}

// ---------------------------------------------------------------------------
// Honest edge case — a zero rate earns zero interest; the engine COMPUTES it
// (it does not special-case or invent), (1000 * 0 * 2)/100 = 0.
// ---------------------------------------------------------------------------

#[test]
fn a_zero_rate_earns_zero_interest() {
    let dir = scratch("zero");
    with_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"simple-interest.adj\"\n\
         observe principal(1000)\n\
         observe rate(0)\n\
         observe time(2)\n\
         ? simple_interest(principal, rate, time)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    assert!(
        s.contains("\"name\":\"simple_interest\"") && s.contains("\"value\":0"),
        "simple_interest(1000, 0, 2) = 0: {s}"
    );
}
