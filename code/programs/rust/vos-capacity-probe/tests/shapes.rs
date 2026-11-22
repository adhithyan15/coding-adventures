//! Every obligation shape at a small knob: it builds, it is UNSAT, its proof is accepted under the
//! default checker limits, its G3 control is SAT and replays on the concrete machines, and it is
//! deterministic in `(knob, seed)` (G4).

use std::time::{Duration, Instant};
use vos_capacity_probe::inst::{dimacs_sha, entropy, Instance};
use vos_capacity_probe::shapes::*;
use vos_capacity_probe::solve::{check_default, model_satisfies, solve_cnf, SolveOpts, Verdict};

fn unsat_with_proof(inst: &Instance) {
    let (cnf, _) = inst.cnf();
    let out = solve_cnf(&cnf, &SolveOpts::certified(Duration::from_secs(600)));
    assert_eq!(out.verdict, Verdict::Unsat);
    let chk = check_default(&cnf, out.drat.as_deref().unwrap());
    assert!(chk.accepted, "{chk:?}");
}

/// The control at the first site that is SAT: model checks, premises hold, replay says "differs".
fn control_replays(sh: &dyn Shape, seed: u64) {
    for site in sh.sites(seed).into_iter().take(8) {
        let ci = sh.build(seed, Some(site)).unwrap();
        let (cnf, ts) = ci.cnf();
        let o = solve_cnf(
            &cnf,
            &SolveOpts {
                deadline: Instant::now() + Duration::from_secs(300),
                proof: false,
                conflict_cap: Some(200_000),
            },
        );
        if let Verdict::Sat(m) = o.verdict {
            assert!(model_satisfies(&cnf, &m));
            let (inputs, vals) = ci.assignment(&ts, &m);
            assert!(
                ci.eval(&inputs, &ci.premises).iter().all(|b| *b),
                "premises hold on the model"
            );
            assert!(
                ci.eval(&inputs, &[ci.miter])[0],
                "the AIG itself says the sides differ"
            );
            assert_eq!(
                (ci.replay)(&vals),
                Ok(true),
                "the counterexample replays on the concrete machines"
            );
            return;
        }
    }
    panic!("no SAT control found");
}

fn full(name: &str, sh: &dyn Shape) {
    for seed in [1u64, 2] {
        let inst = sh
            .build(seed, None)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        unsat_with_proof(&inst);
        control_replays(sh, seed);
        // Deterministic: the same (knob, seed) gives the same CNF.
        let again = sh.build(seed, None).unwrap();
        assert_eq!(
            dimacs_sha(&inst.cnf().0).1,
            dimacs_sha(&again.cnf().0).1,
            "{name} seed {seed}"
        );
    }
}

#[test]
fn a1_pick_at_every_width() {
    for n in [8, 16, 32, 64] {
        full(&format!("A1 {n}"), &A1 { n });
    }
}

#[test]
fn a2_insert_remove_at_every_width() {
    for n in [8, 16, 32, 64] {
        full(&format!("A2 {n}"), &A2 { n });
    }
}

#[test]
fn b_programs_and_their_cut_segments() {
    full(
        "B 25 25%",
        &BShape {
            len: 25,
            divergence: 0.25,
        },
    );
    full(
        "B3 segment",
        &BSegment {
            len: 25,
            divergence: 0.25,
            range: (6, 14),
        },
    );
}

#[test]
fn b1_an_identical_sequence_collapses() {
    let inst = BShape {
        len: 60,
        divergence: 0.0,
    }
    .build(1, None)
    .unwrap();
    assert_eq!(inst.cone_ands(), 0);
    let (cnf, _) = inst.cnf();
    assert_eq!(
        inst.collapse_reason(cnf.clauses.len(), 0),
        Some("identical-structure")
    );
    assert!(inst
        .verify_collapse_reason("identical-structure", cnf.clauses.len(), 0)
        .is_ok());
    control_replays(
        &BShape {
            len: 60,
            divergence: 0.0,
        },
        1,
    );
}

#[test]
fn b4_arithmetic_shapes() {
    full(
        "assoc 3",
        &B4 {
            kind: B4Kind::Assoc(3),
            w: 32,
        },
    );
    full(
        "mul5 8",
        &B4 {
            kind: B4Kind::Mul5,
            w: 8,
        },
    );
    full(
        "mul12 16",
        &B4 {
            kind: B4Kind::Mul12,
            w: 16,
        },
    );
}

#[test]
fn b4_two_terms_collapse_because_reassociation_is_trivial() {
    let inst = B4 {
        kind: B4Kind::Assoc(2),
        w: 32,
    }
    .build(1, None)
    .unwrap();
    assert_eq!(inst.cone_ands(), 0);
}

#[test]
fn c1_self_composition_is_unsat_and_its_leak_variant_is_sat() {
    full(
        "C1 A1",
        &C1 {
            base: C1Base::A1(8),
        },
    );
    full(
        "C1 B",
        &C1 {
            base: C1Base::B(12, 500),
        },
    );
}

#[test]
fn c2_padded_loads_are_constant_time_and_the_indexed_load_leaks() {
    full("C2", &C2 { t: 8 });
}

#[test]
fn d_memory_variants() {
    full(
        "D1",
        &D {
            t: 4,
            k: 2,
            kind: DKind::Sym,
        },
    );
    full(
        "D2",
        &D {
            t: 8,
            k: 3,
            kind: DKind::Concrete(5),
        },
    );
    full(
        "D4",
        &D {
            t: 8,
            k: 3,
            kind: DKind::Cut(1),
        },
    );
}

#[test]
fn the_seed_changes_the_variable_numbering_of_a_deterministic_shape() {
    let a = A1 { n: 16 }.build(1, None).unwrap();
    let b = A1 { n: 16 }.build(2, None).unwrap();
    assert_ne!(
        dimacs_sha(&a.cnf().0).1,
        dimacs_sha(&b.cnf().0).1,
        "the input-order shuffle is the seed's effect"
    );
    assert_eq!(a.cone_ands(), b.cone_ands(), "but not the circuit");
}

#[test]
fn liveness_of_a_generated_program_is_measured_on_behaviour() {
    let inst = BShape {
        len: 100,
        divergence: 0.25,
    }
    .build(1, None)
    .unwrap();
    let e = entropy(&inst, 1);
    assert_eq!(
        e.sensitive, e.declared,
        "every input bit of the accumulator program reaches an output"
    );
    assert!(!e.below_half());
}

#[test]
fn a_unit_test_of_the_mask_in_b4_widths_below_32_bits() {
    // With W = 8 only the low byte is compared and only 8 input bits per operand are declared.
    let inst = B4 {
        kind: B4Kind::Mul5,
        w: 8,
    }
    .build(1, None)
    .unwrap();
    assert_eq!(inst.declared_bits(), 8);
    assert_eq!(inst.code_out.len(), 8);
}

#[test]
fn d5_touched_and_frame_obligations_and_both_controls() {
    for mode in [D5Mode::Index { c: 2, j: 0 }, D5Mode::Symbolic { j: 0 }] {
        full(
            &format!("D5Touched {mode:?}"),
            &D5Touched { t: 4, k: 2, mode },
        );
        // Unmutated frame: identical-structure (Index) or UNSAT / propagation-only (Symbolic).
        let frame = D5Frame { t: 4, k: 2, mode };
        let inst = frame.build(1, None).unwrap();
        if matches!(mode, D5Mode::Index { .. }) {
            assert_eq!(inst.cone_ands(), 0);
            let (cnf, _) = inst.cnf();
            assert_eq!(
                inst.collapse_reason(cnf.clauses.len(), 0),
                Some("identical-structure")
            );
        } else {
            unsat_with_proof(&inst);
        }
        // Stray extra store mutant: caught by the frame obligation, SAT and replayed.
        control_replays(&frame, 1);
    }
}

#[test]
fn d5_and_b3_cuts_run_row_and_structure_agree() {
    use vos_capacity_probe::exp::{b3_c3_knobs, d5_knobs, run_row, structure_of, RowSpec};
    use vos_capacity_probe::results::compare_structure;
    use vos_capacity_probe::row::{Knob, Kv};

    assert_eq!(b3_c3_knobs().len(), 72);
    assert!(b3_c3_knobs().iter().all(|k| !k.quick));
    let d5k = d5_knobs();
    assert_eq!(d5k.len(), 24);
    assert_eq!(d5k.iter().filter(|k| k.quick).count(), 2);

    for cut in ["every2", "every4"] {
        for mode in ["certified", "incremental"] {
            let spec = RowSpec {
                experiment: "B3".into(),
                knob: Knob::of(&[
                    ("length", Kv::I(25)),
                    ("divergence", Kv::F(0.25)),
                    ("cut", Kv::S(cut.into())),
                ]),
                mode: mode.into(),
                seed: 1,
                seeds: 5,
                tier: "full".into(),
            };
            let row = run_row(&spec, Duration::from_secs(60), &mut |_| {});
            assert_eq!(row.result, "unsat", "{cut} {mode}: {:?}", row.note);
            assert!(row.control.ok(), "{cut} {mode}: {:?}", row.control);
            assert_eq!(row.wall_s, Some(600));
            let s = structure_of(&spec).unwrap();
            assert_eq!(row.instance_sha256, s.instance_sha256);
            assert_eq!(row.ands, s.ands);
        }
    }

    for (split, t, k) in [
        ("index", 4, 1),
        ("index", 16, 4),
        ("none", 4, 1),
        ("none", 16, 1),
    ] {
        let spec = RowSpec {
            experiment: "D5".into(),
            knob: Knob::of(&[
                ("t", Kv::I(t)),
                ("k", Kv::I(k)),
                ("split", Kv::S(split.into())),
            ]),
            mode: "certified".into(),
            seed: 1,
            seeds: 5,
            tier: if t == 4 && k == 1 { "quick" } else { "full" }.into(),
        };
        let row = run_row(&spec, Duration::from_secs(120), &mut |_| {});
        assert_eq!(row.result, "unsat", "D5 {split} {t} {k}: {}", row.note);
        assert!(row.control.ok(), "D5 {split} {t} {k}: {:?}", row.control);
        assert_eq!(row.pass_default_limits, Some(true));
        assert_eq!(row.wall_s, Some(600));
        let touched_n = if split == "index" {
            (t * k) as f64
        } else {
            k as f64
        };
        assert_eq!(row.metric("touched_obligations"), Some(touched_n));
        assert_eq!(row.metric("frame_obligations"), Some(touched_n));
        if split == "index" {
            assert_eq!(row.metric("frame_collapsed"), Some(touched_n));
            assert_eq!(row.metric("frame_conflicts"), Some(0.0));
        }
        let s = structure_of(&spec).unwrap();
        let mut rebuilt = row.clone();
        rebuilt.instance_sha256 = s.instance_sha256;
        rebuilt.ands = s.ands;
        rebuilt.vars = s.vars;
        rebuilt.clauses = s.clauses;
        rebuilt.entropy_bits = s.entropy_bits;
        rebuilt.declared_bits = s.declared_bits;
        assert!(compare_structure(&row, &rebuilt).is_empty());
    }
}
