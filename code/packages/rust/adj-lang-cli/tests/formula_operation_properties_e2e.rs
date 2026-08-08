//! End-to-end test for the K-8 "operation properties" gap
//! (ADJ-STDLIB-COVERAGE.md 5.1): `mathematics/operation-properties.adj`'s
//! `addition_is_commutative`/`subtraction_is_commutative`/
//! `addition_is_associative` formulas — verified via FL-8's comparison-
//! formula shape (`==`) rather than asserted — driven through the built CLI
//! binary against the SHIPPED stdlib. Imports `arithmetic.adj` across a
//! directory boundary, so the entry program sits at the stdlib root, matching
//! `cardinality.adj`/`word-problems.adj`'s established pattern.

use std::path::{Path, PathBuf};
use std::process::Command;

fn stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib")
        .canonicalize()
        .expect("shipped adj-formula-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "adjcli_operation_properties_{tag}_{}",
        std::process::id()
    ));
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

fn place_stdlib(dir: &Path) {
    for rel in [
        "arithmetic/arithmetic.adj",
        "mathematics/operation-properties.adj",
    ] {
        let src = stdlib().join(rel);
        let dst = dir.join(rel);
        std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
        std::fs::copy(&src, &dst).unwrap_or_else(|e| panic!("copy {rel}: {e}"));
    }
}

#[test]
fn addition_is_commutative_computes_true_and_carries_the_mathworld_citation() {
    let dir = scratch("addition_commutes");
    place_stdlib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"mathematics/operation-properties.adj\"\n\
         observe addend_one(4)\n\
         observe addend_two(7)\n\
         ? addition_is_commutative(addend_one, addend_two)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    assert!(
        s.contains("\"name\":\"addition_is_commutative\"") && s.contains("\"value\":1"),
        "4+7 == 7+4 is true: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"")
            && s.contains("mathworld.wolfram.com/Commutative.html"),
        "carries the MathWorld commutativity citation: {s}"
    );
}

#[test]
fn subtraction_is_commutative_computes_false_when_operands_differ() {
    let dir = scratch("subtraction_commutes");
    place_stdlib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"mathematics/operation-properties.adj\"\n\
         observe minuend(10)\n\
         observe subtrahend(3)\n\
         ? subtraction_is_commutative(minuend, subtrahend)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    assert!(
        s.contains("\"name\":\"subtraction_is_commutative\"") && s.contains("\"value\":0"),
        "10-3 != 3-10, so this is false: {s}"
    );
}

#[test]
fn addition_is_associative_computes_true_and_carries_the_mathworld_citation() {
    let dir = scratch("addition_associates");
    place_stdlib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"mathematics/operation-properties.adj\"\n\
         observe addend_one(2)\n\
         observe addend_two(5)\n\
         observe addend_three(6)\n\
         ? addition_is_associative(addend_one, addend_two, addend_three)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    assert!(
        s.contains("\"name\":\"addition_is_associative\"") && s.contains("\"value\":1"),
        "(2+5)+6 == 2+(5+6) is true: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"")
            && s.contains("mathworld.wolfram.com/Associative.html"),
        "carries the MathWorld associativity citation: {s}"
    );
}
