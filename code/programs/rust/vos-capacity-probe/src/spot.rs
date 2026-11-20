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
//! whose `(experiment, mode, knob)` is not in the plan is refused without being run.

use crate::cost::is_cheap;
use crate::exp::RowSpec;
use crate::results::{compare_rerun, compare_timing, planned_identities, row_text_id, Results};
use crate::row::Row;
use crate::runner;
use coding_adventures_sha256::sha256_hex;
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

/// The fewest rows a spot check must have verified (re-run to completion and compared), or all
/// the eligible rows if there are fewer (finding M4): a check that re-ran nothing proves nothing.
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
    let id = (rec.experiment.clone(), rec.mode.clone(), rec.knob.key());
    if !planned.contains(&id) {
        return vec![format!(
            "{}: not in the plan, refused without running",
            row_text_id(rec)
        )];
    }
    let now = runner::run_one_bounded(exe, &spec_of(rec), bound);
    if bound.is_some() && now.result == "killed" && rec.result != "killed" {
        return vec![format!(
            "{}: not verified: the re-run did not finish inside the spot-check budget",
            row_text_id(rec)
        )];
    }
    let mut v = compare_rerun(rec, &now);
    if timing {
        v.extend(compare_timing(rec, &now));
    }
    v
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
                let mut g = state.lock().unwrap_or_else(|e| e.into_inner());
                g.0 += 1;
                g.2 += diffs
                    .iter()
                    .filter(|d| d.contains(": not verified"))
                    .count();
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
    let floor = SPOT_FLOOR.min(eligible_rows);
    let verified = rerun.saturating_sub(unverified);
    if verified < floor {
        violations.push(format!(
            "spot check: only {verified} rows were verified, below the floor of {floor} \
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
                let diffs = rerun_one(exe, rec, &planned, None, true);
                out.lock().unwrap_or_else(|e| e.into_inner()).extend(diffs);
                on_row(done.fetch_add(1, Ordering::SeqCst) + 1, rows.len());
            });
        }
    });
    let mut v = out.into_inner().unwrap_or_else(|e| e.into_inner());
    v.sort();
    v
}
