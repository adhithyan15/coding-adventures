//! End-to-end tests for the `chemistry/molarity.adj` library — the definition of
//! molarity (M = n/V) and its two exact rearrangements (n = M·V, V = n/M) — driven
//! through the built CLI binary against the SHIPPED stdlib. Each proves the same
//! invariant as the other formula libraries: a consumer states NO arithmetic; it
//! imports the grounded library, binds the measured quantities with `observe`, and
//! the engine applies the cited definition on the CPU — computing the EXACT value
//! and rendering the definition's citation + trust tier in the `derived` section
//! (the auditable answer). The three formulas INVERT: 6 / 2 = 3, 3 * 2 = 6, 6 / 3 = 2.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped molarity library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_molarity_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/chemistry/molarity.adj")
        .canonicalize()
        .expect("shipped molarity.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_molar_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_molarity_lib()).unwrap();
    std::fs::write(dir.join("molarity.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// molarity — the definition: the moles of solute divided by the volume.
// ---------------------------------------------------------------------------

#[test]
fn imports_molarity_library_and_computes_definition_with_citation() {
    let dir = scratch("def");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"molarity.adj\"\n\
         observe moles(6)\n\
         observe volume(2)\n\
         ? molarity(moles, volume)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied definition's result: 6 / 2 = 3.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"molarity\"") && s.contains("\"value\":3"),
        "molarity(6, 2) = 3: {s}"
    );
    // … AND the LibreTexts citation + trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("chem.libretexts.org"),
        "applied definition carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// moles — the same definition solved for n: the molarity times the volume, which
// INVERTS the molarity just produced.
// ---------------------------------------------------------------------------

#[test]
fn computes_moles_as_molarity_times_volume_with_citation() {
    let dir = scratch("moles");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"molarity.adj\"\n\
         observe molarity(3)\n\
         observe volume(2)\n\
         ? moles(molarity, volume)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 3 * 2 = 6, computed on the CPU.
    assert!(
        s.contains("\"name\":\"moles\"") && s.contains("\"value\":6"),
        "moles(3, 2) = 6: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("chem.libretexts.org"),
        "moles carries its LibreTexts citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// volume — the same definition solved for V: the moles divided by the molarity,
// the third exact reading of the one definition.
// ---------------------------------------------------------------------------

#[test]
fn computes_volume_as_moles_over_molarity_with_citation() {
    let dir = scratch("vol");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"molarity.adj\"\n\
         observe moles(6)\n\
         observe molarity(3)\n\
         ? volume(moles, molarity)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 6 / 3 = 2, computed on the CPU.
    assert!(
        s.contains("\"name\":\"volume\"") && s.contains("\"value\":2"),
        "volume(6, 3) = 2: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("chem.libretexts.org"),
        "volume carries its LibreTexts citation: {s}"
    );
}
