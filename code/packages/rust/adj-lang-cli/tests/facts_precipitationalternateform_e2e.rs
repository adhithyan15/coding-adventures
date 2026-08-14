//! End-to-end test for the meteorology FACTS library
//! (`adj-facts-stdlib/meteorology/precipitation-alternate-form.adj`) driven
//! through the built CLI: a native `table` recording, for hail already
//! tabled in `precipitation-types.adj`/`precipitation-minimum-diameter.adj`,
//! the second descriptive term an already-cited NWS Glossary sentence
//! lists alongside its primary form -- a sibling decoding the second listed
//! term as its own row instead of folding the whole clause into one
//! compound `form` atom. Resolves forward and backward recall queries with
//! the source's citation, plus honest abstention on rain (whose cited span
//! names only one descriptive term) -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_precipalternateform_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("meteorology/precipitation-alternate-form.adj");
    std::fs::copy(&src, dir.join("precipitation-alternate-form.adj"))
        .expect("copy shipped precipitation-alternate-form.adj");
}

#[test]
fn precipitation_alternate_form_recalls_hail_form_with_citation() {
    let dir = scratch("hail");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"precipitation-alternate-form.adj\"\n\
         ? precipitation_alternate_form(hail, $Form)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"precipitation_alternate_form(hail, irregular_pellets)\""),
        "hail should recall irregular_pellets: {out}"
    );
    assert!(
        out.contains("weather.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NWS citation: {out}"
    );
}

#[test]
fn precipitation_alternate_form_backward_recalls_hail_for_irregular_pellets() {
    let dir = scratch("pellets");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"precipitation-alternate-form.adj\"\n\
         ? precipitation_alternate_form($Precip, irregular_pellets)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"precipitation_alternate_form(hail, irregular_pellets)\""),
        "hail should be the only recalled type for irregular_pellets: {out}"
    );
}

#[test]
fn precipitation_alternate_form_abstains_on_rain() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"precipitation-alternate-form.adj\"\n\
         ? precipitation_alternate_form(rain, $FormRain)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "rain's cited span names only one descriptive term, no listed alternatives -- honest abstention expected: {out}"
    );
}
