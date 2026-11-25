//! # `results`: the results file, the green predicate, `check` and the decision
//!
//! `code/specs/data/vos30-capacity-results.json` is written only by this tool.  This module
//! owns four things:
//!
//! 1. the file's shape (schema 2, VOS30 section 9.6) and its reader and writer;
//! 2. [`green`], the predicate of section 8.3, computed from a row's other fields and never typed;
//! 3. [`validate`] and [`rerun`], the two halves of `check`: the schema and guard rules every row
//!    must satisfy, and the re-execution of every quick-tier row against its recorded digest,
//!    verdict, control and conflict count;
//! 4. [`decide`], the section 8.3 decision procedure applied to the rows.
//!
//! ## What `check` can and cannot verify
//!
//! `dirty_tree`, `binary_sha256` and the regeneration banner are *self-reported by the tool*: an
//! edited tool or file can write any value it likes.  Reviewer regeneration of the full tier is
//! therefore a **process control**, not a technical one.  What `check` does verify technically is
//! that every quick-tier row reproduces from `(experiment, knob, seed)`, bit for bit in its digest
//! and in its conflict count, and that every row obeys the guards.

use crate::exp::{
    is_segmented, knobs, plan, row_wall_s, run_row, RowSpec, Tier, CHAIN_EXPERIMENTS, EXPERIMENTS,
};
use crate::meta::TREE_ENTRIES;
use crate::row::{Control, Knob, Row};
use crate::solve::{CONFLICT_SLICE, PROOF_CONFLICT_CAP};
use coding_adventures_bounded_json::{parse, JsonNumber, JsonValue};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

/// The schema number of the file.
pub const SCHEMA: i64 = 2;
/// Seeds required per knob.
pub const MIN_SEEDS: u32 = 5;
/// The solver's own wall timeout per row, seconds.
pub const TIMEOUT_S: u64 = 120;
/// The resident-memory cap per child, KiB (2 GiB).
pub const RSS_CAP_KB: u64 = 2 * 1024 * 1024;

/// The green thresholds of section 8.3.
pub const GREEN_MAX_CONFLICTS: u64 = 100_000;
/// Solve time of one obligation, milliseconds.
pub const GREEN_MAX_SOLVE_MS: f64 = 10_000.0;
/// DRAT of one obligation, bytes (32 MiB).
pub const GREEN_MAX_DRAT: u64 = 32 * 1024 * 1024;
/// Peak resident memory, KiB (1 GiB).
pub const GREEN_MAX_RSS_KB: u64 = 1024 * 1024;

/// The route-budget conflict cap of VOS30 section 8.6.1 (frozen pre-registered constant).
pub const ROUTE_MAX_CONFLICTS: u64 = 50_000;
/// The route-budget checker-work cap of VOS30 section 8.6.1 (2^31, half of `DEFAULT_MAX_WORK`).
pub const ROUTE_MAX_CHECK_WORK: u64 = 1 << 31;

/// The B5 clause of VOS30 section 8.6.3: a B5 row's total, the sweep's whole elapsed time
/// recorded as the `sweep_ms` metric (instance build through verdict: simulation, AIG
/// construction, every lemma solve and proof check, the final obligation; the G3 control is
/// excluded), must be strictly under 600 s (milliseconds).  A gate, pre-registered; the B5
/// measurement walls (`exp::B5_ROW_WALL_S`) are looser than it so that a slow sweep is
/// recorded, then judged here.  Fail-closed: a row without a finite, non-negative `sweep_ms`
/// is not route-green.
pub const B5_ROUTE_MAX_TOTAL_MS: f64 = 600_000.0;

/// Everything the file records besides the rows.
#[derive(Clone, Debug, PartialEq)]
pub struct Meta {
    /// Tool name.
    pub name: String,
    /// Tool version.
    pub version: String,
    /// SHA-256 of the running executable (self-reported).
    pub binary_sha256: String,
    /// Whether the working tree was dirty at the start of the run (self-reported).
    pub dirty_tree: bool,
    /// Target triple.
    pub triple: String,
    /// `rustc --version`.
    pub rustc: String,
    /// Operating system.
    pub os: String,
    /// Git revisions by crate name.
    pub revisions: Vec<(String, String)>,
    /// The reviewer's sign-off, if `sign` has written one (VOS-D120); `null` in the file otherwise.
    pub regeneration: Option<Regeneration>,
    /// The subset refreshes applied to the file by `merge` (VOS30 section 9.2), oldest first.
    pub merges: Vec<Merge>,
    /// How the file was produced (a shard, a join), if recorded; omitted from the file when `None`.
    pub recording: Option<Recording>,
    /// Content-addressed tree ids (`git rev-parse HEAD:<path>`) of the probe and its four solver
    /// crates when the run was made (VOS-D097), empty for a file recorded before 0.2.0.  A tree
    /// id, not a revision, because the repository squash-merges and a recorded revision is
    /// generally not an ancestor of HEAD.
    pub trees: Vec<(String, String)>,
}

/// How this file came to be, for `join` to check (VOS30 section 9.2, VOS-D125, VOS-D127 L1).
/// Optional: a file recorded before VOS-028c3c-2b has none, and then the key is omitted, so the
/// committed file keeps its bytes and the schema number stays 2.  Self-reported like the rest of
/// `Meta`; its job is to let a workflow bind a shard to its run and attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recording {
    /// `record` (a `run`) or `verify` (a re-run of a recording, VOS-028c3c-2b-ii).
    pub mode: String,
    /// The workflow run id: ASCII digits, 1 to 20.  `None` for a local run.  Always from an
    /// explicit flag, never from the environment.
    pub run_id: Option<String>,
    /// The workflow run attempt, same form.  `None` for a local run.
    pub run_attempt: Option<String>,
    /// `(K, N)` for one shard's file.  Exactly one of `shard` and `shards` is set.
    pub shard: Option<(u32, u32)>,
    /// `N` for a joined file.
    pub shards: Option<u32>,
    /// `--jobs` the rows ran at (record and verify must agree, VOS30 section 9.2).
    pub jobs: u32,
    /// Verify mode only (VOS-D141, VOS-028c3c-3b-1a): the SHA-256 (64 hex digits) of the
    /// RECORDED file this verification re-ran, rendered WITHOUT its signing banner
    /// ([`Results::unsigned_sha256`]).  Without the banner, so the value is the same before and
    /// after `sign`: the reviewer signs the file a verify already vouched for.  `join` requires
    /// it equal across shards and `decision-diff` requires it to equal the digest of `--recorded`,
    /// which is what binds a verification to the recording it re-ran now that the verify's tree
    /// ids may legitimately differ from the recorded ones.  `None` in a record-mode file (and in
    /// a verify file written before this field existed, which `decision-diff` therefore refuses).
    pub verifies_sha256: Option<String>,
}

/// The modes a `recording` may carry.
pub const RECORDING_MODES: [&str; 2] = ["record", "verify"];

/// Is `s` a run id or attempt: 1 to 20 ASCII digits and nothing else?  A GitHub run id is a
/// decimal integer, and refusing every other shape keeps a value that came from an
/// attacker-controlled string out of a file the tool later splices into a spec.
pub fn is_run_number(s: &str) -> bool {
    (1..=20).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit())
}

impl Recording {
    fn to_json(&self) -> JsonValue {
        let mut e = vec![("mode", string(&self.mode))];
        if let Some(r) = &self.run_id {
            e.push(("run_id", string(r)));
        }
        if let Some(a) = &self.run_attempt {
            e.push(("run_attempt", string(a)));
        }
        if let Some((k, n)) = self.shard {
            e.push(("shard", string(&format!("{k}/{n}"))));
        }
        if let Some(n) = self.shards {
            e.push(("shards", int(i64::from(n))));
        }
        e.push(("jobs", int(i64::from(self.jobs))));
        if let Some(h) = &self.verifies_sha256 {
            e.push(("verifies_sha256", string(h)));
        }
        obj(e)
    }

    fn from_json(v: &JsonValue) -> Result<Recording, String> {
        let JsonValue::Object(p) = v else {
            return Err("recording is not an object".into());
        };
        const KEYS: [&str; 7] = [
            "mode",
            "run_id",
            "run_attempt",
            "shard",
            "shards",
            "jobs",
            "verifies_sha256",
        ];
        if let Some((k, _)) = p.iter().find(|(k, _)| !KEYS.contains(&k.as_str())) {
            return Err(format!("recording has an unknown key {k:?}"));
        }
        let find = |k: &str| p.iter().find(|(n, _)| n == k).map(|(_, x)| x);
        let text = |k: &str| -> Result<Option<String>, String> {
            match find(k) {
                None => Ok(None),
                Some(JsonValue::String(s)) => Ok(Some(s.clone())),
                Some(_) => Err(format!("recording {k} is not a string")),
            }
        };
        let number = |k: &str| -> Result<Option<u32>, String> {
            match find(k) {
                None => Ok(None),
                Some(JsonValue::Number(JsonNumber::Integer(i))) => u32::try_from(*i)
                    .map(Some)
                    .map_err(|_| format!("recording {k} is out of range")),
                Some(_) => Err(format!("recording {k} is not an integer")),
            }
        };
        let mode = text("mode")?.ok_or("recording has no mode")?;
        if !RECORDING_MODES.contains(&mode.as_str()) {
            return Err(format!("recording mode {mode:?} is not record or verify"));
        }
        let run_id = text("run_id")?;
        let run_attempt = text("run_attempt")?;
        for (what, x) in [("run_id", &run_id), ("run_attempt", &run_attempt)] {
            if x.as_deref().is_some_and(|s| !is_run_number(s)) {
                return Err(format!("recording {what} is not 1 to 20 ASCII digits"));
            }
        }
        if run_id.is_some() != run_attempt.is_some() {
            return Err("recording has one of run_id and run_attempt, not both".into());
        }
        let shard = text("shard")?
            .map(|s| crate::exp::parse_shard(&s))
            .transpose()?;
        let shards = number("shards")?;
        if shard.is_some() == shards.is_some() {
            return Err("recording must have exactly one of shard and shards".into());
        }
        if shards.is_some_and(|n| n == 0 || n > crate::exp::MAX_SHARDS) {
            return Err("recording shards is out of range".into());
        }
        let jobs = number("jobs")?.ok_or("recording has no jobs")?;
        // Only a verification re-runs a recording, so only a verify-mode file can name the one it
        // verified; a record-mode file carrying the field was edited by hand.
        let verifies_sha256 = text("verifies_sha256")?;
        if let Some(h) = &verifies_sha256 {
            if mode != "verify" {
                return Err("recording verifies_sha256 is only for mode verify".into());
            }
            if !is_hex(h, 64) {
                return Err("recording verifies_sha256 is not 64 hex digits".into());
            }
        }
        Ok(Recording {
            mode,
            run_id,
            run_attempt,
            shard,
            shards,
            jobs,
            verifies_sha256,
        })
    }
}

/// The reviewer's sign-off written by `sign` (VOS-D120, VOS30 section 9.2): who regenerated the
/// full tier, on which host, which workflow run recorded it, at which revision, and the digest of
/// the file as it was BEFORE the banner went in.  Optional and `null` in the committed file, so
/// that file keeps its bytes; self-reported like the rest of `Meta` (VOS30 section 9.6): it binds
/// the signature to the rows and the run, never to the person.
///
/// ```text
///   unsigned file ──sha256──▶ unsigned_sha256 ──┐
///   unsigned file ──sign────▶ unsigned file + banner (reviewer, host, run, head, digest)
///   validate: banner removed, file rendered again, sha256 == unsigned_sha256 ?
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Regeneration {
    /// The reviewer's name, as typed (see [`is_banner_text`]).
    pub reviewer: String,
    /// The host's label, as typed (same rule).
    pub host: String,
    /// The workflow run id of the recording (ASCII digits, 1 to 20).
    pub run_id: String,
    /// The workflow run attempt of the recording (same form).
    pub run_attempt: String,
    /// The recorded repository revision (40 hex digits), equal to `revisions.repo`.
    pub head_sha: String,
    /// SHA-256 (64 hex digits) of the bytes of the file `sign` read, i.e. the file without the
    /// banner; `validate` recomputes it, so a row edited after signing is caught.
    pub unsigned_sha256: String,
    /// The verify run that vouched for this recording at the reviewer's checkout (VOS-028c3c-3b-1b,
    /// VOS-D141).  REQUIRED in any banner: no file was signed before the field existed.
    pub verified: Verified,
}

/// The second anchor of a signed file (VOS-028c3c-3b-1b, VOS-D141): the joined verify file the
/// reviewer cited.  A verify at the checkout runs at a tree that may differ from the recorded one,
/// so the banner must name WHERE it ran as well as which recording it vouches for:
///
/// ```text
///   recording  run_id/run_attempt/head_sha        (the top of the banner: where it was recorded)
///   verify     verified.run_id/run_attempt/head_sha/trees   (where the rows were reproduced)
///              verified.verify_sha256               (the joined verify file's SHA-256)
/// ```
///
/// Like the rest of the banner it is self-reported evidence; `sign` takes every field from a file
/// it has authenticated and `validate` checks only the shape (the verify file is not in this one,
/// so its digest cannot be recomputed here).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Verified {
    /// The verify workflow run id (ASCII digits, 1 to 20): the joined verify file's
    /// `recording.run_id`.
    pub run_id: String,
    /// The verify workflow run attempt (same form).
    pub run_attempt: String,
    /// The commit the verify ran at (40 hex digits): the joined verify file's `revisions.repo`.
    pub head_sha: String,
    /// SHA-256 (64 hex digits) of the joined verify file's bytes, as the reviewer passed it in
    /// `--expect-verify-sha256`.
    pub verify_sha256: String,
    /// The input ids the verify ran at, in the form of [`Meta::trees`]; they equal the reviewer's
    /// working tree when `sign` ran, and the guard's rule 2 accepts them in place of `tool.trees`.
    pub trees: Vec<(String, String)>,
}

impl Verified {
    fn to_json(&self) -> JsonValue {
        obj(vec![
            ("run_id", string(&self.run_id)),
            ("run_attempt", string(&self.run_attempt)),
            ("head_sha", string(&self.head_sha)),
            ("verify_sha256", string(&self.verify_sha256)),
            ("trees", trees_json(&self.trees)),
        ])
    }

    /// Strict, like [`Regeneration::from_json`]: exactly the five keys; their VALUES are checked
    /// by `validate`, so a malformed one is a finding and not an unreadable file.
    fn from_json(v: &JsonValue) -> Result<Verified, String> {
        let JsonValue::Object(p) = v else {
            return Err("regeneration verified is not an object".into());
        };
        const KEYS: [&str; 5] = [
            "run_id",
            "run_attempt",
            "head_sha",
            "verify_sha256",
            "trees",
        ];
        if let Some((k, _)) = p.iter().find(|(k, _)| !KEYS.contains(&k.as_str())) {
            return Err(format!("regeneration verified has an unknown key {k:?}"));
        }
        let get = |k: &str| -> Result<&JsonValue, String> {
            p.iter()
                .find(|(n, _)| n == k)
                .map(|(_, x)| x)
                .ok_or_else(|| format!("regeneration verified has no {k}"))
        };
        let text = |k: &str| -> Result<String, String> {
            match get(k)? {
                JsonValue::String(s) => Ok(s.clone()),
                _ => Err(format!("regeneration verified {k} is not a string")),
            }
        };
        Ok(Verified {
            run_id: text("run_id")?,
            run_attempt: text("run_attempt")?,
            head_sha: text("head_sha")?,
            verify_sha256: text("verify_sha256")?,
            trees: trees_from(get("trees")?)?,
        })
    }
}

/// May `s` appear in a banner's `reviewer` or `host`?  A strict WHITELIST: 1 to 64 characters,
/// each an ASCII letter, an ASCII digit, a space or one of `. _ , ' @ -`; no leading or trailing
/// space; no two spaces in a row.
///
/// `table` splices the banner into the spec's Markdown.  A blacklist of the four characters
/// `|`, backtick, `<` and `>` is not enough there: `[Dr X](https://evil.example)`, `![a](x)`, a
/// bare `https://x`, `a*b` and `#x` all pass it and still make a link, an image, emphasis or a
/// heading.  Those need one of `[ ] ( ) ! * # : / & ~ = +`, none of which is in the whitelist,
/// and neither is any control character or non-ASCII byte, so a banner can only be text.
pub fn is_banner_text(s: &str) -> bool {
    (1..=64).contains(&s.len())
        && s.bytes().all(|b| {
            b.is_ascii_alphanumeric()
                || matches!(b, b' ' | b'.' | b'_' | b',' | b'\'' | b'@' | b'-')
        })
        && !s.starts_with(' ')
        && !s.ends_with(' ')
        && !s.contains("  ")
}

/// `n` hex digits and nothing else?
pub fn is_hex(s: &str, n: usize) -> bool {
    s.len() == n && s.bytes().all(|b| b.is_ascii_hexdigit())
}

impl Regeneration {
    fn to_json(&self) -> JsonValue {
        obj(vec![
            ("reviewer", string(&self.reviewer)),
            ("host", string(&self.host)),
            ("run_id", string(&self.run_id)),
            ("run_attempt", string(&self.run_attempt)),
            ("head_sha", string(&self.head_sha)),
            ("unsigned_sha256", string(&self.unsigned_sha256)),
            ("verified", self.verified.to_json()),
        ])
    }

    /// Strict: exactly the seven keys, the first six strings and `verified` an object.  (Their
    /// VALUES are checked by `validate`, so that a malformed banner is a finding and not an
    /// unreadable file.)
    fn from_json(v: &JsonValue) -> Result<Regeneration, String> {
        let JsonValue::Object(p) = v else {
            return Err("regeneration is not an object".into());
        };
        const KEYS: [&str; 7] = [
            "reviewer",
            "host",
            "run_id",
            "run_attempt",
            "head_sha",
            "unsigned_sha256",
            "verified",
        ];
        if let Some((k, _)) = p.iter().find(|(k, _)| !KEYS.contains(&k.as_str())) {
            return Err(format!("regeneration has an unknown key {k:?}"));
        }
        let text = |k: &str| -> Result<String, String> {
            match p.iter().find(|(n, _)| n == k) {
                Some((_, JsonValue::String(s))) => Ok(s.clone()),
                Some(_) => Err(format!("regeneration {k} is not a string")),
                None => Err(format!("regeneration has no {k}")),
            }
        };
        Ok(Regeneration {
            reviewer: text("reviewer")?,
            host: text("host")?,
            run_id: text("run_id")?,
            run_attempt: text("run_attempt")?,
            head_sha: text("head_sha")?,
            unsigned_sha256: text("unsigned_sha256")?,
            verified: match p.iter().find(|(n, _)| n == "verified") {
                Some((_, x)) => Verified::from_json(x)?,
                None => return Err("regeneration has no verified".into()),
            },
        })
    }
}

/// The structural findings about a file's banner: every field well formed, the file a JOINED
/// `record` recording whose run id, attempt and revision the banner repeats, and the digest equal
/// to the file's own digest with the banner taken out again.  Empty for an unsigned file.
///
/// The recomputation is what binds the banner to the rows: the text of `self` without the banner
/// is exactly the text `sign` read (`sign` demands canonical form, so the render is byte for
/// byte that input), so any edit of a row, of `meta` or of the recording after signing changes
/// the digest.
fn regeneration_violations(res: &Results) -> Vec<String> {
    let Some(g) = &res.meta.regeneration else {
        return Vec::new();
    };
    let mut v = Vec::new();
    // A banner attests a joined recording; `merge` clears it, so a file that records merges and
    // still carries one was edited by hand (VOS-D120).
    if !res.meta.merges.is_empty() {
        v.push("regeneration: a file with merges cannot carry a banner".to_string());
    }
    for (what, s) in [("reviewer", &g.reviewer), ("host", &g.host)] {
        if !is_banner_text(s) {
            v.push(format!(
                "regeneration: {what} is not 1 to 64 characters of letters, digits, spaces and . _ , ' @ - \
                 (no leading, trailing or doubled space)"
            ));
        }
    }
    for (what, s) in [("run_id", &g.run_id), ("run_attempt", &g.run_attempt)] {
        if !is_run_number(s) {
            v.push(format!("regeneration: {what} is not 1 to 20 ASCII digits"));
        }
    }
    if !is_hex(&g.head_sha, 40) {
        v.push("regeneration: head_sha is not a 40-hex revision".to_string());
    }
    if !is_hex(&g.unsigned_sha256, 64) {
        v.push("regeneration: unsigned_sha256 is not 64 hex digits".to_string());
    }
    // The verify anchor (VOS-028c3c-3b-1b): same shapes as the recording's, and `trees` held to the
    // rule a verify file's own trees must pass, so a banner cannot name an uncommitted or
    // legacy-named tree.  Its digest cannot be recomputed (the verify file is not in this one).
    let w = &g.verified;
    for (what, s) in [("run_id", &w.run_id), ("run_attempt", &w.run_attempt)] {
        if !is_run_number(s) {
            v.push(format!(
                "regeneration: verified {what} is not 1 to 20 ASCII digits"
            ));
        }
    }
    if !is_hex(&w.head_sha, 40) {
        v.push("regeneration: verified head_sha is not a 40-hex revision".to_string());
    }
    if !is_hex(&w.verify_sha256, 64) {
        v.push("regeneration: verified verify_sha256 is not 64 hex digits".to_string());
    }
    for m in crate::meta::committed_tree_refusals(&w.trees) {
        v.push(format!("regeneration: verified trees: {m}"));
    }
    match &res.meta.recording {
        Some(r)
            if r.mode == "record"
                && r.shards.is_some()
                && r.run_id.as_deref() == Some(g.run_id.as_str())
                && r.run_attempt.as_deref() == Some(g.run_attempt.as_str()) => {}
        _ => v.push(
            "regeneration: the file is not a joined record-mode recording of the banner's run \
             id and attempt"
                .to_string(),
        ),
    }
    if res
        .meta
        .revisions
        .iter()
        .find(|(k, _)| k == "repo")
        .map(|(_, r)| r.as_str())
        != Some(g.head_sha.as_str())
    {
        v.push("regeneration: head_sha is not the recorded repo revision".to_string());
    }
    let again = res.unsigned_sha256();
    if again != g.unsigned_sha256 {
        v.push(format!(
            "regeneration: unsigned_sha256 {} does not match the file without its banner ({again}): \
             the file was edited after it was signed",
            g.unsigned_sha256
        ));
    }
    v
}

/// One `merge`: which rows were refreshed, by which run.  The file's `tool`, `host` and
/// `revisions` stay the original provenance; a refreshed row's provenance is this record.
#[derive(Clone, Debug, PartialEq)]
pub struct Merge {
    /// The fresh run's `repo` revision.
    pub rev: String,
    /// The fresh run's binary digest (self-reported).
    pub binary_sha256: String,
    /// The fresh run's `dirty_tree` (must be false in a committed file).
    pub dirty_tree: bool,
    /// The experiment refreshed.
    pub experiment: String,
    /// The `field=value` selector, or empty.
    pub where_: String,
    /// The ids ([`row_text_id`]) of the rows replaced.
    pub rows: Vec<String>,
    /// UTC time of the merge.
    pub at: String,
    /// `Some("add")` for `merge --add` (VOS30 section 9.2), `None` for a refresh (the key is then
    /// omitted, so an existing entry keeps its bytes).
    pub kind: Option<String>,
    /// The fresh run's tree ids (as [`Meta::trees`]); empty for an entry recorded before 0.2.0.
    pub trees: Vec<(String, String)>,
}

/// The names a `trees` object must carry, in order: those of [`crate::meta::TREE_ENTRIES`] (the
/// build inputs whose git ids are recorded, VOS-D140).
pub const TREE_NAMES: [&str; TREE_ENTRIES.len()] = {
    let mut names = [""; TREE_ENTRIES.len()];
    let mut i = 0;
    while i < TREE_ENTRIES.len() {
        names[i] = TREE_ENTRIES[i].0;
        i += 1;
    }
    names
};

/// The names the five whole-crate tree ids carried before VOS-D140.  A merge entry recorded then
/// (the committed file has several) keeps them as history: [`validate`] accepts them in a
/// `merges[i]` record and nowhere else, and nothing ever compares them with a working tree.
pub const LEGACY_TREE_NAMES: [&str; 5] = [
    "vos-capacity-probe",
    "bit-blaster",
    "cdcl-sat",
    "drat-check",
    "riscv-simulator",
];

fn trees_json(t: &[(String, String)]) -> JsonValue {
    JsonValue::Object(t.iter().map(|(k, v)| (k.clone(), string(v))).collect())
}

fn trees_from(v: &JsonValue) -> Result<Vec<(String, String)>, String> {
    match v {
        JsonValue::Object(p) => p
            .iter()
            .map(|(k, x)| match x {
                JsonValue::String(s) => Ok((k.clone(), s.clone())),
                _ => Err("a tree id is not a string".to_string()),
            })
            .collect(),
        _ => Err("trees is not an object".into()),
    }
}

/// The text identity of a row, as used in `merges` and in messages.
pub fn row_text_id(r: &Row) -> String {
    format!(
        "{} {} {} seed {}",
        r.experiment,
        r.knob.key(),
        r.mode,
        r.seed
    )
}

/// The size cap on a results file read by `merge`, bytes.
pub const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;

/// The results file.
#[derive(Clone, Debug, PartialEq)]
pub struct Results {
    /// The run metadata.
    pub meta: Meta,
    /// The rows.
    pub rows: Vec<Row>,
}

/// The row verdict words the file may contain.
pub const RESULTS: [&str; 10] = [
    "unsat",
    "sat",
    "unknown",
    "collapsed",
    "refused",
    "killed",
    "oom",
    "crashed",
    "pass",
    "fail",
];

fn int(v: i64) -> JsonValue {
    JsonValue::Number(JsonNumber::Integer(v))
}
fn string(s: &str) -> JsonValue {
    JsonValue::String(s.to_string())
}
fn obj(pairs: Vec<(&str, JsonValue)>) -> JsonValue {
    JsonValue::Object(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

impl Results {
    /// The JSON document.
    pub fn to_json(&self) -> JsonValue {
        let m = &self.meta;
        let mut tool = vec![
            ("name", string(&m.name)),
            ("version", string(&m.version)),
            ("binary_sha256", string(&m.binary_sha256)),
            ("dirty_tree", JsonValue::Bool(m.dirty_tree)),
        ];
        if !m.trees.is_empty() {
            tool.push(("trees", trees_json(&m.trees)));
        }
        let mut top = vec![
            ("schema", int(SCHEMA)),
            ("tool", obj(tool)),
            (
                "host",
                obj(vec![
                    ("triple", string(&m.triple)),
                    ("rustc", string(&m.rustc)),
                    ("os", string(&m.os)),
                ]),
            ),
            (
                "revisions",
                JsonValue::Object(
                    m.revisions
                        .iter()
                        .map(|(k, v)| (k.clone(), string(v)))
                        .collect(),
                ),
            ),
            (
                "limits",
                obj(vec![
                    ("timeout_s", int(TIMEOUT_S as i64)),
                    ("conflict_slice", int(CONFLICT_SLICE as i64)),
                    ("proof_conflict_cap", int(PROOF_CONFLICT_CAP as i64)),
                    ("rss_cap_kb", int(RSS_CAP_KB as i64)),
                    ("seeds", int(MIN_SEEDS as i64)),
                    (
                        "drat_check",
                        obj(vec![
                            ("max_vars", int(drat_check::DEFAULT_MAX_VARS as i64)),
                            ("max_work", int(drat_check::DEFAULT_MAX_WORK as i64)),
                        ]),
                    ),
                ]),
            ),
            (
                "regeneration",
                match &m.regeneration {
                    Some(g) => g.to_json(),
                    None => JsonValue::Null,
                },
            ),
        ];
        if let Some(r) = &m.recording {
            top.push(("recording", r.to_json()));
        }
        if !m.merges.is_empty() {
            top.push((
                "merges",
                JsonValue::Array(
                    m.merges
                        .iter()
                        .map(|g| {
                            let mut e = vec![
                                ("rev", string(&g.rev)),
                                ("binary_sha256", string(&g.binary_sha256)),
                                ("dirty_tree", JsonValue::Bool(g.dirty_tree)),
                                ("experiment", string(&g.experiment)),
                                ("where", string(&g.where_)),
                                (
                                    "rows",
                                    JsonValue::Array(g.rows.iter().map(|x| string(x)).collect()),
                                ),
                                ("at", string(&g.at)),
                            ];
                            if let Some(k) = &g.kind {
                                e.push(("kind", string(k)));
                            }
                            if !g.trees.is_empty() {
                                e.push(("trees", trees_json(&g.trees)));
                            }
                            obj(e)
                        })
                        .collect(),
                ),
            ));
        }
        top.push((
            "rows",
            JsonValue::Array(self.rows.iter().map(Row::to_json).collect()),
        ));
        obj(top)
    }

    /// SHA-256 (64 hex digits) of this file rendered WITHOUT its signing banner: the digest `sign`
    /// records as `unsigned_sha256` (`sign` hashes the canonical text it read, which is exactly
    /// this render), that `validate` recomputes to catch an edit after signing, and that a
    /// verify-mode file names as `recording.verifies_sha256`.  One definition for all three, so
    /// the value is the same before and after `sign` by construction.
    pub fn unsigned_sha256(&self) -> String {
        let mut bare = self.clone();
        bare.meta.regeneration = None;
        coding_adventures_sha256::sha256_hex(bare.to_text().as_bytes())
    }

    /// The file text: one row per line so a diff shows the rows that changed.
    pub fn to_text(&self) -> String {
        let JsonValue::Object(top) = self.to_json() else {
            unreachable!()
        };
        let mut out = String::from("{\n");
        let n = top.len();
        for (i, (k, v)) in top.iter().enumerate() {
            let comma = if i + 1 < n { "," } else { "" };
            if k == "rows" {
                out.push_str("  \"rows\": [\n");
                if let JsonValue::Array(rows) = v {
                    for (j, r) in rows.iter().enumerate() {
                        let text = coding_adventures_bounded_json::serialize(r).unwrap_or_default();
                        out.push_str(&format!(
                            "    {text}{}\n",
                            if j + 1 < rows.len() { "," } else { "" }
                        ));
                    }
                }
                out.push_str(&format!("  ]{comma}\n"));
            } else {
                let text = coding_adventures_bounded_json::serialize(v).unwrap_or_default();
                out.push_str(&format!("  \"{k}\": {text}{comma}\n"));
            }
        }
        out.push_str("}\n");
        out
    }

    /// Read a results file that must be in the tool's own **canonical form** (round 2, finding
    /// 4): the text must equal `to_text` of what it parses to.  The JSON parser keeps the first of
    /// two duplicate keys and the reader ignores unknown ones, so a file can say one thing to the
    /// reader and another to a human or to a different parser; refusing every file the tool would
    /// not itself have written closes that (the committed file round-trips exactly).
    pub fn from_canonical_text(text: &str) -> Result<Results, String> {
        let res = Results::from_text(text)?;
        if res.to_text() != text {
            return Err(
                "the results file is not in canonical form: it differs from what the tool \
                 writes for the same content (a duplicate or unknown key, a reordered or \
                 re-spaced field, or a number written another way)"
                    .into(),
            );
        }
        Ok(res)
    }

    /// Read a results file.
    pub fn from_text(text: &str) -> Result<Results, String> {
        let v = parse(text).map_err(|e| format!("the results file is not JSON: {e}"))?;
        let JsonValue::Object(top) = &v else {
            return Err("the results file is not an object".into());
        };
        let get = |k: &str| {
            top.iter()
                .find(|(n, _)| n == k)
                .map(|(_, x)| x)
                .ok_or_else(|| format!("no field {k}"))
        };
        match get("schema")? {
            JsonValue::Number(JsonNumber::Integer(SCHEMA)) => {}
            _ => return Err(format!("schema is not {SCHEMA}")),
        }
        let sub = |v: &JsonValue, k: &str| -> Result<JsonValue, String> {
            match v {
                JsonValue::Object(p) => p
                    .iter()
                    .find(|(n, _)| n == k)
                    .map(|(_, x)| x.clone())
                    .ok_or_else(|| format!("no field {k}")),
                _ => Err("not an object".into()),
            }
        };
        let st = |v: JsonValue| -> Result<String, String> {
            match v {
                JsonValue::String(s) => Ok(s),
                _ => Err("not a string".into()),
            }
        };
        let tool = get("tool")?;
        let host = get("host")?;
        let revisions = match get("revisions")? {
            JsonValue::Object(p) => p
                .iter()
                .map(|(k, x)| match x {
                    JsonValue::String(s) => Ok((k.clone(), s.clone())),
                    _ => Err("revision is not a string".to_string()),
                })
                .collect::<Result<Vec<_>, _>>()?,
            _ => return Err("revisions is not an object".into()),
        };
        let regeneration = match get("regeneration")? {
            JsonValue::Null => None,
            r => Some(Regeneration::from_json(r)?),
        };
        let recording = top
            .iter()
            .find(|(n, _)| n == "recording")
            .map(|(_, v)| Recording::from_json(v))
            .transpose()?;
        let merges = match top.iter().find(|(n, _)| n == "merges") {
            None => Vec::new(),
            Some((_, JsonValue::Array(a))) => a
                .iter()
                .map(|g| {
                    let strs = match sub(g, "rows")? {
                        JsonValue::Array(x) => {
                            x.into_iter().map(st).collect::<Result<Vec<_>, _>>()?
                        }
                        _ => return Err("merges rows is not an array".to_string()),
                    };
                    Ok(Merge {
                        rev: st(sub(g, "rev")?)?,
                        binary_sha256: st(sub(g, "binary_sha256")?)?,
                        dirty_tree: strict_bool(sub(g, "dirty_tree")?)?,
                        experiment: st(sub(g, "experiment")?)?,
                        where_: st(sub(g, "where")?)?,
                        rows: strs,
                        at: st(sub(g, "at")?)?,
                        kind: sub(g, "kind").ok().map(st).transpose()?,
                        trees: sub(g, "trees")
                            .ok()
                            .map(|t| trees_from(&t))
                            .transpose()?
                            .unwrap_or_default(),
                    })
                })
                .collect::<Result<Vec<_>, String>>()?,
            Some(_) => return Err("merges is not an array".into()),
        };
        let rows = match get("rows")? {
            JsonValue::Array(a) => a
                .iter()
                .map(Row::from_json)
                .collect::<Result<Vec<_>, _>>()?,
            _ => return Err("rows is not an array".into()),
        };
        Ok(Results {
            meta: Meta {
                name: st(sub(tool, "name")?)?,
                version: st(sub(tool, "version")?)?,
                binary_sha256: st(sub(tool, "binary_sha256")?)?,
                dirty_tree: strict_bool(sub(tool, "dirty_tree")?)?,
                triple: st(sub(host, "triple")?)?,
                rustc: st(sub(host, "rustc")?)?,
                os: st(sub(host, "os")?)?,
                revisions,
                regeneration,
                merges,
                recording,
                trees: sub(tool, "trees")
                    .ok()
                    .map(|t| trees_from(&t))
                    .transpose()?
                    .unwrap_or_default(),
            },
            rows,
        })
    }
}

/// `dirty_tree` must be the JSON boolean: a string, a number or `null` is an error, never "clean".
fn strict_bool(v: JsonValue) -> Result<bool, String> {
    match v {
        JsonValue::Bool(b) => Ok(b),
        _ => Err("dirty_tree is not a JSON boolean".into()),
    }
}

/// The section 8.3 predicate.  Solver rows: UNSAT, a replayed SAT control, within the conflict,
/// time, proof and memory caps, accepted under the default checker limits, and no cap tripped.
/// For a chain or split the conflict and time clauses apply to the worst obligation.  An
/// enumeration row is green when it passed and its control was caught.
pub fn green(r: &Row) -> bool {
    if r.mode == "enum" {
        return r.result == "pass" && r.control.ok() && r.cap == "none";
    }
    // The incremental mode logs no proof (informational, section 9.5): the proof and checker
    // clauses do not apply to it, and the decision never uses it.
    let proof_ok = r.mode == "incremental"
        || (r.drat_bytes.is_some_and(|b| b <= GREEN_MAX_DRAT)
            && r.pass_default_limits == Some(true));
    r.result == "unsat"
        && r.control.ok()
        && r.control.result == "sat"
        && r.worst_step_conflicts.unwrap_or(r.conflicts) <= GREEN_MAX_CONFLICTS
        && r.worst_step_solve_ms <= GREEN_MAX_SOLVE_MS
        && proof_ok
        && r.peak_rss_kb > 0
        && r.peak_rss_kb <= GREEN_MAX_RSS_KB
        && r.cap == "none"
}

/// The VOS30 section 8.6.1 route-green predicate: strictly tighter than [`green`], requiring at
/// most [`ROUTE_MAX_CONFLICTS`] (50,000) conflicts per obligation and `check_work` at most
/// [`ROUTE_MAX_CHECK_WORK`] (2^31) on certified/monolithic/split rows.
pub fn route_green(r: &Row) -> bool {
    if !green(r) {
        return false;
    }
    if r.mode == "enum" {
        return true;
    }
    // Section 8.6.3: a B5 row additionally needs its sweep's elapsed time under 600 s.  Fail
    // closed: a missing, NaN, infinite or negative `sweep_ms` is not route-green (a NaN cannot
    // pass: it fails `is_finite` before the comparison is reached).
    if r.experiment == "B5" {
        let total = r
            .metrics
            .iter()
            .find(|(k, _)| k == "sweep_ms")
            .map(|(_, x)| *x);
        if !total.is_some_and(|x| x.is_finite() && (0.0..B5_ROUTE_MAX_TOTAL_MS).contains(&x)) {
            return false;
        }
    }
    let work_ok =
        r.mode == "incremental" || r.check_work.is_some_and(|w| w <= ROUTE_MAX_CHECK_WORK);
    r.worst_step_conflicts.unwrap_or(r.conflicts) <= ROUTE_MAX_CONFLICTS && work_ok
}

/// The reason a row is not green, in words (for the table).
pub fn red_reason(r: &Row) -> String {
    if r.result != "unsat" && r.result != "pass" {
        return r.result.clone();
    }
    if !r.control.ok() {
        return format!("control {}", r.control.result);
    }
    let mut why = Vec::new();
    if r.worst_step_conflicts.unwrap_or(r.conflicts) > GREEN_MAX_CONFLICTS {
        why.push("conflicts > 100k");
    }
    if r.worst_step_solve_ms > GREEN_MAX_SOLVE_MS {
        why.push("solve > 10 s");
    }
    if r.drat_bytes.is_some_and(|b| b > GREEN_MAX_DRAT) {
        why.push("DRAT > 32 MB");
    }
    if r.pass_default_limits == Some(false) {
        why.push("default limits reject");
    }
    if r.peak_rss_kb > GREEN_MAX_RSS_KB {
        why.push("RSS > 1 GB");
    }
    why.join(", ")
}

/// The knob-and-mode group key of a row.
pub fn group_key(r: &Row) -> String {
    format!("{}|{}|{}", r.experiment, r.mode, r.knob.key())
}

/// The largest time a row may record, milliseconds (one hour).  The longest budget is a chain's
/// 600 s; the solve and check times of a row sum obligations that all ended inside that budget,
/// so an hour is far beyond anything a run can write and well inside what a hand edit can
/// (round 2, finding 5: a negative or 1e300 time was accepted).
pub const MAX_TIME_MS: f64 = 3_600_000.0;
/// The largest conflict count a row may record (2^40): one obligation is capped at 100k
/// conflicts to be green and at 120 s of solving to finish; no solver here does 10^12.
pub const MAX_CONFLICTS: u64 = 1 << 40;
/// The largest propagation count (2^50): a few hundred million per second for an hour.
pub const MAX_PROPAGATIONS: u64 = 1 << 50;
/// The largest AND, variable, clause, entropy or declared-bit count (2^36): the child's resident
/// set is capped at 2 GiB, so a CNF cannot have more clauses than that, even summed over the 256
/// obligations of the largest chain.
pub const MAX_SIZE: u64 = 1 << 36;
/// The largest DRAT size or checker work figure (2^40): the checker's own work limit is 2^32 per
/// obligation and a proof is held in memory (2 GiB cap).
pub const MAX_PROOF: u64 = 1 << 40;
/// The largest obligation count of a row (2^16); the largest plan has 256.
pub const MAX_STEPS: u64 = 1 << 16;
/// The largest peak resident set a row may record, KiB (16 GiB): the cap is 2 GiB and a polled
/// figure may overshoot it, but not by 8x.
pub const MAX_RSS_KB: u64 = 16 * 1024 * 1024;
/// The largest seed count per knob a row may record.
pub const MAX_SEEDS: u32 = 1000;
/// The largest value a metric may hold (10^18): counts and rates of enumeration rows.
pub const MAX_METRIC: f64 = 1e18;

/// The numeric sanity rules of one row (round 2, finding 5): times finite and in `0..=MAX_TIME_MS`,
/// `check_ms` finite or absent, every counter under its stated bound, and the orderings that hold
/// by construction (sensitive bits at most declared bits, the worst obligation's conflicts at most
/// the total, a seed within the seed count).
///
/// Each message is tagged with its [`FindingClass`] (security review of VOS-028c3c-2a).  The two
/// checks that read only IDENTITY fields, `seeds` over [`MAX_SEEDS`] and `seed` outside
/// `1..=seeds`, are **structural**: they say the row is not a row of the plan, and no measured
/// value can excuse them.  Every other check reads something the run MEASURED (times, counters,
/// sizes, resident set, metrics, and the entropy and worst-step orderings between measured
/// counters) and stays a **row** finding.  One tagged list, in the old order, so the untagged
/// [`bounds_violations`] is exactly the old output.
fn bounds_tagged(id: &str, r: &Row) -> Vec<(FindingClass, String)> {
    use FindingClass::{Row as RowC, Structural};
    let mut v: Vec<(FindingClass, String)> = Vec::new();
    let mut time = |name: &str, x: f64| {
        if !x.is_finite() || !(0.0..=MAX_TIME_MS).contains(&x) {
            v.push((
                RowC,
                format!("{id}: {name} {x} is not a finite time in 0..={MAX_TIME_MS} ms"),
            ));
        }
    };
    time("solve_ms", r.solve_ms);
    time("worst_step_solve_ms", r.worst_step_solve_ms);
    if let Some(c) = r.check_ms {
        time("check_ms", c);
    }
    let mut count = |name: &str, x: u64, max: u64| {
        if x > max {
            v.push((RowC, format!("{id}: {name} {x} is over the bound {max}")));
        }
    };
    count("conflicts", r.conflicts, MAX_CONFLICTS);
    if let Some(w) = r.worst_step_conflicts {
        count("worst_step_conflicts", w, MAX_CONFLICTS);
    }
    count("propagations", r.propagations, MAX_PROPAGATIONS);
    for (name, x) in [
        ("ands", r.ands),
        ("vars", r.vars),
        ("clauses", r.clauses),
        ("entropy_bits", r.entropy_bits),
        ("declared_bits", r.declared_bits),
    ] {
        count(name, x, MAX_SIZE);
    }
    count("drat_bytes", r.drat_bytes.unwrap_or(0), MAX_PROOF);
    count("check_work", r.check_work.unwrap_or(0), MAX_PROOF);
    count("steps", r.steps, MAX_STEPS);
    count("peak_rss_kb", r.peak_rss_kb, MAX_RSS_KB);
    // Identity, not measurement: the seed count is a plan field.
    if r.seeds > MAX_SEEDS {
        v.push((
            Structural,
            format!("{id}: seeds {} is over the bound {MAX_SEEDS}", r.seeds),
        ));
    }
    if r.entropy_bits > r.declared_bits {
        v.push((
            RowC,
            format!(
                "{id}: entropy_bits {} exceeds declared_bits {}",
                r.entropy_bits, r.declared_bits
            ),
        ));
    }
    if r.worst_step_conflicts.is_some_and(|w| w > r.conflicts) {
        v.push((
            RowC,
            format!("{id}: worst_step_conflicts exceeds the total conflicts"),
        ));
    }
    // Identity, not measurement: which seed of the sweep this row is.
    if r.seed == 0 || r.seed > u64::from(r.seeds) {
        v.push((
            Structural,
            format!("{id}: seed {} is not in 1..={}", r.seed, r.seeds),
        ));
    }
    for (k, x) in &r.metrics {
        if !x.is_finite() || !(0.0..=MAX_METRIC).contains(x) {
            v.push((
                RowC,
                format!("{id}: metric {k} {x} is not finite in 0..={MAX_METRIC}"),
            ));
        }
    }
    v
}

/// [`bounds_tagged`] without the tags: the messages in the old order.
pub fn bounds_violations(id: &str, r: &Row) -> Vec<String> {
    bounds_tagged(id, r).into_iter().map(|(_, m)| m).collect()
}

/// The cross-rule of VOS30 section 8.6.3: **`fuse = 1` is B2**.  Every B5 row with `fuse = 1`
/// must agree with every PRESENT B2 row of equal length, divergence and seed on `instance_sha256`
/// (the sweep's instance, before sweeping, is B2's instance); a missing twin is not a finding
/// here (VOS-D138, see below).  A violation names both rows.  Kept as
/// its own function so a test can call it directly: `validate` refuses a B5 row as an experiment
/// outside the plan (VOS-D119) until VOS-028c3c registers B5.
///
/// Two refusals (security review): a B5 row with an empty `instance_sha256` is a violation (an
/// empty digest equal to an empty B2 digest proved nothing), and the digest is compared with
/// EVERY B2 row of equal length, divergence and seed, not just the first, so one tampered twin
/// cannot hide behind an honest one.
pub fn b5_fuse1_violations(rows: &[Row]) -> Vec<String> {
    let id = |r: &Row| {
        format!(
            "{} {} {} seed {}",
            r.experiment,
            r.knob.key(),
            r.mode,
            r.seed
        )
    };
    let mut v = Vec::new();
    for b5 in rows
        .iter()
        .filter(|r| r.experiment == "B5" && r.knob.int("fuse") == 1)
    {
        if b5.instance_sha256.is_empty() {
            v.push(format!("{}: instance_sha256 is empty", id(b5)));
        }
        let twins: Vec<&Row> = rows
            .iter()
            .filter(|r| {
                r.experiment == "B2"
                    && r.knob.int("length") == b5.knob.int("length")
                    && (r.knob.f("divergence") - b5.knob.f("divergence")).abs() < 1e-9
                    && r.seed == b5.seed
            })
            .collect();
        // VOS-D138: a missing twin is NOT this function's finding.  Completeness is
        // `full_plan_covered`'s job: on a complete (committed / joined) file every B2 row must be
        // present or `check`/`sign` refuses the file, so a missing twin cannot be used to dodge
        // the comparison.  On a partial file (the quick tier, or one record shard) the twin
        // legitimately lives elsewhere, and reporting it would put a false row finding on every
        // such shard.  So: compare against every twin that IS present, and only those.
        for b2 in twins {
            if b2.instance_sha256 != b5.instance_sha256 {
                v.push(format!(
                    "{}: fuse = 1 is B2, but its instance_sha256 {} differs from {} of {}",
                    id(b5),
                    b5.instance_sha256,
                    b2.instance_sha256,
                    id(b2)
                ));
            }
        }
    }
    v
}

/// Which kind of finding a `validate` message is (VOS-D127 L3, VOS-D129, VOS30 9.12.1).
///
/// *Structural* findings are decidable without trusting a row's MEASURED values: the file has
/// no rows, a dirty tree, bad tree ids or merges, a value outside a closed set (result, cap,
/// experiment, plan knob, tier), a bad `wall_s`, or too few seeds.  They mean the recording is
/// malformed or not what the plan asked for, and a recording with one is discarded.
///
/// *Row* findings read what a row measured or concluded: its control, counters, resident set,
/// `green`, collapse rules and the B5/B2 twin digests.  A failed control is the standing
/// example.  The recording is kept (a row outcome is data, never an edit target).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FindingClass {
    Structural,
    Row,
}

/// `validate`'s findings in the order they were found, each tagged with its class.  One list
/// with tags, not two passes, so [`validate`] is exactly the old output in the old order.
struct Tagged(Vec<(FindingClass, String)>);

impl Tagged {
    fn structural(&mut self, m: String) {
        self.0.push((FindingClass::Structural, m));
    }
    fn row(&mut self, m: String) {
        self.0.push((FindingClass::Row, m));
    }
    fn extend_structural(&mut self, ms: Vec<String>) {
        self.0
            .extend(ms.into_iter().map(|m| (FindingClass::Structural, m)));
    }
    fn extend_rows(&mut self, ms: Vec<String>) {
        self.0
            .extend(ms.into_iter().map(|m| (FindingClass::Row, m)));
    }
}

/// The two classes of [`validate`]'s findings, each in the order `validate` reports them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Validation {
    /// Findings that make a recording structurally unusable (the recording is discarded).
    pub structural: Vec<String>,
    /// Findings about what rows measured or concluded (the recording is kept; it blocks commit).
    pub rows: Vec<String>,
}

/// [`validate`], split into [`Validation::structural`] and [`Validation::rows`] (VOS-D127 L3).
pub fn validate_classes(res: &Results) -> Validation {
    let mut out = Validation::default();
    for (class, m) in validate_tagged(res).0 {
        match class {
            FindingClass::Structural => out.structural.push(m),
            FindingClass::Row => out.rows.push(m),
        }
    }
    out
}

/// The schema and guard rules of `check`, row by row and across rows.  Returns every violation,
/// structural and row findings together, in the order found; [`validate_classes`] splits them.
pub fn validate(res: &Results) -> Vec<String> {
    validate_tagged(res).0.into_iter().map(|(_, m)| m).collect()
}

fn validate_tagged(res: &Results) -> Tagged {
    let mut v = Tagged(Vec::new());
    if res.meta.dirty_tree {
        v.structural("dirty_tree is true: a dirty-tree run may not be committed".to_string());
    }
    if res.rows.is_empty() {
        v.structural("the file has no rows".to_string());
    }
    let ids: std::collections::BTreeSet<String> = res.rows.iter().map(row_text_id).collect();
    v.extend_structural(trees_violations("tool", &res.meta.trees));
    v.extend_structural(regeneration_violations(res));
    for (i, g) in res.meta.merges.iter().enumerate() {
        let at = format!("merges[{i}]");
        let hex = |x: &str, n: usize| x.len() == n && x.bytes().all(|b| b.is_ascii_hexdigit());
        if !hex(&g.rev, 40) {
            v.structural(format!("{at}: rev is not a 40-hex revision"));
        }
        if !hex(&g.binary_sha256, 64) {
            v.structural(format!("{at}: binary_sha256 is not 64 hex digits"));
        }
        if g.dirty_tree {
            v.structural(format!(
                "{at}: dirty_tree is true: a dirty-tree run may not be merged"
            ));
        }
        if !EXPERIMENTS.contains(&g.experiment.as_str()) {
            v.structural(format!("{at}: unknown experiment {}", g.experiment));
        }
        if g.rows.is_empty() {
            v.structural(format!("{at}: lists no rows"));
        }
        if g.kind.as_deref().is_some_and(|k| k != "add") {
            v.structural(format!("{at}: kind {:?} is not \"add\"", g.kind));
        }
        v.extend_structural(trees_violations(&at, &g.trees));
        let mut once = std::collections::BTreeSet::new();
        for id in &g.rows {
            if !ids.contains(id) {
                v.structural(format!("{at}: lists a row that is not in the file: {id}"));
            }
            if !once.insert(id) {
                v.structural(format!("{at}: lists a row twice: {id}"));
            }
        }
    }
    let planned = planned_identities();
    let tiers = planned_tiers();
    let mut seeds: BTreeMap<String, (std::collections::BTreeSet<u64>, u32)> = BTreeMap::new();
    for r in &res.rows {
        let id = format!(
            "{} {} {} seed {}",
            r.experiment,
            r.knob.key(),
            r.mode,
            r.seed
        );
        if !RESULTS.contains(&r.result.as_str()) {
            v.structural(format!(
                "{id}: result {:?} is not one of the closed set",
                r.result
            ));
        }
        // VOS-D097 item 4: the knob VALUES are a closed set too.  A hand-edited length of 10^9
        // is refused here, never run (a row that is re-run is built from its knob).
        //
        // VOS-D119 (security review of VOS-028c3b): this check used to run only for an experiment
        // in `EXPERIMENTS`, so a row of an unregistered experiment (B5 and D5 before VOS-028c3c,
        // or any made-up name) skipped it and passed with any knob.  An experiment the plan does
        // not list is now refused outright, and the knob check applies to every row.
        if !EXPERIMENTS.contains(&r.experiment.as_str()) {
            v.structural(format!(
                "{id}: experiment {:?} is not one of the plan's experiments",
                r.experiment
            ));
        }
        if !planned.contains(&(r.experiment.clone(), r.mode.clone(), r.knob.key())) {
            v.structural(format!(
                "{id}: the (mode, knob) is not one of the plan's knobs for {}",
                r.experiment
            ));
        }
        // VOS-D095: only a chain row records a budget, and it is the chain budget; a chain row
        // recorded before the budget existed omits the field (120 s).
        if let Some(w) = r.wall_s {
            if !CHAIN_EXPERIMENTS.contains(&r.experiment.as_str()) || w != row_wall_s(&r.experiment)
            {
                v.structural(format!(
                    "{id}: wall_s {w} is not the row budget of {} (only a chain records one)",
                    r.experiment
                ));
            }
        }
        // Finding H1: the tier is one of two closed values AND the one the plan gives this
        // identity.  `check` trusts `tier` to choose what it re-runs in process.
        if !["quick", "full"].contains(&r.tier.as_str()) {
            v.structural(format!("{id}: tier {:?} is not quick or full", r.tier));
        } else if let Some(quick) = tiers.get(&(r.experiment.clone(), r.mode.clone(), r.knob.key()))
        {
            let want = if *quick { "quick" } else { "full" };
            if r.tier != want {
                v.structural(format!(
                    "{id}: tier is {:?} but the plan gives this knob the {want} tier",
                    r.tier
                ));
            }
        }
        if r.peak_rss_kb == 0 {
            v.row(format!(
                "{id}: peak RSS is missing or 0 (an unparsed value is invalid)"
            ));
        }
        v.0.extend(bounds_tagged(&id, r));
        if r.green != green(r) {
            v.row(format!(
                "{id}: green is {} but section 8.3 gives {}",
                r.green,
                green(r)
            ));
        }
        if r.green
            && matches!(
                r.result.as_str(),
                "collapsed" | "unknown" | "killed" | "oom" | "crashed"
            )
        {
            v.row(format!("{id}: a {} row is green", r.result));
        }
        if r.mode != "enum" && r.result == "unsat" {
            if !r.control.ok() || r.control.result != "sat" {
                v.row(format!(
                    "{id}: an UNSAT row has no replayed SAT control ({:?})",
                    r.control
                ));
            }
            if r.control_instance_sha256.is_empty() {
                v.row(format!("{id}: control_instance_sha256 is empty"));
            }
            if r.mode != "incremental" && r.drat_sha256.is_none() {
                v.row(format!("{id}: an UNSAT row has no DRAT digest"));
            }
        }
        if r.mode == "enum" && matches!(r.result.as_str(), "pass" | "refused") && !r.control.ok() {
            v.row(format!(
                "{id}: the enumeration control did not pass ({:?})",
                r.control
            ));
        }
        if r.result == "collapsed" {
            v.extend_rows(collapse_violations(&id, r));
        }
        if r.result == "unknown" && r.cap == "none" {
            v.row(format!(
                "{id}: an unknown row must name the cap that ended it"
            ));
        }
        if r.result == "oom" && r.cap != "rss" {
            v.row(format!("{id}: an oom row must record cap rss"));
        }
        if !["none", "conflicts", "wall", "rss"].contains(&r.cap.as_str()) {
            v.structural(format!(
                "{id}: cap {:?} is not one of none, conflicts, wall, rss",
                r.cap
            ));
        }
        if r.seeds < MIN_SEEDS {
            v.structural(format!(
                "{id}: only {} seeds are recorded for the knob (at least {MIN_SEEDS})",
                r.seeds
            ));
        }
        let e = seeds.entry(group_key(r)).or_default();
        e.0.insert(r.seed);
        e.1 = r.seeds;
    }
    v.extend_rows(b5_fuse1_violations(&res.rows));
    for (k, (set, _)) in &seeds {
        if u32::try_from(set.len()).unwrap_or(u32::MAX) < MIN_SEEDS {
            v.structural(format!(
                "{k}: {} distinct seeds (at least {MIN_SEEDS} per knob)",
                set.len()
            ));
        }
    }
    v
}

/// Every `(experiment, mode, knob key)` the plan has: the closed set `validate` holds rows to.
pub fn planned_identities() -> std::collections::BTreeSet<(String, String, String)> {
    let mut set = std::collections::BTreeSet::new();
    for exp in EXPERIMENTS {
        for k in knobs(exp) {
            set.insert((exp.to_string(), k.mode.to_string(), k.knob.key()));
        }
    }
    set
}

/// The tier the plan gives each `(experiment, mode, knob key)`: `true` for the quick tier.  A
/// row's `tier` field must equal this (VOS-D097 review, finding H1): it decides whether the row
/// is re-run in process by `check` and which sample population it falls in, so a relabelled row
/// would escape or abuse both.
pub fn planned_tiers() -> BTreeMap<(String, String, String), bool> {
    let mut map = BTreeMap::new();
    for exp in EXPERIMENTS {
        for k in knobs(exp) {
            map.insert((exp.to_string(), k.mode.to_string(), k.knob.key()), k.quick);
        }
    }
    map
}

/// Is a recorded `trees` object well formed: empty (a file from before 0.2.0), or exactly the
/// names of [`TREE_NAMES`] (or, in a merge record, the historical [`LEGACY_TREE_NAMES`]), each a 40- or 64-digit hex object id?
fn trees_violations(at: &str, t: &[(String, String)]) -> Vec<String> {
    let mut v = Vec::new();
    if t.is_empty() {
        return v;
    }
    let names: Vec<&str> = t.iter().map(|(k, _)| k.as_str()).collect();
    // Only a merged-in record may keep the historical whole-crate names (never the tool block).
    let legacy_ok = at != "tool" && names == LEGACY_TREE_NAMES;
    if names != TREE_NAMES && !legacy_ok {
        v.push(format!("{at}: trees must name exactly {TREE_NAMES:?}"));
    }
    for (k, x) in t {
        if !((x.len() == 40 || x.len() == 64) && x.bytes().all(|b| b.is_ascii_hexdigit())) {
            v.push(format!("{at}: tree id of {k} is not a hex object id"));
        }
    }
    v
}

/// The quick-tier rows of a results file, as the specs to re-run.
pub fn quick_specs(res: &Results) -> Vec<(RowSpec, &Row)> {
    quick_specs_in(res, None)
}

/// [`quick_specs`], restricted to the rows whose identity is in `only` when it is given (the rows
/// of one shard, `check --full --shard K/N`, VOS-028c3c-2b part ii).
pub fn quick_specs_in<'a>(
    res: &'a Results,
    only: Option<&BTreeSet<Identity>>,
) -> Vec<(RowSpec, &'a Row)> {
    // Finding M2: only a row the plan puts in the quick tier is ever re-run in process, so a
    // hand-edited knob can never reach an instance builder.
    let tiers = planned_tiers();
    res.rows
        .iter()
        .filter(|r| {
            r.tier == "quick"
                && tiers.get(&(r.experiment.clone(), r.mode.clone(), r.knob.key())) == Some(&true)
                && only.is_none_or(|ids| ids.contains(&identity_of(r)))
        })
        .map(|r| {
            (
                RowSpec {
                    experiment: r.experiment.clone(),
                    knob: r.knob.clone(),
                    mode: r.mode.clone(),
                    seed: r.seed,
                    seeds: r.seeds,
                    tier: r.tier.clone(),
                },
                r,
            )
        })
        .collect()
}

/// The identity of a row within the plan: experiment, mode, knob key, seed.
pub type Identity = (String, String, String, u64);

/// The [`Identity`] of a row.
pub fn identity_of(r: &Row) -> Identity {
    (r.experiment.clone(), r.mode.clone(), r.knob.key(), r.seed)
}

/// The identities of the rows of shard `k` of `n` of the plan (at [`MIN_SEEDS`] seeds): the set a
/// shard's file must hold exactly (`join` re-checks it) and the set `check --full --shard K/N`
/// re-runs.  Pure, so the selection is testable without running a solver.
pub fn shard_identities(k: u32, n: u32) -> BTreeSet<Identity> {
    crate::exp::plan_shard(MIN_SEEDS, k, n)
        .into_iter()
        .map(|s| (s.experiment, s.mode, s.knob.key(), s.seed))
        .collect()
}

/// How many quick-tier rows the plan has: the most `check` ever re-runs in process (VOS-D134).
pub fn quick_row_cap() -> usize {
    plan(Tier::Quick, &[], MIN_SEEDS).len()
}

/// The findings that stop `check` before anything is re-run (VOS-D134), all structural (exit 2):
///
/// * (a) every structural finding of [`validate_classes`];
/// * (b) a row identity `(experiment, mode, knob, seed)` that appears twice, which would let one
///   row stand in for another;
/// * (c) more quick-tier specs than the plan has quick rows, which no honest file has (with (b)
///   it cannot happen, so this is the belt to (b)'s braces).
pub fn check_guards(res: &Results) -> Vec<String> {
    let mut v = validate_classes(res).structural;
    let mut seen = BTreeSet::new();
    for r in &res.rows {
        let id = identity_of(r);
        if !seen.insert(id.clone()) {
            v.push(format!(
                "the row {} {} {} seed {} appears more than once",
                id.0, id.2, id.1, id.3
            ));
        }
    }
    let quick = quick_specs(res).len();
    let cap = quick_row_cap();
    if quick > cap {
        v.push(format!(
            "{quick} quick-tier rows to re-run, but the plan has {cap}"
        ));
    }
    v
}

/// The metrics whose VALUES are timings or rates, so they differ on every run.  Every other
/// metric is a count and is compared exactly (round 2, finding 2).  A nondeterministic metric
/// must still be PRESENT on both sides: the key set is compared symmetrically.
pub const NONDETERMINISTIC_METRICS: [&str; 3] = ["run_ms", "cases_per_s", "sweep_ms"];

/// How much of a row [`compare_rerun`] may compare (VOS-D124, VOS-D128; VOS30 section 9.12.1).
///
/// Why classes at all: a row's counters are a deterministic function of the instance ONLY when
/// the solver stopped for a deterministic reason.  When the wall clock stopped it, the counters
/// are whatever was done by then, and when the child was killed they are partial.  Comparing
/// them exactly would make `check --full` fail on every honest recording.  Each class names
/// what is still compared:
///
/// ```text
///   class      when                                    compared
///   ---------  --------------------------------------  ---------------------------------------
///   Exact      neither row is wall-capped or killed    every deterministic field
///   WallWhole  a wall-capped single instance           identity, instance digest and structure,
///                                                      result, cap, wall_s
///   Minimal    a wall-capped SEGMENTED row, or a       identity, result, cap, wall_s
///              killed / oom / crashed row
/// ```
///
/// A single instance is built whole before the solve starts, so its digest and size do not
/// depend on where the clock ran out (what `compare_structure` already relies on), and keeping
/// them stops an edited instance hiding behind a timeout.  A chain row hashes only the segments
/// it reached before the deadline, so for it those fields are as partial as the counters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompareClass {
    Exact,
    WallWhole,
    Minimal,
}

/// The [`CompareClass`] of a recorded row and its re-run.  Either side being wall-capped or
/// ended badly is enough: a flip between a completed verdict and `unknown`/`wall` must be ONE
/// finding about the result, not a heap of counter differences.
pub fn compare_class(rec: &Row, now: &Row) -> CompareClass {
    let ended_badly = |r: &Row| matches!(r.result.as_str(), "killed" | "oom" | "crashed");
    let wall = |r: &Row| r.cap == "wall";
    if !(wall(rec) || wall(now) || ended_badly(rec) || ended_badly(now)) {
        return CompareClass::Exact;
    }
    // The experiment is part of the identity compared below, so a disagreeing pair is reported
    // there; `rec`'s experiment decides whether the row is segmented.
    let whole = (wall(rec) || wall(now))
        && rec.result == "unknown"
        && now.result == "unknown"
        && !is_segmented(&rec.experiment)
        && !is_segmented(&now.experiment);
    if whole {
        CompareClass::WallWhole
    } else {
        CompareClass::Minimal
    }
}

/// Compare a re-run row with its recorded one: **every deterministic field**, exactly (for a
/// wall-capped or killed row, the reduced set of its [`CompareClass`]).
///
/// Deterministic (compared): experiment, knob, mode, tier, seed, seeds, the three digests, the
/// instance's size and entropy (`ands`, `vars`, `clauses`, `entropy_bits`, `declared_bits`), the
/// verdict (`result`, `collapsed_reason`, `control`, `cap`, `pass_default_limits`), the solver's
/// work (`conflicts`, `propagations`, `worst_step_conflicts`, `steps`), the proof (`drat_bytes`,
/// `check_work`), whether a `check_ms` is present, the `note` and `wall_s`.  The solver and the
/// checker are deterministic functions of the instance, so their counters and the proof's length
/// are as reproducible as its digest (re-running a row twice gives them bit for bit).
///
/// Nondeterministic (not compared here): `solve_ms`, `worst_step_solve_ms`, `check_ms`,
/// `peak_rss_kb` (compared in a two-sided band by [`compare_timing`] under `check --full`), `green`
/// (a function of those, recomputed by `validate` and, under `--full`, from the re-run), and the
/// values of the metrics in [`NONDETERMINISTIC_METRICS`].
pub fn compare_rerun(rec: &Row, now: &Row) -> Vec<String> {
    let id = row_text_id(rec);
    let mut v = Vec::new();
    let mut diff = |name: &str, a: String, b: String| {
        if a != b {
            v.push(format!(
                "{id}: {name} differs on re-run (recorded {a}, now {b})"
            ));
        }
    };
    let d = |x: &dyn std::fmt::Debug| format!("{x:?}");
    diff("experiment", rec.experiment.clone(), now.experiment.clone());
    diff("knob", rec.knob.key(), now.knob.key());
    diff("mode", rec.mode.clone(), now.mode.clone());
    diff("tier", rec.tier.clone(), now.tier.clone());
    diff("seed", rec.seed.to_string(), now.seed.to_string());
    diff("seeds", rec.seeds.to_string(), now.seeds.to_string());
    let class = compare_class(rec, now);
    if class == CompareClass::Minimal {
        // Identity, then the verdict.  When the verdicts differ the cap differs as a
        // consequence (`unsat`/`none` against `unknown`/`wall`), so only the verdict is
        // reported: a flip is ONE finding.  Everything else is partial and excluded.
        diff("result", rec.result.clone(), now.result.clone());
        if rec.result == now.result {
            diff("cap", rec.cap.clone(), now.cap.clone());
        }
        diff("wall_s", d(&rec.wall_s), d(&now.wall_s));
        return v;
    }
    diff(
        "instance_sha256",
        rec.instance_sha256.clone(),
        now.instance_sha256.clone(),
    );
    if class == CompareClass::Exact {
        diff("drat_sha256", d(&rec.drat_sha256), d(&now.drat_sha256));
        diff(
            "control_instance_sha256",
            rec.control_instance_sha256.clone(),
            now.control_instance_sha256.clone(),
        );
    }
    diff("ands", rec.ands.to_string(), now.ands.to_string());
    diff("vars", rec.vars.to_string(), now.vars.to_string());
    diff("clauses", rec.clauses.to_string(), now.clauses.to_string());
    diff(
        "entropy_bits",
        rec.entropy_bits.to_string(),
        now.entropy_bits.to_string(),
    );
    diff(
        "declared_bits",
        rec.declared_bits.to_string(),
        now.declared_bits.to_string(),
    );
    diff("result", rec.result.clone(), now.result.clone());
    if class == CompareClass::WallWhole {
        // Both rows are `unknown` and at least one is wall-capped: the counters, proof fields,
        // note and metrics are whatever the clock allowed.  The cap and the budget are still
        // compared (`unknown`/`wall` against `unknown`/`conflicts` is a finding).
        diff("cap", rec.cap.clone(), now.cap.clone());
        diff("wall_s", d(&rec.wall_s), d(&now.wall_s));
        return v;
    }
    diff(
        "collapsed_reason",
        d(&rec.collapsed_reason),
        d(&now.collapsed_reason),
    );
    diff("control", d(&rec.control), d(&now.control));
    diff(
        "conflicts",
        rec.conflicts.to_string(),
        now.conflicts.to_string(),
    );
    diff(
        "propagations",
        rec.propagations.to_string(),
        now.propagations.to_string(),
    );
    diff(
        "worst_step_conflicts",
        d(&rec.worst_step_conflicts),
        d(&now.worst_step_conflicts),
    );
    diff("steps", rec.steps.to_string(), now.steps.to_string());
    diff("drat_bytes", d(&rec.drat_bytes), d(&now.drat_bytes));
    diff(
        "check_ms present",
        rec.check_ms.is_some().to_string(),
        now.check_ms.is_some().to_string(),
    );
    diff("check_work", d(&rec.check_work), d(&now.check_work));
    diff(
        "pass_default_limits",
        d(&rec.pass_default_limits),
        d(&now.pass_default_limits),
    );
    diff("cap", rec.cap.clone(), now.cap.clone());
    diff("note", rec.note.clone(), now.note.clone());
    diff("wall_s", d(&rec.wall_s), d(&now.wall_s));
    // Metrics: a symmetric key set (an extra or a missing key is a finding), exact values except
    // for the timing metrics.
    fn keys(r: &Row) -> Vec<&str> {
        let mut k: Vec<&str> = r.metrics.iter().map(|(n, _)| n.as_str()).collect();
        k.sort_unstable();
        k
    }
    diff("metric keys", d(&keys(rec)), d(&keys(now)));
    for (name, val) in &rec.metrics {
        if NONDETERMINISTIC_METRICS.contains(&name.as_str()) {
            continue;
        }
        if let Some(other) = now.metric(name) {
            diff(
                &format!("metric {name}"),
                val.to_string(),
                other.to_string(),
            );
        }
    }
    v
}

/// The structural fields of a row that [`crate::exp::structure_of`] reproduces without solving:
/// the digest of the instance and its size and entropy.  The `structure` child fills ONLY these
/// (the rest of its row is a blank), so only these are compared.
pub fn compare_structure(rec: &Row, now: &Row) -> Vec<String> {
    let id = row_text_id(rec);
    let mut v = Vec::new();
    let mut diff = |name: &str, a: String, b: String| {
        if a != b {
            v.push(format!(
                "{id}: {name} differs on structural rebuild (recorded {a}, now {b})"
            ));
        }
    };
    diff(
        "instance_sha256",
        rec.instance_sha256.clone(),
        now.instance_sha256.clone(),
    );
    diff("ands", rec.ands.to_string(), now.ands.to_string());
    diff("vars", rec.vars.to_string(), now.vars.to_string());
    diff("clauses", rec.clauses.to_string(), now.clauses.to_string());
    diff(
        "entropy_bits",
        rec.entropy_bits.to_string(),
        now.entropy_bits.to_string(),
    );
    diff(
        "declared_bits",
        rec.declared_bits.to_string(),
        now.declared_bits.to_string(),
    );
    v.extend(collapse_agreement(&id, rec, now));
    v
}

/// The structural verdicts (round 3, finding 1).  The `structure` child reports `collapsed` (with
/// the reason or the G2 note the solving path would write) when the rebuilt instance collapses by
/// itself, and `unknown` ("built") otherwise.  A recorded `unsat` row must not collapse on
/// rebuild, and a recorded `collapsed` row must say exactly what the rebuild says.  The one
/// collapse a rebuild cannot see is `propagation-only` on a CNF of more than two clauses (zero
/// conflicts is the solver's measurement); such a row is refused here and left to a re-solve
/// (`check --full`, or the spot check when it samples the row), which is the honest statement of
/// what a rebuild can verify.
fn collapse_agreement(id: &str, rec: &Row, now: &Row) -> Vec<String> {
    let mut v = Vec::new();
    let now_collapsed = now.result == "collapsed";
    let says = if now_collapsed {
        now.collapsed_reason
            .clone()
            .unwrap_or_else(|| now.note.clone())
    } else {
        "no collapse".to_string()
    };
    match rec.result.as_str() {
        "unsat" if now_collapsed => v.push(format!(
            "{id}: recorded unsat, but the rebuilt instance collapses ({says}): a refuted row \
             is not a collapse"
        )),
        "collapsed" => {
            let agrees = match (&rec.collapsed_reason, now_collapsed) {
                (Some(r), true) => now.collapsed_reason.as_ref() == Some(r),
                (None, true) => {
                    now.collapsed_reason.is_none() && rec.note.starts_with(now.note.as_str())
                }
                (_, false) => false,
            };
            if !agrees {
                v.push(format!(
                    "{id}: recorded collapsed ({}), but the rebuilt instance gives: {says} \
                     (a collapse is verified structurally, or by a re-solve in `check --full`)",
                    rec.collapsed_reason
                        .clone()
                        .unwrap_or_else(|| rec.note.clone())
                ));
            }
        }
        _ => {}
    }
    v
}

/// The ratio band of the timing comparison of `check --full` (finding M1): a recorded time or
/// resident set must lie within `1/TIMING_BAND ..= TIMING_BAND` times the re-run's, in BOTH
/// directions, plus an absolute slack.  A one-sided bound ("recorded at most k times the re-run")
/// is no defence: an editor flips `green` by LOWERING a recorded time, and that passes it.
pub const TIMING_BAND: f64 = 4.0;

/// Absolute slack of the timing band, in milliseconds, for the solve and check times (so two
/// sub-millisecond rows with a 10x ratio are not a finding).
pub const TIMING_SLACK_MS: f64 = 50.0;

/// Absolute slack of the timing band for the peak resident set, in KiB.
pub const RSS_SLACK_KB: f64 = 8192.0;

/// Is `recorded` within the two-sided band of `rerun`?
pub fn within_band(recorded: f64, rerun: f64, slack: f64) -> bool {
    recorded <= rerun * TIMING_BAND + slack && rerun <= recorded * TIMING_BAND + slack
}

/// The whole per-row comparison of a verification: [`compare_rerun`] (every deterministic field,
/// in the reduced set of the row's [`CompareClass`]) followed by [`compare_timing`] (the
/// two-sided [`TIMING_BAND`] and the recomputed `green`, which only an [`CompareClass::Exact`]
/// pair has).  It is the ONE rule `check --full` applies to a quick row in process
/// ([`rerun_collect_budgeted`]), to a full-tier row in a child (`spot::rerun_one_row`) and that
/// `decision-diff` applies to every identity of a joined verify file (VOS-D141).  Pure: it reads
/// two rows and nothing else, so a verification can be judged again from its data, without the
/// exit codes of the shards that made it.
///
/// It adds and removes nothing: the tier rule (Exact, WallWhole, Minimal) lives in
/// [`compare_class`] and the timing band applies only where `compare_timing` applies it, as
/// before this function existed.  The spot check (`timing` false) still calls `compare_rerun`
/// alone, as it always did.
pub fn compare_verify(rec: &Row, now: &Row) -> Vec<String> {
    let mut v = compare_rerun(rec, now);
    v.extend(compare_timing(rec, now));
    v
}

/// What `check --full` additionally compares: the fields that **drive `green`** and that the
/// CI spot check ignores (finding M1).  The recorded solve, worst-step solve, check time and peak
/// RSS must each lie within the two-sided [`TIMING_BAND`] of the re-run, and `green` is
/// RECOMPUTED from the re-run's own measurements and must equal the recorded one.  A row in a
/// reduced [`CompareClass`] (wall-capped, or ended `killed`, `oom` or `crashed`, on either side)
/// has no reproducible time and is not banded.
pub fn compare_timing(rec: &Row, now: &Row) -> Vec<String> {
    let id = row_text_id(rec);
    let mut v = Vec::new();
    // A reduced-class row (wall-capped, killed, oom or crashed on EITHER side) has no
    // reproducible time or counters, so there is nothing for a band or for `green` to rest on.
    // This replaces the old test on `rec` alone: a re-run that was killed is as unmeasurable
    // as a recording that was.
    if compare_class(rec, now) != CompareClass::Exact {
        return v;
    }
    let mut band = |name: &str, a: f64, b: f64, slack: f64| {
        if !within_band(a, b, slack) {
            v.push(format!(
                "{id}: {name} is outside the {TIMING_BAND}x band of the re-run (recorded {a:.1}, now {b:.1})"
            ));
        }
    };
    band("solve_ms", rec.solve_ms, now.solve_ms, TIMING_SLACK_MS);
    band(
        "worst_step_solve_ms",
        rec.worst_step_solve_ms,
        now.worst_step_solve_ms,
        TIMING_SLACK_MS,
    );
    band(
        "check_ms",
        rec.check_ms.unwrap_or(0.0),
        now.check_ms.unwrap_or(0.0),
        TIMING_SLACK_MS,
    );
    band(
        "peak_rss_kb",
        rec.peak_rss_kb as f64,
        now.peak_rss_kb as f64,
        RSS_SLACK_KB,
    );
    let fresh = green(now);
    if rec.green != fresh {
        v.push(format!(
            "{id}: green is {} but the re-run's own measurements give {fresh}",
            rec.green
        ));
    }
    v
}

/// What a `collapsed` row must record to be believable, by the rules of VOS30 section 4.5 and the
/// guards of `exp::apply_guards` / `chain_row` (round 3, finding 1).  A `collapsed` row is
/// excluded from every statistic, so a red `unsat` row relabelled `collapsed` used to turn a
/// decision group green on a plain `check`.  Every rule here is a necessary condition of the
/// genuine writer, checked against every collapsed row of the committed file:
///
/// * a reason needs `conflicts == 0 || clauses <= 2` (the two ways `collapse_reason` fires);
/// * `identical-structure` and `constant-fold` need `ands == 0` (a pair of equal outputs has a
///   constant miter and an empty cone); a chain row can collapse for no other reason (its
///   `propagation-only` would need a solve the chain writer never classifies);
/// * with no reason the note must be one of the two G2 notes, naming numbers the row's own
///   fields reproduce: `entropy_bits * 2 < declared_bits`, or `ands * 4 < full` for the 25% rule.
pub fn collapse_violations(id: &str, r: &Row) -> Vec<String> {
    let mut v = Vec::new();
    // B5 shares the chain budget but is one instance (`exp::is_segmented`).
    let chain = crate::exp::is_segmented(&r.experiment);
    if let Some(reason) = &r.collapsed_reason {
        if !crate::inst::COLLAPSE_REASONS.contains(&reason.as_str()) {
            v.push(format!(
                "{id}: collapsed_reason {reason:?} is not in the section 4.5 enum"
            ));
            return v;
        }
        if r.conflicts != 0 && r.clauses > 2 {
            v.push(format!(
                "{id}: collapsed_reason {reason:?} needs conflicts == 0 or at most 2 clauses, \
                 but the row records {} conflicts and {} clauses (a refuted row relabelled \
                 collapsed)",
                r.conflicts, r.clauses
            ));
        }
        let pair_reason = matches!(reason.as_str(), "identical-structure" | "constant-fold");
        if pair_reason && r.ands != 0 {
            v.push(format!(
                "{id}: collapsed_reason {reason:?} needs an empty miter cone (ands 0), but the \
                 row records {} ANDs",
                r.ands
            ));
        }
        if chain && reason != "identical-structure" {
            v.push(format!(
                "{id}: a chain row collapses only as identical-structure, not {reason:?}"
            ));
        }
        return v;
    }
    if r.note.is_empty() {
        v.push(format!("{id}: a collapsed row without a reason or a note"));
    } else if let Some((sens, decl)) = parse_g2_entropy(&r.note) {
        if sens != r.entropy_bits || decl != r.declared_bits || sens * 2 >= decl {
            v.push(format!(
                "{id}: the G2 entropy note {:?} does not match the row's entropy_bits {} of \
                 declared_bits {} (or they are not below half)",
                r.note, r.entropy_bits, r.declared_bits
            ));
        }
    } else if let Some((ands, full)) = parse_g2_ands(&r.note).filter(|_| !chain) {
        if ands != r.ands || ands * 4 >= full {
            v.push(format!(
                "{id}: the G2 note {:?} does not match the row's {} ANDs (or they are not under \
                 25%)",
                r.note, r.ands
            ));
        }
    } else {
        v.push(format!(
            "{id}: a collapsed row with no reason must carry a G2 note, not {:?}",
            r.note
        ));
    }
    v
}

fn two_numbers(note: &str, prefix: &str, mid: &str, suffix: &str) -> Option<(u64, u64)> {
    let rest = note.strip_prefix(prefix)?;
    let (a, rest) = rest.split_once(mid)?;
    let b = rest.strip_suffix(suffix)?;
    Some((a.parse().ok()?, b.parse().ok()?))
}

/// `(sensitive, declared)` of a G2 entropy note (the text of `exp::g2_entropy_note`, possibly
/// followed by another sentence a later step appended).
fn parse_g2_entropy(note: &str) -> Option<(u64, u64)> {
    let rest = note.strip_prefix("G2: ")?;
    let (a, rest) = rest.split_once(" of ")?;
    let (b, _) = rest.split_once(" declared bits are sensitive")?;
    Some((a.parse().ok()?, b.parse().ok()?))
}

/// `(ands, full)` of a G2 AND-count note (the text of `exp::g2_ands_note`).
fn parse_g2_ands(note: &str) -> Option<(u64, u64)> {
    two_numbers(
        note,
        "G2: ",
        " ANDs is under 25% of ",
        " at divergence 100%",
    )
}

/// The solver wall of an in-process re-run of a row of `experiment` (VOS-D131): the row's own
/// experiment wall (600 s for a chain, 1,800 s for B5, else 120 s), the clock it was recorded
/// under.  A pure function so the rule is testable without running a solver.
pub fn rerun_wall(experiment: &str) -> Duration {
    Duration::from_secs(row_wall_s(experiment))
}

/// The overall wall budget of plain `check`'s in-process re-run, seconds (VOS-D134, VOS30 section
/// 9.2).  Each row has its own wall ([`rerun_wall`]: up to 1,800 s), and a crafted file can hold
/// many rows; the budget bounds their SUM.  The 85 quick rows committed before VOS-028c3c commit A (105 in the plan now) reproduce in about 10 s
/// (measured on a Windows developer machine at `--jobs 4`, 2026-11-23), so 1,800 s is more than
/// 150 times what an honest file needs: it bites only a file built to be slow, never a slow
/// runner.  It equals the longest single row wall, so one honest B5 quick row still fits.
pub const QUICK_RERUN_BUDGET_S: u64 = 1_800;

/// Is the overall budget spent after `elapsed`?  `None` is unbounded (`check --full`, whose rows
/// run under their own walls and shard clocks).  A pure function of its inputs so the rule is
/// tested without waiting for a clock.
pub fn budget_spent(elapsed: Duration, budget: Option<Duration>) -> bool {
    budget.is_some_and(|b| elapsed >= b)
}

/// Re-run every quick-tier row in this process and compare (`jobs` threads), under the overall
/// budget [`QUICK_RERUN_BUDGET_S`].
pub fn rerun(res: &Results, jobs: usize) -> Vec<String> {
    rerun_within(res, jobs, Duration::from_secs(QUICK_RERUN_BUDGET_S))
}

/// [`rerun`] under an explicit overall `budget` (the tests pass a tiny one).
pub fn rerun_within(res: &Results, jobs: usize, budget: Duration) -> Vec<String> {
    rerun_collect_budgeted(res, jobs, false, None, Some(budget)).0
}

/// [`rerun`], and under `timing` (`check --full`, round 2 finding 2) also the timing-derived
/// comparison of [`compare_timing`] on these in-process rows: they were never compared on time
/// before, because only the full-tier children were.  An in-process row measures no resident set,
/// so it borrows the recorded one (the band on it is vacuous and `green` is recomputed from the
/// re-run's own times, counters and proof figures plus that recorded set).  Also returns the
/// re-run rows by [`row_text_id`] (with `green` recomputed), for the decision comparison.
pub fn rerun_collect(
    res: &Results,
    jobs: usize,
    timing: bool,
) -> (Vec<String>, BTreeMap<String, Row>) {
    rerun_collect_in(res, jobs, timing, None)
}

/// [`rerun_collect`] over the quick rows whose identity is in `only` (one shard; `None` is all).
/// A shard with no quick-tier row is not a finding, so "the file has no quick-tier row" is
/// reported only for the whole-file run.
pub fn rerun_collect_in(
    res: &Results,
    jobs: usize,
    timing: bool,
    only: Option<&BTreeSet<Identity>>,
) -> (Vec<String>, BTreeMap<String, Row>) {
    rerun_collect_budgeted(res, jobs, timing, only, None)
}

/// [`rerun_collect_in`] under an overall wall `budget` (VOS-D134): before a worker starts a row it
/// asks [`budget_spent`] about the time since the re-run began; once spent, no further row
/// starts and one finding says how many were re-run.  A row already running is bounded by its
/// own wall and is not interrupted.  `None` is unbounded.
pub fn rerun_collect_budgeted(
    res: &Results,
    jobs: usize,
    timing: bool,
    only: Option<&BTreeSet<Identity>>,
    budget: Option<Duration>,
) -> (Vec<String>, BTreeMap<String, Row>) {
    let specs = quick_specs_in(res, only);
    if specs.is_empty() {
        let msg = if only.is_none() {
            vec!["the file has no quick-tier row to re-run".to_string()]
        } else {
            Vec::new()
        };
        return (msg, BTreeMap::new());
    }
    let next = std::sync::atomic::AtomicUsize::new(0);
    let started = std::time::Instant::now();
    let stopped = std::sync::atomic::AtomicBool::new(false);
    let out = std::sync::Mutex::new((Vec::new(), BTreeMap::new()));
    std::thread::scope(|s| {
        for _ in 0..jobs.max(1) {
            s.spawn(|| loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let Some((spec, rec)) = specs.get(i) else {
                    break;
                };
                if budget_spent(started.elapsed(), budget) {
                    stopped.store(true, std::sync::atomic::Ordering::SeqCst);
                    break;
                }
                // The row's own experiment wall (VOS-D131), not a flat 120 s: a chain row was
                // recorded under 600 s and B5's under 1,800 s, so a shorter clock would flip a
                // row that is slow but honest from its recorded verdict to `unknown`.
                let mut now = run_row(spec, rerun_wall(&spec.experiment), &mut |_| {});
                // An in-process row measures no resident set, so it borrows the recorded one and
                // `green` is recomputed.  `compare_rerun` reads neither field (they are
                // nondeterministic), so doing this BEFORE the comparison, as `compare_verify`
                // needs, changes nothing it reports.
                now.peak_rss_kb = rec.peak_rss_kb;
                now.green = green(&now);
                let diffs = if timing {
                    compare_verify(rec, &now)
                } else {
                    compare_rerun(rec, &now)
                };
                let mut g = out.lock().unwrap_or_else(|e| e.into_inner());
                g.0.extend(diffs);
                g.1.insert(row_text_id(rec), now);
            });
        }
    });
    let (mut v, rows) = out.into_inner().unwrap_or_else(|e| e.into_inner());
    if let (true, Some(b)) = (stopped.load(std::sync::atomic::Ordering::SeqCst), budget) {
        v.push(format!(
            "the quick-tier re-run exceeded its overall budget of {} s: {} of {} rows were \
             re-run and the rest were not started",
            b.as_secs(),
            rows.len(),
            specs.len()
        ));
    }
    v.sort();
    (v, rows)
}

/// The decision's `because` lines with the one nondeterministic figure removed: `describe` prints
/// the worst solve time, which differs on every run, so a line is cut at that figure.
fn because_stable(d: &Decision) -> Vec<String> {
    d.because
        .iter()
        .map(|b| match b.find(", worst solve") {
            Some(at) => b[..at].to_string(),
            None => b.clone(),
        })
        .collect()
}

/// A re-plan `because` line with its counters cut: [`describe_route`] ends each line with the
/// worst conflict count and check work, which a wall-capped row varies from run to run.  What
/// stays is the row, its name and its route-green verdict.
fn replan_because_stable(d: &ReplanDecision) -> Vec<String> {
    d.because
        .iter()
        .map(|b| match b.find("; worst conflicts") {
            Some(at) => b[..at].to_string(),
            None => b.clone(),
        })
        .collect()
}

/// `check --full` (round 2, finding 3; VOS-D132): the decision the RE-RUN rows give must equal
/// the decision the recorded rows give, in its row number and its (time-stripped) `because`
/// lines.  This covers every predicate `decide` reads, including those outside `green`, such as
/// row 3's `solve_ms + check_ms <= 60 000`, which the band on single rows does not bound tightly
/// enough.
///
/// VOS-D132: the re-plan decision of section 8.6.4 ([`decide_replan`]) and the cut density
/// ([`cut_density_k`]) are compared the same way.  Before, a verification could pass while the
/// re-plan outcome (R1 to R4), its orthogonal memory verdict or `K` differed.  The re-plan
/// `because` lines are compared with their counters cut ([`replan_because_stable`]).
pub fn decision_differences(recorded: &[Row], rerun: &[Row]) -> Vec<String> {
    let (a, b) = (decide(recorded), decide(rerun));
    let mut v = Vec::new();
    if a.row != b.row {
        v.push(format!(
            "the decision is row {} on the recorded rows but row {} on the re-run rows",
            a.row, b.row
        ));
    } else if because_stable(&a) != because_stable(&b) {
        v.push(format!(
            "the decision's reasons differ: recorded {:?}, re-run {:?}",
            because_stable(&a),
            because_stable(&b)
        ));
    }
    let (ka, kb) = (cut_density_k(recorded), cut_density_k(rerun));
    if ka != kb {
        v.push(format!(
            "the cut density K is {ka} on the recorded rows but {kb} on the re-run rows"
        ));
    }
    let (ra, rb) = (decide_replan(recorded), decide_replan(rerun));
    if ra.row != rb.row {
        v.push(format!(
            "the re-plan decision is R{} on the recorded rows but R{} on the re-run rows",
            ra.row, rb.row
        ));
    } else if replan_because_stable(&ra) != replan_because_stable(&rb) {
        v.push(format!(
            "the re-plan decision's reasons differ: recorded {:?}, re-run {:?}",
            replan_because_stable(&ra),
            replan_because_stable(&rb)
        ));
    }
    if ra.memory_shipped_t != rb.memory_shipped_t {
        v.push(format!(
            "the re-plan memory verdict is shipped-T {} on the recorded rows but {} on the \
             re-run rows",
            ra.memory_shipped_t, rb.memory_shipped_t
        ));
    }
    v
}

fn identity_text(i: &Identity) -> String {
    format!("{} {} {} seed {}", i.0, i.2, i.1, i.3)
}

/// How a file differs from the rows a plan would run: the one routine behind
/// [`quick_plan_covered`] and [`full_plan_covered`].  The quick tier is a subset of the file (its
/// rows are those labelled `quick`); the full plan is all of it.  Each gap is one line, and the
/// lines for a missing row all have the shape `the plan's {label} row ID is not in the results
/// file`, which [`full_plan_missing`] relies on.
fn plan_gaps(res: &Results, tier: Tier, label: &str) -> Vec<String> {
    let mut v = Vec::new();
    let expected: std::collections::BTreeSet<Identity> = plan(tier, &[], MIN_SEEDS)
        .into_iter()
        .map(|s| (s.experiment, s.mode, s.knob.key(), s.seed))
        .collect();
    let mut seen: std::collections::BTreeSet<Identity> = std::collections::BTreeSet::new();
    let mut groups: BTreeMap<(String, String, String), (std::collections::BTreeSet<u64>, u32)> =
        BTreeMap::new();
    for r in &res.rows {
        if tier == Tier::Quick && r.tier != "quick" {
            continue;
        }
        let id: Identity = (r.experiment.clone(), r.mode.clone(), r.knob.key(), r.seed);
        if !expected.contains(&id) {
            v.push(format!(
                "the {label} plan does not contain the file's row {}",
                identity_text(&id)
            ));
        }
        if !seen.insert(id.clone()) {
            v.push(format!(
                "the file has the {label} row {} more than once",
                identity_text(&id)
            ));
        }
        let g = groups
            .entry((id.0.clone(), id.1.clone(), id.2.clone()))
            .or_default();
        g.0.insert(r.seed);
        // The rows of a group must agree on how many seeds it has; the largest is compared.
        g.1 = g.1.max(r.seeds);
    }
    for id in &expected {
        if !seen.contains(id) {
            v.push(format!(
                "the plan's {label} row {} is not in the results file",
                identity_text(id)
            ));
        }
    }
    for ((e, m, k), (set, seeds)) in &groups {
        if u32::try_from(set.len()).unwrap_or(u32::MAX) != *seeds {
            v.push(format!(
                "{e} {k} {m}: rows record seeds = {seeds} but {} seeds are present",
                set.len()
            ));
        }
    }
    v
}

/// Does the file contain exactly the quick-tier rows the current plan would run: none missing,
/// none extra, none twice, and every group's `seeds` equal to the seeds present?  (A knob added to
/// the plan without regenerating the file is drift.)
pub fn quick_plan_covered(res: &Results) -> Vec<String> {
    plan_gaps(res, Tier::Quick, "quick")
}

/// The full-plan rows the file lacks, as identity texts, before any exemption.
pub fn full_plan_missing(res: &Results) -> Vec<String> {
    plan_gaps(res, Tier::Full, "full")
        .into_iter()
        .filter_map(|g| {
            g.strip_prefix("the plan's full row ")
                .and_then(|x| x.strip_suffix(" is not in the results file"))
                .map(str::to_string)
        })
        .collect()
}

/// The full-plan rows the committed file is *reviewed* not to contain, each with its reason
/// (VOS30 section 9.12 lists them; a test pins this list to the exact set of missing rows, so a
/// stale exemption and a newly missing row both fail).
pub const FULL_PLAN_EXEMPT: &[(&str, &str)] = &[];

/// Does the file contain exactly the rows of the full plan (which includes the quick knobs): no
/// missing row beyond the reviewed [`FULL_PLAN_EXEMPT`] list, none extra, none twice, and every
/// group's `seeds` equal to the seeds present?  Finding H2: without it a row, or a whole knob,
/// can be deleted from the file and `check` still passes.
pub fn full_plan_covered(res: &Results) -> Vec<String> {
    plan_gaps(res, Tier::Full, "full")
        .into_iter()
        .filter(|g| {
            !FULL_PLAN_EXEMPT
                .iter()
                .any(|(id, _)| *g == format!("the plan's full row {id} is not in the results file"))
        })
        .collect()
}

// ---------------------------------------------------------------------------------------
// Groups and the decision
// ---------------------------------------------------------------------------------------

/// The rows of one `(experiment, mode, knob)`, all seeds.
pub struct Group<'a> {
    /// The rows.
    pub rows: Vec<&'a Row>,
}

impl Group<'_> {
    /// The first row (they share experiment, mode and knob).
    pub fn first(&self) -> &Row {
        self.rows[0]
    }
    /// Rows that count (collapsed rows are excluded from every statistic).
    pub fn counted(&self) -> Vec<&Row> {
        self.rows
            .iter()
            .copied()
            .filter(|r| r.result != "collapsed")
            .collect()
    }
    /// Green iff at least one row counts and every counted seed is green.
    pub fn green(&self) -> bool {
        let c = self.counted();
        !c.is_empty() && c.iter().all(|r| r.green)
    }
    /// The maximum over counted seeds of a measurement.
    pub fn max<F: Fn(&Row) -> f64>(&self, f: F) -> f64 {
        self.counted().iter().map(|r| f(r)).fold(0.0, f64::max)
    }
}

/// Group rows in file order.
pub fn groups(rows: &[Row]) -> Vec<Group<'_>> {
    let mut index: BTreeMap<String, usize> = BTreeMap::new();
    let mut out: Vec<Group<'_>> = Vec::new();
    for r in rows {
        let k = group_key(r);
        match index.get(&k) {
            Some(&i) => out[i].rows.push(r),
            None => {
                index.insert(k, out.len());
                out.push(Group { rows: vec![r] });
            }
        }
    }
    out
}

/// The outcome of section 8.3.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decision {
    /// 1, 2 or 3.
    pub row: u8,
    /// The row's title.
    pub title: &'static str,
    /// The conditions that placed the entry there.
    pub because: Vec<String>,
}

/// The three outcome titles of section 8.3.
pub const TITLES: [&str; 3] = [
    "Infeasible: re-plan X3",
    "Feasible, reduced and mitigated",
    "Feasible as planned",
];

fn find<'a>(
    gs: &'a [Group<'a>],
    exp: &str,
    mode: &str,
    pred: impl Fn(&Knob) -> bool,
) -> Option<&'a Group<'a>> {
    gs.iter()
        .find(|g| g.first().experiment == exp && g.first().mode == mode && pred(&g.first().knob))
}

fn is_green(g: Option<&Group<'_>>) -> bool {
    g.is_some_and(|g| g.green())
}

fn b3<'a>(gs: &'a [Group<'a>], len: i64, cut: &str) -> Option<&'a Group<'a>> {
    find(gs, "B3", "certified", |k| {
        k.int("length") == len && (k.f("divergence") - 0.25).abs() < 1e-9 && k.s("cut") == cut
    })
}

/// Is this row in a group that [`decide`] reads?  (Round 2, finding 1.)  `check` re-solves only
/// the plan's cheap rows, so which groups the decision rests on, and which of them are never
/// re-solved by a plain `check`, must be visible: `check` prints that count.  The predicate
/// mirrors the `find` calls of `decide`, one line per condition; a test pins that every group
/// whose redness changes the decision on an all-green file satisfies it.
pub fn decision_read(r: &Row) -> bool {
    let k = &r.knob;
    let d25 = (k.f("divergence") - 0.25).abs() < 1e-9;
    match (r.experiment.as_str(), r.mode.as_str()) {
        ("A1" | "A2", "monolithic") => matches!(k.int("n"), 32 | 64),
        ("D3", "split") => k.int("k") == 16 && matches!(k.int("t"), 8 | 64 | 256),
        ("D2", "monolithic") => k.int("t") == 64 && k.int("k") == 16,
        ("B3", "certified") => {
            d25 && matches!(
                (k.int("length"), k.s("cut")),
                (200, "every1") | (1000, "every1" | "every8" | "every32")
            )
        }
        ("C1", "monolithic") => k.s("base") == "b" && d25 && matches!(k.int("length"), 200 | 1000),
        ("B4", "monolithic") => k.s("shape") == "assoc" && (5..=7).contains(&k.int("n")),
        _ => false,
    }
}

/// The groups [`decide`] reads that THIS run did not re-solve: no row of the group is in the
/// quick tier (re-run in process by every `check`) and none is in `resolved`, the sorted indexes
/// of the rows the spot check re-ran in full to completion (round 3, finding 3).  The count
/// follows what the run did, not the plan's classes, so a slow host that sampled fewer rows prints
/// a larger number.  Their timing-derived and solver-derived fields are verified only by
/// `check --full`.
pub fn decision_groups_not_resolved(rows: &[Row], resolved: &[usize]) -> Vec<String> {
    let mut all = std::collections::BTreeSet::new();
    let mut solved = std::collections::BTreeSet::new();
    for (i, r) in rows.iter().enumerate() {
        if decision_read(r) {
            let k = group_key(r);
            if r.tier == "quick" || resolved.binary_search(&i).is_ok() {
                solved.insert(k.clone());
            }
            all.insert(k);
        }
    }
    all.difference(&solved).cloned().collect()
}

/// Section 8.3 applied to the rows, worst case first; any shape that is missing or not green
/// counts as red ("unclassified is red").  The conditions are the ones section 9.11 states.
pub fn decide(rows: &[Row]) -> Decision {
    let gs = groups(rows);
    let describe = |g: Option<&Group<'_>>, name: &str| -> String {
        match g {
            None => format!("{name}: no rows"),
            Some(g) => format!(
                "{name}: {}; worst conflicts {}, worst solve {:.0} ms",
                if g.green() { "green" } else { "NOT green" },
                g.max(|r| r.worst_step_conflicts.unwrap_or(r.conflicts) as f64),
                g.max(|r| r.worst_step_solve_ms)
            ),
        }
    };
    // Row 1: a required shape is not green at the finest decomposition tested.
    let a1_32 = find(&gs, "A1", "monolithic", |k| k.int("n") == 32);
    let d3_t8 = find(&gs, "D3", "split", |k| k.int("t") == 8 && k.int("k") == 16);
    let b3_200 = b3(&gs, 200, "every1");
    let c1_200 = find(&gs, "C1", "monolithic", |k| {
        k.s("base") == "b" && k.int("length") == 200 && (k.f("divergence") - 0.25).abs() < 1e-9
    });
    let required = [
        (a1_32, "A1 at N = 32"),
        (d3_t8, "D3 at T = 8, k = 16"),
        (
            b3_200,
            "B3 certified, every instruction, 200 instructions, 25% divergence",
        ),
        (c1_200, "C1 at 200 instructions, 25% divergence"),
    ];
    let red: Vec<String> = required
        .iter()
        .filter(|(g, _)| !is_green(*g))
        .map(|(g, n)| describe(*g, n))
        .collect();
    if !red.is_empty() {
        return Decision {
            row: 1,
            title: TITLES[0],
            because: red,
        };
    }
    // Row 2: feasible only in a reduced form.
    let mut reduced = Vec::new();
    let b3_1000_1 = is_green(b3(&gs, 1000, "every1"));
    let b3_1000_32 = is_green(b3(&gs, 1000, "every32"));
    if (b3_1000_1 && !b3_1000_32) || (is_green(b3_200) && !b3_1000_1) {
        reduced.push("B3 is green only with cuts at 8 instructions or fewer, or only up to about 200 instructions".to_string());
    }
    let d3_big = |t: i64| find(&gs, "D3", "split", |k| k.int("t") == t && k.int("k") == 16);
    if !is_green(d3_big(64)) || !is_green(d3_big(256)) {
        reduced.push("D3 is not green above T = 16".to_string());
    }
    if (5..=7).any(|n| {
        !is_green(find(&gs, "B4", "monolithic", |k| {
            k.s("shape") == "assoc" && k.int("n") == n
        }))
    }) {
        reduced.push("B4 re-association is green only up to n = 4".to_string());
    }
    // Row 3 conditions, all of.
    let a_ok = [("A1", 32), ("A1", 64), ("A2", 32), ("A2", 64)]
        .iter()
        .all(|(e, n)| is_green(find(&gs, e, "monolithic", |k| k.int("n") == *n)));
    let b3_ok = ["every1", "every8", "every32"].iter().any(|cut| {
        b3(&gs, 1000, cut).is_some_and(|g| {
            g.green()
                && g.max(|r| r.worst_step_conflicts.unwrap_or(0) as f64) <= 10_000.0
                && g.max(|r| r.solve_ms + r.check_ms.unwrap_or(0.0)) <= 60_000.0
        })
    });
    let d_ok = is_green(find(&gs, "D2", "monolithic", |k| {
        k.int("t") == 64 && k.int("k") == 16
    })) && is_green(d3_big(64));
    let c1_1000 = find(&gs, "C1", "monolithic", |k| {
        k.s("base") == "b" && k.int("length") == 1000 && (k.f("divergence") - 0.25).abs() < 1e-9
    });
    let c_ok = is_green(c1_1000);
    if !reduced.is_empty() {
        return Decision {
            row: 2,
            title: TITLES[1],
            because: reduced,
        };
    }
    if a_ok && b3_ok && d_ok && c_ok {
        return Decision {
            row: 3,
            title: TITLES[2],
            because: vec![
                "A1 and A2 green at N = 32 and 64".into(),
                describe(
                    b3(&gs, 1000, "every8"),
                    "B3 certified every 8, 1000 instructions",
                ),
                "D2 and D3 green at T = 64, k = 16".into(),
                describe(c1_1000, "C1 at 1000 instructions"),
            ],
        };
    }
    // Neither row 1 nor row 2 named it and row 3 does not hold: the table is total, so it is row 1.
    let mut because = vec!["no row of section 8.3 holds; unclassified is red".to_string()];
    if !a_ok {
        because.push("A1 or A2 is not green at N = 32 and 64".into());
    }
    if !b3_ok {
        because.push(
            "B3 is not green at 1000 instructions within 10k conflicts a step and 60 s".into(),
        );
    }
    if !d_ok {
        because.push("D2 or D3 is not green at T = 64, k = 16".into());
    }
    if !c_ok {
        because.push("C1 is not green at 1000 instructions".into());
    }
    Decision {
        row: 1,
        title: TITLES[0],
        because,
    }
}

/// A control record for a row that did not run its control.
pub fn no_control() -> Control {
    Control::none("not run")
}

/// True iff `g` has at least one counted (non-collapsed) row and every counted row is
/// [`route_green`] (VOS30 section 9.12).
pub fn group_route_green(g: &Group<'_>) -> bool {
    let c = g.counted();
    !c.is_empty() && c.iter().all(|r| route_green(r))
}

fn is_route_green(g: Option<&Group<'_>>) -> bool {
    g.is_some_and(group_route_green)
}

/// The cut density `K` of VOS30 section 8.6.2: the largest spacing in `{1, 2, 4}` instructions at
/// which `B3` in `certified` mode is route-green at every length (`25..=1000`) and every
/// divergence (`5%`, `25%`, `100%`) of the recorded grid; `0` if even `every1` is not.
pub fn cut_density_k(rows: &[Row]) -> u8 {
    let gs = groups(rows);
    for (k, cut) in [(4u8, "every4"), (2u8, "every2"), (1u8, "every1")] {
        let ok = [25i64, 50, 100, 200, 500, 1000].iter().all(|&len| {
            [0.05f64, 0.25, 1.0].iter().all(|&div| {
                is_route_green(find(&gs, "B3", "certified", |kb| {
                    kb.int("length") == len
                        && (kb.f("divergence") - div).abs() < 1e-9
                        && kb.s("cut") == cut
                }))
            })
        });
        if ok {
            return k;
        }
    }
    0
}

/// The four outcome titles of the VOS30 section 8.6.4 re-plan decision table (R1 to R4).
pub const REPLAN_TITLES: [&str; 4] = [
    "No lane",
    "Owned-compile lane mandatory",
    "Compiled lane, reduced",
    "Compiled lane",
];

/// The outcome of the VOS30 section 8.6.4 re-plan decision table, plus cut density `K` and the
/// orthogonal memory verdict.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplanDecision {
    /// 1..=4 corresponding to R1..=R4.
    pub row: u8,
    /// The outcome title ([`REPLAN_TITLES`]).
    pub title: &'static str,
    /// The conditions that placed the outcome here.
    pub because: Vec<String>,
    /// Cut density `K` in `{0, 1, 2, 4}` (section 8.6.2).
    pub cut_density_k: u8,
    /// True iff `D5` (`split = index`, `k = 16`) is route-green at `T = 64` and `T = 256`
    /// (`thread-table` X3 runs at its shipped `T`); otherwise `T` at most 16 per obligation.
    pub memory_shipped_t: bool,
    /// Human-readable orthogonal memory verdict.
    pub memory_verdict: String,
}

fn describe_route(g: Option<&Group<'_>>, name: &str) -> String {
    match g {
        None => format!("{name}: no rows"),
        Some(g) => format!(
            "{name}: {}; worst conflicts {}, worst check_work {}",
            if group_route_green(g) {
                "route-green"
            } else {
                "NOT route-green"
            },
            g.max(|r| r.worst_step_conflicts.unwrap_or(r.conflicts) as f64),
            g.max(|r| r.check_work.unwrap_or(0) as f64)
        ),
    }
}

/// VOS30 section 8.6.4 applied to the rows, worst case first (`R1` to `R4`, total); missing or
/// uncounted groups are red.
pub fn decide_replan(rows: &[Row]) -> ReplanDecision {
    let gs = groups(rows);
    let k_cut = cut_density_k(rows);
    let d5_idx = |t: i64| {
        find(&gs, "D5", "certified", |kb| {
            kb.int("t") == t && kb.int("k") == 16 && kb.s("split") == "index"
        })
    };
    let d5_64 = d5_idx(64);
    let d5_256 = d5_idx(256);
    let memory_shipped_t = is_route_green(d5_64) && is_route_green(d5_256);
    let memory_verdict = if memory_shipped_t {
        "shipped T (D5 split=index route-green at T = 64 and T = 256, k = 16)".to_string()
    } else {
        format!(
            "T at most 16 per obligation ({}; {})",
            describe_route(d5_64, "D5 T = 64, k = 16, split = index"),
            describe_route(d5_256, "D5 T = 256, k = 16, split = index")
        )
    };
    let mut r1_red = Vec::new();
    for exp in ["A1", "A2"] {
        for n in [8i64, 16, 32, 64] {
            let g = find(&gs, exp, "monolithic", |kb| kb.int("n") == n);
            if !is_route_green(g) {
                r1_red.push(describe_route(g, &format!("{exp} at N = {n}")));
            }
        }
    }
    let b3_200 = b3(&gs, 200, "every1");
    if !is_route_green(b3_200) {
        r1_red.push(describe_route(
            b3_200,
            "B3 certified, every1, 200 instructions, 25% divergence",
        ));
    }
    if !r1_red.is_empty() {
        return ReplanDecision {
            row: 1,
            title: REPLAN_TITLES[0],
            because: r1_red,
            cut_density_k: k_cut,
            memory_shipped_t,
            memory_verdict,
        };
    }
    let b5 = |len: i64, div: f64, fuse: i64| {
        find(&gs, "B5", "certified", |kb| {
            kb.int("length") == len
                && (kb.f("divergence") - div).abs() < 1e-9
                && kb.int("fuse") == fuse
        })
    };
    let b5_200_f2 = b5(200, 0.25, 2);
    if !is_route_green(b5_200_f2) {
        return ReplanDecision {
            row: 2,
            title: REPLAN_TITLES[1],
            because: vec![describe_route(
                b5_200_f2,
                "B5 at 200 instructions, 25% divergence, fuse = 2",
            )],
            cut_density_k: k_cut,
            memory_shipped_t,
            memory_verdict,
        };
    }
    let mut r3_reasons = Vec::new();
    for div in [0.25f64, 1.0] {
        for fuse in [1i64, 2, 4, 8] {
            let g = b5(1000, div, fuse);
            if !is_route_green(g) {
                r3_reasons.push(describe_route(
                    g,
                    &format!(
                        "B5 at 1,000 instructions, div = {}%, fuse = {fuse}",
                        (div * 100.0).round() as i64
                    ),
                ));
            }
        }
    }
    for len in [25i64, 200] {
        for div in [0.25f64, 1.0] {
            let g = b5(len, div, 4);
            if !is_route_green(g) {
                r3_reasons.push(describe_route(
                    g,
                    &format!(
                        "B5 at {len} instructions, div = {}%, fuse = 4",
                        (div * 100.0).round() as i64
                    ),
                ));
            }
        }
    }
    if k_cut <= 1 {
        r3_reasons.push(format!("cut density K = {k_cut} (at most 1)"));
    }
    if !r3_reasons.is_empty() {
        return ReplanDecision {
            row: 3,
            title: REPLAN_TITLES[2],
            because: r3_reasons,
            cut_density_k: k_cut,
            memory_shipped_t,
            memory_verdict,
        };
    }
    ReplanDecision {
        row: 4,
        title: REPLAN_TITLES[3],
        because: vec![format!(
            "A1/A2 (N = 8..64), B3 (K = {k_cut}), and B5 (1,000 instructions and fuse = 4) are route-green"
        )],
        cut_density_k: k_cut,
        memory_shipped_t,
        memory_verdict,
    }
}
