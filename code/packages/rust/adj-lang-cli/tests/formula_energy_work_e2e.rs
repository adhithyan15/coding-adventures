//! End-to-end tests for `physics/energy-work.adj` — the foundational work and
//! kinetic-energy laws, plus this session's two new rung-0 CAS-wiring
//! companions (ADJ-FORMULA-LIBRARIES FL-10, §3D): `force_from_work` and
//! `distance_from_work`, solving the SAME cited NASA `W = F d` equation as
//! the forward `work` formula for a different unknown. Driven through the
//! built CLI binary against the SHIPPED stdlib.

use std::path::{Path, PathBuf};
use std::process::Command;

fn shipped_energy_work_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/physics/energy-work.adj")
        .canonicalize()
        .expect("shipped energy-work.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_energy_work_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_energy_work_lib()).unwrap();
    std::fs::write(dir.join("energy-work.adj"), lib).unwrap();
}

#[test]
fn imports_energy_work_library_and_computes_both_laws_forward() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"energy-work.adj\"\n\
         observe force(10)\n\
         observe distance(3)\n\
         ? work(force, distance)\n\
         observe mass(2)\n\
         observe velocity(3)\n\
         ? kinetic_energy(mass, velocity)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    assert!(
        s.contains("\"name\":\"work\"") && s.contains("\"value\":30"),
        "work(10, 3) = 30: {s}"
    );
    assert!(
        s.contains("\"name\":\"kinetic_energy\"") && s.contains("\"value\":9"),
        "kinetic_energy(2, 3) = 9: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("grc.nasa.gov"),
        "work carries its NASA provenance: {s}"
    );
    assert!(
        s.contains("hyperphysics.gsu.edu"),
        "kinetic_energy carries its HyperPhysics provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// W = F d, solved for a different unknown (ADJ-FORMULA-LIBRARIES FL-10,
// §3D rung-0 CAS-wiring companions) — the SAME cited NASA equation as `work`
// above, rearranged rather than computed forward.
// ---------------------------------------------------------------------------

#[test]
fn solves_for_force_from_work_with_the_same_citation() {
    let dir = scratch("force_solve");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"energy-work.adj\"\n\
         observe work(30)\n\
         observe distance(3)\n\
         ? force_from_work(work, distance)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 30 = f * 3  =>  f = 10.
    assert!(
        s.contains("\"name\":\"force_from_work\"") && s.contains("\"value\":10"),
        "force_from_work(30, 3) = 10: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("grc.nasa.gov"),
        "carries the same NASA citation as the forward work formula: {s}"
    );
}

#[test]
fn solves_for_distance_from_work_with_the_same_citation() {
    let dir = scratch("distance_solve");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"energy-work.adj\"\n\
         observe work(30)\n\
         observe force(10)\n\
         ? distance_from_work(work, force)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 30 = 10 * d  =>  d = 3.
    assert!(
        s.contains("\"name\":\"distance_from_work\"") && s.contains("\"value\":3"),
        "distance_from_work(30, 10) = 3: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("grc.nasa.gov"),
        "carries the same NASA citation as the forward work formula: {s}"
    );
}
