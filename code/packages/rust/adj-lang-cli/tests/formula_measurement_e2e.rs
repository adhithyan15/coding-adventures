//! End-to-end test for the K-8 measurement gap (ADJ-STDLIB-COVERAGE.md 5.1):
//! `mathematics/geometry-formulas.adj`'s new `square_perimeter` formula and
//! the new `mathematics/volume-formulas.adj` library (`cube_volume`,
//! `rectangular_prism_volume`), driven through the built CLI binary against
//! the SHIPPED stdlib. Both libraries are self-contained (no cross-directory
//! `import`), unlike `place-value.adj`'s dependency on `arithmetic.adj`.

use std::path::{Path, PathBuf};
use std::process::Command;

fn stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib")
        .canonicalize()
        .expect("shipped adj-formula-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_measurement_{tag}_{}", std::process::id()));
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
fn square_perimeter_computes_and_carries_the_mathworld_citation() {
    let dir = scratch("square_perimeter");
    place_at(
        &dir,
        "mathematics/geometry-formulas.adj",
        "geometry-formulas.adj",
    );
    std::fs::write(
        dir.join("case.adj"),
        "import \"geometry-formulas.adj\"\n\
         observe side(5)\n\
         ? square_perimeter(side)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 4 * 5 = 20.
    assert!(
        s.contains("\"name\":\"square_perimeter\"") && s.contains("\"value\":20"),
        "square_perimeter(5) = 20: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"")
            && s.contains("mathworld.wolfram.com/Perimeter.html"),
        "carries the MathWorld perimeter-table citation: {s}"
    );
}

#[test]
fn cube_volume_computes_and_carries_the_mathworld_citation() {
    let dir = scratch("cube_volume");
    place_at(
        &dir,
        "mathematics/volume-formulas.adj",
        "volume-formulas.adj",
    );
    std::fs::write(
        dir.join("case.adj"),
        "import \"volume-formulas.adj\"\n\
         observe side(3)\n\
         ? cube_volume(side)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 3 * 3 * 3 = 27.
    assert!(
        s.contains("\"name\":\"cube_volume\"") && s.contains("\"value\":27"),
        "cube_volume(3) = 27: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("mathworld.wolfram.com/Cube.html"),
        "carries the MathWorld cube-volume citation: {s}"
    );
}

#[test]
fn rectangular_prism_volume_computes_and_carries_the_mathworld_citation() {
    let dir = scratch("prism_volume");
    place_at(
        &dir,
        "mathematics/volume-formulas.adj",
        "volume-formulas.adj",
    );
    std::fs::write(
        dir.join("case.adj"),
        "import \"volume-formulas.adj\"\n\
         observe box_length(4)\n\
         observe box_width(3)\n\
         observe box_height(2)\n\
         ? rectangular_prism_volume(box_length, box_width, box_height)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 4 * 3 * 2 = 24.
    assert!(
        s.contains("\"name\":\"rectangular_prism_volume\"") && s.contains("\"value\":24"),
        "rectangular_prism_volume(4, 3, 2) = 24: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"")
            && s.contains("mathworld.wolfram.com/Cuboid.html"),
        "carries the MathWorld cuboid-volume citation: {s}"
    );
}
