//! End-to-end test for the earth-science FACTS library
//! (`adj-facts-stdlib/earth-science/seismic-wave-arrival-order.adj`) driven
//! through the built CLI: a native `table` naming the two named seismic
//! body waves and which one an earthquake sends out first, quoted verbatim
//! from Cal OES News' "What Are P-Waves and S-Waves?" article. 0
//! answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_seismic_wave_arrival_order_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("earth-science/seismic-wave-arrival-order.adj");
    std::fs::copy(&src, dir.join("seismic-wave-arrival-order.adj"))
        .expect("copy shipped seismic-wave-arrival-order.adj");
}

#[test]
fn seismic_wave_arrival_order_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"seismic-wave-arrival-order.adj\"\n\
         ? seismic_wave_arrival_order(p_wave, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"are_the_first_waves_to_arrive_after_an_earthquake\""),
        "p_wave means are_the_first_waves_to_arrive_after_an_earthquake: {out}"
    );
    assert!(
        out.contains("news.caloes.ca.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the Cal OES citation: {out}"
    );
}

#[test]
fn seismic_wave_arrival_order_reverse_binds_the_wave_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"seismic-wave-arrival-order.adj\"\n\
         ? seismic_wave_arrival_order($W, are_the_next_waves_to_arrive_after_p_waves)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"W\":\"s_wave\""),
        "the shipped are_the_next_waves_to_arrive_after_p_waves example is s_wave: {out}"
    );
}

#[test]
fn seismic_wave_arrival_order_abstains_honestly_on_an_untabled_wave() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"seismic-wave-arrival-order.adj\"\n\
         ? seismic_wave_arrival_order(surface_wave, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "surface_wave is a real seismic wave commonly grouped with p_wave/s_wave, but no clean single-fact parallel definition was found -- honest abstention, never invented: {out}"
    );
}
