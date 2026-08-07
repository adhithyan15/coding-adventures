//! End-to-end tests for **RS-4 PR-D2 — `adj-verify`** (`ADJ-REASON-MATH.md`
//! §E.5, §E.6), driven through the built binary.
//!
//! Everything earlier in this arc made the trail richer. This binary is the
//! first thing that can say the trail is **wrong**. A verifier's bugs are the
//! most dangerous kind in the system, because a broken checker fails by
//! reporting success — so these tests are weighted toward the directions where
//! that could happen: an empty check that reads as a pass, a soft verdict
//! dressed up as a hard one, and a headline that outruns what was actually
//! confirmed.

use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjverify_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(dir: &Path, name: &str, src: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, src).unwrap();
    p
}

fn verify(program: &Path) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_adj-verify"))
        .arg(program)
        .output()
        .expect("run adj-verify");
    (out.status.success(), String::from_utf8(out.stdout).unwrap())
}

/// A tiny recall library: one relation, one citation, one query.
const RECALL: &str = "relate inhibits(aspirin, cyclooxygenase)\n\
     source \"Aspirin inhibits cyclooxygenase.\"\n\
     trust authoritative\n\
 ? inhibits(aspirin, $Target)\n";

// ---------------------------------------------------------------------------
// (1) The trail re-executes.
// ---------------------------------------------------------------------------

#[test]
fn a_sound_recall_trail_re_executes_and_exits_zero() {
    let dir = scratch("sound");
    let p = write(&dir, "recall.adj", RECALL);
    let (ok, out) = verify(&p);

    assert!(ok, "a sound trail must exit 0 so this composes as a CI gate:\n{out}");
    assert!(out.contains("\"verified\":true"), "{out}");
    assert!(
        out.contains("\"logic\":\"rechecked\""),
        "the fact step must be re-unified, not taken on trust:\n{out}"
    );
    assert!(
        out.contains("\"kind\":\"FromFact\""),
        "and the report must name WHICH kind of step it re-ran:\n{out}"
    );
}

// ---------------------------------------------------------------------------
// (2) `verified` and `fully_verified` are NOT the same claim.
// ---------------------------------------------------------------------------

#[test]
fn an_unmigrated_library_is_verified_but_never_fully_verified() {
    // Today's stdlib records `source` labels, not pinned spans. The logic
    // re-executes, so the trail is sound — but not one byte has been checked,
    // and the report must not let those two read the same. An earlier draft
    // computed `fully_verified` from re-execution alone and reported the
    // system's strongest verdict over an entirely unchecked corpus.
    let dir = scratch("unmigrated");
    let p = write(&dir, "recall.adj", RECALL);
    let (ok, out) = verify(&p);

    assert!(ok);
    assert!(out.contains("\"verified\":true"), "{out}");
    assert!(
        out.contains("\"fully_verified\":false"),
        "nothing was checked against a snapshot; the headline must say so:\n{out}"
    );
    assert!(
        out.contains("\"status\":\"unverified\",\"why\":\"unmigrated\""),
        "and it must say WHY, per step:\n{out}"
    );
    assert!(
        out.contains("\"quotes_verified\":0"),
        "the count of confirmed spans is the honest headline number:\n{out}"
    );
}

// ---------------------------------------------------------------------------
// (3) Absence is re-run, not assumed.
// ---------------------------------------------------------------------------

#[test]
fn a_negation_step_is_re_run_and_reported_as_not_applicable_for_quotes() {
    let dir = scratch("negation");
    let src = "relate safe_for(aspirin, adult)\n\
         source \"Aspirin is appropriate for adults without contraindications.\"\n\
         trust authoritative\n\
     rule {\n\
         head: may_prescribe(aspirin, adult)\n\
         when: safe_for(aspirin, adult), not contraindicated(aspirin, adult)\n\
         source \"Prescribe an appropriate drug absent a contraindication.\"\n\
         trust authoritative\n\
     }\n\
     ? may_prescribe(aspirin, adult)\n";
    let p = write(&dir, "neg.adj", src);
    let (ok, out) = verify(&p);

    assert!(ok, "{out}");
    assert!(
        out.contains("\"kind\":\"FromNegation\""),
        "the check that actually licensed the conclusion must appear:\n{out}"
    );
    assert!(
        out.contains("\"status\":\"not_applicable\""),
        "an absence has no sentence in any document — that is NOT 'unverified':\n{out}"
    );
}

// ---------------------------------------------------------------------------
// (4) A trail that no longer holds must FAIL, loudly and with an exit code.
// ---------------------------------------------------------------------------

#[test]
fn a_negation_whose_absence_has_been_filled_in_fails_the_run() {
    // Same program, plus the contraindication the rule was guarding against.
    // The rule can no longer fire — so no proof exists, and there is nothing
    // to verify. The honest report is an empty proof list, NOT a pass over a
    // trail that would have been unsound.
    let dir = scratch("filled");
    let src = "relate safe_for(aspirin, adult)\n\
         source \"Aspirin is appropriate for adults without contraindications.\"\n\
         trust authoritative\n\
     relate contraindicated(aspirin, adult)\n\
         source \"Aspirin is contraindicated in this population.\"\n\
         trust authoritative\n\
     rule {\n\
         head: may_prescribe(aspirin, adult)\n\
         when: safe_for(aspirin, adult), not contraindicated(aspirin, adult)\n\
         source \"Prescribe an appropriate drug absent a contraindication.\"\n\
         trust authoritative\n\
     }\n\
     ? may_prescribe(aspirin, adult)\n";
    let p = write(&dir, "neg2.adj", src);
    let (_ok, out) = verify(&p);

    assert!(
        !out.contains("\"kind\":\"FromNegation\""),
        "the negation cannot re-check, because the rule no longer fires:\n{out}"
    );
    assert!(
        out.contains("\"fully_verified\":false"),
        "and with nothing confirmed, the strongest verdict must not be claimed:\n{out}"
    );
}

// ---------------------------------------------------------------------------
// (5) Checking nothing is not the same as checking everything.
// ---------------------------------------------------------------------------

#[test]
fn a_program_with_no_provable_query_does_not_report_full_verification() {
    // `all()` over an empty step list is `true`. That vacuous truth is exactly
    // the manufactured confidence this whole arc exists to prevent: a program
    // that proves nothing would otherwise earn the system's strongest verdict
    // for having done no work at all.
    let dir = scratch("vacuous");
    let src = "relate inhibits(aspirin, cyclooxygenase)\n\
         source \"Aspirin inhibits cyclooxygenase.\"\n\
         trust authoritative\n\
     ? inhibits(warfarin, $Target)\n";
    let p = write(&dir, "empty.adj", src);
    let (_ok, out) = verify(&p);

    assert!(
        out.contains("\"fully_verified\":false"),
        "nothing was checked, so nothing is fully verified:\n{out}"
    );
    assert!(out.contains("\"steps\":0"), "{out}");
}

// ---------------------------------------------------------------------------
// (6) Both reasoning paths are examined, and the report says which is which.
// ---------------------------------------------------------------------------

#[test]
fn the_report_labels_the_sld_and_lr_passes_separately() {
    // Verifying only one path while reporting "verified" would leave half the
    // trail unexamined behind a clean headline.
    let dir = scratch("passes");
    let p = write(&dir, "recall.adj", RECALL);
    let (_ok, out) = verify(&p);

    assert!(out.contains("\"pass\":\"sld\""), "{out}");
}

// ---------------------------------------------------------------------------
// (7) Untrusted text cannot forge trail structure.
// ---------------------------------------------------------------------------

#[test]
fn a_goal_carrying_control_characters_is_escaped_not_interpolated_raw() {
    // Terms reach the report as text. If a newline or a quote passed through
    // unescaped, a cited span could forge its own `"logic":"rechecked"` line in
    // any line-oriented consumer of this output.
    let dir = scratch("escape");
    let src = "relate says(x, \"line one\\nline two\")\n\
         source \"A source with awkward characters.\"\n\
         trust authoritative\n\
     ? says(x, $What)\n";
    let p = write(&dir, "esc.adj", src);
    let (_ok, out) = verify(&p);

    // Exactly one JSON object per line: if a raw newline had leaked through,
    // the output would span more than one line.
    assert_eq!(
        out.trim().lines().count(),
        1,
        "a quoted span must not be able to add lines to the report:\n{out}"
    );
}

// ---------------------------------------------------------------------------
// (8) "I stopped looking" must never exit 0.
// ---------------------------------------------------------------------------

#[test]
fn a_truncated_search_fails_the_run_rather_than_passing_with_zero_proofs() {
    // A self-recursive rule has no base case, so the resolver hits its depth cap
    // and gives up. The proof set is empty — for a reason that has nothing to do
    // with the program being sound. Reporting `verified: true` here would hand a
    // green CI gate to any input crafted to make the search bail, which is the
    // same conflation the negation re-check already treats as a hard failure.
    let dir = scratch("truncated");
    let src = "rule {\n\
         head: loops(a)\n\
         when: loops(a)\n\
         source \"A rule with no base case.\"\n\
         trust authoritative\n\
     }\n\
     ? loops(a)\n";
    let p = write(&dir, "loop.adj", src);
    let (ok, out) = verify(&p);

    assert!(
        !ok,
        "a truncated search must exit non-zero, not silently pass:\n{out}"
    );
    assert!(out.contains("\"verified\":false"), "{out}");
    assert!(
        out.contains("loops"),
        "and the report must name WHICH query was abandoned:\n{out}"
    );
}
