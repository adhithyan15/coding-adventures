//! End-to-end test for `mathematics/word-problems.adj` (CCSS 1.OA.A.1's TAKE
//! FROM and COMPARE situation types), driven through the built CLI binary
//! against the SHIPPED stdlib. Both formulas compose `arithmetic.adj`'s
//! `difference` (a cross-directory import, like `mathematics/cardinality.adj`
//! and `mathematics/number-sequence.adj`).

use std::path::{Path, PathBuf};
use std::process::Command;

fn stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib")
        .canonicalize()
        .expect("shipped adj-formula-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_wordproblems_{tag}_{}", std::process::id()));
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

fn place_at(dir: &Path, src_rel: &str, dst_rel: &str) {
    let src = stdlib().join(src_rel);
    let dst = dir.join(dst_rel);
    std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
    std::fs::copy(&src, &dst).unwrap_or_else(|e| panic!("copy {src_rel} -> {dst_rel}: {e}"));
}

#[test]
fn separate_result_take_from_composes_arithmetic_difference() {
    let dir = scratch("separate");
    place_at(&dir, "arithmetic/arithmetic.adj", "arithmetic/arithmetic.adj");
    place_at(
        &dir,
        "mathematics/word-problems.adj",
        "mathematics/word-problems.adj",
    );
    std::fs::write(
        dir.join("case.adj"),
        "import \"mathematics/word-problems.adj\"\n\
         observe start_amount(8)\n\
         observe change_amount(3)\n\
         ? separate_result(start_amount, change_amount)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 8 - 3 = 5.
    assert!(
        s.contains("\"name\":\"separate_result\"") && s.contains("\"value\":5"),
        "separate_result(8, 3) = 5: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"")
            && s.contains("mathworld.wolfram.com/Subtraction.html"),
        "carries the MathWorld Subtraction citation: {s}"
    );
    // The composed `difference` primitive's own citation corroborates.
    assert!(
        s.contains("mathworld.wolfram.com/Difference.html"),
        "corroborated by the composed arithmetic.adj difference primitive's citation: {s}"
    );
}

#[test]
fn compare_difference_composes_arithmetic_difference() {
    let dir = scratch("compare");
    place_at(&dir, "arithmetic/arithmetic.adj", "arithmetic/arithmetic.adj");
    place_at(
        &dir,
        "mathematics/word-problems.adj",
        "mathematics/word-problems.adj",
    );
    std::fs::write(
        dir.join("case.adj"),
        "import \"mathematics/word-problems.adj\"\n\
         observe greater_amount(7)\n\
         observe lesser_amount(4)\n\
         ? compare_difference(greater_amount, lesser_amount)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 7 - 4 = 3.
    assert!(
        s.contains("\"name\":\"compare_difference\"") && s.contains("\"value\":3"),
        "compare_difference(7, 4) = 3: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"")
            && s.contains("mathworld.wolfram.com/Difference.html"),
        "carries the MathWorld Difference citation: {s}"
    );
}
