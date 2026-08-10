//! End-to-end tests for `physics/mechanics-laws.adj` — the forward Newton's
//! second law (`force`), plus this session's two new rung-0 CAS-wiring
//! companions (ADJ-FORMULA-LIBRARIES FL-10, §3D): `mass_from_force` and
//! `acceleration_from_force`, solving the SAME cited NASA `F = m a` equation
//! as the forward `force` formula for a different unknown. Driven through the
//! built CLI binary against the SHIPPED stdlib.

use std::path::{Path, PathBuf};
use std::process::Command;

fn shipped_mechanics_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/physics/mechanics-laws.adj")
        .canonicalize()
        .expect("shipped mechanics-laws.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_mechanics_force_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_mechanics_lib()).unwrap();
    std::fs::write(dir.join("mechanics-laws.adj"), lib).unwrap();
}

#[test]
fn imports_mechanics_library_and_computes_force_forward() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"mechanics-laws.adj\"\n\
         observe mass(2)\n\
         observe acceleration(3)\n\
         ? force(mass, acceleration)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    assert!(
        s.contains("\"name\":\"force\"") && s.contains("\"value\":6"),
        "force(2, 3) = 6: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("grc.nasa.gov"),
        "force carries its NASA provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// F = m a, solved for a different unknown (ADJ-FORMULA-LIBRARIES FL-10, §3D
// rung-0 CAS-wiring companions) — the SAME cited NASA equation as `force`
// above, rearranged rather than computed forward.
// ---------------------------------------------------------------------------

#[test]
fn solves_for_mass_from_force_with_the_same_citation() {
    let dir = scratch("mass_solve");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"mechanics-laws.adj\"\n\
         observe force(12)\n\
         observe acceleration(3)\n\
         ? mass_from_force(force, acceleration)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 12 = m * 3  =>  m = 4.
    assert!(
        s.contains("\"name\":\"mass_from_force\"") && s.contains("\"value\":4"),
        "mass_from_force(12, 3) = 4: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("grc.nasa.gov"),
        "carries the same NASA citation as the forward force formula: {s}"
    );
}

#[test]
fn solves_for_acceleration_from_force_with_the_same_citation() {
    let dir = scratch("acceleration_solve");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"mechanics-laws.adj\"\n\
         observe force(12)\n\
         observe mass(4)\n\
         ? acceleration_from_force(force, mass)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 12 = 4 * a  =>  a = 3.
    assert!(
        s.contains("\"name\":\"acceleration_from_force\"") && s.contains("\"value\":3"),
        "acceleration_from_force(12, 4) = 3: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("grc.nasa.gov"),
        "carries the same NASA citation as the forward force formula: {s}"
    );
}
