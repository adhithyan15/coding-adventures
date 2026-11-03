//! # Tseitin is exact, and miters give certified proofs
//!
//! Two kinds of test:
//!
//! 1. **Tseitin exactness.** For random AIGs and every assignment of their
//!    inputs, the CNF with the inputs fixed (as assumptions) is SAT, and the
//!    output literal's value in the model equals the simulated output.
//!    Equisatisfiable is not enough here: the encoding must make every gate
//!    variable hold exactly its gate's value.
//! 2. **Miters.** To prove two circuits equal for **every** input, assert
//!    "their outputs differ" and show that is UNSAT. Each UNSAT is
//!    certified by the trusted kernel `drat-check`, so the result is VER00's
//!    `proved` label: all 2^(2w) input pairs at once. A deliberately wrong
//!    identity must instead be SAT, with a counterexample that really is one.

use bit_blaster::aig::{Aig, Edge};
use bit_blaster::bv::{self, BitVec};
use bit_blaster::tseitin::Tseitin;
use cdcl_sat::{write_dimacs, Cnf, SolveResult, Solver};

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

#[test]
fn tseitin_outputs_match_simulation_on_every_input() {
    let mut rng = Rng(0xA16_0000_0001);
    for round in 0..200 {
        let n_inputs = 1 + rng.below(8);
        let mut g = Aig::new();
        let mut pool: Vec<Edge> = (0..n_inputs).map(|_| g.input()).collect();
        pool.push(Edge::TRUE);
        for _ in 0..rng.below(40) {
            let a = pool[rng.below(pool.len())];
            let b = pool[rng.below(pool.len())];
            let (a, b) = (if rng.next() & 1 == 1 { !a } else { a }, if rng.next() & 1 == 1 { !b } else { b });
            let e = match rng.below(3) {
                0 => g.and(a, b),
                1 => g.xor(a, b),
                _ => {
                    let s = pool[rng.below(pool.len())];
                    g.mux(s, a, b)
                }
            };
            pool.push(e);
        }
        let out = *pool.last().unwrap();

        let mut s = Solver::new();
        let mut t = Tseitin::new();
        let out_lit = t.lit(&g, out, &mut s);
        let input_lits: Vec<_> = (0..n_inputs).map(|i| t.lit(&g, g.input_edge(i).unwrap(), &mut s)).collect();
        for bits in 0..1u32 << n_inputs {
            let assignment: Vec<bool> = (0..n_inputs).map(|i| bits >> i & 1 == 1).collect();
            let assumptions: Vec<_> =
                input_lits.iter().zip(&assignment).map(|(&l, &v)| if v { l } else { !l }).collect();
            assert_eq!(s.solve_with_assumptions(&assumptions), SolveResult::Sat, "round {round}");
            let got = s.model()[out_lit.var().index()] != out_lit.is_negated();
            assert_eq!(got, g.eval(out, &assignment), "round {round}, inputs {bits:b}");
        }
    }
}

/// Prove `lhs == rhs` for every input by refuting the miter, and certify
/// the refutation with the trusted kernel.
fn prove_equal(g: &Aig, lhs: &BitVec, rhs: &BitVec) {
    let mut g = g.clone();
    let same = bv::eq(&mut g, lhs, rhs);
    let mut cnf = Cnf::new(0);
    Tseitin::new().assert_true(&g, !same, &mut cnf);
    let mut s = Solver::from_cnf_with_proof(&cnf);
    assert_eq!(s.solve(), SolveResult::Unsat, "the miter has a counterexample");
    let proof = s.proof().unwrap().to_drat_text();
    drat_check::check(&write_dimacs(&cnf), &proof).expect("the kernel certifies the proof");
}

/// Prove an Edge-valued identity (a predicate that must always hold).
fn prove_always(g: &Aig, claim: Edge) {
    let mut cnf = Cnf::new(0);
    Tseitin::new().assert_true(g, !claim, &mut cnf);
    let mut s = Solver::from_cnf_with_proof(&cnf);
    assert_eq!(s.solve(), SolveResult::Unsat);
    drat_check::check(&write_dimacs(&cnf), &s.proof().unwrap().to_drat_text()).expect("certified");
}

#[test]
fn arithmetic_identities_are_proved_and_certified() {
    for w in [8, 16, 32] {
        let mut g = Aig::new();
        let a = BitVec::input(&mut g, w);
        let b = BitVec::input(&mut g, w);

        // a − b = a + ¬b + 1, written with a separate adder and constant.
        let lhs = bv::sub(&mut g, &a, &b);
        let not_b = bv::not(&b);
        let partial = bv::add(&mut g, &a, &not_b);
        let rhs = bv::add(&mut g, &partial, &BitVec::constant(w, 1));
        prove_equal(&g, &lhs, &rhs);

        // a + b = b + a (structural hashing does not make these identical:
        // the carry chains differ).
        let ab = bv::add(&mut g, &a, &b);
        let ba = bv::add(&mut g, &b, &a);
        prove_equal(&g, &ab, &ba);

        // De Morgan, bit-vector style: ¬(a ∧ b) = ¬a ∨ ¬b.
        let and = bv::and(&mut g, &a, &b);
        let lhs = bv::not(&and);
        let rhs = bv::or(&mut g, &bv::not(&a), &bv::not(&b));
        prove_equal(&g, &lhs, &rhs);

        // a ⊕ a = 0 and (a ⊕ b) ⊕ b = a.
        let x = bv::xor(&mut g, &a, &b);
        let back = bv::xor(&mut g, &x, &b);
        prove_equal(&g, &back, &a);

        // Comparisons: a <u b ⇔ ¬(a ≥u b), and a <s b ⇔ b >s a.
        let lt = bv::ult(&mut g, &a, &b);
        let ge = bv::uge(&mut g, &a, &b);
        let claim = g.xor(lt, ge);
        prove_always(&g, claim);
        let slt = bv::slt(&mut g, &a, &b);
        let sgt = bv::sgt(&mut g, &b, &a);
        let claim = g.xnor(slt, sgt);
        prove_always(&g, claim);
    }
}

#[test]
fn shift_and_multiply_identities_at_eight_bits() {
    let w = 8;
    let mut g = Aig::new();
    let a = BitVec::input(&mut g, w);
    // a << 1 = a + a, and a · 2 = a << 1.
    let shifted = bv::shl(&mut g, &a, &BitVec::constant(w, 1));
    let doubled = bv::add(&mut g, &a, &a);
    prove_equal(&g, &shifted, &doubled);
    let times_two = bv::mul(&mut g, &a, &BitVec::constant(w, 2));
    prove_equal(&g, &times_two, &shifted);
    // lshr by a variable amount then shl back clears the low bits only:
    // ((a >> s) << s) = a ∧ (ones << s).
    let s = BitVec::input(&mut g, w);
    let down = bv::lshr(&mut g, &a, &s);
    let round_trip = bv::shl(&mut g, &down, &s);
    let ones = BitVec::constant(w, 0xFF);
    let high_mask = bv::shl(&mut g, &ones, &s);
    let masked = bv::and(&mut g, &a, &high_mask);
    prove_equal(&g, &round_trip, &masked);
}

#[test]
fn a_false_identity_yields_a_real_counterexample() {
    // Claim: a + b = a ∨ b. False whenever a and b share a set bit.
    let w = 16;
    let mut g = Aig::new();
    let a = BitVec::input(&mut g, w);
    let b = BitVec::input(&mut g, w);
    let sum = bv::add(&mut g, &a, &b);
    let or = bv::or(&mut g, &a, &b);
    let same = bv::eq(&mut g, &sum, &or);
    let mut s = Solver::new();
    let mut t = Tseitin::new();
    t.assert_true(&g, !same, &mut s);
    assert_eq!(s.solve(), SolveResult::Sat);
    let read = |v: &BitVec| -> u32 {
        v.bits.iter().enumerate().map(|(i, &e)| {
            let input = (0..g.num_inputs()).find(|&k| g.input_edge(k) == Some(e)).unwrap();
            (t.input_value(&g, input, s.model()).unwrap_or(false) as u32) << i
        }).sum()
    };
    let (va, vb) = (read(&a), read(&b));
    assert_ne!(va.wrapping_add(vb) & 0xFFFF, va | vb, "the counterexample must be one: a={va} b={vb}");
}
