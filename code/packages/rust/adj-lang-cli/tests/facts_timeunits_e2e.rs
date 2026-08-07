//! End-to-end test for the metrology TIME-UNITS facts library
//! (`adj-facts-stdlib/metrology/time-units.adj`) driven through the built CLI:
//! a native `table` of time-unit → length-in-seconds resolves a binding-query
//! recall with the NIST citation, and abstains on a non-listed unit — 0 model
//! calls. A recalled length (e.g. hour → 3600 s) is the number that flows into
//! duration arithmetic.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factstu_{tag}_{}", std::process::id()));
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
fn metrology_time_units_recall_binds_seconds_with_citation() {
    let dir = scratch("timeunits");
    // Copy the shipped metrology table beside the entry program and import it.
    let src = facts_stdlib().join("metrology/time-units.adj");
    std::fs::copy(&src, dir.join("time-units.adj")).expect("copy shipped time-units.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"time-units.adj\"\n\
         ? time_unit_seconds(hour, $S)\n\
         ? time_unit_seconds(day, $S)\n\
         ? time_unit_seconds(fortnight, $S)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // An hour is 3600 seconds; a day is 86400 — the recalled lengths that feed
    // duration arithmetic.
    assert!(out.contains("\"S\":\"3600\""), "hour -> 3600 s: {out}");
    assert!(out.contains("\"S\":\"86400\""), "day -> 86400 s: {out}");
    // The answer carries the NIST citation (locator + trust) as its proof.
    assert!(
        out.contains("nist.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NIST source citation: {out}"
    );
    // A "fortnight" is not in this table — honest abstention, never a fabricated
    // length.
    assert!(out.contains("\"abstained\":true"), "fortnight abstains: {out}");
}
