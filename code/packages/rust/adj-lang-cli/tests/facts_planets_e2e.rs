//! End-to-end test for the astronomy planets FACTS library
//! (`adj-facts-stdlib/astronomy/planets.adj`): a native `table` of
//! planet → order-from-the-Sun resolves forward AND reverse binding queries with
//! the NASA citation, and abstains on a non-planet — 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsk_{tag}_{}", std::process::id()));
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
fn astronomy_planets_recall_binds_order_forward_and_reverse() {
    let dir = scratch("planets");
    let src = facts_stdlib().join("astronomy/planets.adj");
    std::fs::copy(&src, dir.join("planets.adj")).expect("copy shipped planets.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"planets.adj\"\n\
         ? planet_order(earth, $N)\n\
         ? planet_order($Planet, 1)\n\
         ? planet_order(pluto, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Forward: Earth is the third planet from the Sun.
    assert!(out.contains("\"N\":\"3\""), "earth → 3: {out}");
    // Reverse: the first planet from the Sun is Mercury (binds the other column).
    assert!(out.contains("\"Planet\":\"mercury\""), "order 1 → mercury: {out}");
    // The answer carries the NASA citation as its proof.
    assert!(
        out.contains("science.nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NASA citation: {out}"
    );
    // Pluto (a dwarf planet) is not a row — honest abstention, no fabricated order.
    assert!(out.contains("\"abstained\":true"), "pluto abstains: {out}");
}
