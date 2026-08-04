//! End-to-end tests for **RS-4 PR-D4 — the pinned-quote verify path**
//! (`ADJ-REASON-MATH.md` §E.3.1, §E.5): the D4a `quote "…" at <offset> snapshot
//! "<sha256>"` surface binding, driven all the way through `adj-verify
//! --snapshots <DIR>` to a *verified* quote.
//!
//! This is the capstone of the audit-trail arc. Each earlier PR made the trail
//! richer; D4a let a clause *pin* the exact bytes it asserts; this proves those
//! bytes are actually re-checked against the pinned document snapshot at verify
//! time. The tests are weighted toward the fail-closed directions — a drifted
//! snapshot and a missing snapshot must NOT read as verified — because a broken
//! checker fails by reporting success.
//!
//! The snapshot file is named by its own SHA-256, computed here with the same
//! `ContentHash::of` the verifier uses, so the test never hard-codes a digest
//! and can never drift from the implementation's hashing.

use logic_engine::ContentHash;
use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjverify_d4_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(dir: &Path, name: &str, src: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, src).unwrap();
    p
}

/// Write `doc` into `dir/<sha256-of-doc>` so a `--snapshots dir` run resolves a
/// pin whose `snapshot` hex is that digest. Returns the lowercase hex.
fn place_snapshot(dir: &Path, doc: &str) -> String {
    let hex = ContentHash::of(doc.as_bytes()).as_hex().to_string();
    std::fs::write(dir.join(&hex), doc).unwrap();
    hex
}

/// Run `adj-verify --snapshots <snap_dir> <program>`.
fn verify_with_snapshots(program: &Path, snap_dir: &Path) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_adj-verify"))
        .arg("--snapshots")
        .arg(snap_dir)
        .arg(program)
        .output()
        .expect("run adj-verify");
    (out.status.success(), String::from_utf8(out.stdout).unwrap())
}

// The document the pin quotes from. "Aspirin inhibits cyclooxygenase" is the
// verbatim span; it sits at byte offset 0 and is 31 bytes long.
const DOC: &str = "Aspirin inhibits cyclooxygenase in humans.";
const QUOTE: &str = "Aspirin inhibits cyclooxygenase";

// ---------------------------------------------------------------------------
// (1) The capstone: a pinned quote is re-checked against its snapshot and
//     reported VERIFIED, at the exact offset and length the pin claimed.
// ---------------------------------------------------------------------------

#[test]
fn a_pinned_quote_is_verified_against_its_snapshot() {
    let dir = scratch("verified");
    let snaps = dir.join("snaps");
    std::fs::create_dir_all(&snaps).unwrap();
    let hex = place_snapshot(&snaps, DOC);

    let src = format!(
        "relate inhibits(aspirin, cyclooxygenase)\n    \
         quote \"{QUOTE}\" at 0 snapshot \"{hex}\"\n    \
         source \"Pharmacology reference\"\n    trust authoritative\n\
         ? inhibits(aspirin, $Target)\n"
    );
    let p = write(&dir, "pin.adj", &src);

    let (ok, out) = verify_with_snapshots(&p, &snaps);
    assert!(ok, "adj-verify must exit zero on a verifiable pin:\n{out}");
    // At least one quote was actually checked against a snapshot — not the
    // no-snapshot `quotes_verified:0` path.
    assert!(
        out.contains("\"quotes_verified\":1"),
        "exactly the one pinned quote was confirmed:\n{out}"
    );
    // The quote status is `verified`, anchored at the offset/length the pin
    // claimed — the whole point of D4a's byte offset.
    assert!(
        out.contains("\"status\":\"verified\""),
        "the pinned span re-checks against the snapshot:\n{out}"
    );
    assert!(
        out.contains("\"byte_offset\":0") && out.contains("\"byte_len\":31"),
        "the verified span is anchored where the pin said (offset 0, len 31):\n{out}"
    );
    // The SLD proof that used the pinned fact is fully verified.
    assert!(
        out.contains("\"fully_verified\":true"),
        "the proof carrying the confirmed quote is fully verified:\n{out}"
    );
}

// ---------------------------------------------------------------------------
// (2) Fail-closed: a snapshot whose bytes do NOT contain the quote at the
//     claimed offset must NOT read as verified.
// ---------------------------------------------------------------------------

#[test]
fn a_drifted_snapshot_is_not_reported_verified() {
    let dir = scratch("drifted");
    let snaps = dir.join("snaps");
    std::fs::create_dir_all(&snaps).unwrap();
    // The snapshot we ship does not contain the quoted span at all.
    let drifted = "This document says something entirely different.";
    let hex = place_snapshot(&snaps, drifted);

    let src = format!(
        "relate inhibits(aspirin, cyclooxygenase)\n    \
         quote \"{QUOTE}\" at 0 snapshot \"{hex}\"\n    \
         source \"Pharmacology reference\"\n    trust authoritative\n\
         ? inhibits(aspirin, $Target)\n"
    );
    let p = write(&dir, "pin.adj", &src);

    let (_ok, out) = verify_with_snapshots(&p, &snaps);
    // The snapshot exists and matches its hash, but the bytes at the pinned
    // offset are not the quoted span — the verifier reports `quote_missing`
    // with `why: text_differs`, never `verified`, and never claims full
    // verification. (A whole-document hash mismatch would instead be
    // `source_drifted`; here the file matches its hash but the slice differs.)
    assert!(
        out.contains("\"status\":\"quote_missing\"") && out.contains("\"why\":\"text_differs\""),
        "a span whose bytes differ from the snapshot is not a confirmation:\n{out}"
    );
    assert!(
        out.contains("\"quotes_verified\":0"),
        "nothing was confirmed against the snapshot:\n{out}"
    );
    assert!(
        out.contains("\"fully_verified\":false"),
        "a drifted quote cannot yield a fully-verified run:\n{out}"
    );
}

// ---------------------------------------------------------------------------
// (3) Fail-closed: the pin names a snapshot the directory does not hold. The
//     quote is unresolvable, never invented as verified.
// ---------------------------------------------------------------------------

#[test]
fn a_missing_snapshot_leaves_the_quote_unconfirmed() {
    let dir = scratch("missing");
    let snaps = dir.join("snaps");
    std::fs::create_dir_all(&snaps).unwrap();
    // Compute the hex for DOC but DO NOT write the snapshot file.
    let hex = ContentHash::of(DOC.as_bytes()).as_hex().to_string();

    let src = format!(
        "relate inhibits(aspirin, cyclooxygenase)\n    \
         quote \"{QUOTE}\" at 0 snapshot \"{hex}\"\n    \
         source \"Pharmacology reference\"\n    trust authoritative\n\
         ? inhibits(aspirin, $Target)\n"
    );
    let p = write(&dir, "pin.adj", &src);

    let (_ok, out) = verify_with_snapshots(&p, &snaps);
    assert!(
        out.contains("\"quotes_verified\":0"),
        "a pin with no available snapshot confirms nothing:\n{out}"
    );
    assert!(
        !out.contains("\"status\":\"verified\""),
        "a missing snapshot must never be reported as a verified quote:\n{out}"
    );
    assert!(
        out.contains("\"fully_verified\":false"),
        "an unresolved pin cannot yield a fully-verified run:\n{out}"
    );
}
