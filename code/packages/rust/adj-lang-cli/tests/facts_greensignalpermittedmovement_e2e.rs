//! End-to-end test for the transportation FACTS library
//! (`adj-facts-stdlib/transportation/green-signal-permitted-movement.adj`)
//! driven through the built CLI: a native `table` enumerating the four
//! individual movements the SAME MUTCD sentence already states for a
//! steady CIRCULAR GREEN signal -- a sibling to the already-shipped
//! `traffic-lights.adj` (which only carries the single atomic meaning
//! `proceed` for green), decoding the four permitted movements already
//! sitting unused inside that table's own header quote. Resolves a
//! keyless recall query returning all four movements with the source's
//! citation -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_greensignalpermittedmovement_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("transportation/green-signal-permitted-movement.adj");
    std::fs::copy(&src, dir.join("green-signal-permitted-movement.adj"))
        .expect("copy shipped green-signal-permitted-movement.adj");
}

#[test]
fn green_signal_permitted_movement_recalls_straight_through_with_citation() {
    let dir = scratch("straight");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"green-signal-permitted-movement.adj\"\n\
         ? green_signal_permitted_movement($Movement)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"green_signal_permitted_movement(straight_through)\""),
        "straight_through is a permitted movement: {out}"
    );
    assert!(
        out.contains("mutcd.fhwa.dot.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the MUTCD citation: {out}"
    );
}

#[test]
fn green_signal_permitted_movement_recalls_all_four_movements() {
    let dir = scratch("all");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"green-signal-permitted-movement.adj\"\n\
         ? green_signal_permitted_movement($Movement)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    for movement in ["straight_through", "turn_right", "turn_left", "u_turn"] {
        assert!(
            out.contains(&format!("\"term\":\"green_signal_permitted_movement({movement})\"")),
            "{movement} should be a recalled permitted movement: {out}"
        );
    }
}

#[test]
fn green_signal_permitted_movement_covers_all_movements_without_abstention() {
    let dir = scratch("noabstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"green-signal-permitted-movement.adj\"\n\
         ? green_signal_permitted_movement($Movement)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        !out.contains("\"abstained\":true"),
        "the MUTCD span names four movements -- no abstention expected: {out}"
    );
}
