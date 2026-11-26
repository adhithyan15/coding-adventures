//! # `sign`: the reviewer's banner, bound to the rows and to the run (VOS-D120)
//!
//! Nothing wrote the results file's `regeneration` field until now: `meta::gather` records
//! `None`, and the file may not be edited by hand (a hand edit is exactly what `check` exists
//! to catch).  `sign` is the one tool path that fills it in.  It takes a JOINED recording, the
//! output of `join` over the shards of one workflow run, and returns the same file with the
//! banner added and nothing else changed:
//!
//! ```text
//!   joined.json (bytes B) --sign--> signed.json
//!        every row, every meta field, byte for byte the same,
//!        except   "regeneration": null
//!                 becomes { reviewer, host, run_id, run_attempt, head_sha, unsigned_sha256 }
//!        where    unsigned_sha256 = SHA-256(B)        (the digest the join job printed)
//! ```
//!
//! ## What the banner proves, and what it cannot
//!
//! It proves that this exact file, with these rows, is the one the named workflow run recorded
//! (`unsigned_sha256` is recomputed by `validate`, so a row edited afterwards is caught), and
//! that the reviewer typed the run's id and attempt and the revision it recorded.  It does NOT
//! prove who the reviewer is: the name is self-reported (VOS30 section 9.6).  The process control
//! around it (the reviewer lists every record-mode run, section 9.12.2 item 7) is what makes the
//! name mean something.
//!
//! ## The verify evidence (VOS-028c3c-3b-1b, VOS-D141)
//!
//! A recording is signed together with the joined VERIFY file that reproduced it at the reviewer's
//! checkout.  The recorded `trees` need no longer equal the working tree's (a shared crate may have
//! moved since the recording); the verify file is what vouches for the tree at hand:
//!
//! ```text
//!   recorded.json  --(verify at the checkout, 64 shards, join)-->  verify.json
//!        sign refuses unless verify.json
//!          * hashes to --expect-verify-sha256 (the bytes read once, hashed as read),
//!          * is an authentic joined verify file of THIS recording (verifies_sha256),
//!          * ran at the working tree, with the recording's shards and jobs,
//!          * reproduces every row (compare_verify) and the decision (decision_differences)
//!        and the banner then records both anchors: the recording run and the verify run.
//! ```
//!
//! The authentication and comparison are [`crate::cli::compare_rerun`], the very function
//! `decision-diff` runs in the workflow, so the reviewer re-makes the fan-in's judgement from data
//! and the two cannot disagree.
//!
//! The whole of it is a pure function over the input texts, so every refusal is a unit test:
//! the two things that need the outside world, the working tree's ids and "the tree ids at this
//! commit" (VOS-D137), are passed in.  Reading the files, the `--out` write and the flags are
//! `cli::cmd_sign`'s job.

use crate::cli::{compare_rerun, missing_tree_refusals, RerunLabels};
use crate::results::{
    full_plan_covered, is_banner_text, is_hex, is_run_number, validate_classes, Regeneration,
    Results, Verified,
};
use coding_adventures_sha256::sha256_hex;

/// The values `sign` is given on its command line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignRequest {
    /// `--reviewer`.
    pub reviewer: String,
    /// `--host`.
    pub host: String,
    /// `--run-id`.
    pub run_id: String,
    /// `--run-attempt`.
    pub run_attempt: String,
    /// `--head-sha`.
    pub head_sha: String,
    /// `--expect-sha256`: the digest of the joined file the join job printed.  REQUIRED: `sign`
    /// refuses `None`, so a signature is always tied to the file the join job produced.
    pub expect_sha256: Option<String>,
    /// `--verify-file`: the TEXT of the joined verify file, read once by the caller.  `sign`
    /// hashes these very bytes for `--expect-verify-sha256` and parses these very bytes, so the
    /// file cannot change between the digest check and its use.  REQUIRED: `sign` refuses `None`.
    pub verify_text: Option<String>,
    /// `--expect-verify-sha256`: the digest of the joined verify file the verify run printed.
    /// REQUIRED, as `expect_sha256` is.
    pub expect_verify_sha256: Option<String>,
}

/// The tree ids at a commit: `(name, id)` pairs, or the refusal (`meta::trees_at`).
pub type TreesAt = dyn Fn(&str) -> Result<Vec<(String, String)>, String>;

/// Sign `input` (the text of a joined recording).  `current` is the working tree's ids
/// (`meta::working_trees`) and `trees_at` answers "what are the input ids at this 40-hex
/// commit" (`meta::trees_at`).  `Err` is a refusal: the caller exits 2 and writes
/// nothing.
pub fn sign(
    input: &str,
    req: &SignRequest,
    current: &[(String, String)],
    trees_at: &TreesAt,
) -> Result<Results, String> {
    // 1. The words and numbers typed on the command line are well formed.  The reviewer and the
    //    host end up spliced into the spec's Markdown, so their charset is closed (VOS-D120).
    for (what, s) in [("--reviewer", &req.reviewer), ("--host", &req.host)] {
        if !is_banner_text(s) {
            return Err(format!(
                "sign refuses: {what} must be 1 to 64 characters of letters, digits, spaces and \
                 . _ , ' @ - (no leading, trailing or doubled space)"
            ));
        }
    }
    for (what, s) in [
        ("--run-id", &req.run_id),
        ("--run-attempt", &req.run_attempt),
    ] {
        if !is_run_number(s) {
            return Err(format!("sign refuses: {what} must be 1 to 20 ASCII digits"));
        }
    }
    if !is_hex(&req.head_sha, 40) {
        return Err("sign refuses: --head-sha must be a 40-hex revision".into());
    }

    // 2. The input is the file the reviewer thinks it is: its bytes hash to the digest the join
    //    job printed.
    let digest = sha256_hex(input.as_bytes());
    let Some(want) = &req.expect_sha256 else {
        return Err(
            "sign refuses: --expect-sha256 is required (the joined file's SHA-256 the join job \
             printed)"
                .into(),
        );
    };
    if !is_hex(want, 64) {
        return Err("sign refuses: --expect-sha256 must be 64 hex digits".into());
    }
    if !want.eq_ignore_ascii_case(&digest) {
        return Err(format!(
            "sign refuses: the file's SHA-256 is {digest}, not the expected {want}"
        ));
    }
    // The same two checks for the verify file, on the bytes the caller read once.
    let Some(verify_text) = &req.verify_text else {
        return Err(
            "sign refuses: --verify-file is required (the joined verify file of a green verify of \
             this recording)"
                .into(),
        );
    };
    let Some(vwant) = &req.expect_verify_sha256 else {
        return Err(
            "sign refuses: --expect-verify-sha256 is required (the joined verify file's SHA-256 \
             the verify run printed)"
                .into(),
        );
    };
    if !is_hex(vwant, 64) {
        return Err("sign refuses: --expect-verify-sha256 must be 64 hex digits".into());
    }
    let vdigest = sha256_hex(verify_text.as_bytes());
    if !vwant.eq_ignore_ascii_case(&vdigest) {
        return Err(format!(
            "sign refuses: the verify file's SHA-256 is {vdigest}, not the expected {vwant}"
        ));
    }

    // 3. Canonical, so that `validate` can render the file again without the banner and reach
    //    these very bytes.
    let mut res = Results::from_canonical_text(input).map_err(|e| format!("sign refuses: {e}"))?;
    if res.meta.regeneration.is_some() {
        return Err("sign refuses: the file already carries a regeneration banner".into());
    }
    if !res.meta.merges.is_empty() {
        return Err("sign refuses: the file records merges (a banner attests a joined run)".into());
    }

    // 4. It is a recording of THIS run: a joined `record` file whose run id and attempt are the
    //    ones typed.  A local run (no run id), a shard, a verify file or another run's file is
    //    not something a reviewer signs.
    let Some(rec) = &res.meta.recording else {
        return Err(
            "sign refuses: the file has no recording block (a local run is not signed)".into(),
        );
    };
    if rec.mode != "record" {
        return Err(format!(
            "sign refuses: the recording's mode is {:?}, only a `record` file is signed",
            rec.mode
        ));
    }
    if rec.shards.is_none() || rec.shard.is_some() {
        return Err("sign refuses: the file is not a joined recording (run `join` first)".into());
    }
    if rec.run_id.as_deref() != Some(req.run_id.as_str()) {
        return Err(format!(
            "sign refuses: --run-id {} is not the recording's run id {:?}",
            req.run_id, rec.run_id
        ));
    }
    if rec.run_attempt.as_deref() != Some(req.run_attempt.as_str()) {
        return Err(format!(
            "sign refuses: --run-attempt {} is not the recording's run attempt {:?}",
            req.run_attempt, rec.run_attempt
        ));
    }

    // 5. The revision typed is the one recorded (the tree check against it is in step 6).
    let repo = res
        .meta
        .revisions
        .iter()
        .find(|(k, _)| k == "repo")
        .map(|(_, v)| v.as_str());
    if repo != Some(req.head_sha.as_str()) {
        return Err(format!(
            "sign refuses: --head-sha {} is not the recorded repo revision {}",
            req.head_sha,
            repo.unwrap_or("(none)")
        ));
    }

    // 6. The file is fit to sign: structurally valid, describing the tree at hand, covering the
    //    whole plan.  Row findings do not refuse: they are kept row outcomes (VOS30 9.12.1).
    let v = validate_classes(&res);
    if let Some(m) = v.structural.first() {
        return Err(format!(
            "sign refuses: structurally invalid ({} finding(s)); first: {m}",
            v.structural.len()
        ));
    }
    // Only the MISSING half of `full_refusals`: the recorded ids must be present and well formed
    // and the checkout must have no uncommitted input, but they may differ from the working
    // tree's (VOS-D141).  What vouches for the working tree is the verify file, below.
    if let Some(m) = missing_tree_refusals(&res, current).first() {
        return Err(format!("sign refuses: {m}"));
    }
    // The recorded trees are present and well formed (above; since VOS-D141 they need not equal
    // the working tree's, the verify file vouches for that); they must also be the trees AT the
    // typed commit.  Not ancestry (VOS-D137): main is squash-merged, so the commit a
    // CI recording ran at is never in main's history, while the content it ran on is.  A tree id
    // is a hash of content, so equal ids are equal content, which survives a squash.
    let at_head_sha = trees_at(&req.head_sha).map_err(|e| format!("sign refuses: {e}"))?;
    // Name the first tree that differs, so the reviewer sees what moved; a recorded set of a
    // different size differs too.
    let differs = |k: &str, v: &str| {
        at_head_sha
            .iter()
            .find(|(n, _)| n == k)
            .map(|(_, x)| x.as_str())
            != Some(v)
    };
    let first = res.meta.trees.iter().find(|(k, v)| differs(k, v));
    if first.is_some() || at_head_sha.len() != res.meta.trees.len() {
        let first = first.map_or("the set of trees".to_string(), |(k, _)| k.clone());
        return Err(format!(
            "sign refuses: the tree ids at {} differ from the recorded `trees` ({first}); \
             --head-sha must be the commit the recording ran at",
            req.head_sha
        ));
    }
    if let Some(m) = full_plan_covered(&res).first() {
        return Err(format!("sign refuses: the full plan is not covered: {m}"));
    }

    // 7. The verify evidence: this recording reproduces at the working tree.
    let verified = verify_evidence(&res, verify_text, &vdigest, current, trees_at)?;

    // 8. Sign.  Only the banner changes.
    res.meta.regeneration = Some(Regeneration {
        reviewer: req.reviewer.clone(),
        host: req.host.clone(),
        run_id: req.run_id.clone(),
        run_attempt: req.run_attempt.clone(),
        head_sha: req.head_sha.clone(),
        unsigned_sha256: digest,
        verified,
    });
    Ok(res)
}

/// Step 7 of [`sign`]: is `verify_text` an authentic joined verify file that reproduces `recorded`
/// at the working tree `current`?  Returns the banner's `verified` record, taken from the verify
/// file itself, or the refusal.  `vdigest` is the SHA-256 of `verify_text` (already compared with
/// `--expect-verify-sha256`).
fn verify_evidence(
    recorded: &Results,
    verify_text: &str,
    vdigest: &str,
    current: &[(String, String)],
    trees_at: &TreesAt,
) -> Result<Verified, String> {
    let refuse = |m: String| Err(format!("sign refuses: {m}"));
    let j = Results::from_canonical_text(verify_text)
        .map_err(|e| format!("sign refuses: --verify-file: {e}"))?;
    // Everything `decision-diff` asks of a re-run, and the row and decision comparison, in the one
    // shared function: a joined verify-mode file with no structural finding and no merges, at a
    // committed tree, whose `verifies_sha256` is the unsigned digest of the recording, holding the
    // recording's row identities.  (No `--expect-shards`: the recording's own `shards` is
    // compared below, which is stricter.)
    let found = compare_rerun(
        recorded,
        &j,
        None,
        &RerunLabels {
            recorded: "FILE",
            rerun: "--verify-file",
        },
    )
    .map_err(|m| format!("sign refuses: {m}"))?;
    // The core guarantees a verify-mode joined recording block; the recorded side is a joined
    // `record` file (checked above).  The verify must have fanned out exactly as the recording
    // did: same shard count, same `--jobs` (VOS30 section 9.2: record and verify must agree on the
    // parallelism the timings were taken at).
    let (Some(jr), Some(rr)) = (&j.meta.recording, &recorded.meta.recording) else {
        return refuse("--verify-file or FILE has no recording block".into());
    };
    if jr.shards != rr.shards {
        return refuse(format!(
            "--verify-file joins {:?} shards but the recording joins {:?}",
            jr.shards, rr.shards
        ));
    }
    if jr.jobs != rr.jobs {
        return refuse(format!(
            "--verify-file ran at --jobs {} but the recording ran at --jobs {}",
            jr.jobs, rr.jobs
        ));
    }
    // It vouches for the tree the reviewer has checked out: equal ids, entry for entry.  (A
    // `+uncommitted` or legacy-named verify tree was refused by the core; the working tree itself
    // was refused above if it had an uncommitted input.)
    if j.meta.trees != current {
        let first = j
            .meta
            .trees
            .iter()
            .find(|(k, v)| {
                current
                    .iter()
                    .find(|(n, _)| n == k)
                    .map(|(_, x)| x.as_str())
                    != Some(v.as_str())
            })
            .map_or("the set of trees".to_string(), |(k, _)| k.clone());
        return refuse(format!(
            "--verify-file ran at a different tree than the working tree ({first}); check out \
             the commit the verify ran at, or verify again at this one"
        ));
    }
    // Any row or decision difference is a finding, not evidence.  The lines are those of
    // `decision-diff`; the first of each kind is named so the reviewer can see what moved.
    if !found.is_clean() {
        let first = found
            .row_diffs
            .first()
            .or(found.decision_diffs.first())
            .map_or("", String::as_str);
        return refuse(format!(
            "--verify-file does not reproduce the recording: {} row difference(s) and {} decision \
             difference(s); first: {first} (run `decision-diff` for the full list)",
            found.row_diffs.len(),
            found.decision_diffs.len()
        ));
    }
    // The verify's own run, from its own recording block and its own revision.
    let (Some(run_id), Some(run_attempt)) = (&jr.run_id, &jr.run_attempt) else {
        return refuse(
            "--verify-file carries no run id and attempt (a local verify is not cited)".into(),
        );
    };
    if !is_run_number(run_id) || !is_run_number(run_attempt) {
        return refuse("--verify-file's run id and attempt must be 1 to 20 ASCII digits".into());
    }
    let head = j
        .meta
        .revisions
        .iter()
        .find(|(k, _)| k == "repo")
        .map(|(_, v)| v.as_str());
    let Some(head_sha) = head.filter(|h| is_hex(h, 40)) else {
        return refuse(format!(
            "--verify-file's repo revision {} is not a 40-hex revision",
            head.unwrap_or("(none)")
        ));
    };
    // The revision the banner will cite must be the commit whose input ids are the verify's
    // `trees` (as step 6 does for the recording, VOS-D137: content, not ancestry), so the banner
    // cannot misreport where the verify ran.  A commit missing locally is refused with the fetch
    // hint `trees_at` gives.
    let at_verify = trees_at(head_sha).map_err(|e| format!("sign refuses: {e}"))?;
    if at_verify != j.meta.trees {
        let first = j
            .meta
            .trees
            .iter()
            .find(|(k, v)| {
                at_verify
                    .iter()
                    .find(|(n, _)| n == k)
                    .map(|(_, x)| x.as_str())
                    != Some(v.as_str())
            })
            .map_or("the set of trees".to_string(), |(k, _)| k.clone());
        return refuse(format!(
            "the tree ids at the verify's revision {head_sha} differ from the verify file's `trees` ({first}); its repo revision must be the commit the verify ran at"
        ));
    }
    Ok(Verified {
        run_id: run_id.clone(),
        run_attempt: run_attempt.clone(),
        head_sha: head_sha.to_string(),
        verify_sha256: vdigest.to_string(),
        trees: j.meta.trees.clone(),
    })
}
