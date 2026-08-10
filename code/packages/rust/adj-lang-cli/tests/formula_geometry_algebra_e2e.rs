//! End-to-end tests for `mathematics/geometry-formulas.adj`'s four rung-0
//! CAS-wiring companions (ADJ-FORMULA-LIBRARIES FL-10, §3D — this session's
//! Wave 3 opener): `width_from_rectangle_area`, `height_from_triangle_area`,
//! `side_from_square_perimeter`, `width_from_rectangle_perimeter`. Each is
//! the SAME cited equation as its forward `formula` sibling, solved for a
//! different unknown through `cas-solve`'s real linear-equation solver —
//! driven through the built CLI binary against the SHIPPED stdlib, run in
//! its own isolated scratch program per test (a `symbolic`'s target may not
//! already be observed elsewhere in the same program, and two of these four
//! share a target name — `width` — so they can never coexist in one
//! compiled unit; see `geometry-formulas-solve.query.adj`'s header).

use std::path::{Path, PathBuf};
use std::process::Command;

fn stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib")
        .canonicalize()
        .expect("shipped adj-formula-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_geom_algebra_{tag}_{}", std::process::id()));
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

fn place_geometry_lib(dir: &Path) {
    let src = stdlib().join("mathematics/geometry-formulas.adj");
    let dst = dir.join("geometry-formulas.adj");
    std::fs::copy(&src, &dst).unwrap_or_else(|e| panic!("copy geometry-formulas.adj: {e}"));
}

#[test]
fn width_from_rectangle_area_solves_and_carries_the_mathworld_citation() {
    let dir = scratch("width_from_area");
    place_geometry_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"geometry-formulas.adj\"\n\
         observe rectangle_area(48)\n\
         observe length(8)\n\
         ? width_from_rectangle_area(rectangle_area, length)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 48 = 8 * width  =>  width = 6.
    assert!(
        s.contains("\"name\":\"width_from_rectangle_area\"") && s.contains("\"value\":6"),
        "width_from_rectangle_area(48, 8) = 6: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("mathworld.wolfram.com/Rectangle.html"),
        "carries the same MathWorld rectangle-area citation as the forward formula: {s}"
    );
}

#[test]
fn height_from_triangle_area_solves_through_the_division_rewrite() {
    let dir = scratch("height_from_area");
    place_geometry_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"geometry-formulas.adj\"\n\
         observe triangle_area(15)\n\
         observe base(10)\n\
         ? height_from_triangle_area(triangle_area, base)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 15 = 10 * height / 2  =>  height = 3.
    assert!(
        s.contains("\"name\":\"height_from_triangle_area\"") && s.contains("\"value\":3"),
        "height_from_triangle_area(15, 10) = 3: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("mathworld.wolfram.com/Triangle.html"),
        "carries the same MathWorld triangle-area citation as the forward formula: {s}"
    );
}

#[test]
fn side_from_square_perimeter_solves_and_carries_the_mathworld_citation() {
    let dir = scratch("side_from_perimeter");
    place_geometry_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"geometry-formulas.adj\"\n\
         observe square_perimeter(20)\n\
         ? side_from_square_perimeter(square_perimeter)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 20 = 4 * side  =>  side = 5.
    assert!(
        s.contains("\"name\":\"side_from_square_perimeter\"") && s.contains("\"value\":5"),
        "side_from_square_perimeter(20) = 5: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("mathworld.wolfram.com/Perimeter.html"),
        "carries the same MathWorld perimeter-table citation as the forward formula: {s}"
    );
}

#[test]
fn width_from_rectangle_perimeter_solves_through_the_nested_sum() {
    let dir = scratch("width_from_perimeter");
    place_geometry_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"geometry-formulas.adj\"\n\
         observe rectangle_perimeter(30)\n\
         observe length(9)\n\
         ? width_from_rectangle_perimeter(rectangle_perimeter, length)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 30 = 2 * (9 + width)  =>  width = 6.
    assert!(
        s.contains("\"name\":\"width_from_rectangle_perimeter\"") && s.contains("\"value\":6"),
        "width_from_rectangle_perimeter(30, 9) = 6: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("mathworld.wolfram.com/Perimeter.html"),
        "carries the same MathWorld perimeter-table citation as the forward formula: {s}"
    );
}

#[test]
fn square_area_inverse_is_a_clean_nonlinear_error_not_a_crash() {
    // Documents the rung-0 boundary: side*side is quadratic in the target, so
    // solving for `side` from `square_area` is a clean typed compile error,
    // not a formula this library ships. Confirms the boundary stays correct
    // if `cas-solve`'s behavior ever changes underneath this content.
    let dir = scratch("square_area_nonlinear");
    std::fs::write(
        dir.join("case.adj"),
        "formulabook probe {\n\
         \x20\x20\x20\x20symbolic side_from_square_area(square_area) { square_area == side * side } for side\n\
         \x20\x20\x20\x20\x20\x20\x20\x20source \"test\" trust consensus\n\
         }\n\
         observe square_area(49)\n\
         ? side_from_square_area(square_area)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(!ok, "a quadratic target must fail: {s}");
    assert!(
        s.contains("SymbolicNonLinear"),
        "fails with the typed SymbolicNonLinear error, not a crash: {s}"
    );
}
