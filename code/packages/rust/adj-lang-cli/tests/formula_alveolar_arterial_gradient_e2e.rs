//! End-to-end tests for the `clinical/alveolar-arterial-gradient.adj` library — the
//! definition of the alveolar–arterial oxygen gradient (A–a = alveolar PO2 − arterial PO2)
//! and its two exact rearrangements (alveolar PO2 = A–a + arterial PO2, arterial PO2 =
//! alveolar PO2 − A–a) — driven through the built CLI binary against the SHIPPED stdlib. Each
//! proves the same invariant as the other formula libraries: a consumer states NO arithmetic;
//! it imports the grounded library, binds the measured partial pressures with `observe`, and
//! the engine applies the cited relation on the CPU, computing the EXACT value and rendering
//! the relation's citation and trust tier in the `derived` section (the auditable answer). The
//! three formulas INVERT around the worked case PAO2 = 100 mmHg, PaO2 = 90 mmHg: 100 − 90 =
//! 10, and both 10 + 90 = 100 and 100 − 10 = 90 recover the inputs.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped A–a gradient library, resolved from this crate's manifest dir
/// so the test is location-independent.
fn shipped_aa_gradient_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/alveolar-arterial-gradient.adj")
        .canonicalize()
        .expect("shipped alveolar-arterial-gradient.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_aag_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_aa_gradient_lib()).unwrap();
    std::fs::write(dir.join("alveolar-arterial-gradient.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// aa_gradient — the definition: the alveolar oxygen partial pressure less the arterial.
// ---------------------------------------------------------------------------

#[test]
fn imports_aa_gradient_library_and_computes_gradient_with_citation() {
    let dir = scratch("aag");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"alveolar-arterial-gradient.adj\"\n\
         observe alveolar_partial_pressure_o2(100)\n\
         observe arterial_partial_pressure_o2(90)\n\
         ? aa_gradient(alveolar_partial_pressure_o2, arterial_partial_pressure_o2)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied relation's result: 100 - 90 = 10.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"aa_gradient\"") && s.contains("\"value\":10"),
        "aa_gradient(100, 90) = 10: {s}"
    );
    // … AND the StatPearls/NCBI Bookshelf citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// alveolar_partial_pressure_o2 — the same relation solved for PAO2: A–a + PaO2, which INVERTS
// the gradient just produced.
// ---------------------------------------------------------------------------

#[test]
fn computes_alveolar_po2_from_gradient_and_arterial_po2_with_citation() {
    let dir = scratch("pao2");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"alveolar-arterial-gradient.adj\"\n\
         observe aa_gradient(10)\n\
         observe arterial_partial_pressure_o2(90)\n\
         ? alveolar_partial_pressure_o2(aa_gradient, arterial_partial_pressure_o2)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 10 + 90 = 100, computed on the CPU.
    assert!(
        s.contains("\"name\":\"alveolar_partial_pressure_o2\"") && s.contains("\"value\":100"),
        "alveolar_partial_pressure_o2(10, 90) = 100: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "alveolar_partial_pressure_o2 carries its StatPearls citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// arterial_partial_pressure_o2 — the same relation solved for PaO2: PAO2 − A–a, the third exact
// reading of the one definition.
// ---------------------------------------------------------------------------

#[test]
fn computes_arterial_po2_from_alveolar_po2_and_gradient_with_citation() {
    let dir = scratch("paco2");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"alveolar-arterial-gradient.adj\"\n\
         observe alveolar_partial_pressure_o2(100)\n\
         observe aa_gradient(10)\n\
         ? arterial_partial_pressure_o2(alveolar_partial_pressure_o2, aa_gradient)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 100 - 10 = 90, computed on the CPU.
    assert!(
        s.contains("\"name\":\"arterial_partial_pressure_o2\"") && s.contains("\"value\":90"),
        "arterial_partial_pressure_o2(100, 10) = 90: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "arterial_partial_pressure_o2 carries its StatPearls citation: {s}"
    );
}
