//! End-to-end tests for the `clinical/fick-principle.adj` library — the Fick principle
//! for cardiac output (CO = VO2 / (CaO2 - CvO2)) and its two exact rearrangements
//! (VO2 = CO * (CaO2 - CvO2), CvO2 = CaO2 - VO2 / CO) — driven through the built CLI
//! binary against the SHIPPED stdlib. Each proves the same invariant as the other formula
//! libraries: a consumer states NO arithmetic; it imports the grounded library, binds the
//! measured quantities with `observe`, and the engine applies the cited relation on the
//! CPU, computing the EXACT value and rendering the relation's citation and trust tier in
//! the `derived` section (the auditable answer). The three formulas INVERT around the
//! worked case VO2 = 250 mL/min, CaO2 = 200 mL O2/L, CvO2 = 150 mL O2/L: 250 / (200 - 150)
//! = 5, and both 5 * (200 - 150) = 250 and 200 - 250 / 5 = 150 recover the inputs.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped fick-principle library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_fick_principle_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/fick-principle.adj")
        .canonicalize()
        .expect("shipped fick-principle.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_fick_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_fick_principle_lib()).unwrap();
    std::fs::write(dir.join("fick-principle.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// cardiac_output — the Fick relation: the oxygen consumption over the arteriovenous
// oxygen-content difference.
// ---------------------------------------------------------------------------

#[test]
fn imports_fick_library_and_computes_cardiac_output_with_citation() {
    let dir = scratch("co");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fick-principle.adj\"\n\
         observe oxygen_consumption(250)\n\
         observe arterial_oxygen_content(200)\n\
         observe venous_oxygen_content(150)\n\
         ? cardiac_output(oxygen_consumption, arterial_oxygen_content, venous_oxygen_content)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied relation's result:
    // 250 / (200 - 150) = 5.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"cardiac_output\"") && s.contains("\"value\":5"),
        "cardiac_output(250, 200, 150) = 5: {s}"
    );
    // … AND the StatPearls/NCBI Bookshelf citation and trust tier, so the answer is
    // auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// oxygen_consumption — the same relation solved for VO2: CO * (CaO2 - CvO2), which INVERTS
// the cardiac output just produced.
// ---------------------------------------------------------------------------

#[test]
fn computes_oxygen_consumption_from_cardiac_output_and_the_contents_with_citation() {
    let dir = scratch("vo2");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fick-principle.adj\"\n\
         observe cardiac_output(5)\n\
         observe arterial_oxygen_content(200)\n\
         observe venous_oxygen_content(150)\n\
         ? oxygen_consumption(cardiac_output, arterial_oxygen_content, venous_oxygen_content)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 5 * (200 - 150) = 250, computed on the CPU.
    assert!(
        s.contains("\"name\":\"oxygen_consumption\"") && s.contains("\"value\":250"),
        "oxygen_consumption(5, 200, 150) = 250: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "oxygen_consumption carries its StatPearls citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// venous_oxygen_content — the same relation solved for CvO2: CaO2 - VO2 / CO, the third
// exact reading of the one relation.
// ---------------------------------------------------------------------------

#[test]
fn computes_venous_oxygen_content_from_the_other_three_with_citation() {
    let dir = scratch("cvo2");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fick-principle.adj\"\n\
         observe arterial_oxygen_content(200)\n\
         observe oxygen_consumption(250)\n\
         observe cardiac_output(5)\n\
         ? venous_oxygen_content(arterial_oxygen_content, oxygen_consumption, cardiac_output)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 200 - 250 / 5 = 200 - 50 = 150, computed on the CPU.
    assert!(
        s.contains("\"name\":\"venous_oxygen_content\"") && s.contains("\"value\":150"),
        "venous_oxygen_content(200, 250, 5) = 150: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "venous_oxygen_content carries its StatPearls citation: {s}"
    );
}
