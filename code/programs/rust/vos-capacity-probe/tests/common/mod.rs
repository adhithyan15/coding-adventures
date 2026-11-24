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
use vos_capacity_probe::results::{Results, MIN_SEEDS};
use vos_capacity_probe::row::{Knob, Kv, Row};

type Id = (String, String, String, u64);

fn id_of(r: &Row) -> Id {
    (r.experiment.clone(), r.mode.clone(), r.knob.key(), r.seed)
}

/// The committed file with a fixture row added for every plan row it lacks.
pub fn complete_plan(mut res: Results) -> Results {
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
        add.push(row);
    }
    res.rows.extend(add);
    res
}
