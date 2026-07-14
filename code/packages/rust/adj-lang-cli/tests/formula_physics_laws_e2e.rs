//! End-to-end test for the foundational physics-law library through the built
//! CLI binary: a consumer `import`s the SHIPPED `physics/mechanics-laws.adj`
//! formula library, binds the quantities from its own `observe`d facts, and
//! applies one of the cited laws (F = ma, V = IR, ρ = m/V, v = d/t). For each,
//! the CLI must compute the value on the CPU and render the applied formula's
//! citation in the `derived` section — the auditable answer, zero math by the
//! model.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped physics-law library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_physics_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/physics/mechanics-laws.adj")
        .canonicalize()
        .expect("shipped physics/mechanics-laws.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_physlaw_{tag}_{}", std::process::id()));
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

/// Run one law: copy the shipped library next to a consumer that binds the two
/// quantities and applies `call`, then assert the derived value AND that the
/// applied formula carries its cited provenance (trust tier + locator host).
#[allow(clippy::too_many_arguments)]
fn check_law(
    tag: &str,
    obs_one: &str,
    obs_two: &str,
    call: &str,
    name: &str,
    value: &str,
    trust: &str,
    locator_host: &str,
) {
    let dir = scratch(tag);
    let lib = std::fs::read_to_string(shipped_physics_lib()).unwrap();
    std::fs::write(dir.join("mechanics-laws.adj"), lib).unwrap();
    let consumer = format!(
        "import \"mechanics-laws.adj\"\n\
         observe {obs_one}\n\
         observe {obs_two}\n\
         ? {call}\n"
    );
    std::fs::write(dir.join("case.adj"), consumer).unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains(&format!("\"name\":\"{name}\"")) && s.contains(&format!("\"value\":{value}")),
        "{name} computed to {value}: {s}"
    );
    // The applied law carries its cited definition + trust tier + locator — auditable.
    assert!(
        s.contains(&format!("\"trust\":\"{trust}\"")) && s.contains(locator_host),
        "{name} carries its cited provenance ({trust} @ {locator_host}): {s}"
    );
}

/// Newton's second law — "a 2 kg block accelerates at 3 m/s²" → the model binds
/// mass and acceleration; the engine multiplies them → 6 N, carrying NASA's F=ma.
#[test]
fn newtons_second_law_binds_and_computes_with_citation() {
    check_law(
        "force",
        "mass(2)",
        "acceleration(3)",
        "force(mass, acceleration)",
        "force",
        "6",
        "authoritative",
        "grc.nasa.gov",
    );
}

/// Ohm's law — 4 A through a 5 Ω resistor → the engine multiplies → 20 V. The
/// clean V=IR product form is quoted from an encyclopedia, so this law is
/// `consensus`-tier and cites the Wikipedia locator.
#[test]
fn ohms_law_binds_and_computes_with_citation() {
    check_law(
        "voltage",
        "current(4)",
        "resistance(5)",
        "voltage(current, resistance)",
        "voltage",
        "20",
        "consensus",
        "en.wikipedia.org",
    );
}

/// Density — 12 kg in 4 m³ → the engine divides → 3 kg/m³, carrying NASA Glenn.
#[test]
fn density_binds_and_computes_with_citation() {
    check_law(
        "density",
        "mass(12)",
        "volume(4)",
        "density(mass, volume)",
        "density",
        "3",
        "authoritative",
        "grc.nasa.gov",
    );
}

/// Average speed — 150 m in 3 s → the engine divides → 50 m/s, carrying the
/// Alabama State Department of Education physical-science course.
#[test]
fn average_speed_binds_and_computes_with_citation() {
    check_law(
        "speed",
        "distance(150)",
        "time(3)",
        "speed(distance, time)",
        "speed",
        "50",
        "authoritative",
        "accessdl.state.al.us",
    );
}
