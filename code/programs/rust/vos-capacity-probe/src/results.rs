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

use crate::exp::{plan, run_row, RowSpec, Tier};
use crate::row::{Control, Knob, Row};
use crate::solve::{CONFLICT_SLICE, PROOF_CONFLICT_CAP};
use coding_adventures_bounded_json::{parse, JsonNumber, JsonValue};
use std::collections::BTreeMap;
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
    /// Who regenerated the full tier on which host, if anyone.
    pub regeneration: Option<(String, String)>,
}

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
        obj(vec![
            ("schema", int(SCHEMA)),
            (
                "tool",
                obj(vec![
                    ("name", string(&m.name)),
                    ("version", string(&m.version)),
                    ("binary_sha256", string(&m.binary_sha256)),
                    ("dirty_tree", JsonValue::Bool(m.dirty_tree)),
                ]),
            ),
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
                    Some((who, host)) => {
                        obj(vec![("reviewer", string(who)), ("host", string(host))])
                    }
                    None => JsonValue::Null,
                },
            ),
            (
                "rows",
                JsonValue::Array(self.rows.iter().map(Row::to_json).collect()),
            ),
        ])
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
            r => Some((st(sub(r, "reviewer")?)?, st(sub(r, "host")?)?)),
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
                dirty_tree: matches!(sub(tool, "dirty_tree")?, JsonValue::Bool(true)),
                triple: st(sub(host, "triple")?)?,
                rustc: st(sub(host, "rustc")?)?,
                os: st(sub(host, "os")?)?,
                revisions,
                regeneration,
            },
            rows,
        })
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

/// The schema and guard rules of `check`, row by row and across rows.  Returns every violation.
pub fn validate(res: &Results) -> Vec<String> {
    let mut v = Vec::new();
    if res.meta.dirty_tree {
        v.push("dirty_tree is true: a dirty-tree run may not be committed".to_string());
    }
    if res.rows.is_empty() {
        v.push("the file has no rows".to_string());
    }
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
            v.push(format!(
                "{id}: result {:?} is not one of the closed set",
                r.result
            ));
        }
        if r.peak_rss_kb == 0 {
            v.push(format!(
                "{id}: peak RSS is missing or 0 (an unparsed value is invalid)"
            ));
        }
        if r.green != green(r) {
            v.push(format!(
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
            v.push(format!("{id}: a {} row is green", r.result));
        }
        if r.mode != "enum" && r.result == "unsat" {
            if !r.control.ok() || r.control.result != "sat" {
                v.push(format!(
                    "{id}: an UNSAT row has no replayed SAT control ({:?})",
                    r.control
                ));
            }
            if r.control_instance_sha256.is_empty() {
                v.push(format!("{id}: control_instance_sha256 is empty"));
            }
            if r.mode != "incremental" && r.drat_sha256.is_none() {
                v.push(format!("{id}: an UNSAT row has no DRAT digest"));
            }
        }
        if r.mode == "enum" && matches!(r.result.as_str(), "pass" | "refused") && !r.control.ok() {
            v.push(format!(
                "{id}: the enumeration control did not pass ({:?})",
                r.control
            ));
        }
        if r.result == "collapsed" {
            if let Some(reason) = &r.collapsed_reason {
                if !crate::inst::COLLAPSE_REASONS.contains(&reason.as_str()) {
                    v.push(format!(
                        "{id}: collapsed_reason {reason:?} is not in the section 4.5 enum"
                    ));
                }
            } else if r.note.is_empty() {
                v.push(format!("{id}: a collapsed row without a reason or a note"));
            }
        }
        if r.result == "unknown" && r.cap == "none" {
            v.push(format!(
                "{id}: an unknown row must name the cap that ended it"
            ));
        }
        if r.result == "oom" && r.cap != "rss" {
            v.push(format!("{id}: an oom row must record cap rss"));
        }
        if !["none", "conflicts", "wall", "rss"].contains(&r.cap.as_str()) {
            v.push(format!(
                "{id}: cap {:?} is not one of none, conflicts, wall, rss",
                r.cap
            ));
        }
        if r.seeds < MIN_SEEDS {
            v.push(format!(
                "{id}: only {} seeds are recorded for the knob (at least {MIN_SEEDS})",
                r.seeds
            ));
        }
        let e = seeds.entry(group_key(r)).or_default();
        e.0.insert(r.seed);
        e.1 = r.seeds;
    }
    for (k, (set, _)) in &seeds {
        if (set.len() as u32) < MIN_SEEDS {
            v.push(format!(
                "{k}: {} distinct seeds (at least {MIN_SEEDS} per knob)",
                set.len()
            ));
        }
    }
    v
}

/// The quick-tier rows of a results file, as the specs to re-run.
pub fn quick_specs(res: &Results) -> Vec<(RowSpec, &Row)> {
    res.rows
        .iter()
        .filter(|r| r.tier == "quick")
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

/// Compare a re-run row with its recorded one: digests, verdict, control, conflict count.
pub fn compare_rerun(rec: &Row, now: &Row) -> Vec<String> {
    let id = format!(
        "{} {} {} seed {}",
        rec.experiment,
        rec.knob.key(),
        rec.mode,
        rec.seed
    );
    let mut v = Vec::new();
    let mut diff = |name: &str, a: String, b: String| {
        if a != b {
            v.push(format!(
                "{id}: {name} differs on re-run (recorded {a}, now {b})"
            ));
        }
    };
    diff(
        "instance_sha256",
        rec.instance_sha256.clone(),
        now.instance_sha256.clone(),
    );
    diff(
        "drat_sha256",
        format!("{:?}", rec.drat_sha256),
        format!("{:?}", now.drat_sha256),
    );
    diff(
        "control_instance_sha256",
        rec.control_instance_sha256.clone(),
        now.control_instance_sha256.clone(),
    );
    diff("result", rec.result.clone(), now.result.clone());
    diff(
        "collapsed_reason",
        format!("{:?}", rec.collapsed_reason),
        format!("{:?}", now.collapsed_reason),
    );
    diff(
        "control",
        format!("{:?}", rec.control),
        format!("{:?}", now.control),
    );
    diff(
        "conflicts",
        rec.conflicts.to_string(),
        now.conflicts.to_string(),
    );
    diff(
        "worst_step_conflicts",
        format!("{:?}", rec.worst_step_conflicts),
        format!("{:?}", now.worst_step_conflicts),
    );
    diff("steps", rec.steps.to_string(), now.steps.to_string());
    diff("ands", rec.ands.to_string(), now.ands.to_string());
    diff(
        "pass_default_limits",
        format!("{:?}", rec.pass_default_limits),
        format!("{:?}", now.pass_default_limits),
    );
    for (name, val) in &rec.metrics {
        // Counts are deterministic; rates and times are not.
        if matches!(
            name.as_str(),
            "cases"
                | "discarded"
                | "binder_product"
                | "control_binder_product"
                | "domain_bytes"
                | "enumerated"
                | "values_held"
                | "live_steps"
                | "segments"
        ) {
            diff(
                &format!("metric {name}"),
                val.to_string(),
                now.metric(name).map(|x| x.to_string()).unwrap_or_default(),
            );
        }
    }
    v
}

/// Re-run every quick-tier row in this process and compare (`jobs` threads).
pub fn rerun(res: &Results, jobs: usize) -> Vec<String> {
    let specs = quick_specs(res);
    if specs.is_empty() {
        return vec!["the file has no quick-tier row to re-run".to_string()];
    }
    let next = std::sync::atomic::AtomicUsize::new(0);
    let out = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..jobs.max(1) {
            s.spawn(|| loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let Some((spec, rec)) = specs.get(i) else {
                    break;
                };
                let now = run_row(spec, Duration::from_secs(TIMEOUT_S), &mut |_| {});
                let diffs = compare_rerun(rec, &now);
                out.lock().unwrap_or_else(|e| e.into_inner()).extend(diffs);
            });
        }
    });
    let mut v = out.into_inner().unwrap_or_else(|e| e.into_inner());
    v.sort();
    v
}

/// Does the committed file contain exactly the quick-tier rows the current plan would run?
/// (A knob added to the plan without regenerating the file is drift.)
pub fn quick_plan_covered(res: &Results) -> Vec<String> {
    let mut v = Vec::new();
    for spec in plan(Tier::Quick, &[], MIN_SEEDS) {
        let found = res.rows.iter().any(|r| {
            r.experiment == spec.experiment
                && r.knob == spec.knob
                && r.mode == spec.mode
                && r.seed == spec.seed
                && r.tier == "quick"
        });
        if !found {
            v.push(format!(
                "the plan's quick row {} {} {} seed {} is not in the results file",
                spec.experiment,
                spec.knob.key(),
                spec.mode,
                spec.seed
            ));
        }
    }
    v
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
