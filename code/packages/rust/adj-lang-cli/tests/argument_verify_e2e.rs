//! End-to-end tests for the **ADR-4 `adj-verify` argument pass** (ADJ-ARGUMENT-IR §4),
//! driven through the built `adj-verify` binary.
//!
//! The key finding these tests lock in: because an `argument` **desugars** to facts +
//! rules (ADR-2), `adj-verify`'s existing machinery already delivers §4 over it — no new
//! verifier code. A premise's pinned citation is byte-anchored by the same `verify_quote`
//! / snapshot re-check `adj-verify` runs over every fact, and the thesis is re-derived by
//! the same SLD re-check that re-runs every rule. These tests prove that end to end: a
//! snapshot-pinned argument's premise cite is confirmed (`quotes_verified ≥ 1`), the thesis
//! re-derives, and a **drifted** citation (a quote that is not a verbatim slice of the
//! pinned source) fails the run — the byte-anchor catching an argument that no longer holds
//! against its sources.

use logic_engine::ContentHash;
use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("argverify_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(dir: &Path, name: &str, src: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, src).unwrap();
    p
}

/// Write `doc` into `dir/<sha256-of-doc>` so a `--snapshots dir` run resolves a pin whose
/// `snapshot` hex is that digest. Returns the lowercase hex.
fn place_snapshot(dir: &Path, doc: &str) -> String {
    let hex = ContentHash::of(doc.as_bytes()).as_hex().to_string();
    std::fs::write(dir.join(&hex), doc).unwrap();
    hex
}

fn verify_with_snapshots(program: &Path, snap_dir: &Path) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_adj-verify"))
        .arg("--snapshots")
        .arg(snap_dir)
        .arg(program)
        .output()
        .expect("run adj-verify");
    (out.status.success(), String::from_utf8(out.stdout).unwrap())
}

// The source document the argument is decomposed from. The premise quotes a verbatim slice
// of it (offset 0), so the byte-anchor can confirm the cite against the pinned snapshot.
const DOC: &str = "The operating stress exceeded the fatigue limit, so the axle cracked.";
const CITE: &str = "The operating stress exceeded the fatigue limit";

// ---------------------------------------------------------------------------
// (1) A snapshot-pinned argument: the premise cite is byte-anchored and the
//     thesis re-derives — §4 delivered by the desugaring, no new verifier code.
// ---------------------------------------------------------------------------

#[test]
fn a_pinned_argument_premise_is_byte_anchored_and_the_thesis_rederives() {
    let dir = scratch("pinned");
    let snaps = dir.join("snaps");
    std::fs::create_dir_all(&snaps).unwrap();
    let hex = place_snapshot(&snaps, DOC);

    let src = format!(
        "argument axle {{\n    \
           premise p1 : extracted exceeds(axle, limit) \
             quote \"{CITE}\" at 0 snapshot \"{hex}\" source \"axle report\" trust authoritative\n    \
           infer s1 : therefore conclude mechanism(axle, fatigue) from p1 \
             source \"so the axle cracked\" trust authoritative\n\
         }}\n\
         ? mechanism(axle, $M)\n"
    );
    let p = write(&dir, "arg.adj", &src);
    let (ok, out) = verify_with_snapshots(&p, &snaps);

    assert!(ok, "a sound, pinned argument must exit zero:\n{out}");
    assert!(out.contains("\"verified\":true"), "{out}");
    // The premise's cite was actually checked against the snapshot — not the
    // no-snapshot `quotes_verified:0` path. This IS the §4 byte-anchor over an argument,
    // delivered by the same verify_quote the desugared fact carries.
    assert!(
        out.contains("\"quotes_verified\":1"),
        "the pinned premise citation must be byte-anchored against the source:\n{out}"
    );
    assert!(
        out.contains("\"status\":\"verified\""),
        "the premise's cite re-checks verbatim against the pinned snapshot:\n{out}"
    );
    // The thesis re-derived by chaining the inference rule over the premise fact — the
    // proof DAG IS the argument, re-executed (kind FromRule, logic rechecked).
    assert!(
        out.contains("\"kind\":\"FromRule\"") && out.contains("\"logic\":\"rechecked\""),
        "the thesis must re-derive from the premise via the inference rule:\n{out}"
    );
}

// ---------------------------------------------------------------------------
// (2) A DRIFTED citation fails: a premise that quotes bytes not in the pinned
//     source is caught — the byte-anchor rejecting an argument that no longer
//     holds against its sources.
// ---------------------------------------------------------------------------

#[test]
fn a_drifted_premise_citation_fails_the_run() {
    let dir = scratch("drift");
    let snaps = dir.join("snaps");
    std::fs::create_dir_all(&snaps).unwrap();
    let hex = place_snapshot(&snaps, DOC);

    // The premise claims a quote that is NOT a verbatim slice of DOC (the source never
    // says "corrosion"), pinned to the real snapshot — a fabricated/drifted citation.
    let src = format!(
        "argument axle {{\n    \
           premise p1 : extracted exceeds(axle, limit) \
             quote \"corrosion pitted the axle surface\" at 0 snapshot \"{hex}\" \
             source \"axle report\" trust authoritative\n    \
           infer s1 : therefore conclude mechanism(axle, fatigue) from p1 \
             source \"so the axle cracked\" trust authoritative\n\
         }}\n\
         ? mechanism(axle, $M)\n"
    );
    let p = write(&dir, "arg.adj", &src);
    let (ok, out) = verify_with_snapshots(&p, &snaps);

    assert!(
        !ok,
        "a drifted citation must exit non-zero, not silently pass:\n{out}"
    );
    assert!(out.contains("\"verified\":false"), "{out}");
    // The verdict must name WHY: the quoted bytes are not in the pinned source.
    assert!(
        out.contains("\"status\":\"quote_missing\""),
        "the drifted citation must be reported as a missing quote:\n{out}"
    );
}
