//! End-to-end test for the ADJ-FORMULA-LIBRARIES rung-0 substrate through the
//! built CLI binary: a consumer `import`s the SHIPPED `bmi.adj` formula library,
//! binds the variables from its own `observe`d facts, and applies the cited
//! formula. The CLI must compute the value on the CPU and render the applied
//! formula's citation in the `derived` section — the auditable answer.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped BMI library, resolved from this crate's manifest
/// dir so the test is location-independent.
fn shipped_bmi_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/bmi.adj")
        .canonicalize()
        .expect("shipped bmi.adj must exist")
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
fn imports_bmi_library_binds_and_computes_with_its_citation() {
    // Copy the shipped library next to a consumer that imports it, so the CLI's
    // sandbox-checked relative import resolves. The consumer states NO arithmetic
    // — it binds the numbers and applies the recalled formula.
    let dir = scratch("bmi");
    let lib = std::fs::read_to_string(shipped_bmi_lib()).unwrap();
    std::fs::write(dir.join("bmi.adj"), lib).unwrap();
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
    // The derived value section carries the applied formula's result …
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"bmi\"") && s.contains("\"value\":22.857142857142858"),
        "computed BMI ≈ 22.857 kg/m²: {s}"
    );
    // … AND the WHO citation + trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("who.int"),
        "applied formula carries its cited provenance: {s}"
    );
}
