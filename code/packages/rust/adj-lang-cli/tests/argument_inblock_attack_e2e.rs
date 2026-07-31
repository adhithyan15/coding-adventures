//! AR-3 — a paper's ATTACK edges now compose INSIDE the `argument` block. The AR-2 rebuttal
//! and undercut required raw `rule`+`functional`+`context_order` bolted alongside the argument;
//! AR-3 adds two pieces of surface sugar on `infer` so support AND attack live in one block:
//!
//!  * REBUT (`rebuttal-inblock.adj`): each `infer` carries a `context:` — the support step in
//!    `initial_report`, the reanalysis step in `reanalysis`. With a `functional` thesis and
//!    `context_order { reanalysis > initial_report }`, the engine WITHDRAWS fatigue (marks it
//!    `defeated`, `defeated_by` overload) and the reanalysis conclusion GOVERNS. `adj-verify`
//!    byte-anchors both premises.
//!  * UNDERCUT (`undercut-inblock.adj`): the support `infer` carries `unless warrant_undercut`
//!    (→ a `not` body literal), and a second `infer` derives that defeater from the contamination
//!    premise. Contaminated → the thesis ABSTAINS. Remove the deriving `infer` → fatigue returns.
//!
//! Both desugar to the proven ADJ73 defeasibility + negation-as-failure — zero new engine code.

use logic_engine::ContentHash;
use std::path::{Path, PathBuf};
use std::process::Command;

fn data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../specs/data/adj-argument-ir/rebuttal")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ar3inblock_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Place every `*.source.txt` paragraph as a content-addressed snapshot in `snaps`.
fn place_all_snapshots(snaps: &Path) -> usize {
    let mut n = 0;
    for entry in std::fs::read_dir(data_dir()).unwrap() {
        let path = entry.unwrap().path();
        if path.file_name().unwrap().to_string_lossy().ends_with(".source.txt") {
            let bytes = std::fs::read(&path).unwrap();
            std::fs::write(snaps.join(ContentHash::of(&bytes).as_hex()), &bytes).unwrap();
            n += 1;
        }
    }
    n
}

fn run(args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .args(args)
        .output()
        .expect("run adj-lang-cli");
    (out.status.success(), String::from_utf8(out.stdout).unwrap())
}

// ---------------------------------------------------------------------------
// REBUT — the in-block `context:` sugar withdraws the defeated conclusion.
// ---------------------------------------------------------------------------

#[test]
fn inblock_context_rebuts_the_thesis_via_precedence() {
    let adj = data_dir().join("rebuttal-inblock.adj");
    let (ok, s) = run(&[adj.to_str().unwrap()]);
    assert!(ok, "the in-block rebuttal must run:\n{s}");
    // The fatigue conclusion — concluded by an `infer … context: initial_report` — is DEFEATED
    // by the reanalysis `infer … context: reanalysis` under the paper's context_order.
    assert!(
        s.contains("\"term\":\"failed_by(axle, fatigue)\"")
            && s.contains("\"status\":\"defeated\"")
            && s.contains("\"defeated_by\":\"failed_by(axle, overload)\""),
        "the in-block fatigue inference must be defeated by the in-block overload inference:\n{s}"
    );
    assert!(
        s.contains("\"term\":\"failed_by(axle, overload)\"") && s.contains("\"status\":\"governing\""),
        "the reanalysis conclusion must govern:\n{s}"
    );
}

#[test]
fn adj_verify_byte_anchors_the_inblock_rebuttal_premises() {
    let snaps = scratch("snaps");
    assert_eq!(place_all_snapshots(&snaps), 3, "three paragraph snapshots");
    let out = Command::new(env!("CARGO_BIN_EXE_adj-verify"))
        .arg("--snapshots")
        .arg(&snaps)
        .arg(data_dir().join("rebuttal-inblock.adj"))
        .output()
        .expect("run adj-verify");
    let s = String::from_utf8(out.stdout).unwrap();
    assert!(out.status.success(), "the pinned in-block rebuttal must verify:\n{s}");
    assert!(s.contains("\"verified\":true"), "{s}");
    assert!(
        s.contains("\"quotes_verified\":2"),
        "both in-block premises (support + rebuttal) must byte-anchor:\n{s}"
    );
}

// ---------------------------------------------------------------------------
// UNDERCUT — the in-block `unless` guard makes the thesis abstain; removing the
// defeater-deriving inference restores the derivation.
// ---------------------------------------------------------------------------

#[test]
fn inblock_unless_makes_the_thesis_abstain_and_removing_the_defeater_restores_it() {
    let adj = data_dir().join("undercut-inblock.adj");
    // With the contamination-derived undercut present, the fatigue warrant is disabled.
    let (ok, s) = run(&[adj.to_str().unwrap()]);
    assert!(ok, "the in-block undercut example must run:\n{s}");
    assert!(
        s.contains("\"abstained\":true"),
        "the in-block `unless` guard must make the thesis abstain:\n{s}"
    );
    assert!(!s.contains("\"Mechanism\":\"fatigue\""), "no fatigue answer while undercut holds:\n{s}");

    // Remove the `infer undercut_warrant …` line → warrant_undercut is never derived → the
    // `unless warrant_undercut` guard is satisfied → the fatigue thesis derives again.
    let text = std::fs::read_to_string(&adj).unwrap();
    let restored: String = text
        .lines()
        .filter(|l| !l.contains("infer undercut_warrant"))
        .collect::<Vec<_>>()
        .join("\n");
    let dir = scratch("restore");
    let prog = dir.join("no_undercut.adj");
    std::fs::write(&prog, restored).unwrap();
    let (ok2, s2) = run(&[prog.to_str().unwrap()]);
    assert!(ok2, "{s2}");
    assert!(
        s2.contains("\"Mechanism\":\"fatigue\""),
        "removing the defeater-deriving inference must restore the fatigue derivation:\n{s2}"
    );
}
