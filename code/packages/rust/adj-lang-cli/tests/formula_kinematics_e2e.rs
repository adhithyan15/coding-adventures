//! End-to-end test for the shipped physics/kinematics.adj formula library through
//! the built CLI binary. A consumer `import`s the SHIPPED library, binds the
//! quantities from its own `observe`d facts, and applies each cited law. The CLI
//! must compute the value on the CPU and render the applied formula's citation in
//! the `derived` section — the auditable answer, with zero math done by the model.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped kinematics library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_kinematics_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/physics/kinematics.adj")
        .canonicalize()
        .expect("shipped kinematics.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_formula_{tag}_{}", std::process::id()));
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

#[test]
fn imports_kinematics_library_binds_and_computes_with_its_citation() {
    // Copy the shipped library next to a consumer that imports it, so the CLI's
    // sandbox-checked relative import resolves. The consumer states NO arithmetic
    // — it binds the numbers and applies the recalled laws.
    let dir = scratch("kinematics");
    let lib = std::fs::read_to_string(shipped_kinematics_lib()).unwrap();
    std::fs::write(dir.join("kinematics.adj"), lib).unwrap();
    std::fs::write(
        dir.join("case.adj"),
        "import \"kinematics.adj\"\n\
         observe initial_velocity(5)\n\
         observe acceleration(2)\n\
         observe time(3)\n\
         ? final_velocity(initial_velocity, acceleration, time)\n\
         observe mass(4)\n\
         observe velocity(5)\n\
         ? momentum(mass, velocity)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries each applied law's result …
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    // v = u + a·t  →  5 + 2·3 = 11
    assert!(
        s.contains("\"name\":\"final_velocity\"") && s.contains("\"value\":11"),
        "computed final velocity = 11 m/s (v = u + a·t): {s}"
    );
    // p = m·v  →  4·5 = 20
    assert!(
        s.contains("\"name\":\"momentum\"") && s.contains("\"value\":20"),
        "computed momentum = 20 kg·m/s (p = m·v): {s}"
    );
    // … AND each answer carries its authoritative citation, so the chain audits.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("openstax.org"),
        "final velocity carries its OpenStax provenance: {s}"
    );
    assert!(
        s.contains("grc.nasa.gov"),
        "momentum carries its NASA Glenn provenance: {s}"
    );
}
