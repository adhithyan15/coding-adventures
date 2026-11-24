//! Experiment B5 (VOS-028c3b): the `fuse` generator, the fused reference, the certified SAT sweep
//! with its negative control, the row runner, and the `fuse = 1` cross-rule of `validate`.
//!
//! The negative control is the important test.  A sweeper that trusts simulation signatures
//! merges two circuits that differ on exactly one input in 2^32; the certified sweep must refute
//! that candidate with a counterexample.  If the first part of the test ever stops merging, the
//! control has stopped being a control, and the test fails by itself.

use std::collections::HashMap;
use std::time::{Duration, Instant};
use vos_capacity_probe::cpu::{input_index, Cpu, Pool};
use vos_capacity_probe::exp::{b5_knobs, run_row, structure_of, RowSpec};
use vos_capacity_probe::inst::{dimacs_sha, Instance};
use vos_capacity_probe::prog::{gen_b, gen_b5, Op, Rhs};
use vos_capacity_probe::results::{b5_fuse1_violations, compare_structure};
use vos_capacity_probe::row::{Knob, Kv, Row};
use vos_capacity_probe::sem::{self, Bin};
use vos_capacity_probe::shapes::*;
use vos_capacity_probe::solve::{solve_cnf, SolveOpts, Verdict};
use vos_capacity_probe::sweep::{
    simulation_signatures, sweep, ProofMode, SweepOpts, SweepReport, SweepVerdict,
};

use bit_blaster::aig::{Aig, Edge, Node};
use bit_blaster::bv::{self, BitVec};

fn opts(mode: ProofMode, seed: u64) -> SweepOpts {
    SweepOpts {
        mode,
        seed,
        row_deadline: Instant::now() + Duration::from_secs(300),
        obligation_wall: Duration::from_secs(120),
        conflict_cap: None,
    }
}

fn run(inst: &Instance, mode: ProofMode, seed: u64) -> SweepReport {
    sweep(inst, &opts(mode, seed))
}

// ---- the generator ------------------------------------------------------------------------

/// `fuse = 1` draws nothing from the B5 stream and is exactly B2's program, with no runs; its CNF
/// is B2's CNF, byte for byte (the digest is what the cross-rule of `validate` compares).
#[test]
fn fuse_1_is_b2_in_program_and_in_cnf() {
    for (len, div, seed) in [
        (25, 0.25, 1u64),
        (25, 1.0, 2),
        (60, 0.25, 3),
        (200, 0.25, 1),
    ] {
        let p = gen_b(seed, len, div);
        let q = gen_b5(seed, len, div, 1);
        assert_eq!(q.ops, p.ops, "{len} {div} {seed}");
        assert_eq!(q.alt, p.alt);
        assert!(q.runs.is_empty());
        let b2 = BShape {
            len,
            divergence: div,
        }
        .build(seed, None)
        .unwrap();
        let b5 = B5Shape {
            len,
            divergence: div,
            fuse: 1,
        }
        .build(seed, None)
        .unwrap();
        assert_eq!(
            dimacs_sha(&b2.cnf().0).1,
            dimacs_sha(&b5.cnf().0).1,
            "{len} {div} {seed}"
        );
        // The control mutation sites are B2's too.
        assert_eq!(
            B5Shape {
                len,
                divergence: div,
                fuse: 1
            }
            .sites(seed),
            BShape {
                len,
                divergence: div
            }
            .sites(seed)
        );
    }
}

/// With `fuse > 1` the program has runs of at least two accumulating adds into one register,
/// never longer than `fuse`, never reading the accumulator, and its length is exactly `len`.
#[test]
fn fused_programs_have_well_formed_runs() {
    for fuse in [2usize, 4, 8] {
        for seed in 1..=3u64 {
            let g = gen_b5(seed, 200, 0.25, fuse);
            assert_eq!(g.ops.len(), 200);
            assert_eq!(g.alt.len(), 200);
            assert!(!g.runs.is_empty(), "fuse {fuse} seed {seed}");
            let mut last_end = 0;
            for &(s, e) in &g.runs {
                assert!(e - s >= 2 && e - s <= fuse, "run [{s}, {e}) at fuse {fuse}");
                assert!(s >= last_end, "runs do not overlap");
                last_end = e;
                let mut acc = None;
                for (i, op) in g.ops[s..e].iter().enumerate() {
                    let Op::Alu {
                        k: Bin::Add,
                        rd,
                        rs1,
                        rhs: Rhs::Reg(r),
                    } = *op
                    else {
                        panic!("run [{s}, {e}) holds a non-add {op:?}");
                    };
                    assert_eq!(rd, rs1);
                    assert_ne!(r, rd, "a run never reads its accumulator");
                    assert!(acc.is_none_or(|a| a == rd), "one accumulator per run");
                    acc = Some(rd);
                    if i > 0 {
                        assert!((1..=8).contains(&r), "added terms come from x1..x8");
                        assert_eq!(g.alt[s + i], None, "only the first add carries an alt");
                    }
                }
            }
        }
    }
}

/// The reference's right-nested run shares no AND node with the code's left-to-right chain: the
/// code's intermediate `a + b` has no counterpart on the reference side, which is what makes the
/// equivalence points sparse.
#[test]
fn the_reference_right_nest_shares_no_intermediate_with_the_code() {
    for w in [2usize, 3, 4] {
        let mut aig = Aig::new();
        let mut pool = Pool::new();
        // a += x2 ; a += x3 ; ... with a = x1: a run of w adds.
        let ops: Vec<Op> = (0..w)
            .map(|j| Op::Alu {
                k: Bin::Add,
                rd: 1,
                rs1: 1,
                rhs: Rhs::Reg(2 + j as u8),
            })
            .collect();
        let alt = vec![None; w];
        let mut code = Cpu::new("");
        let mut intermediates: Vec<BitVec> = Vec::new();
        for op in &ops {
            code.exec(&mut aig, &mut pool, op, None).unwrap();
            intermediates.push(code.reg(&mut aig, &mut pool, 1));
        }
        let mut refc = Cpu::new("");
        run_reference_fused(&mut refc, &mut aig, &mut pool, &ops, &alt, &[(0, w)]).unwrap();
        let ref_final = refc.reg(&mut aig, &mut pool, 1);
        let mut in_ref = vec![false; aig.num_nodes()];
        let mut stack: Vec<usize> = ref_final.bits.iter().map(|e| e.node()).collect();
        while let Some(n) = stack.pop() {
            if std::mem::replace(&mut in_ref[n], true) {
                continue;
            }
            if let Node::And(a, b) = aig.node(n) {
                stack.push(a.node());
                stack.push(b.node());
            }
        }
        // Every code intermediate except the last (the run's result, which IS an equivalence
        // point) has no AND node in common with the reference cone.
        for (j, t) in intermediates[..w - 1].iter().enumerate() {
            for e in &t.bits {
                if let Node::And(..) = aig.node(e.node()) {
                    assert!(
                        !in_ref[e.node()],
                        "run of {w}: the code's intermediate {j} has a twin on the reference side"
                    );
                }
            }
        }
        // ... and the two sides compute the same function (checked by the sweep below, and here
        // concretely on a few values).
        let last = &intermediates[w - 1];
        for t in 0..8u64 {
            let mut r = Rng64(t + 1);
            let vals: Vec<bool> = (0..aig.num_inputs()).map(|_| r.bit()).collect();
            let c = sem::eval_word(&aig, last, &vals);
            let f = sem::eval_word(&aig, &ref_final, &vals);
            assert_eq!(c, f, "associativity holds concretely");
        }
    }
}

struct Rng64(u64);
impl Rng64 {
    fn bit(&mut self) -> bool {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33 & 1 == 1
    }
}

/// The fused miter is UNSAT (the sides agree: addition is associative) and the program replays
/// concretely on the real simulator.  The monolithic proof is deliberately NOT checked here: a
/// monolithic associativity miter is what B5 exists to beat, and its proof is over the default
/// checker's work limit (the sweep's lemmas are what get checked, below).
#[test]
fn the_fused_miter_is_unsat_and_replays_as_equal() {
    let sh = B5Shape {
        len: 25,
        divergence: 0.25,
        fuse: 2,
    };
    let inst = sh.build(1, None).unwrap();
    let (cnf, _) = inst.cnf();
    let out = solve_cnf(
        &cnf,
        &SolveOpts {
            deadline: Instant::now() + Duration::from_secs(300),
            proof: false,
            conflict_cap: None,
        },
    );
    assert_eq!(out.verdict, Verdict::Unsat);
    let mut vals = HashMap::new();
    for r in 1..=10u32 {
        vals.insert(format!("x{r}"), r.wrapping_mul(0x9E37_79B9));
    }
    assert_eq!((inst.replay)(&vals), Ok(false));
}

// ---- the sweep ----------------------------------------------------------------------------

/// The sweep proves B5 at small knobs: UNSAT, every lemma and the final obligation accepted, the
/// counters sane.  `fuse > 1` needs real lemmas (the run boundaries); `fuse = 1` is B2 swept.
///
/// The knobs are the ones a unit test can afford.  At length 25 a `fuse = 4` sweep does not
/// finish in 100 s (one lemma needs about 285,000 conflicts: a four-operand associativity
/// miter), so `fuse = 4` is exercised at length 3, one run of three adds, which still takes about
/// ten seconds.  That is a measurement, not a test limitation to hide: B5 at `fuse = 4` is not
/// route-green on this design.
#[test]
fn the_certified_sweep_proves_b5_at_small_knobs() {
    for (len, div, fuse) in [
        (25usize, 0.25, 1usize),
        (25, 1.0, 1),
        (25, 0.25, 2),
        (25, 1.0, 2),
        (3, 0.25, 4),
    ] {
        let inst = B5Shape {
            len,
            divergence: div,
            fuse,
        }
        .build(1, None)
        .unwrap();
        let rep = run(&inst, ProofMode::Certified, 1);
        let at = format!("len {len} div {div} fuse {fuse}");
        assert_eq!(rep.verdict, SweepVerdict::Unsat, "{at}: {}", rep.note);
        assert!(rep.all_accepted && !rep.invalid, "{at}: {}", rep.note);
        assert!(rep.drat_sha256.is_some());
        assert!(rep.solves > rep.lemmas, "{at}: lemmas plus the final");
        assert!(rep.candidates >= rep.lemmas, "{at}");
        assert!(rep.total_conflicts >= rep.worst_conflicts, "{at}");
        assert!(rep.max_check_work.is_some(), "{at}");
        assert!(rep.max_drat_bytes > 0, "{at}");
        // Every wrong candidate was refuted, and every doubling of the frontier was counted.
        assert!(rep.refuted <= rep.candidates, "{at}");
        if fuse > 1 {
            assert!(rep.lemmas >= 1, "{at}: the run boundaries need lemmas");
            assert!(
                rep.worst_conflicts > 0,
                "{at}: associativity is not propagation"
            );
        }
        if fuse == 1 && div == 0.25 {
            // A second run is bit-identical in its counters and its proof digest (G4).
            let again = run(&inst, ProofMode::Certified, 1);
            assert_eq!(again.drat_sha256, rep.drat_sha256, "{at}");
            assert_eq!(
                (
                    again.lemmas,
                    again.candidates,
                    again.refuted,
                    again.escalations
                ),
                (rep.lemmas, rep.candidates, rep.refuted, rep.escalations),
                "{at}"
            );
        }
    }
}

/// G3: the program mutated at a site ends SAT through the very same sweep, and its model, rebuilt
/// from the counterexample, satisfies the UNSWEPT control CNF and replays on the concrete machines.
#[test]
fn a_mutated_b5_instance_ends_sat_with_a_replayable_model() {
    // The same small knobs as the sweep test: a control at length 25 and `fuse = 4` would first
    // have to prove the lemmas of the hard four-operand runs before it reached the mutation.
    for (len, fuse) in [(25usize, 1usize), (25, 2), (3, 4)] {
        let sh = B5Shape {
            len,
            divergence: 0.25,
            fuse,
        };
        let mut found = false;
        for site in sh.sites(1).into_iter().take(8) {
            let ci = sh.build(1, Some(site)).unwrap();
            for mode in [ProofMode::Solver, ProofMode::Certified] {
                let rep = run(&ci, mode, 1);
                if let SweepVerdict::Sat(inputs) = rep.verdict {
                    let (ccnf, ts) = ci.cnf();
                    let values = vos_capacity_probe::sweep::node_values(&ci.aig, &inputs);
                    let mut model = vec![false; ccnf.num_vars];
                    for (n, v) in values.iter().enumerate() {
                        if let Some(var) = ts.node_var(n) {
                            model[var.index()] = *v;
                        }
                    }
                    assert!(vos_capacity_probe::solve::model_satisfies(&ccnf, &model));
                    let (ins, vals) = ci.assignment(&ts, &model);
                    assert!(ci.eval(&ins, &ci.premises).iter().all(|b| *b));
                    assert!(ci.eval(&ins, &[ci.miter])[0]);
                    assert_eq!((ci.replay)(&vals), Ok(true), "fuse {fuse} {mode:?}");
                    found = true;
                } else if mode == ProofMode::Solver {
                    break;
                }
            }
            if found {
                break;
            }
        }
        assert!(found, "no SAT control at len {len} fuse {fuse}");
    }
}

/// A hand-built pair that agree on every one of the sweep's 256 patterns and differ when
/// `x == 0xDEADBEEF`: `code = x + y`, `reference = (x + y)` through the other adder, with its low
/// bit flipped exactly at that value.
fn needle() -> Instance {
    let mut aig = Aig::new();
    let mut pool = Pool::new();
    let x = pool.word(&mut aig, "x");
    let y = pool.word(&mut aig, "y");
    let code = bv::add(&mut aig, &x, &y);
    let other = sem::alternative(&mut aig, Bin::Add, 0, &x, &y).unwrap();
    let hit = bv::eq(&mut aig, &x, &BitVec::constant(32, 0xDEAD_BEEF));
    let mut reference = other.clone();
    reference.bits[0] = aig.xor(other.bits[0], hit);
    let diffs: Vec<Edge> = code
        .bits
        .iter()
        .zip(&reference.bits)
        .map(|(&a, &b)| aig.xor(a, b))
        .collect();
    let miter = aig.or_all(&diffs);
    Instance {
        aig,
        miter,
        premises: vec![],
        declared: pool.declared,
        code_out: code.bits,
        ref_out: reference.bits,
        replay: Box::new(|vals| {
            let x = vals.get("x").copied().unwrap_or(0);
            Ok(x == 0xDEAD_BEEF)
        }),
        live: None,
    }
}

fn declared_value(inst: &Instance, inputs: &[bool], name: &str) -> u32 {
    let d = inst.declared.iter().find(|d| d.name == name).unwrap();
    let mut v = 0u32;
    for (b, e) in d.bits.iter().enumerate() {
        if inputs[input_index(&inst.aig, *e).unwrap()] {
            v |= 1 << b;
        }
    }
    v
}

/// NEGATIVE CONTROL.  The needle pair is indistinguishable by the sweep's own 256 patterns, a
/// sweeper that merges on signatures alone calls the sides equal (wrongly), and the certified
/// and solver sweeps refute it with a counterexample at `x == 0xDEADBEEF` that replays.
#[test]
fn negative_control_signature_only_sweeping_is_unsound_and_the_certified_sweep_refutes_it() {
    let inst = needle();
    for seed in [1u64, 2, 3] {
        // Part 0: it IS a needle: every output bit pair has equal signatures.
        let sig = simulation_signatures(&inst.aig, seed);
        for (c, r) in inst.code_out.iter().zip(&inst.ref_out) {
            let (wc, wr) = (&sig[c.node()], &sig[r.node()]);
            let flip = c.is_complemented() != r.is_complemented();
            for (a, b) in wc.iter().zip(wr) {
                assert_eq!(a ^ b, if flip { !0 } else { 0 }, "seed {seed}");
            }
        }
        // ... yet the sides differ at the needle.
        let mut at = vec![false; inst.aig.num_inputs()];
        for d in &inst.declared {
            if d.name == "x" {
                for (b, e) in d.bits.iter().enumerate() {
                    at[input_index(&inst.aig, *e).unwrap()] = 0xDEAD_BEEFu32 >> b & 1 == 1;
                }
            }
        }
        assert!(inst.eval(&at, &[inst.miter])[0]);

        // Part 1: merging on signatures alone "proves" the sides equal.  WRONG.
        let wrong = run(&inst, ProofMode::SignatureOnly, seed);
        assert_eq!(wrong.verdict, SweepVerdict::Unsat, "seed {seed}");
        assert_eq!(wrong.solves, 1, "no lemma was solved: only the final");

        // Part 2: the certified sweep and the solver sweep refute it.
        for mode in [ProofMode::Certified, ProofMode::Solver] {
            let rep = run(&inst, mode, seed);
            let SweepVerdict::Sat(ins) = &rep.verdict else {
                panic!("seed {seed} {mode:?}: {:?}", rep.verdict);
            };
            assert!(rep.refuted >= 1, "a wrong candidate was refuted");
            assert_eq!(declared_value(&inst, ins, "x"), 0xDEAD_BEEF);
            assert!(inst.eval(ins, &[inst.miter])[0]);
            let mut vals = HashMap::new();
            vals.insert("x".to_string(), declared_value(&inst, ins, "x"));
            vals.insert("y".to_string(), declared_value(&inst, ins, "y"));
            assert_eq!((inst.replay)(&vals), Ok(true));
        }
    }
}

// ---- the row ------------------------------------------------------------------------------

fn spec(len: i64, div: f64, fuse: i64) -> RowSpec {
    RowSpec {
        experiment: "B5".into(),
        knob: Knob::of(&[
            ("length", Kv::I(len)),
            ("divergence", Kv::F(div)),
            ("fuse", Kv::I(fuse)),
        ]),
        mode: "certified".into(),
        seed: 1,
        seeds: 5,
        tier: "quick".into(),
    }
}

/// The knob grid: 24 knobs, certified; the quick tier is the smallest knob, length 25 at
/// `fuse = 1` (both divergences), because the quick tier fails on an `unknown` row (VOS-D118).
#[test]
fn the_b5_knob_grid_is_24_knobs_with_the_smallest_quick() {
    let k = b5_knobs();
    assert_eq!(k.len(), 24);
    assert!(k.iter().all(|p| p.mode == "certified"));
    assert_eq!(k.iter().filter(|p| p.quick).count(), 2);
    assert!(k
        .iter()
        .all(|p| p.quick == (p.knob.int("length") == 25 && p.knob.int("fuse") == 1)));
    // A B5 row is one instance for the collapse rule and the structural rebuild.
    assert!(!vos_capacity_probe::exp::is_segmented("B5"));
    assert!(vos_capacity_probe::exp::is_segmented("D5"));
    // B5 is in the plan since VOS-028c3c commit A: `knobs("B5")` is this grid, in this order.
    assert_eq!(vos_capacity_probe::exp::knobs("B5").len(), 24);
    assert!(vos_capacity_probe::exp::EXPERIMENTS.contains(&"B5"));
    assert_eq!(vos_capacity_probe::exp::row_wall_s("B5"), 1800);
}

/// `run_row` on the quick knobs: unsat with a replayed control, the four metrics, the budget, and
/// the structure `structure_of` rebuilds agrees with the row.  At `fuse = 1` the row's digest is
/// B2's.
#[test]
fn b5_run_row_and_structure_agree() {
    for (div, fuse) in [(0.25, 1i64), (0.25, 2), (1.0, 1)] {
        let sp = spec(25, div, fuse);
        let row = run_row(&sp, Duration::from_secs(300), &mut |_| {});
        let at = format!("div {div} fuse {fuse}: {}", row.note);
        assert_eq!(row.result, "unsat", "{at}");
        assert!(row.control.ok(), "{at}: {:?}", row.control);
        assert_eq!(row.pass_default_limits, Some(true), "{at}");
        assert_eq!(row.wall_s, Some(1800));
        for m in ["lemmas", "candidates", "refuted", "escalations"] {
            assert!(row.metric(m).is_some(), "{at}: metric {m}");
        }
        assert_eq!(row.steps as f64, row.metric("lemmas").unwrap() + 1.0);
        assert!(row.worst_step_conflicts.unwrap() <= row.conflicts);
        assert!(!row.control_instance_sha256.is_empty());
        assert!(row.drat_sha256.is_some());
        let s = structure_of(&sp).unwrap();
        let mut rebuilt = row.clone();
        rebuilt.instance_sha256 = s.instance_sha256.clone();
        rebuilt.ands = s.ands;
        rebuilt.vars = s.vars;
        rebuilt.clauses = s.clauses;
        rebuilt.entropy_bits = s.entropy_bits;
        rebuilt.declared_bits = s.declared_bits;
        assert!(compare_structure(&row, &rebuilt).is_empty(), "{at}");
        assert_eq!(row.instance_sha256, s.instance_sha256, "{at}");
        if fuse == 1 {
            let mut b2 = sp.clone();
            b2.experiment = "B2".into();
            b2.knob = Knob::of(&[("length", Kv::I(25)), ("divergence", Kv::F(div))]);
            b2.mode = "monolithic".into();
            assert_eq!(
                row.instance_sha256,
                structure_of(&b2).unwrap().instance_sha256,
                "fuse = 1 is B2"
            );
        }
    }
}

// ---- the cross-rule -----------------------------------------------------------------------

fn row(exp: &str, knob: Knob, mode: &str, sha: &str) -> Row {
    let mut r = Row::new(exp, knob, mode, 1, 5);
    r.instance_sha256 = sha.to_string();
    r
}

fn b2_knob(len: i64, div: f64) -> Knob {
    Knob::of(&[("length", Kv::I(len)), ("divergence", Kv::F(div))])
}

fn b5_knob(len: i64, div: f64, fuse: i64) -> Knob {
    Knob::of(&[
        ("length", Kv::I(len)),
        ("divergence", Kv::F(div)),
        ("fuse", Kv::I(fuse)),
    ])
}

/// `fuse = 1` is B2: a matching pair passes; a different digest is reported, naming both rows; a
/// missing B2 twin is not this rule's finding (VOS-D138); `fuse > 1` rows are not compared.
#[test]
fn validate_cross_rule_fuse_1_must_equal_b2() {
    let a = "aa".repeat(32);
    let b = "bb".repeat(32);
    let b2 = row("B2", b2_knob(25, 0.25), "monolithic", &a);
    let good = row("B5", b5_knob(25, 0.25, 1), "certified", &a);
    assert!(b5_fuse1_violations(&[b2.clone(), good.clone()]).is_empty());

    let bad = row("B5", b5_knob(25, 0.25, 1), "certified", &b);
    let v = b5_fuse1_violations(&[b2.clone(), bad]);
    assert_eq!(v.len(), 1, "{v:?}");
    assert!(v[0].contains("B5") && v[0].contains("B2"), "{v:?}");
    assert!(v[0].contains(&a) && v[0].contains(&b), "{v:?}");

    // VOS-D138: no B2 twin of that length, divergence or seed is NOT a finding here (a partial
    // file legitimately lacks it; `full_plan_covered` catches it on a complete file).
    assert!(b5_fuse1_violations(std::slice::from_ref(&good)).is_empty());
    let other_div = row("B2", b2_knob(25, 1.0), "monolithic", &b);
    assert!(b5_fuse1_violations(&[other_div, good.clone()]).is_empty());
    let mut other_seed = row("B2", b2_knob(25, 0.25), "monolithic", &b);
    other_seed.seed = 2;
    assert!(b5_fuse1_violations(&[other_seed, good]).is_empty());

    // fuse > 1 is a different instance by design.
    let fused = row("B5", b5_knob(25, 0.25, 2), "certified", &b);
    assert!(b5_fuse1_violations(&[b2, fused]).is_empty());
}
