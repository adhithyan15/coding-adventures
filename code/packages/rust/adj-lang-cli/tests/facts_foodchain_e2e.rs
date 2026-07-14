//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/food-chain-roles.adj`) driven through the built
//! CLI: a native `table` of food-chain role → one-word job resolves a
//! binding-query recall with the source's citation, runs the relation backward
//! (job → role), and abstains on a non-role — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsbio_{tag}_{}", std::process::id()));
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
fn biology_food_chain_recall_binds_job_with_citation() {
    let dir = scratch("foodchain");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/food-chain-roles.adj");
    std::fs::copy(&src, dir.join("food-chain-roles.adj"))
        .expect("copy shipped food-chain-roles.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"food-chain-roles.adj\"\n\
         ? food_chain_role(producer, $D)\n\
         ? food_chain_role(consumer, $D)\n\
         ? food_chain_role($R, makes_food)\n\
         ? food_chain_role(volcano, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The producer makes its own food; the consumer eats other organisms — the
    // recalled one-word job atoms (two forward binds).
    assert!(out.contains("\"D\":\"makes_food\""), "producer → makes_food: {out}");
    assert!(out.contains("\"D\":\"eats_others\""), "consumer → eats_others: {out}");
    // The relation runs backward: the job makes_food recalls producer.
    assert!(
        out.contains("\"R\":\"producer\""),
        "makes_food → producer (reverse recall): {out}"
    );
    // The answer carries the NOAA citation and the authoritative trust tier as
    // its proof (locator + trust).
    assert!(
        out.contains("noaa.gov/education/resource-collections/marine-life/aquatic-food-webs")
            && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // "volcano" is not a food-chain role — honest abstention, never a fabricated
    // job.
    assert!(out.contains("\"abstained\":true"), "non-role abstains: {out}");
}
