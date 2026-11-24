//! Shared test support: a results file that covers the WHOLE current plan.
//!
//! Since VOS-028c3c commit A the plan holds B3 `every2`/`every4`, D5 and B5, which the committed
//! results file does not contain until the recording lands (commit B).  Tests that need "a file
//! that covers the plan" (join, sign, verify fan-in) therefore complete the committed file with
//! FIXTURE rows: each missing row is a clone of a recorded sibling (B3 `every8`, D4, B2) with its
//! identity changed.  These rows are test data only, never results; once the recording lands the
//! completion adds nothing.

#![allow(dead_code)]

use std::collections::BTreeSet;
use vos_capacity_probe::exp::{plan, Tier};
use vos_capacity_probe::results::{row_text_id, Results, MIN_SEEDS};
use vos_capacity_probe::row::{Knob, Kv, Row};

type Id = (String, String, String, u64);

fn id_of(r: &Row) -> Id {
    (r.experiment.clone(), r.mode.clone(), r.knob.key(), r.seed)
}

/// The committed file with a fixture row added for every plan row it lacks.
pub fn complete_plan(res: Results) -> Results {
    complete_plan_ids(res).0
}

/// How many plan rows `res` lacks: what [`complete_plan_ids`] must fabricate for it.  Counted
/// separately from the fabrication so a test can assert the two agree.
pub fn plan_rows_missing(res: &Results) -> usize {
    let have: BTreeSet<Id> = res.rows.iter().map(id_of).collect();
    plan(Tier::Full, &[], MIN_SEEDS)
        .iter()
        .filter(|s| !have.contains(&(s.experiment.clone(), s.mode.clone(), s.knob.key(), s.seed)))
        .count()
}

/// [`complete_plan`], and the identities of the rows it fabricated, spelled exactly as the probe
/// prints a row in its findings (`results::row_text_id`, the text before the `:` of a
/// `check: <id>: ...` line).
///
/// A fabricated row is a clone of a sibling with a new identity, so no re-run can reproduce it:
/// `check` rightly reports it as differing.  A test that runs `check` on a completed file drops
/// the findings about THESE rows, and only these, with [`is_about`].  The set is empty once the
/// recording of VOS-028c3c commit B lands, and the filter is then a no-op.
pub fn complete_plan_ids(mut res: Results) -> (Results, BTreeSet<String>) {
    let mut fabricated = BTreeSet::new();
    let have: BTreeSet<Id> = res.rows.iter().map(id_of).collect();
    let mut add = Vec::new();
    for spec in plan(Tier::Full, &[], MIN_SEEDS) {
        let id = (
            spec.experiment.clone(),
            spec.mode.clone(),
            spec.knob.key(),
            spec.seed,
        );
        if have.contains(&id) {
            continue;
        }
        let find = |exp: &str, mode: &str, knob: Knob| {
            res.rows
                .iter()
                .find(|r| {
                    r.experiment == exp
                        && r.mode == mode
                        && r.knob.key() == knob.key()
                        && r.seed == spec.seed
                })
                .unwrap_or_else(|| panic!("no template row {exp} {mode} {}", knob.key()))
        };
        let k = &spec.knob;
        let mut row = match spec.experiment.as_str() {
            "B3" => find(
                "B3",
                &spec.mode,
                Knob::of(&[
                    ("length", Kv::I(k.int("length"))),
                    ("divergence", Kv::F(k.f("divergence"))),
                    ("cut", Kv::S("every8".into())),
                ]),
            )
            .clone(),
            "D5" => find(
                "D4",
                "certified",
                Knob::of(&[("t", Kv::I(k.int("t"))), ("k", Kv::I(k.int("k")))]),
            )
            .clone(),
            "B5" => {
                let mut r = find(
                    "B2",
                    "monolithic",
                    Knob::of(&[
                        ("length", Kv::I(k.int("length"))),
                        ("divergence", Kv::F(k.f("divergence"))),
                    ]),
                )
                .clone();
                r.wall_s = Some(1800);
                r
            }
            other => panic!("unexpected missing experiment {other}"),
        };
        row.experiment = spec.experiment.clone();
        row.knob = spec.knob.clone();
        row.mode = spec.mode.clone();
        row.tier = spec.tier.clone();
        fabricated.insert(row_text_id(&row));
        add.push(row);
    }
    res.rows.extend(add);
    (res, fabricated)
}

/// Whether the stderr line `line` (`check: <id>: <finding>`) is a finding about a row in `ids`.
/// The id is followed by `:` in every finding the probe prints, so `seed 1` never matches `seed 10`.
pub fn is_about(line: &str, ids: &BTreeSet<String>) -> bool {
    match line.strip_prefix("check: ") {
        Some(rest) => ids.iter().any(|id| {
            rest.strip_prefix(id.as_str())
                .is_some_and(|tail| tail.starts_with(':'))
        }),
        None => false,
    }
}
