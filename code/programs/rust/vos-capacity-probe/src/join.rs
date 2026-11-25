//! # `join`: N shard files become one results file, or nothing
//!
//! `run --shard K/N` gives each CI runner a slice of the plan (VOS30 section 9.2).  `join` is the
//! step that turns the N slices back into the one file the rest of the tool reads, and it is the
//! integrity gate of the recording: a file that joins has provably come from N shards of one
//! workflow run, one binary, one tree and one plan, and nothing else.
//!
//! The whole of it is a pure function over parsed [`Results`], so every refusal is a unit test and
//! nothing here needs a process, a git checkout or a Unix host.  Reading the files, canonical-form
//! checking and the atomic write are `cli::cmd_join`'s job.
//!
//! ## What a shard must be
//!
//! ```text
//!   shard K of N  ==  a file that
//!     - is canonical and has no structural `validate` finding        (the file is usable)
//!     - records `recording.shard = K/N` and nothing else              (it says which slice)
//!     - holds exactly the rows of plan_shard(K, N)                    (it is that slice)
//!   the N files together  ==  K in 0..N, once each                    (no slice twice or missing)
//!   and they agree on: tool name, version, binary digest, trees, revisions, host triple,
//!                      recording mode, jobs, run id, run attempt       (one run, one binary)
//! ```
//!
//! "Each file's rows equal `plan_shard(K, N)`" is one comparison that gives three properties at
//! once: a row in two files is a row outside one of the slices (disjoint), a row missing from
//! every file is missing from its slice (union equals the plan), and a file swapped for another
//! shard's is not its own slice (the swap test).
//!
//! `os` is deliberately not compared: `uname -sr` may differ between runners of one class
//! (VOS-D133), while the target triple is a property of the binary.

use crate::exp::{plan, plan_shard, Tier, MAX_SHARDS};
use crate::results::{
    is_run_number, validate_classes, Recording, Results, MIN_SEEDS, RECORDING_MODES,
};
use std::collections::{BTreeMap, BTreeSet};

/// The `--expect-*` values of `join`: each, when set, must equal what the inputs record.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Expect {
    /// `--expect-binary-sha256`: the build job's digest (VOS30 section 9.12.2, item 1).
    pub binary_sha256: Option<String>,
    /// `--expect-run-id` (item 2).
    pub run_id: Option<String>,
    /// `--expect-run-attempt` (VOS-D127 L1).
    pub run_attempt: Option<String>,
    /// `--expect-shards`: N.
    pub shards: Option<u32>,
    /// `--expect-jobs`.
    pub jobs: Option<u32>,
}

/// A successful join.
#[derive(Debug)]
pub struct Joined {
    /// The joined file, rows in plan order.
    pub results: Results,
    /// Row-class `validate` findings of the joined file.  They are kept row outcomes (VOS30
    /// section 9.12.1, class 2), reported and never a reason to refuse.
    pub findings: Vec<String>,
}

fn hex64(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

type Identity = (String, String, String, u64);

/// Join `inputs` (a name for messages, and the parsed file).  The order of `inputs` does not
/// matter: the shards are sorted by their own `recording`.  `Err` is a refusal; the caller exits
/// 2 and writes nothing.
pub fn join(inputs: &[(String, Results)], expect: &Expect) -> Result<Joined, String> {
    if inputs.is_empty() {
        return Err("join needs at least one shard file".into());
    }

    // 1. Each input is usable and says which slice it is.
    let mut shards: Vec<(u32, u32, &str, &Results)> = Vec::new();
    for (name, res) in inputs {
        // Merges and a banner first: a file with merges also fails `validate` once its rows are
        // sliced (the merge lists rows that are no longer there), and the useful message is the
        // one that names the cause.
        if !res.meta.merges.is_empty() {
            return Err(format!("join refuses {name}: it has merges"));
        }
        if res.meta.regeneration.is_some() {
            return Err(format!("join refuses {name}: it has a regeneration banner"));
        }
        let v = validate_classes(res);
        if let Some(m) = v.structural.first() {
            return Err(format!(
                "join refuses {name}: structurally invalid ({} finding(s)); first: {m}",
                v.structural.len()
            ));
        }
        let Some(rec) = &res.meta.recording else {
            return Err(format!(
                "join refuses {name}: it has no recording block (not a `run --shard` output)"
            ));
        };
        let Some((k, n)) = rec.shard else {
            return Err(format!(
                "join refuses {name}: its recording is not a single shard's (it has no `shard`)"
            ));
        };
        shards.push((k, n, name, res));
    }
    shards.sort_by_key(|(k, n, name, _)| (*n, *k, name.to_string()));

    // 2. The shard set is exactly 0..N.
    let n = shards[0].1;
    if let Some((_, other, name, _)) = shards.iter().find(|s| s.1 != n) {
        return Err(format!(
            "join refuses {name}: it is a shard of {other} but another input is a shard of {n}"
        ));
    }
    for w in shards.windows(2) {
        if w[0].0 == w[1].0 {
            return Err(format!(
                "join refuses: shard {}/{n} is given twice ({} and {})",
                w[0].0, w[0].2, w[1].2
            ));
        }
    }
    if let Some(missing) = (0..n).find(|k| !shards.iter().any(|s| s.0 == *k)) {
        return Err(format!(
            "join refuses: shard {missing}/{n} is missing ({} of {n} inputs given)",
            shards.len()
        ));
    }
    if shards.len() != n as usize {
        // Defensive: with no duplicate and 0..N all present, the count must already be N.
        return Err(format!(
            "join refuses: {} inputs for {n} shards",
            shards.len()
        ));
    }
    if n == 0 || n > MAX_SHARDS {
        return Err(format!("join refuses: N = {n} is outside 1..={MAX_SHARDS}"));
    }

    // 3. One run: every shard agrees with shard 0 on everything provenance.
    //
    // `rustc` and `os` are deliberately NOT compared.  `binary_sha256` (pinned by
    // `--expect-binary-sha256` from the build job, VOS30 9.12.2 item 1) already fixes the build,
    // and `os` (`uname -sr`) may legitimately differ between runners of one label (VOS-D133).
    // The joined meta takes both from shard 0.
    let (first_name, first) = (shards[0].2, shards[0].3);
    let frec = first.meta.recording.as_ref().expect("checked above");
    for &(k, _, name, res) in &shards[1..] {
        let m = &res.meta;
        let f = &first.meta;
        let rec = m.recording.as_ref().expect("checked above");
        let differs: Option<&str> = if m.name != f.name {
            Some("tool name")
        } else if m.version != f.version {
            Some("tool version")
        } else if m.binary_sha256 != f.binary_sha256 {
            Some("binary_sha256")
        } else if m.trees != f.trees {
            Some("tree ids")
        } else if m.revisions != f.revisions {
            Some("revisions")
        } else if m.triple != f.triple {
            Some("host triple")
        } else if rec.mode != frec.mode {
            Some("recording mode")
        } else if rec.jobs != frec.jobs {
            Some("jobs")
        } else if rec.run_id != frec.run_id {
            Some("run id")
        } else if rec.run_attempt != frec.run_attempt {
            Some("run attempt")
        } else if rec.verifies_sha256 != frec.verifies_sha256 {
            // VOS-D141: the shards of one verification re-ran ONE recorded file.  A shard that
            // names another (or none, among shards that name one) is from a different verify, and
            // its rows would be compared with the wrong recording by `decision-diff`.
            Some("verifies_sha256 (the recorded file it re-ran)")
        } else {
            None
        };
        if let Some(what) = differs {
            return Err(format!(
                "join refuses: {what} of shard {k}/{n} ({name}) differs from shard 0/{n} \
                 ({first_name})"
            ));
        }
    }
    let m = &first.meta;
    if !hex64(&m.binary_sha256) {
        return Err("join refuses: binary_sha256 is not 64 hex digits".into());
    }
    if m.trees.is_empty() {
        return Err("join refuses: the inputs record no tree ids".into());
    }
    if m.revisions.is_empty() {
        return Err("join refuses: the inputs record no revisions".into());
    }
    if m.triple.is_empty() || m.triple == "unknown" {
        return Err("join refuses: the inputs record no host triple".into());
    }
    if !RECORDING_MODES.contains(&frec.mode.as_str()) {
        return Err(format!("join refuses: recording mode {:?}", frec.mode));
    }

    // 4. Each input holds exactly its slice of the plan.
    let mut by_identity: BTreeMap<Identity, &crate::row::Row> = BTreeMap::new();
    for &(k, _, name, res) in &shards {
        let want: BTreeSet<Identity> = plan_shard(MIN_SEEDS, k, n)
            .into_iter()
            .map(|s| (s.experiment, s.mode, s.knob.key(), s.seed))
            .collect();
        let mut have: BTreeSet<Identity> = BTreeSet::new();
        for r in &res.rows {
            // The plan is for MIN_SEEDS seeds per knob; a row claiming another count is not
            // the plan's row, whatever its identity says.
            if r.seeds != MIN_SEEDS {
                return Err(format!(
                    "join refuses {name}: row {} {} {} seed {} records seeds {}, the plan \
                     records {MIN_SEEDS}",
                    r.experiment,
                    r.knob.key(),
                    r.mode,
                    r.seed,
                    r.seeds
                ));
            }
            let id: Identity = (r.experiment.clone(), r.mode.clone(), r.knob.key(), r.seed);
            if !have.insert(id.clone()) {
                return Err(format!(
                    "join refuses {name}: the row {} {} {} seed {} appears twice",
                    id.0, id.2, id.1, id.3
                ));
            }
            by_identity.insert(id, r);
        }
        if have != want {
            let stray = have.difference(&want).next();
            let gap = want.difference(&have).next();
            return Err(format!(
                "join refuses {name}: its rows are not exactly the plan's shard {k}/{n} \
                 ({} rows, the shard has {}; a stray row: {}; a missing row: {})",
                have.len(),
                want.len(),
                stray.map_or("none".to_string(), |i| format!(
                    "{} {} {} seed {}",
                    i.0, i.2, i.1, i.3
                )),
                gap.map_or("none".to_string(), |i| format!(
                    "{} {} {} seed {}",
                    i.0, i.2, i.1, i.3
                )),
            ));
        }
    }

    // 5. The caller's expectations about the run.
    if let Some(h) = &expect.binary_sha256 {
        if !hex64(h) {
            return Err("join refuses: --expect-binary-sha256 is not 64 hex digits".into());
        }
        if *h != m.binary_sha256 {
            return Err(format!(
                "join refuses: the shards record binary_sha256 {} but --expect-binary-sha256 \
                 is {h}",
                m.binary_sha256
            ));
        }
    }
    for (flag, want, got) in [
        ("--expect-run-id", &expect.run_id, &frec.run_id),
        (
            "--expect-run-attempt",
            &expect.run_attempt,
            &frec.run_attempt,
        ),
    ] {
        let Some(w) = want else { continue };
        if !is_run_number(w) {
            return Err(format!("join refuses: {flag} is not 1 to 20 ASCII digits"));
        }
        match got {
            None => {
                return Err(format!(
                    "join refuses: {flag} is given but the shards record no run id or attempt \
                     (a local run)"
                ))
            }
            Some(g) if g != w => {
                return Err(format!(
                    "join refuses: the shards record {g} but {flag} is {w}"
                ))
            }
            Some(_) => {}
        }
    }
    if let Some(w) = expect.shards {
        if w != n {
            return Err(format!(
                "join refuses: the shards are of {n} but --expect-shards is {w}"
            ));
        }
    }
    if let Some(w) = expect.jobs {
        if w != frec.jobs {
            return Err(format!(
                "join refuses: the shards ran at --jobs {} but --expect-jobs is {w}",
                frec.jobs
            ));
        }
    }

    // 6. Build the output in memory: rows in plan order, one provenance, a joined recording.
    let rows: Vec<crate::row::Row> = plan(Tier::Full, &[], MIN_SEEDS)
        .into_iter()
        .filter_map(|s| {
            by_identity
                .get(&(s.experiment, s.mode, s.knob.key(), s.seed))
                .map(|r| (*r).clone())
        })
        .collect();
    let total: usize = shards.iter().map(|s| s.3.rows.len()).sum();
    if rows.len() != total {
        // Unreachable if step 4 held: the shards' union is the plan.
        return Err(format!(
            "join refuses: {total} shard rows but {} plan rows",
            rows.len()
        ));
    }
    let mut meta = first.meta.clone();
    meta.merges = Vec::new();
    meta.regeneration = None;
    meta.recording = Some(Recording {
        mode: frec.mode.clone(),
        run_id: frec.run_id.clone(),
        run_attempt: frec.run_attempt.clone(),
        shard: None,
        shards: Some(n),
        jobs: frec.jobs,
        verifies_sha256: frec.verifies_sha256.clone(),
    });
    let results = Results { meta, rows };
    let v = validate_classes(&results);
    if let Some(m) = v.structural.first() {
        return Err(format!(
            "join refuses: the joined file is structurally invalid: {m}"
        ));
    }
    Ok(Joined {
        results,
        findings: v.rows,
    })
}
