//! AR-2 — the worked REBUTTAL + UNDERCUT example for ADJ-ARGUMENT-REBUTTAL. It drives the
//! committed example under `code/specs/data/adj-argument-ir/rebuttal/` through the built
//! binaries to prove a paper's *dialectic* end to end — the engine WITHDRAWS what its
//! counterarguments defeat, and every attack is byte-anchored to its paragraph:
//!
//! * REBUT (`rebuttal.adj`): a support paragraph concludes `failed_by(axle, fatigue)`; a
//!   reanalysis paragraph rebuts it with `failed_by(axle, overload)` under higher
//!   `context_order` precedence. With the conclusion predicate `functional`, the `governing`
//!   query marks fatigue **defeated** (`defeated_by` overload) and overload **governing**;
//!   `adj-verify --snapshots` byte-anchors both `relate` premises across their snapshots.
//! * UNDERCUT (`undercut.adj`): a limitation paragraph grounds a `not`-guarded warrant defeater,
//!   so the fatigue thesis **abstains** — no rival mechanism asserted. Remove the undercut
//!   condition and the thesis derives again.
//!
//! Both attack kinds reuse ADJ73 defeasibility + negation-as-failure — zero new engine code.

use logic_engine::ContentHash;
use std::path::{Path, PathBuf};
use std::process::Command;

fn data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../specs/data/adj-argument-ir/rebuttal")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rebutwe_{tag}_{}", std::process::id()));
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
// REBUT — the rebutted thesis is WITHDRAWN by the engine, and byte-anchored.
// ---------------------------------------------------------------------------

#[test]
fn a_rebuttal_defeats_the_thesis_via_context_precedence() {
    let adj = data_dir().join("rebuttal.adj");
    let (ok, s) = run(&[adj.to_str().unwrap()]);
    assert!(ok, "the rebuttal example must run:\n{s}");
    // The initial thesis is DEFEATED — withdrawn by the engine — by the reanalysis rival.
    assert!(
        s.contains("\"term\":\"failed_by(axle, fatigue)\"")
            && s.contains("\"status\":\"defeated\"")
            && s.contains("\"defeated_by\":\"failed_by(axle, overload)\""),
        "the fatigue thesis must be defeated by the overload rebuttal:\n{s}"
    );
    // The reanalysis conclusion GOVERNS.
    assert!(
        s.contains("\"term\":\"failed_by(axle, overload)\"") && s.contains("\"status\":\"governing\""),
        "the reanalysis conclusion must govern:\n{s}"
    );
}

#[test]
fn adj_verify_byte_anchors_the_rebuttal_premises() {
    let snaps = scratch("snaps");
    assert_eq!(place_all_snapshots(&snaps), 3, "three paragraph snapshots");
    let out = Command::new(env!("CARGO_BIN_EXE_adj-verify"))
        .arg("--snapshots")
        .arg(&snaps)
        .arg(data_dir().join("rebuttal.adj"))
        .output()
        .expect("run adj-verify");
    let s = String::from_utf8(out.stdout).unwrap();
    assert!(out.status.success(), "the pinned rebuttal must verify:\n{s}");
    assert!(s.contains("\"verified\":true"), "{s}");
    // Both the support and the rebuttal premise are byte-anchored to their paragraphs.
    assert!(
        s.contains("\"quotes_verified\":2"),
        "both grounded premises (support + rebuttal) must byte-anchor:\n{s}"
    );
}

// ---------------------------------------------------------------------------
// UNDERCUT — a grounded undercut removes the warrant; the thesis abstains.
// ---------------------------------------------------------------------------

#[test]
fn an_undercut_makes_the_thesis_abstain_and_removing_it_restores_derivation() {
    let adj = data_dir().join("undercut.adj");
    // With the undercut condition present, the fatigue warrant is disabled → no answer.
    let (ok, s) = run(&[adj.to_str().unwrap()]);
    assert!(ok, "the undercut example must run:\n{s}");
    assert!(
        s.contains("\"abstained\":true"),
        "the undercut warrant must make the thesis abstain (no rival asserted):\n{s}"
    );
    assert!(!s.contains("\"Mechanism\":\"fatigue\""), "no fatigue answer while undercut holds:\n{s}");

    // Remove the undercutting `relate contaminated(sample)` line → the warrant is not undercut →
    // the fatigue thesis derives again. (The break test: the undercut is load-bearing.)
    let text = std::fs::read_to_string(&adj).unwrap();
    let restored: String = text
        .lines()
        .filter(|l| !l.contains("relate contaminated(sample)"))
        .collect::<Vec<_>>()
        .join("\n");
    let dir = scratch("restore");
    let prog = dir.join("no_undercut.adj");
    std::fs::write(&prog, restored).unwrap();
    let (ok2, s2) = run(&[prog.to_str().unwrap()]);
    assert!(ok2, "{s2}");
    assert!(
        s2.contains("\"Mechanism\":\"fatigue\""),
        "removing the undercut condition must restore the fatigue derivation:\n{s2}"
    );
}
