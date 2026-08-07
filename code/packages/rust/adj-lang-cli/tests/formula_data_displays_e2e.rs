//! End-to-end test for the K-8 "data displays" gap (ADJ-STDLIB-COVERAGE.md
//! 5.1): `mathematics/data-displays.adj`'s `range_two`/`range_three`
//! formulas — the first content library built on FL-11's dyadic `min`/`max`
//! runtime built-ins — driven through the built CLI binary against the
//! SHIPPED stdlib. Self-contained (no cross-directory `import`).

use std::path::{Path, PathBuf};
use std::process::Command;

fn stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib")
        .canonicalize()
        .expect("shipped adj-formula-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_data_displays_{tag}_{}", std::process::id()));
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
fn range_two_computes_and_carries_the_mathworld_citation() {
    let dir = scratch("range_two");
    place_at(
        &dir,
        "mathematics/data-displays.adj",
        "data-displays.adj",
    );
    std::fs::write(
        dir.join("case.adj"),
        "import \"data-displays.adj\"\n\
         observe value_one(3)\n\
         observe value_two(9)\n\
         ? range_two(value_one, value_two)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // max(3, 9) - min(3, 9) = 9 - 3 = 6.
    assert!(
        s.contains("\"name\":\"range_two\"") && s.contains("\"value\":6"),
        "range_two(3, 9) = 6: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"")
            && s.contains("mathworld.wolfram.com/StatisticalRange.html"),
        "carries the MathWorld statistical-range citation: {s}"
    );
}

#[test]
fn range_three_computes_and_carries_the_mathworld_citation() {
    let dir = scratch("range_three");
    place_at(
        &dir,
        "mathematics/data-displays.adj",
        "data-displays.adj",
    );
    std::fs::write(
        dir.join("case.adj"),
        "import \"data-displays.adj\"\n\
         observe value_one(5)\n\
         observe value_two(12)\n\
         observe value_three(8)\n\
         ? range_three(value_one, value_two, value_three)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // max(5, 12, 8) - min(5, 12, 8) = 12 - 5 = 7.
    assert!(
        s.contains("\"name\":\"range_three\"") && s.contains("\"value\":7"),
        "range_three(5, 12, 8) = 7: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"")
            && s.contains("mathworld.wolfram.com/StatisticalRange.html"),
        "carries the MathWorld statistical-range citation: {s}"
    );
}
