//! End-to-end test for `mathematics/place-value.adj` — CCSS-M 1.NBT.B.2
//! (a two-digit number is composed of, and decomposes into, tens and ones),
//! driven through the built CLI binary against the SHIPPED stdlib. COMPOSE
//! composes `arithmetic.adj`'s `product`/`sum` (a cross-directory import,
//! like `cockcroft_gault.adj` and `mathematics/number-sequence.adj`/
//! `cardinality.adj`); DECOMPOSE uses ADJ-FORMULA-LIBRARIES FL-9's
//! `floor`/`mod` built-ins.

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

#[test]
fn tens_digit_decomposes_via_floor() {
    let dir = scratch("decompose_tens");
    place_at(&dir, "arithmetic/arithmetic.adj", "arithmetic/arithmetic.adj");
    place_at(
        &dir,
        "mathematics/place-value.adj",
        "mathematics/place-value.adj",
    );
    std::fs::write(
        dir.join("case.adj"),
        "import \"mathematics/place-value.adj\"\n\
         observe n(47)\n\
         ? tens_digit(n)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    assert!(
        s.contains("\"name\":\"tens_digit\"") && s.contains("\"value\":4"),
        "tens_digit(47) = 4: {s}"
    );
    assert!(
        s.contains("\"trust\":\"consensus\"")
            && s.contains("mathsisfun.com/definitions/positional-notation.html"),
        "carries the positional-notation citation: {s}"
    );
}

#[test]
fn ones_digit_decomposes_via_mod() {
    let dir = scratch("decompose_ones");
    place_at(&dir, "arithmetic/arithmetic.adj", "arithmetic/arithmetic.adj");
    place_at(
        &dir,
        "mathematics/place-value.adj",
        "mathematics/place-value.adj",
    );
    std::fs::write(
        dir.join("case.adj"),
        "import \"mathematics/place-value.adj\"\n\
         observe n(47)\n\
         ? ones_digit(n)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(
        s.contains("\"name\":\"ones_digit\"") && s.contains("\"value\":7"),
        "ones_digit(47) = 7: {s}"
    );
}

#[test]
fn decompose_and_compose_round_trip() {
    // Decomposing 83 (tens_digit/ones_digit) and independently recomposing
    // the digits it should yield (tens_and_ones_to_number(8, 3)) both land on
    // 83 — the algebraic-inverse relationship the library's own header
    // documents. Two top-level queries rather than nesting one application
    // inside another's query arguments: `? f(g(x))` parses its argument list
    // as logic TERMS (for relational recall queries), not the compute-`expr`
    // grammar a formula BODY uses, so cross-formula composition happens
    // inside a formula body (as `tens_and_ones_to_number` itself already
    // demonstrates via `product`/`sum`), not at the top-level query site.
    let dir = scratch("round_trip");
    place_at(&dir, "arithmetic/arithmetic.adj", "arithmetic/arithmetic.adj");
    place_at(
        &dir,
        "mathematics/place-value.adj",
        "mathematics/place-value.adj",
    );
    std::fs::write(
        dir.join("case.adj"),
        "import \"mathematics/place-value.adj\"\n\
         observe n(83)\n\
         ? tens_digit(n)\n\
         ? ones_digit(n)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    assert!(
        s.contains("\"name\":\"ones_digit\"") && s.contains("\"value\":3"),
        "ones_digit(83) = 3 (the second query wins the shared per-name derived slot): {s}"
    );

    // Independently: composing the digits 83 decomposes into recovers 83.
    std::fs::write(
        dir.join("recompose.adj"),
        "import \"mathematics/place-value.adj\"\n\
         observe tens(8)\n\
         observe ones(3)\n\
         ? tens_and_ones_to_number(tens, ones)\n",
    )
    .unwrap();
    let (ok2, s2) = run(&dir.join("recompose.adj"));
    assert!(ok2, "CLI exited non-zero: {s2}");
    assert!(
        s2.contains("\"name\":\"tens_and_ones_to_number\"") && s2.contains("\"value\":83"),
        "tens_and_ones_to_number(8, 3) = 83, recovering the original n: {s2}"
    );
}
