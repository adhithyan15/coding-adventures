//! End-to-end tests for the `clinical/iv-drip-rate.adj` library — the gravity IV infusion drip rate
//! (drops per minute = [total IV volume / time] × drop factor) and its two exact rearrangements — driven
//! through the built CLI binary against the SHIPPED stdlib. The same invariant as every other formula
//! library: a consumer states NO arithmetic; it imports the grounded library, binds the total volume, the
//! time in minutes, and the giving-set drop factor with `observe`, and the engine applies the cited formula
//! on the CPU, computing the EXACT value (over exact rationals) and rendering the citation and trust tier in
//! the `derived` section (the auditable answer). The three formulas INVERT around the worked case
//! volume = 1000, time = 100, drop factor = 15: 1000 / 100 × 15 = 150 (drops/min),
//! 150 × 100 / 15 = 1000 (volume), 150 × 100 / 1000 = 15 (drop factor).
//!
//! The assertions match the ADJACENT `"name":...,"value":...` pair the engine renders, rather than a bare
//! `"value":N`: the derivation tree contains the intermediate 10 (= 1000 / 100) and the drop-factor input
//! 15, and `"value":15` is a leading-digit prefix of `"value":150`, so a bare substring could spuriously
//! match. The adjacent form is collision-proof.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped iv-drip-rate library, resolved from this crate's manifest dir so the
/// test is location-independent.
fn shipped_drip_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/iv-drip-rate.adj")
        .canonicalize()
        .expect("shipped iv-drip-rate.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_drip_{tag}_{}", std::process::id()));
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

/// Copy the shipped library next to a consumer that imports it, so the CLI's
/// sandbox-checked relative import resolves.
fn place_lib(dir: &Path) {
    let lib = std::fs::read_to_string(shipped_drip_lib()).unwrap();
    std::fs::write(dir.join("iv-drip-rate.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// drops_per_minute — the drip rate: [total IV volume / time] × drop factor.
// ---------------------------------------------------------------------------

#[test]
fn imports_iv_drip_rate_library_and_computes_it_with_citation() {
    let dir = scratch("dpm");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"iv-drip-rate.adj\"\n\
         observe total_iv_volume(1000)\n\
         observe infusion_time(100)\n\
         observe drop_factor(15)\n\
         ? drops_per_minute(total_iv_volume, infusion_time, drop_factor)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: 1000 / 100 × 15 = 150, computed
    // EXACTLY over rationals. Match the adjacent name/value pair so the 10 intermediate and the 15 input in
    // the derivation cannot spuriously satisfy a bare "value":150.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"drops_per_minute\",\"value\":150"),
        "drops_per_minute(1000, 100, 15) = 150: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// total_iv_volume — the same equation solved for the volume: drops/min × time / drop factor.
// ---------------------------------------------------------------------------

#[test]
fn computes_total_iv_volume_from_drip_rate_with_citation() {
    let dir = scratch("vol");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"iv-drip-rate.adj\"\n\
         observe drops_per_minute(150)\n\
         observe infusion_time(100)\n\
         observe drop_factor(15)\n\
         ? total_iv_volume(drops_per_minute, infusion_time, drop_factor)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 150 × 100 / 15 = 15000 / 15 = 1000, computed on the CPU.
    assert!(
        s.contains("\"name\":\"total_iv_volume\",\"value\":1000"),
        "total_iv_volume(150, 100, 15) = 1000: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "total_iv_volume carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// drop_factor — the same equation solved for the drop factor: drops/min × time / volume, the third
// reading of the one rate.
// ---------------------------------------------------------------------------

#[test]
fn computes_drop_factor_from_drip_rate_with_citation() {
    let dir = scratch("df");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"iv-drip-rate.adj\"\n\
         observe drops_per_minute(150)\n\
         observe total_iv_volume(1000)\n\
         observe infusion_time(100)\n\
         ? drop_factor(drops_per_minute, total_iv_volume, infusion_time)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 150 × 100 / 1000 = 15000 / 1000 = 15, computed on the CPU.
    assert!(
        s.contains("\"name\":\"drop_factor\",\"value\":15"),
        "drop_factor(150, 1000, 100) = 15: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "drop_factor carries its cited provenance: {s}"
    );
}
