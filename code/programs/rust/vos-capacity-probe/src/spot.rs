//! # `spot`: the sampled full-row re-run of `check` and the complete one of `check --full`
//!
//! `check` has always re-run every *quick-tier* row, in process, on every build.  The full tier
//! (about 1,600 rows, about two hours) cannot be re-run on every build, so before VOS-D097 a
//! single hand-edited full-tier row passed `check` unnoticed.  This module gives `check` two
//! more teeth (VOS30 section 9.12, "Hardening"):
//!
//! ```text
//!   check          validate + plan coverage + quick re-run + a SPOT CHECK:
//!                    population = every full-tier non-E row
//!                    eligible   = those whose (experiment, mode, knob) the PLAN classes cheap
//!                                 (`cost::CHEAP`; never the recorded times)
//!                    seed       = SHA-256 of the results file's own canonical bytes
//!                    order      = rows sorted by SHA-256(seed ":" row index)
//!                    re-run the eligible rows in that order, each in a CHILD PROCESS under
//!                    `run`'s limits and under what is left of the budget, until a fixed total
//!                    time budget is spent; an expensive row is skipped but COUNTED; fewer than
//!                    the floor of verified rows fails the check
//!   check --full   refuse unless the recorded tree ids equal the working tree's, then re-run
//!                  EVERY full-tier row, no sampling, each in a child process, and compare the
//!                  timing-derived fields too (`results::compare_timing`)
//! ```
//!
//! ## What the sample proves, stated honestly (finding M3)
//!
//! The seed is the hash of the file, so the sample is **deterministic**: CI must be reproducible.
//! That has a price the first draft of this text understated.  An editor holds the file and this
//! code, so the editor can compute the sample of any candidate edit and **grind** the file (change
//! a byte somewhere harmless, such as a note) until the edited row falls outside the sample.
//! "Any edit reshuffles the sample" is true and is no defence against someone who tries edits
//! until one lands outside it.  What the spot check does catch is an **accidental** drift (a
//! regenerated row at another tree, a fixture that changed) and a **lazy** edit that does not
//! grind; for a single edited row among about 1,600 it catches only about `sample / population`
//! of the unprepared attempts.  It is **not proof** of the file against a determined editor.
//! The real teeth are `check --full` and the release gate that requires it (section 9.12), which
//! re-run everything and also compare the timing-derived fields; the CI spot check verifies
//! digests, verdicts, controls and conflict counts only.
//!
//! ## Why no row is re-run in process
//!
//! A re-run builds an instance from the recorded knob.  Run in process, a hand-edited knob of
//! 10^9 would exhaust this process's memory.  Every row here runs in a child under the limits
//! of `runner` (CPU `ulimit` backstop, 2 GiB resident-set poll, process-group kill), and a row
//! whose `(experiment, mode, knob)` is not in the plan is refused without being run.  `validate`
//! refuses such a row too (an unregistered experiment such as B5 before VOS-028c3c is a
//! violation, VOS-D119), so the file is rejected before this module is reached.

use crate::cost::is_cheap;
use crate::exp::RowSpec;
use crate::results::{
    compare_rerun, compare_structure, compare_verify, decision_read, planned_identities,
    row_text_id, Results,
};
use crate::row::Row;
use crate::runner;
use coding_adventures_sha256::sha256_hex;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// The total time `check` spends on sampled full-row re-runs, so the CI cost is a fixed number.
pub const SPOT_BUDGET: Duration = Duration::from_secs(60);

/// How far past the end of the budget a child that started inside it may run before it is
/// killed: its hard limit is `min(hard_limit_for(row), remaining budget + SPOT_SLACK)` (finding
/// M5).  A cheap-class row finishes in about a second; a child still running after this is not
/// verified.
pub const SPOT_SLACK: Duration = Duration::from_secs(15);

/// The fewest rows a spot check must have verified (re-run to completion and compared): a check
/// that re-ran nothing proves nothing.  A plain constant (round 2, finding 6): the floor used to
/// be `min(16, eligible)`, so a commit that emptied the cheap table also emptied the floor.  An
/// eligible count below the floor is now a `check` failure of its own.
pub const SPOT_FLOOR: usize = 16;

/// The indexes of the rows the spot check draws from: every full-tier row that is not an
/// enumeration row (E1 and E2 are untouched by this hardening; VOS-D103).
pub fn population(res: &Results) -> Vec<usize> {
    res.rows
        .iter()
        .enumerate()
        .filter(|(_, r)| r.tier == "full" && !r.experiment.starts_with('E'))
        .map(|(i, _)| i)
        .collect()
}

/// The seed of the sample: the SHA-256 of the file's canonical serialisation.
pub fn sample_seed(res: &Results) -> String {
    sha256_hex(res.to_text().as_bytes())
}

/// The population in seeded order.  Deterministic for a given file, different for any edit.
pub fn sample_order(res: &Results) -> Vec<usize> {
    let seed = sample_seed(res);
    let mut keyed: Vec<(String, usize)> = population(res)
        .into_iter()
        .map(|i| (sha256_hex(format!("{seed}:{i}").as_bytes()), i))
        .collect();
    keyed.sort();
    keyed.into_iter().map(|(_, i)| i).collect()
}

/// Is this row eligible for the spot check, by the plan alone?  (Finding M4: the class is
/// `cost::CHEAP`, keyed by `(experiment, mode, knob)`; nothing the file records about the row's
/// time or result is read.)
pub fn eligible(r: &Row) -> bool {
    is_cheap(&r.experiment, &r.mode, &r.knob.key())
}

/// What a spot check did.
#[derive(Debug, Default, PartialEq)]
pub struct SpotReport {
    /// Rows in the population.
    pub population: usize,
    /// Rows re-run.
    pub rerun: usize,
    /// Rows the plan classes expensive (counted, never dropped).
    pub skipped_expensive: usize,
    /// Rows started that did not finish inside the budget: not verified (a failure).
    pub unverified: usize,
    /// Rows of the seeded order not reached before the budget was spent.
    pub not_reached: usize,
    /// What differed on re-run.
    pub violations: Vec<String>,
    /// Why the spot check did not run at all, if it did not (no process limits off unix).
    pub skipped: Option<String>,
    /// The indexes of the rows that were re-run to completion (every deterministic field
    /// compared): the structural rebuild need not rebuild them again.
    pub verified_rows: Vec<usize>,
}

impl SpotReport {
    /// The one line `check` prints.
    pub fn line(&self) -> String {
        match &self.skipped {
            Some(why) => format!("spot check: skipped ({why})"),
            None => format!(
                "spot check: population {}, re-run {}, skipped (plan cost class expensive): {}, \
                 not reached: {}",
                self.population, self.rerun, self.skipped_expensive, self.not_reached
            ),
        }
    }
}

/// The spec a recorded row re-runs from.
fn spec_of(r: &Row) -> RowSpec {
    RowSpec {
        experiment: r.experiment.clone(),
        knob: r.knob.clone(),
        mode: r.mode.clone(),
        seed: r.seed,
        seeds: r.seeds,
        tier: r.tier.clone(),
    }
}

/// Re-run one recorded row in a child process and compare.  A row that is not in the plan is
/// refused and not run.  `bound` is the most the child may run (the spot check's remaining
/// budget plus slack); `None` is the row's own hard limit.  A child the bound kills is **not
/// verified**, which is a violation here, never a pass (finding M5).  `timing` adds the
/// timing-derived comparison of `check --full` (finding M1).
pub fn rerun_one(
    exe: &Path,
    rec: &Row,
    planned: &std::collections::BTreeSet<(String, String, String)>,
    bound: Option<Duration>,
    timing: bool,
) -> Vec<String> {
    rerun_one_row(exe, rec, planned, bound, timing).0
}

/// [`rerun_one`], and the re-run row (`None` for a row refused without running).
pub fn rerun_one_row(
    exe: &Path,
    rec: &Row,
    planned: &std::collections::BTreeSet<(String, String, String)>,
    bound: Option<Duration>,
    timing: bool,
) -> (Vec<String>, Option<Row>) {
    let id = (rec.experiment.clone(), rec.mode.clone(), rec.knob.key());
    if !planned.contains(&id) {
        return (
            vec![format!(
                "{}: not in the plan, refused without running",
                row_text_id(rec)
            )],
            None,
        );
    }
    let now = runner::run_one_bounded(exe, &spec_of(rec), bound);
    if bound.is_some() && now.result == "killed" {
        return (
            vec![format!(
                "{}: not verified: the re-run did not finish inside the spot-check budget",
                row_text_id(rec)
            )],
            Some(now),
        );
    }
    let v = if timing {
        compare_verify(rec, &now)
    } else {
        compare_rerun(rec, &now)
    };
    (v, Some(now))
}

/// The sampled re-run with the standard slack.
pub fn spot_check(res: &Results, exe: &Path, jobs: usize, budget: Duration) -> SpotReport {
    spot_check_in(res, exe, jobs, budget, SPOT_SLACK)
}

/// The sampled re-run: eligible rows in seeded order, each in a child, until `budget` is spent.
/// Off unix there are no process limits and nothing is run (the report says so).
pub fn spot_check_in(
    res: &Results,
    exe: &Path,
    jobs: usize,
    budget: Duration,
    slack: Duration,
) -> SpotReport {
    let order = sample_order(res);
    let mut report = SpotReport {
        population: order.len(),
        ..SpotReport::default()
    };
    if !cfg!(unix) {
        report.skipped = Some("the process limits are unix-only".to_string());
        return report;
    }
    let planned = planned_identities();
    let eligible_rows = order.iter().filter(|&&i| eligible(&res.rows[i])).count();
    let start = Instant::now();
    let next = AtomicUsize::new(0);
    let examined = AtomicUsize::new(0);
    // (re-run, skipped as expensive, unverified, violations)
    let state = Mutex::new((0usize, 0usize, 0usize, Vec::<String>::new()));
    let done_rows = Mutex::new(Vec::<usize>::new());
    std::thread::scope(|s| {
        for _ in 0..runner::clamp_jobs(jobs) {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::SeqCst);
                let Some(&row_index) = order.get(i) else {
                    break;
                };
                let rec = &res.rows[row_index];
                let in_plan =
                    planned.contains(&(rec.experiment.clone(), rec.mode.clone(), rec.knob.key()));
                if !in_plan {
                    // Never run, always reported (`rerun_one` refuses it): `check` has already
                    // refused such a file, so this is defence in depth for a direct caller.
                    examined.fetch_add(1, Ordering::SeqCst);
                    let diffs = rerun_one(exe, rec, &planned, None, false);
                    state
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .3
                        .extend(diffs);
                    continue;
                }
                if !eligible(rec) {
                    examined.fetch_add(1, Ordering::SeqCst);
                    state.lock().unwrap_or_else(|e| e.into_inner()).1 += 1;
                    continue;
                }
                let spent = start.elapsed();
                if spent >= budget {
                    break;
                }
                examined.fetch_add(1, Ordering::SeqCst);
                let bound = budget.saturating_sub(spent) + slack;
                let diffs = rerun_one(exe, rec, &planned, Some(bound), false);
                let not_verified = diffs
                    .iter()
                    .filter(|d| d.contains(": not verified"))
                    .count();
                if not_verified == 0 {
                    done_rows
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .push(row_index);
                }
                let mut g = state.lock().unwrap_or_else(|e| e.into_inner());
                g.0 += 1;
                g.2 += not_verified;
                g.3.extend(diffs);
            });
        }
    });
    let (rerun, skipped, unverified, mut violations) =
        state.into_inner().unwrap_or_else(|e| e.into_inner());
    report.rerun = rerun;
    report.skipped_expensive = skipped;
    report.unverified = unverified;
    report.not_reached = order.len().saturating_sub(examined.load(Ordering::SeqCst));
    let mut done = done_rows.into_inner().unwrap_or_else(|e| e.into_inner());
    done.sort_unstable();
    report.verified_rows = done;
    if eligible_rows < SPOT_FLOOR {
        violations.push(format!(
            "spot check: only {eligible_rows} rows are eligible (the plan's cheap class), below \
             the floor of {SPOT_FLOOR}"
        ));
    }
    let verified = rerun.saturating_sub(unverified);
    if verified < SPOT_FLOOR {
        violations.push(format!(
            "spot check: only {verified} rows were verified, below the floor of {SPOT_FLOOR} \
             (eligible rows: {eligible_rows})"
        ));
    }
    violations.sort();
    report.violations = violations;
    report
}

/// `check --full`: every full-tier row, no sampling, each in a child.  `on_row(done, total)` is
/// called as each finishes.  Returns every difference.
pub fn full_rerun(
    res: &Results,
    exe: &Path,
    jobs: usize,
    on_row: &(dyn Fn(usize, usize) + Sync),
) -> Vec<String> {
    full_rerun_collect(res, exe, jobs, on_row).0
}

/// [`full_rerun`], and the re-run rows by [`row_text_id`] for the decision comparison (round 2,
/// finding 3).  A child whose row did not finish is returned as the `killed` row it is, so the
/// decision comparison sees it as the red it is.
pub fn full_rerun_collect(
    res: &Results,
    exe: &Path,
    jobs: usize,
    on_row: &(dyn Fn(usize, usize) + Sync),
) -> (Vec<String>, BTreeMap<String, Row>) {
    full_rerun_collect_in(res, exe, jobs, on_row, None)
}

/// [`full_rerun_collect`] over the full-tier rows whose identity is in `only` (one shard of
/// `check --full --shard K/N`; `None` is every full-tier row).
pub fn full_rerun_collect_in(
    res: &Results,
    exe: &Path,
    jobs: usize,
    on_row: &(dyn Fn(usize, usize) + Sync),
    only: Option<&std::collections::BTreeSet<crate::results::Identity>>,
) -> (Vec<String>, BTreeMap<String, Row>) {
    let rows: Vec<&Row> = res
        .rows
        .iter()
        .filter(|r| {
            r.tier == "full" && only.is_none_or(|ids| ids.contains(&crate::results::identity_of(r)))
        })
        .collect();
    let planned = planned_identities();
    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let out = Mutex::new((Vec::new(), BTreeMap::new()));
    std::thread::scope(|s| {
        for _ in 0..runner::clamp_jobs(jobs) {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::SeqCst);
                let Some(rec) = rows.get(i) else { break };
                let (diffs, now) = rerun_one_row(exe, rec, &planned, None, true);
                {
                    let mut g = out.lock().unwrap_or_else(|e| e.into_inner());
                    g.0.extend(diffs);
                    if let Some(now) = now {
                        g.1.insert(row_text_id(rec), now);
                    }
                }
                on_row(done.fetch_add(1, Ordering::SeqCst) + 1, rows.len());
            });
        }
    });
    let (mut v, rows) = out.into_inner().unwrap_or_else(|e| e.into_inner());
    v.sort();
    (v, rows)
}

// ---------------------------------------------------------------------------------------
// The structural rebuild (round 2, finding 1(b))
// ---------------------------------------------------------------------------------------

/// The total time `check` spends on structural rebuilds of full-tier rows.
pub const STRUCT_BUDGET: Duration = Duration::from_secs(45);

/// The total time `check` spends on the structural rebuild of the rows `decide` reads (round 3,
/// finding 2).  It is its own budget, separate from [`STRUCT_BUDGET`], because these rows are not
/// optional: every comparable, non-[`crate::cost::STRUCT_HEAVY`] decision row must be rebuilt and
/// compared, and one not reached or not verified is a violation naming the row.  Sized from the
/// plan: the 80 such rows rebuild in about 75 s on one job on the reference host (Apple silicon,
/// release build), so 150 s is twice that; a host more than twice as slow at `--jobs 1` fails
/// loudly rather than skipping rows quietly.
pub const DECISION_STRUCT_BUDGET: Duration = Duration::from_secs(150);

/// Slack past the end of the structural budget for a child that started inside it.
pub const STRUCT_SLACK: Duration = Duration::from_secs(30);

/// Is this full-tier row one whose structure is compared by a rebuild?  A row that ended before
/// every obligation was built (`killed`, `oom`, `crashed`, or a chain that stopped on a cap or on
/// a SAT segment) records a partial structure, which a complete rebuild cannot equal; such a row
/// is counted and left to `check --full`.  A single-instance `unknown` row built its whole
/// instance before the solve, so it is comparable.  An enumeration row has no instance: it is
/// re-run and compared like any other deterministic row.
pub fn structure_comparable(r: &Row) -> bool {
    if r.experiment.starts_with('E') {
        return true;
    }
    // A B5 row rebuilds its one unswept miter whole, whatever its result (`exp::is_segmented`).
    let chain = crate::exp::is_segmented(&r.experiment);
    match r.result.as_str() {
        "unsat" | "collapsed" => true,
        // A chain that stopped on a SAT segment built only the segments up to it (round 3,
        // finding 4); a single instance that came out SAT was built whole.
        "sat" | "unknown" => !chain,
        _ => false,
    }
}

/// What a structural rebuild did.
#[derive(Debug, Default, PartialEq)]
pub struct StructReport {
    /// Full-tier rows (the population).
    pub population: usize,
    /// Rows the spot check had already re-run in full (not rebuilt again).
    pub covered_by_spot: usize,
    /// Rows rebuilt and compared.
    pub verified: usize,
    /// Rows whose recorded structure is partial (counted, left to `check --full`).
    pub not_comparable: usize,
    /// Rows the plan classes too slow to rebuild in CI ([`STRUCT_HEAVY`]).
    pub heavy: usize,
    /// Rows started that did not finish inside the budget: not verified (a failure).
    pub unverified: usize,
    /// Rows not reached before the budget was spent.
    pub not_reached: usize,
    /// Rows of groups `decide` reads, in the population.
    pub critical: usize,
    /// Of those, rows re-run by the spot check or rebuilt, and compared.
    pub critical_verified: usize,
    /// Of those, rows the plan classes too slow to rebuild (left to `check --full`).
    pub critical_heavy: usize,
    /// Of those, rows whose recorded structure is partial (left to `check --full`).
    pub critical_partial: usize,
    /// What differed.
    pub violations: Vec<String>,
    /// Why nothing ran (no process limits off unix).
    pub skipped: Option<String>,
}

impl StructReport {
    /// The one line `check` prints.
    pub fn line(&self) -> String {
        match &self.skipped {
            Some(why) => format!("structural rebuild: skipped ({why})"),
            None => format!(
                "structural rebuild: population {}, re-run in full by the spot check {}, rebuilt \
                 {}, plan-heavy (left to --full): {}, partial record: {}, not reached: {}; \
                 decision rows verified {} of {} ({} plan-heavy, {} partial record: left to \
                 --full; any other decision row not verified is a violation)",
                self.population,
                self.covered_by_spot,
                self.verified,
                self.heavy,
                self.not_comparable,
                self.not_reached,
                self.critical_verified,
                self.critical,
                self.critical_heavy,
                self.critical_partial
            ),
        }
    }
}

/// The full-tier rows in the order the rebuild visits them: the rows of the groups `decide` reads
/// first, then the rest, each part in the file-seeded order.
pub fn structure_order(res: &Results) -> Vec<usize> {
    let seed = sample_seed(res);
    let mut keyed: Vec<(bool, String, usize)> = res
        .rows
        .iter()
        .enumerate()
        .filter(|(_, r)| r.tier == "full")
        .map(|(i, r)| {
            (
                !decision_read(r),
                sha256_hex(format!("{seed}:structure:{i}").as_bytes()),
                i,
            )
        })
        .collect();
    keyed.sort();
    keyed.into_iter().map(|(_, _, i)| i).collect()
}

/// Rebuild one full-tier row's instance in a child, WITHOUT solving it, and compare the
/// deterministic structure: the digest, size and entropy (an enumeration row is re-run and
/// compared in full).  A child the bound kills is **not verified**, a violation, never a pass.
pub fn structure_one(
    exe: &Path,
    rec: &Row,
    planned: &std::collections::BTreeSet<(String, String, String)>,
    bound: Option<Duration>,
) -> Vec<String> {
    let id = (rec.experiment.clone(), rec.mode.clone(), rec.knob.key());
    if !planned.contains(&id) {
        return vec![format!(
            "{}: not in the plan, refused without rebuilding",
            row_text_id(rec)
        )];
    }
    let now = runner::run_sub_bounded(exe, "structure", &spec_of(rec), bound);
    if now.result == "killed" || now.result == "oom" {
        return vec![format!(
            "{}: not verified: the structural rebuild did not finish inside its budget",
            row_text_id(rec)
        )];
    }
    if rec.experiment.starts_with('E') {
        return compare_rerun(rec, &now);
    }
    if now.result == "crashed" {
        return vec![format!(
            "{}: the structural rebuild failed: {}",
            row_text_id(rec),
            now.note
        )];
    }
    compare_structure(rec, &now)
}

/// What one pass of the structural rebuild did over a list of rows.
#[derive(Default)]
struct Pass {
    verified: Vec<usize>,
    not_comparable: usize,
    heavy: usize,
    unverified: usize,
    violations: Vec<String>,
    /// The rows whose turn came (compared, counted as partial or heavy) before the budget ended.
    reached: std::collections::BTreeSet<usize>,
}

/// Rebuild the rows of `order` in children, `jobs` at a time, until `budget` is spent.
fn structure_pass(
    res: &Results,
    exe: &Path,
    jobs: usize,
    budget: Duration,
    slack: Duration,
    order: &[usize],
) -> Pass {
    let planned = planned_identities();
    let start = Instant::now();
    let next = AtomicUsize::new(0);
    let state = Mutex::new(Pass::default());
    std::thread::scope(|s| {
        for _ in 0..runner::clamp_jobs(jobs) {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::SeqCst);
                let Some(&row_index) = order.get(i) else {
                    break;
                };
                let rec = &res.rows[row_index];
                if !structure_comparable(rec) {
                    let mut g = state.lock().unwrap_or_else(|e| e.into_inner());
                    g.not_comparable += 1;
                    g.reached.insert(row_index);
                    continue;
                }
                if crate::cost::structure_heavy(&rec.experiment, &rec.mode, &rec.knob.key()) {
                    let mut g = state.lock().unwrap_or_else(|e| e.into_inner());
                    g.heavy += 1;
                    g.reached.insert(row_index);
                    continue;
                }
                let spent = start.elapsed();
                if spent >= budget {
                    break;
                }
                let bound = budget.saturating_sub(spent) + slack;
                let diffs = structure_one(exe, rec, &planned, Some(bound));
                let mut g = state.lock().unwrap_or_else(|e| e.into_inner());
                g.reached.insert(row_index);
                let unverified = diffs
                    .iter()
                    .filter(|d| d.contains(": not verified"))
                    .count();
                g.unverified += unverified;
                if unverified == 0 {
                    g.verified.push(row_index);
                }
                g.violations.extend(diffs);
            });
        }
    });
    state.into_inner().unwrap_or_else(|e| e.into_inner())
}

/// Rebuild, in children, the structure of every full-tier row the plan does not class heavy, in
/// two passes (round 3, finding 2).  First the rows `decide` reads, under
/// [`DECISION_STRUCT_BUDGET`]: each comparable, non-heavy one must be rebuilt and compared, and one
/// left unreached (or not verified) is a violation naming it.  Then every other row under
/// `budget`, where a row past the budget is only counted.  `covered` (sorted) are the rows the
/// spot check already re-ran in full and compared; they are not rebuilt again.  Off unix nothing
/// runs.
pub fn structure_check(
    res: &Results,
    exe: &Path,
    jobs: usize,
    budget: Duration,
    slack: Duration,
    covered: &[usize],
) -> StructReport {
    structure_check_with(
        res,
        exe,
        jobs,
        DECISION_STRUCT_BUDGET,
        budget,
        slack,
        covered,
    )
}

/// [`structure_check`] with the decision pass's budget given (tests shorten it).
pub fn structure_check_with(
    res: &Results,
    exe: &Path,
    jobs: usize,
    decision_budget: Duration,
    budget: Duration,
    slack: Duration,
    covered: &[usize],
) -> StructReport {
    // A row the spot check already re-ran in full has had its structure compared too.
    let all = structure_order(res);
    let (done, order): (Vec<usize>, Vec<usize>) =
        all.iter().partition(|i| covered.binary_search(i).is_ok());
    let (decision_order, rest_order): (Vec<usize>, Vec<usize>) =
        order.iter().partition(|&&i| decision_read(&res.rows[i]));
    let in_decision = |v: &[usize]| v.iter().filter(|&&i| decision_read(&res.rows[i])).count();
    let mut report = StructReport {
        population: all.len(),
        covered_by_spot: done.len(),
        critical: in_decision(&all),
        critical_verified: in_decision(&done),
        ..StructReport::default()
    };
    if !cfg!(unix) {
        report.skipped = Some("the process limits are unix-only".to_string());
        return report;
    }
    let first = structure_pass(res, exe, jobs, decision_budget, slack, &decision_order);
    let second = structure_pass(res, exe, jobs, budget, slack, &rest_order);
    let mut violations = first.violations;
    // A comparable, non-heavy decision row whose turn never came is a violation, never a count.
    for &i in &decision_order {
        let rec = &res.rows[i];
        if !first.reached.contains(&i) {
            violations.push(format!(
                "{}: decision row not verified: the structural rebuild did not reach it inside \
                 the decision budget ({}s)",
                row_text_id(rec),
                decision_budget.as_secs()
            ));
        }
    }
    report.verified = first.verified.len() + second.verified.len();
    report.critical_verified += first.verified.len();
    report.not_comparable = first.not_comparable + second.not_comparable;
    report.heavy = first.heavy + second.heavy;
    report.critical_heavy = decision_order
        .iter()
        .filter(|&&i| {
            let r = &res.rows[i];
            structure_comparable(r)
                && crate::cost::structure_heavy(&r.experiment, &r.mode, &r.knob.key())
        })
        .count();
    report.critical_partial = decision_order
        .iter()
        .filter(|&&i| !structure_comparable(&res.rows[i]))
        .count();
    report.unverified = first.unverified + second.unverified;
    report.not_reached = rest_order.len().saturating_sub(second.reached.len());
    violations.extend(second.violations);
    violations.sort();
    report.violations = violations;
    report
}
