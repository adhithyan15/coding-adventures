//! End-to-end test for `mathematics/place-value.adj` — CCSS-M 1.NBT.B.2
//! (a two-digit number is composed of tens and ones), driven through the
//! built CLI binary against the SHIPPED stdlib. Composes `arithmetic.adj`'s
//! `product`/`sum` (a cross-directory import, like `cockcroft_gault.adj` and
//! `mathematics/number-sequence.adj`/`cardinality.adj`).

use std::path::{Path, PathBuf};
use std::process::Command;

fn stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib")
        .canonicalize()
        .expect("shipped adj-formula-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_placeval_{tag}_{}", std::process::id()));
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

/// Copy a shipped `.adj` file into `dir` at the given relative destination
/// (creating parent dirs), preserving the directory layout so relative
/// imports (`../arithmetic/…`) resolve from an entry program at the scratch
/// root.
fn place_at(dir: &Path, src_rel: &str, dst_rel: &str) {
    let src = stdlib().join(src_rel);
    let dst = dir.join(dst_rel);
    std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
    std::fs::copy(&src, &dst).unwrap_or_else(|e| panic!("copy {src_rel} -> {dst_rel}: {e}"));
}

#[test]
fn tens_and_ones_compose_a_two_digit_number_via_product_and_sum() {
    let dir = scratch("compose");
    place_at(&dir, "arithmetic/arithmetic.adj", "arithmetic/arithmetic.adj");
    place_at(
        &dir,
        "mathematics/place-value.adj",
        "mathematics/place-value.adj",
    );
    std::fs::write(
        dir.join("case.adj"),
        "import \"mathematics/place-value.adj\"\n\
         observe tens(4)\n\
         observe ones(7)\n\
         ? tens_and_ones_to_number(tens, ones)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 4 tens (4 * 10 = 40) + 7 ones = 47.
    assert!(
        s.contains("\"name\":\"tens_and_ones_to_number\"") && s.contains("\"value\":47"),
        "tens_and_ones_to_number(4, 7) = 47: {s}"
    );
    assert!(
        s.contains("\"trust\":\"consensus\"")
            && s.contains("mathsisfun.com/definitions/positional-notation.html"),
        "carries the positional-notation citation: {s}"
    );
    // Composed via the cited `product`/`sum` primitives, not bare arithmetic.
    assert!(
        s.contains("mathworld.wolfram.com/Product.html")
            && s.contains("mathworld.wolfram.com/Sum.html"),
        "corroborated by both composed arithmetic.adj primitives' citations: {s}"
    );
}

#[test]
fn zero_ones_is_a_clean_multiple_of_ten() {
    let dir = scratch("zero_ones");
    place_at(&dir, "arithmetic/arithmetic.adj", "arithmetic/arithmetic.adj");
    place_at(
        &dir,
        "mathematics/place-value.adj",
        "mathematics/place-value.adj",
    );
    std::fs::write(
        dir.join("case.adj"),
        "import \"mathematics/place-value.adj\"\n\
         observe tens(3)\n\
         observe ones(0)\n\
         ? tens_and_ones_to_number(tens, ones)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(
        s.contains("\"name\":\"tens_and_ones_to_number\"") && s.contains("\"value\":30"),
        "tens_and_ones_to_number(3, 0) = 30: {s}"
    );
}
