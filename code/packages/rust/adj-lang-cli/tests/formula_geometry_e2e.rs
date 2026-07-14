//! End-to-end tests for the `mathematics/geometry-formulas.adj` library — the
//! foundational plane-geometry area & perimeter formulas — driven through the
//! built CLI binary against the SHIPPED stdlib. Each proves the same invariant as
//! the other formula libraries: a consumer states NO arithmetic; it imports the
//! grounded library, binds side lengths with `observe`, and the engine applies the
//! cited formula on the CPU — computing the EXACT value and rendering the applied
//! formula's MathWorld citation + trust tier in the `derived` section (the
//! auditable answer).

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped geometry library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_geometry_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/mathematics/geometry-formulas.adj")
        .canonicalize()
        .expect("shipped geometry-formulas.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_geom_{tag}_{}", std::process::id()));
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

/// Copy the shipped library next to a consumer that imports it, so the CLI's
/// sandbox-checked relative import resolves.
fn place_lib(dir: &Path) {
    let lib = std::fs::read_to_string(shipped_geometry_lib()).unwrap();
    std::fs::write(dir.join("geometry-formulas.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// rectangle_area — the product of the two side lengths.
// ---------------------------------------------------------------------------

#[test]
fn imports_geometry_library_and_computes_rectangle_area_with_citation() {
    let dir = scratch("rectarea");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"geometry-formulas.adj\"\n\
         observe length(4)\n\
         observe width(3)\n\
         ? rectangle_area(length, width)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: 4 * 3 = 12.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"rectangle_area\"") && s.contains("\"value\":12"),
        "rectangle_area(4, 3) = 12: {s}"
    );
    // … AND the MathWorld citation + trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"")
            && s.contains("mathworld.wolfram.com/Rectangle.html"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// triangle_area — half the base times the height (a distinct formula & source).
// ---------------------------------------------------------------------------

#[test]
fn computes_triangle_area_as_half_base_times_height_with_citation() {
    let dir = scratch("triarea");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"geometry-formulas.adj\"\n\
         observe base(10)\n\
         observe height(3)\n\
         ? triangle_area(base, height)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 10 * 3 / 2 = 15, computed on the CPU.
    assert!(
        s.contains("\"name\":\"triangle_area\"") && s.contains("\"value\":15"),
        "triangle_area(10, 3) = 15: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"")
            && s.contains("mathworld.wolfram.com/Triangle.html"),
        "triangle area carries its MathWorld citation: {s}"
    );
}
