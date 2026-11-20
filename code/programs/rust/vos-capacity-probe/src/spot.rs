//! # `spot`: the sampled full-row re-run of `check` and the complete one of `check --full`
//!
//! `check` has always re-run every *quick-tier* row, in process, on every build.  The full tier
//! (about 1,600 rows, about two hours) cannot be re-run on every build, so before VOS-D097 a
//! single hand-edited full-tier row passed `check` unnoticed.  This module gives `check` two
//! more teeth (VOS30 section 9.12, "Hardening"):
//!
//! ```text
//!   check          validate + quick re-run + a SPOT CHECK:
//!                    population = every full-tier non-E row
//!                    seed       = SHA-256 of the results file's own canonical bytes
//!                    order      = rows sorted by SHA-256(seed ":" row index)
//!                    re-run the rows in that order, each in a CHILD PROCESS under `run`'s
//!                    limits, until a fixed total time budget is spent; a row whose recorded
//!                    solve + check exceeds the per-row cap is skipped but COUNTED
//!   check --full   refuse unless the recorded tree ids equal the working tree's, then re-run
//!                  EVERY full-tier row, no sampling, each in a child process
//! ```
//!
//! ## What the sample proves, stated honestly
//!
//! Because the seed is the hash of the file, **any edit to the file reshuffles the sample**: an
//! editor cannot pick which rows will be re-run by looking at the repository, and cannot tune an
//! edit to dodge a sample it can predict.  But a single edited row among about 1,625 is caught
//! with probability only about `sample / population` per commit (1 to 2 percent for 16 to 30
//! rows).  The spot check makes a wholesale edit expensive; it is **not proof** of the file.
//! The real teeth are `check --full` and the release gate that requires it (section 9.12).
//!
//! ## Why no row is re-run in process
//!
//! A re-run builds an instance from the recorded knob.  Run in process, a hand-edited knob of
//! 10^9 would exhaust this process's memory.  Every row here runs in a child under the limits
//! of `runner` (CPU `ulimit` backstop, 2 GiB resident-set poll, process-group kill), and a row
//! whose `(experiment, mode, knob)` is not in the plan is refused without being run.

use crate::exp::RowSpec;
use crate::results::{compare_rerun, planned_identities, row_text_id, Results};
use crate::row::Row;
use crate::runner;
use coding_adventures_sha256::sha256_hex;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// The total time `check` spends on sampled full-row re-runs, so the CI cost is a fixed number.
pub const SPOT_BUDGET: Duration = Duration::from_secs(60);

/// A row whose recorded solve plus check time exceeds this is skipped by the spot check (counted
/// and printed, never silently dropped), so one slow row cannot spend the whole budget.
pub const SPOT_ROW_CAP_MS: f64 = 20_000.0;

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

/// Is this row too slow (or too time-dependent) for the spot check?  A row killed or out of
/// memory has no reproducible time; every other row costs its recorded solve plus check.
pub fn over_cap(r: &Row, cap_ms: f64) -> bool {
    matches!(r.result.as_str(), "killed" | "oom") || r.solve_ms + r.check_ms.unwrap_or(0.0) > cap_ms
}

/// What a spot check did.
#[derive(Debug, Default, PartialEq)]
pub struct SpotReport {
    /// Rows in the population.
    pub population: usize,
    /// Rows re-run.
    pub rerun: usize,
    /// Rows skipped because their recorded cost is over the cap (counted, never dropped).
    pub skipped_over_cap: usize,
    /// Rows of the seeded order not reached before the budget was spent.
    pub not_reached: usize,
    /// What differed on re-run.
    pub violations: Vec<String>,
    /// Why the spot check did not run at all, if it did not (no process limits off unix).
    pub skipped: Option<String>,
}

impl SpotReport {
    /// The one line `check` prints.
    pub fn line(&self) -> String {
        match &self.skipped {
            Some(why) => format!("spot check: skipped ({why})"),
            None => format!(
                "spot check: population {}, re-run {}, skipped over cap: {}, not reached: {}",
                self.population, self.rerun, self.skipped_over_cap, self.not_reached
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
/// refused and not run.
pub fn rerun_one(
    exe: &Path,
    rec: &Row,
    planned: &std::collections::BTreeSet<(String, String, String)>,
) -> Vec<String> {
    let id = (rec.experiment.clone(), rec.mode.clone(), rec.knob.key());
    if !planned.contains(&id) {
        return vec![format!(
            "{}: not in the plan, refused without running",
            row_text_id(rec)
        )];
    }
    let now = runner::run_one(exe, &spec_of(rec));
    compare_rerun(rec, &now)
}

/// The sampled re-run: rows in seeded order, each in a child, until `budget` is spent.  Off unix
/// there are no process limits and nothing is run (the report says so).
pub fn spot_check(
    res: &Results,
    exe: &Path,
    jobs: usize,
    budget: Duration,
    cap_ms: f64,
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
    let start = Instant::now();
    let next = AtomicUsize::new(0);
    let examined = AtomicUsize::new(0);
    let state = Mutex::new((0usize, 0usize, Vec::<String>::new()));
    std::thread::scope(|s| {
        for _ in 0..runner::clamp_jobs(jobs) {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::SeqCst);
                let Some(&row_index) = order.get(i) else {
                    break;
                };
                let rec = &res.rows[row_index];
                if over_cap(rec, cap_ms) {
                    examined.fetch_add(1, Ordering::SeqCst);
                    state.lock().unwrap_or_else(|e| e.into_inner()).1 += 1;
                    continue;
                }
                if start.elapsed() >= budget {
                    break;
                }
                examined.fetch_add(1, Ordering::SeqCst);
                let diffs = rerun_one(exe, rec, &planned);
                let mut g = state.lock().unwrap_or_else(|e| e.into_inner());
                g.0 += 1;
                g.2.extend(diffs);
            });
        }
    });
    let (rerun, skipped, mut violations) = state.into_inner().unwrap_or_else(|e| e.into_inner());
    violations.sort();
    report.rerun = rerun;
    report.skipped_over_cap = skipped;
    report.not_reached = order.len().saturating_sub(examined.load(Ordering::SeqCst));
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
    let rows: Vec<&Row> = res.rows.iter().filter(|r| r.tier == "full").collect();
    let planned = planned_identities();
    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let out = Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..runner::clamp_jobs(jobs) {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::SeqCst);
                let Some(rec) = rows.get(i) else { break };
                let diffs = rerun_one(exe, rec, &planned);
                out.lock().unwrap_or_else(|e| e.into_inner()).extend(diffs);
                on_row(done.fetch_add(1, Ordering::SeqCst) + 1, rows.len());
            });
        }
    });
    let mut v = out.into_inner().unwrap_or_else(|e| e.into_inner());
    v.sort();
    v
}
