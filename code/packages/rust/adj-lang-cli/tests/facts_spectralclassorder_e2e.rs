//! End-to-end test for the astronomy FACTS library
//! (`adj-facts-stdlib/astronomy/spectral-class-order.adj`) driven through the
//! built CLI: a native `table` recording each stellar spectral class's rank
//! in the hottest-to-coolest sequence, decoded from the SAME NASA sentence
//! already tabled by color in `spectral-classes.adj` -- a sibling decoding
//! the order half of that already-verified quote. Resolves forward and
//! backward recall queries with the source's citation, plus honest
//! abstention on a non-class letter -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_spectralclassorder_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("astronomy/spectral-class-order.adj");
    std::fs::copy(&src, dir.join("spectral-class-order.adj"))
        .expect("copy shipped spectral-class-order.adj");
}

#[test]
fn spectral_class_order_recalls_g_class_with_citation() {
    let dir = scratch("g");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"spectral-class-order.adj\"\n\
         ? spectral_class_order(g, $Order)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"spectral_class_order(g, 5)\""),
        "the G class should recall its cited rank: {out}"
    );
    assert!(
        out.contains("science.nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NASA citation: {out}"
    );
}

#[test]
fn spectral_class_order_backward_recalls_o_for_rank_one() {
    let dir = scratch("rank1");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"spectral-class-order.adj\"\n\
         ? spectral_class_order($Class, 1)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"spectral_class_order(o, 1)\""),
        "the O class should be the only recalled rank-1 class: {out}"
    );
    assert!(
        !out.contains("spectral_class_order(m, 1)"),
        "the M class's cited rank is 7, not 1: {out}"
    );
}

#[test]
fn spectral_class_order_abstains_on_non_class_letter() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"spectral-class-order.adj\"\n\
         ? spectral_class_order(z, $OrderZ)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "\"z\" is not one of the seven main-sequence spectral classes -- honest abstention expected: {out}"
    );
}
