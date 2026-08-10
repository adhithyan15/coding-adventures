//! End-to-end tests for the `clinical/bmi.adj` library — the WHO body-mass-index
//! definition (BMI = mass/height²), plus this session's rung-0 CAS-wiring
//! companion (ADJ-FORMULA-LIBRARIES FL-10, §3D): `body_mass_from_bmi`, solving
//! the SAME cited WHO definition for the body mass instead of computed forward
//! for BMI. Driven through the built CLI binary against the SHIPPED stdlib.

use std::path::{Path, PathBuf};
use std::process::Command;

fn shipped_bmi_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/bmi.adj")
        .canonicalize()
        .expect("shipped clinical/bmi.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_bmi_{tag}_{}", std::process::id()));
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

fn place_lib(dir: &Path) {
    let lib = std::fs::read_to_string(shipped_bmi_lib()).unwrap();
    std::fs::write(dir.join("bmi.adj"), lib).unwrap();
}

#[test]
fn imports_bmi_library_and_computes_forward_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"bmi.adj\"\n\
         observe body_mass(70)\n\
         observe height(1.75)\n\
         ? bmi(body_mass, height)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    assert!(
        s.contains("\"name\":\"bmi\"") && s.contains("\"value\":22.857142857142858"),
        "bmi(70, 1.75) = 22.857142857142858: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("who.int"),
        "bmi carries its WHO provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// BMI = mass/height², solved for a different unknown (ADJ-FORMULA-LIBRARIES
// FL-10, §3D rung-0 CAS-wiring companion) — the SAME cited WHO definition as
// `bmi` above, rearranged rather than computed forward.
// ---------------------------------------------------------------------------

#[test]
fn solves_for_body_mass_from_bmi_with_the_same_citation() {
    let dir = scratch("mass_solve");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"bmi.adj\"\n\
         observe bmi(17.5)\n\
         observe height(2)\n\
         ? body_mass_from_bmi(bmi, height)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 17.5 = m / 4  =>  m = 70.
    assert!(
        s.contains("\"name\":\"body_mass_from_bmi\"") && s.contains("\"value\":70"),
        "body_mass_from_bmi(17.5, 2) = 70: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("who.int"),
        "carries the same WHO citation as the forward bmi formula: {s}"
    );
}
