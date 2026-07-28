//! End-to-end tests for the `clinical/total-lung-capacity.adj` library — the definition of the
//! total lung capacity (total lung capacity = vital capacity + residual volume) and its two
//! exact rearrangements (vital capacity = TLC − RV, residual volume = TLC − VC) — driven
//! through the built CLI binary against the SHIPPED stdlib. Each proves the same invariant as
//! the other formula libraries: a consumer states NO arithmetic; it imports the grounded
//! library, binds the measured quantities with `observe`, and the engine applies the cited
//! relation on the CPU, computing the EXACT value and rendering the relation's citation and
//! trust tier in the `derived` section (the auditable answer). The three formulas INVERT around
//! the worked case VC = 4 L, RV = 2 L: 4 + 2 = 6, and both 6 − 2 = 4 and 6 − 4 = 2 recover the
//! inputs.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped total-lung-capacity library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_tlc_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/total-lung-capacity.adj")
        .canonicalize()
        .expect("shipped total-lung-capacity.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_tlc_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_tlc_lib()).unwrap();
    std::fs::write(dir.join("total-lung-capacity.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// total_lung_capacity — the definition: the vital capacity plus the residual volume.
// ---------------------------------------------------------------------------

#[test]
fn imports_tlc_library_and_computes_total_lung_capacity_with_citation() {
    let dir = scratch("tlc");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"total-lung-capacity.adj\"\n\
         observe vital_capacity(4)\n\
         observe residual_volume(2)\n\
         ? total_lung_capacity(vital_capacity, residual_volume)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied relation's result: 4 + 2 = 6.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"total_lung_capacity\"") && s.contains("\"value\":6"),
        "total_lung_capacity(4, 2) = 6: {s}"
    );
    // … AND the StatPearls/NCBI Bookshelf citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// vital_capacity — the same relation solved for VC: TLC − RV, which INVERTS the total lung
// capacity just produced.
// ---------------------------------------------------------------------------

#[test]
fn computes_vital_capacity_from_tlc_and_rv_with_citation() {
    let dir = scratch("vc");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"total-lung-capacity.adj\"\n\
         observe total_lung_capacity(6)\n\
         observe residual_volume(2)\n\
         ? vital_capacity(total_lung_capacity, residual_volume)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 6 - 2 = 4, computed on the CPU.
    assert!(
        s.contains("\"name\":\"vital_capacity\"") && s.contains("\"value\":4"),
        "vital_capacity(6, 2) = 4: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "vital_capacity carries its StatPearls citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// residual_volume — the same relation solved for RV: TLC − VC, the third exact reading of the
// one definition.
// ---------------------------------------------------------------------------

#[test]
fn computes_residual_volume_from_tlc_and_vc_with_citation() {
    let dir = scratch("rv");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"total-lung-capacity.adj\"\n\
         observe total_lung_capacity(6)\n\
         observe vital_capacity(4)\n\
         ? residual_volume(total_lung_capacity, vital_capacity)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 6 - 4 = 2, computed on the CPU.
    assert!(
        s.contains("\"name\":\"residual_volume\"") && s.contains("\"value\":2"),
        "residual_volume(6, 4) = 2: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "residual_volume carries its StatPearls citation: {s}"
    );
}
