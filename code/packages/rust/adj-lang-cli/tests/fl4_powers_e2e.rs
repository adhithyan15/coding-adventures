//! End-to-end tests for ADJ-FORMULA-LIBRARIES FL-4 — the `powers.adj`
//! elementary library — driven through the built CLI binary against the SHIPPED
//! stdlib. Each proves the FL-4 invariant: the library COMPOSES the cited
//! `product` primitive from `arithmetic.adj` (it re-derives no arithmetic),
//! computes the exact value on the CPU, and carries BOTH its own citation and
//! the `product` primitive's as a corroboration. `cube` additionally proves the
//! RS-1 nesting case — one `product` feeding another — bottoming out at a single
//! observed base.

use std::path::{Path, PathBuf};
use std::process::Command;

fn stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib")
        .canonicalize()
        .expect("shipped adj-formula-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_fl4pow_{tag}_{}", std::process::id()));
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
    place(dir, "arithmetic/powers.adj");
}

// ---------------------------------------------------------------------------
// square — a base multiplied by itself, composing product.
// ---------------------------------------------------------------------------

#[test]
fn square_composes_product_and_carries_both_citations() {
    let dir = scratch("square");
    with_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"powers.adj\"\n\
         observe base(5)\n\
         ? square(base)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 5 * 5 = 25, via the product primitive on the CPU.
    assert!(
        s.contains("\"name\":\"square\"") && s.contains("\"value\":25"),
        "square(5) = 25: {s}"
    );
    // BOTH citations: square's own definition (primary) AND the product
    // primitive it composed (corroboration).
    assert!(
        s.contains("mathworld.wolfram.com/Square.html"),
        "primary cites the square definition: {s}"
    );
    assert!(
        s.contains("mathworld.wolfram.com/Product.html"),
        "corroboration cites the product primitive it composed: {s}"
    );
}

// ---------------------------------------------------------------------------
// cube — a base multiplied by itself twice: product nested inside product.
// ---------------------------------------------------------------------------

#[test]
fn cube_nests_product_inside_product() {
    let dir = scratch("cube");
    with_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"powers.adj\"\n\
         observe base(3)\n\
         ? cube(base)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // (3 * 3) * 3 = 27, composing product twice.
    assert!(
        s.contains("\"name\":\"cube\"") && s.contains("\"value\":27"),
        "cube(3) = 27: {s}"
    );
    assert!(
        s.contains("mathworld.wolfram.com/Cube.html"),
        "primary cites the cube definition: {s}"
    );
    assert!(
        s.contains("mathworld.wolfram.com/Product.html"),
        "corroboration cites the product primitive it composed: {s}"
    );
}

// ---------------------------------------------------------------------------
// The exact value is a rational, and the trace bottoms out at the observed base
// through a chain of multiplications — no model arithmetic.
// ---------------------------------------------------------------------------

#[test]
fn the_derivation_tree_names_the_product_op_and_the_observed_base() {
    let dir = scratch("tree");
    with_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"powers.adj\"\n\
         observe base(4)\n\
         ? square(base)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    // 4 * 4 = 16, exact, and the trace shows the multiplication over the single
    // observed base used twice — reconstructable operand by operand.
    assert!(s.contains("\"value\":16"), "square(4) = 16: {s}");
    assert!(
        s.contains("\"num\":\"16\"") && s.contains("\"den\":\"1\""),
        "the exact value is 16/1, not a lossy decimal: {s}"
    );
    assert!(
        s.contains("\"node\":\"op\"") && s.contains("\"op\":\"*\""),
        "the derivation names the product op: {s}"
    );
    assert!(
        s.contains("\"slot\":\"base\""),
        "the leaves name the observed base the value was built from: {s}"
    );
}
