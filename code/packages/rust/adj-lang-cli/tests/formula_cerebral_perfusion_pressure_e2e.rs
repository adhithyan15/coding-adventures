//! End-to-end tests for the `clinical/cerebral-perfusion-pressure.adj` library — the
//! definition of cerebral perfusion pressure (CPP = mean arterial pressure − intracranial
//! pressure) and its two exact rearrangements (MAP = CPP + ICP, ICP = MAP − CPP) — driven
//! through the built CLI binary against the SHIPPED stdlib. Each proves the same invariant as
//! the other formula libraries: a consumer states NO arithmetic; it imports the grounded
//! library, binds the measured pressures with `observe`, and the engine applies the cited
//! relation on the CPU, computing the EXACT value and rendering the relation's citation and
//! trust tier in the `derived` section (the auditable answer). The three formulas INVERT
//! around the worked case MAP = 90 mmHg, ICP = 10 mmHg: 90 − 10 = 80, and both 80 + 10 = 90
//! and 90 − 80 = 10 recover the inputs.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped CPP library, resolved from this crate's manifest dir so the
/// test is location-independent.
fn shipped_cpp_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/cerebral-perfusion-pressure.adj")
        .canonicalize()
        .expect("shipped cerebral-perfusion-pressure.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_cpp_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_cpp_lib()).unwrap();
    std::fs::write(dir.join("cerebral-perfusion-pressure.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// cerebral_perfusion_pressure — the definition: the mean arterial pressure less the
// intracranial pressure.
// ---------------------------------------------------------------------------

#[test]
fn imports_cpp_library_and_computes_cpp_with_citation() {
    let dir = scratch("cpp");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"cerebral-perfusion-pressure.adj\"\n\
         observe mean_arterial_pressure(90)\n\
         observe intracranial_pressure(10)\n\
         ? cerebral_perfusion_pressure(mean_arterial_pressure, intracranial_pressure)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied relation's result: 90 - 10 = 80.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"cerebral_perfusion_pressure\"") && s.contains("\"value\":80"),
        "cerebral_perfusion_pressure(90, 10) = 80: {s}"
    );
    // … AND the StatPearls/NCBI Bookshelf citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// mean_arterial_pressure — the same relation solved for MAP: CPP + ICP, which INVERTS the CPP
// just produced.
// ---------------------------------------------------------------------------

#[test]
fn computes_map_from_cpp_and_icp_with_citation() {
    let dir = scratch("map");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"cerebral-perfusion-pressure.adj\"\n\
         observe cerebral_perfusion_pressure(80)\n\
         observe intracranial_pressure(10)\n\
         ? mean_arterial_pressure(cerebral_perfusion_pressure, intracranial_pressure)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 80 + 10 = 90, computed on the CPU.
    assert!(
        s.contains("\"name\":\"mean_arterial_pressure\"") && s.contains("\"value\":90"),
        "mean_arterial_pressure(80, 10) = 90: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "mean_arterial_pressure carries its StatPearls citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// intracranial_pressure — the same relation solved for ICP: MAP − CPP, the third exact
// reading of the one definition.
// ---------------------------------------------------------------------------

#[test]
fn computes_icp_from_map_and_cpp_with_citation() {
    let dir = scratch("icp");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"cerebral-perfusion-pressure.adj\"\n\
         observe mean_arterial_pressure(90)\n\
         observe cerebral_perfusion_pressure(80)\n\
         ? intracranial_pressure(mean_arterial_pressure, cerebral_perfusion_pressure)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 90 - 80 = 10, computed on the CPU.
    assert!(
        s.contains("\"name\":\"intracranial_pressure\"") && s.contains("\"value\":10"),
        "intracranial_pressure(90, 80) = 10: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "intracranial_pressure carries its StatPearls citation: {s}"
    );
}
