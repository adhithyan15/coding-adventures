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
//! The whole of it is a pure function over the input text, so every refusal is a unit test:
//! the two things that need the outside world, the working tree's ids and "the tree ids at this
//! commit" (VOS-D137), are passed in.  Reading the file, the `--out` write and the flags are
//! `cli::cmd_sign`'s job.

use crate::cli::full_refusals;
use crate::results::{
    full_plan_covered, is_banner_text, is_hex, is_run_number, validate_classes, Regeneration,
    Results,
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
    if let Some(m) = full_refusals(&res, current).first() {
        return Err(format!("sign refuses: {m}"));
    }
    // The recorded trees are present and are the working tree's (above); they must also be the
    // trees AT the typed commit.  Not ancestry (VOS-D137): main is squash-merged, so the commit a
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

    // 7. Sign.  Only the banner changes.
    res.meta.regeneration = Some(Regeneration {
        reviewer: req.reviewer.clone(),
        host: req.host.clone(),
        run_id: req.run_id.clone(),
        run_attempt: req.run_attempt.clone(),
        head_sha: req.head_sha.clone(),
        unsigned_sha256: digest,
    });
    Ok(res)
}
