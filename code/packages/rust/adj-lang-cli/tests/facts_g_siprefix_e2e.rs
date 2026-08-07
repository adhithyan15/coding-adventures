//! End-to-end test for the metrology FACTS library
//! (`adj-facts-stdlib/metrology/metric-prefixes.adj`) driven through the built
//! CLI: a native `table` of SI-prefix → power-of-ten resolves a binding-query
//! recall with the source's citation, binds a NEGATIVE exponent for a sub-unit
//! prefix (milli → -3), and abstains on a word that is not an SI prefix — with 0
//! model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsg_{tag}_{}", std::process::id()));
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
fn metrology_metric_prefix_recall_binds_power_of_ten_with_citation() {
    let dir = scratch("siprefix");
    // Copy the shipped metrology table beside the entry program and import it.
    let src = facts_stdlib().join("metrology/metric-prefixes.adj");
    std::fs::copy(&src, dir.join("metric-prefixes.adj")).expect("copy shipped metric-prefixes.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"metric-prefixes.adj\"\n\
         ? si_prefix_power(kilo, $P)\n\
         ? si_prefix_power(milli, $P)\n\
         ? si_prefix_power(dozen, $P)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // kilo means 10^3; milli means 10^-3 — the recalled exponents. Mind the
    // minus sign: the negative literal round-trips as the string "-3".
    assert!(out.contains("\"P\":\"3\""), "kilo → 3: {out}");
    assert!(out.contains("\"P\":\"-3\""), "milli → -3: {out}");
    // The answer carries the NIST citation as its proof.
    assert!(
        out.contains("nist.gov/pml/owm/metric-si-prefixes")
            && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // "dozen" is not an SI prefix — honest abstention, never a fabricated power.
    assert!(out.contains("\"abstained\":true"), "dozen abstains: {out}");
}
