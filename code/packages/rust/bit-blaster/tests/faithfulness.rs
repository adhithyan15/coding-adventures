//! # Is the encoding faithful? (VER10 §9)
//!
//! `exhaustive.rs` checks the *circuits* against Rust arithmetic by
//! simulating the AIG. That never touches Tseitin or the solver. This file
//! checks the whole pipeline the way a proof uses it:
//!
//! ```text
//!   term ──bit-blast──► AIG ──Tseitin──► CNF ──cdcl-sat──► model / UNSAT + DRAT
//!                                                              │
//!                          direct Rust reference semantics ◄───┘ compared
//! ```
//!
//! For every operator, every assignment of its inputs (all of them at small
//! widths, a deterministic sample at the larger ones):
//!
//! 1. **Value.** Fix the inputs as unit clauses; the CNF must be SAT and
//!    the output bits in the model must equal the reference value.
//! 2. **Uniqueness.** Add "the output differs from the reference value";
//!    the CNF must be UNSAT. (A faithful encoding is *functional*: one
//!    extension per input assignment, F2/F3 in VER10 §9.2.) At the smallest
//!    widths the UNSAT answer is certified by `drat-check` for every
//!    assignment; beyond that it is checked on a sample, and on every
//!    assignment by an assumption-based solve.
//!
//! The reference semantics below share no code with the crate.

use bit_blaster::aig::{Aig, Edge};
use bit_blaster::bv::{self, BitVec};
use bit_blaster::tseitin::Tseitin;
use cdcl_sat::{write_dimacs, Cnf, Lit, SolveResult, Solver};

// ---------------------------------------------------------------------------
// Reference semantics (SMT-LIB QF_BV), written from the definitions.
// ---------------------------------------------------------------------------

fn mask(w: usize) -> u128 {
    (1u128 << w) - 1
}

fn signed(x: u128, w: usize) -> i128 {
    if x >> (w - 1) & 1 == 1 {
        x as i128 - (1i128 << w)
    } else {
        x as i128
    }
}

fn from_signed(x: i128, w: usize) -> u128 {
    (x as u128) & mask(w)
}

fn shl_ref(a: u128, s: u128, w: usize) -> u128 {
    if s >= w as u128 {
        0
    } else {
        (a << s) & mask(w)
    }
}

fn lshr_ref(a: u128, s: u128, w: usize) -> u128 {
    if s >= w as u128 {
        0
    } else {
        a >> s
    }
}

fn ashr_ref(a: u128, s: u128, w: usize) -> u128 {
    let k = s.min(w as u128 - 1) as u32; // an arithmetic shift by ≥ w−1 is all sign bits
    from_signed(signed(a, w) >> k, w)
}

fn udiv_ref(a: u128, b: u128, w: usize) -> u128 {
    a.checked_div(b).unwrap_or(mask(w)) // SMT-LIB: a / 0 = all ones
}

fn urem_ref(a: u128, b: u128) -> u128 {
    a.checked_rem(b).unwrap_or(a) // SMT-LIB: a mod 0 = a
}

fn sdiv_ref(a: u128, b: u128, w: usize) -> u128 {
    let (sa, sb) = (signed(a, w), signed(b, w));
    if sb == 0 {
        // SMT-LIB: bvsdiv s 0 = −1 if s ≥ 0, 1 if s < 0.
        return if sa < 0 { 1 } else { mask(w) };
    }
    from_signed(sa / sb, w) // wrapping: −2^(w−1) / −1 = −2^(w−1)
}

fn srem_ref(a: u128, b: u128, w: usize) -> u128 {
    let (sa, sb) = (signed(a, w), signed(b, w));
    if sb == 0 {
        return a;
    }
    from_signed(sa % sb, w)
}

// ---------------------------------------------------------------------------
// The harness: build a circuit, encode it, solve it under every assignment.
// ---------------------------------------------------------------------------

/// How hard to check one operator.
#[derive(Clone, Copy)]
struct Effort {
    /// Check every assignment when the input space has at most this many
    /// bits; otherwise a deterministic sample of `sample` assignments.
    exhaustive_bits: usize,
    sample: usize,
    /// Certify uniqueness with a DRAT proof on every `certify_stride`-th
    /// assignment (1 = every one, 0 = none).
    certify_stride: usize,
}

/// A deterministic xorshift generator.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

/// All assignments of variables with the given widths (mixed radix), or a
/// sample that always contains the corner values.
fn assignments(widths: &[usize], effort: Effort, rng: &mut Rng) -> Vec<Vec<u128>> {
    let total_bits: usize = widths.iter().sum();
    if total_bits <= effort.exhaustive_bits {
        let mut out = Vec::new();
        for code in 0..1u128 << total_bits {
            let mut rest = code;
            let mut vals = Vec::new();
            for &w in widths {
                vals.push(rest & mask(w));
                rest >>= w;
            }
            out.push(vals);
        }
        return out;
    }
    let corners = |w: usize| [0u128, 1, 2, mask(w), mask(w) - 1, 1 << (w - 1), (1 << (w - 1)) - 1, w as u128, w as u128 - 1];
    let mut out = Vec::new();
    for &c in &corners(widths[0]) {
        for &d in &corners(*widths.last().unwrap()) {
            let mut vals: Vec<u128> = widths.iter().map(|&w| rng.next() as u128 & mask(w)).collect();
            vals[0] = c & mask(widths[0]);
            let last = vals.len() - 1;
            vals[last] = d & mask(widths[last]);
            out.push(vals);
        }
    }
    while out.len() < effort.sample {
        out.push(widths.iter().map(|&w| ((rng.next() as u128) << 64 | rng.next() as u128) & mask(w)).collect());
    }
    out
}

/// Statistics a test run reports (for the audit tables).
#[derive(Default, Debug, Clone, Copy)]
struct Stats {
    assignments: usize,
    certified: usize,
}

impl std::ops::AddAssign for Stats {
    fn add_assign(&mut self, o: Stats) {
        self.assignments += o.assignments;
        self.certified += o.certified;
    }
}

/// Encode `outs` (and every input bit, so unused inputs still get a
/// variable) and return the CNF with the literals of outputs and inputs.
fn encode(g: &Aig, outs: &[Edge]) -> (Cnf, Vec<Lit>, Vec<Lit>) {
    let mut cnf = Cnf::new(0);
    let mut t = Tseitin::new();
    let out_lits: Vec<Lit> = outs.iter().map(|&e| t.lit(g, e, &mut cnf)).collect();
    let in_lits: Vec<Lit> = (0..g.num_inputs()).map(|i| t.lit(g, g.input_edge(i).unwrap(), &mut cnf)).collect();
    (cnf, out_lits, in_lits)
}

/// The input literals fixed to the bits of `vals`.
fn fixing(in_lits: &[Lit], widths: &[usize], vals: &[u128]) -> Vec<Lit> {
    let mut lits = Vec::new();
    let mut k = 0;
    for (&w, &v) in widths.iter().zip(vals) {
        for i in 0..w {
            let l = in_lits[k];
            lits.push(if v >> i & 1 == 1 { l } else { !l });
            k += 1;
        }
    }
    lits
}

fn lit_value(model: &[bool], l: Lit) -> bool {
    model[l.var().index()] != l.is_negated()
}

/// Check one circuit against its reference on the chosen assignments.
///
/// `build` receives the AIG and one fresh `BitVec` per entry of `widths`
/// and returns the output bits (a Boolean result is one bit).
fn check_op(
    name: &str,
    widths: &[usize],
    effort: Effort,
    build: impl Fn(&mut Aig, &[BitVec]) -> Vec<Edge>,
    reference: impl Fn(&[u128]) -> u128,
) -> Stats {
    let mut g = Aig::new();
    let vars: Vec<BitVec> = widths.iter().map(|&w| BitVec::input(&mut g, w)).collect();
    let outs = build(&mut g, &vars);
    let (cnf, out_lits, in_lits) = encode(&g, &outs);

    // One incremental solver for the assumption-based checks, with a
    // selector per assignment to switch on that assignment's "output
    // differs" clause.
    let mut solver = Solver::from_cnf(&cnf);
    let mut rng = Rng(0x5EED_0000 ^ (name.len() as u64) << 8 ^ widths.iter().sum::<usize>() as u64);
    let mut stats = Stats::default();

    for (n, vals) in assignments(widths, effort, &mut rng).iter().enumerate() {
        let want = reference(vals);
        assert!(outs.len() >= 128 || want >> outs.len() == 0, "{name}: reference wider than the circuit");
        let fixed = fixing(&in_lits, widths, vals);

        // 1. Value: SAT, outputs equal the reference.
        assert_eq!(solver.solve_with_assumptions(&fixed), SolveResult::Sat, "{name} {widths:?} {vals:?}: not SAT");
        let model = solver.model();
        assert!(cnf.is_satisfied_by(model), "{name}: model does not satisfy the CNF");
        let got: u128 = out_lits.iter().enumerate().map(|(i, &l)| (lit_value(model, l) as u128) << i).sum();
        assert_eq!(got, want, "{name} {widths:?} inputs {vals:?}: CNF output {got} != reference {want}");
        // …and the AIG simulation agrees with the CNF (F1 ↔ F2).
        let bools: Vec<bool> = vals.iter().zip(widths).flat_map(|(&v, &w)| (0..w).map(move |i| v >> i & 1 == 1)).collect();
        let sim: u128 = outs.iter().enumerate().map(|(i, &e)| (g.eval(e, &bools) as u128) << i).sum();
        assert_eq!(sim, want, "{name} {widths:?} {vals:?}: AIG simulation != reference");

        // 2. Uniqueness: "some output bit differs from the reference" is UNSAT.
        let differs: Vec<Lit> =
            out_lits.iter().enumerate().map(|(i, &l)| if want >> i & 1 == 1 { !l } else { l }).collect();
        let selector = solver.new_var();
        let mut clause = vec![selector.negative()];
        clause.extend(&differs);
        solver.add_clause(&clause);
        let mut assumptions = fixed.clone();
        assumptions.push(selector.positive());
        assert_eq!(
            solver.solve_with_assumptions(&assumptions),
            SolveResult::Unsat,
            "{name} {widths:?} {vals:?}: a second, different output is possible"
        );
        solver.add_clause(&[selector.negative()]); // retire the selector

        // Certified uniqueness: a fresh proof-logging solver, checked by the kernel.
        if effort.certify_stride != 0 && n % effort.certify_stride == 0 {
            let mut c = cnf.clone();
            for &l in &fixed {
                c.add_clause(&[l]);
            }
            c.add_clause(&differs);
            let mut s = Solver::from_cnf_with_proof(&c);
            assert_eq!(s.solve(), SolveResult::Unsat, "{name}: certified run not UNSAT");
            let proof = s.proof().expect("proof logging was on").to_drat_text();
            drat_check::check(&write_dimacs(&c), &proof)
                .unwrap_or_else(|e| panic!("{name} {widths:?} {vals:?}: kernel rejected the proof: {e:?}"));
            stats.certified += 1;
        }
        stats.assignments += 1;
    }
    stats
}

// ---------------------------------------------------------------------------
// The operator catalogue.
// ---------------------------------------------------------------------------

type Build = fn(&mut Aig, &[BitVec]) -> Vec<Edge>;
type Reference = fn(&[u128], usize) -> u128;

fn bits(v: BitVec) -> Vec<Edge> {
    v.bits
}

fn one(e: Edge) -> Vec<Edge> {
    vec![e]
}

/// Binary operators on two `w`-bit operands.
fn binary_ops() -> Vec<(&'static str, Build, Reference)> {
    vec![
        ("and", |g, v| bits(bv::and(g, &v[0], &v[1])), |x, _| x[0] & x[1]),
        ("or", |g, v| bits(bv::or(g, &v[0], &v[1])), |x, _| x[0] | x[1]),
        ("xor", |g, v| bits(bv::xor(g, &v[0], &v[1])), |x, _| x[0] ^ x[1]),
        ("add", |g, v| bits(bv::add(g, &v[0], &v[1])), |x, w| x[0].wrapping_add(x[1]) & mask(w)),
        ("sub", |g, v| bits(bv::sub(g, &v[0], &v[1])), |x, w| x[0].wrapping_sub(x[1]) & mask(w)),
        ("mul", |g, v| bits(bv::mul(g, &v[0], &v[1])), |x, w| x[0].wrapping_mul(x[1]) & mask(w)),
        ("shl", |g, v| bits(bv::shl(g, &v[0], &v[1])), |x, w| shl_ref(x[0], x[1], w)),
        ("lshr", |g, v| bits(bv::lshr(g, &v[0], &v[1])), |x, w| lshr_ref(x[0], x[1], w)),
        ("ashr", |g, v| bits(bv::ashr(g, &v[0], &v[1])), |x, w| ashr_ref(x[0], x[1], w)),
        ("udiv", |g, v| bits(bv::udiv(g, &v[0], &v[1])), |x, w| udiv_ref(x[0], x[1], w)),
        ("urem", |g, v| bits(bv::urem(g, &v[0], &v[1])), |x, _| urem_ref(x[0], x[1])),
        ("sdiv", |g, v| bits(bv::sdiv(g, &v[0], &v[1])), |x, w| sdiv_ref(x[0], x[1], w)),
        ("srem", |g, v| bits(bv::srem(g, &v[0], &v[1])), |x, w| srem_ref(x[0], x[1], w)),
        ("eq", |g, v| one(bv::eq(g, &v[0], &v[1])), |x, _| (x[0] == x[1]) as u128),
        ("ult", |g, v| one(bv::ult(g, &v[0], &v[1])), |x, _| (x[0] < x[1]) as u128),
        ("ule", |g, v| one(bv::ule(g, &v[0], &v[1])), |x, _| (x[0] <= x[1]) as u128),
        ("ugt", |g, v| one(bv::ugt(g, &v[0], &v[1])), |x, _| (x[0] > x[1]) as u128),
        ("uge", |g, v| one(bv::uge(g, &v[0], &v[1])), |x, _| (x[0] >= x[1]) as u128),
        ("slt", |g, v| one(bv::slt(g, &v[0], &v[1])), |x, w| (signed(x[0], w) < signed(x[1], w)) as u128),
        ("sle", |g, v| one(bv::sle(g, &v[0], &v[1])), |x, w| (signed(x[0], w) <= signed(x[1], w)) as u128),
        ("sgt", |g, v| one(bv::sgt(g, &v[0], &v[1])), |x, w| (signed(x[0], w) > signed(x[1], w)) as u128),
        ("sge", |g, v| one(bv::sge(g, &v[0], &v[1])), |x, w| (signed(x[0], w) >= signed(x[1], w)) as u128),
        ("concat", |_, v| bits(bv::concat(&v[0], &v[1])), |x, w| x[0] << w | x[1]),
    ]
}

/// Unary operators on one `w`-bit operand.
fn unary_ops() -> Vec<(&'static str, Build, Reference)> {
    vec![
        ("not", |_, v| bits(bv::not(&v[0])), |x, w| !x[0] & mask(w)),
        ("neg", |g, v| bits(bv::neg(g, &v[0])), |x, w| x[0].wrapping_neg() & mask(w)),
        ("zext3", |_, v| bits(bv::zext(&v[0], 3)), |x, _| x[0]),
        ("sext3", |_, v| bits(bv::sext(&v[0], 3)), |x, w| from_signed(signed(x[0], w), w + 3)),
        ("zext0", |_, v| bits(bv::zext(&v[0], 0)), |x, _| x[0]),
        ("sext0", |_, v| bits(bv::sext(&v[0], 0)), |x, _| x[0]),
    ]
}

fn effort_for(w: usize) -> Effort {
    match w {
        1..=3 => Effort { exhaustive_bits: 2 * w, sample: 0, certify_stride: 1 },
        4 => Effort { exhaustive_bits: 8, sample: 0, certify_stride: 3 },
        5 | 6 => Effort { exhaustive_bits: 12, sample: 0, certify_stride: 211 },
        7 => Effort { exhaustive_bits: 0, sample: 600, certify_stride: 40 },
        _ => Effort { exhaustive_bits: 0, sample: 600, certify_stride: 50 },
    }
}

/// Run a catalogue at the given widths; returns total stats.
fn run_catalogue(widths: std::ops::RangeInclusive<usize>) -> Stats {
    let mut total = Stats::default();
    for w in widths {
        for (name, build, reference) in binary_ops() {
            total += check_op(name, &[w, w], effort_for(w), build, move |x| reference(x, w));
            // concat of unequal widths too.
            if name == "concat" && w > 1 {
                total += check_op("concat-uneven", &[w - 1, w], effort_for(w), build, move |x| x[0] << w | x[1]);
            }
        }
        for (name, build, reference) in unary_ops() {
            total += check_op(name, &[w], effort_for(w), build, move |x| reference(x, w));
        }
    }
    total
}

#[test]
fn every_operator_widths_1_to_4_exhaustively_with_certified_uniqueness() {
    let t = run_catalogue(1..=4);
    eprintln!("widths 1..=4: {} assignments solved, {} proofs certified", t.assignments, t.certified);
    assert!(t.assignments > 8_000 && t.certified > 4_000, "{t:?}");
}

#[test]
fn every_operator_widths_5_to_6_exhaustively() {
    let t = run_catalogue(5..=6);
    eprintln!("widths 5..=6: {} assignments solved, {} proofs certified", t.assignments, t.certified);
    assert!(t.assignments > 120_000, "{t:?}");
}

#[test]
fn every_operator_widths_7_and_8_on_corners_and_samples() {
    let t = run_catalogue(7..=8);
    eprintln!("widths 7..=8: {} assignments solved, {} proofs certified", t.assignments, t.certified);
    assert!(t.assignments > 36_000 - 1 && t.certified > 800, "{t:?}");
}

#[test]
fn ite_every_assignment() {
    for w in 1..=4usize {
        let e = Effort { exhaustive_bits: 1 + 2 * w, sample: 0, certify_stride: if w <= 3 { 1 } else { 5 } };
        check_op(
            "ite",
            &[1, w, w],
            e,
            |g, v| {
                let c = v[0].bits[0];
                bits(bv::ite(g, c, &v[1], &v[2]))
            },
            |x| if x[0] == 1 { x[1] } else { x[2] },
        );
        // ite whose condition is itself computed (a < b).
        check_op(
            "ite(ult)",
            &[w, w, w],
            Effort { exhaustive_bits: 3 * w, sample: 0, certify_stride: 0 },
            |g, v| {
                let c = bv::ult(g, &v[0], &v[1]);
                bits(bv::ite(g, c, &v[1], &v[2]))
            },
            |x| if x[0] < x[1] { x[1] } else { x[2] },
        );
    }
}

#[test]
fn extract_every_slice() {
    for w in 1..=6usize {
        for lo in 0..w {
            for hi in lo..w {
                check_op(
                    "extract",
                    &[w],
                    Effort { exhaustive_bits: 6, sample: 0, certify_stride: 7 },
                    move |_, v| bits(bv::extract(&v[0], hi, lo)),
                    move |x| x[0] >> lo & mask(hi - lo + 1),
                );
            }
        }
    }
}

#[test]
fn constant_shifts_every_amount() {
    for w in 1..=6usize {
        for k in 0..=w + 2 {
            let e = Effort { exhaustive_bits: 6, sample: 0, certify_stride: 9 };
            check_op("shl_const", &[w], e, move |_, v| bits(bv::shl_const(&v[0], k)), move |x| shl_ref(x[0], k as u128, w));
            check_op("lshr_const", &[w], e, move |_, v| bits(bv::lshr_const(&v[0], k)), move |x| lshr_ref(x[0], k as u128, w));
            check_op("ashr_const", &[w], e, move |_, v| bits(bv::ashr_const(&v[0], k)), move |x| ashr_ref(x[0], k as u128, w));
        }
    }
}

#[test]
fn operations_against_constants() {
    // A constant operand folds in the AIG; the folded circuit must still be
    // faithful (this is where simplification bugs would show).
    for w in 2..=4usize {
        for c in 0..1u128 << w {
            let e = Effort { exhaustive_bits: w, sample: 0, certify_stride: 0 };
            check_op("add-const", &[w], e, move |g, v| bits(bv::add(g, &v[0], &BitVec::constant(w, c))), move |x| (x[0] + c) & mask(w));
            check_op("mul-const", &[w], e, move |g, v| bits(bv::mul(g, &v[0], &BitVec::constant(w, c))), move |x| (x[0] * c) & mask(w));
            check_op("shl-const-amount-bv", &[w], e, move |g, v| bits(bv::shl(g, &v[0], &BitVec::constant(w, c))), move |x| shl_ref(x[0], c, w));
            check_op("shl-const-value", &[w], e, move |g, v| bits(bv::shl(g, &BitVec::constant(w, c), &v[0])), move |x| shl_ref(c, x[0], w));
            check_op("udiv-const", &[w], e, move |g, v| bits(bv::udiv(g, &v[0], &BitVec::constant(w, c))), move |x| udiv_ref(x[0], c, w));
            check_op("sdiv-const", &[w], e, move |g, v| bits(bv::sdiv(g, &BitVec::constant(w, c), &v[0])), move |x| sdiv_ref(c, x[0], w));
            check_op("slt-const", &[w], e, move |g, v| one(bv::slt(g, &v[0], &BitVec::constant(w, c))), move |x| (signed(x[0], w) < signed(c, w)) as u128);
        }
    }
}

// ---------------------------------------------------------------------------
// Composite terms: every expression tree up to depth 2.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Bin {
    And,
    Or,
    Xor,
    Add,
    Sub,
    Mul,
    Shl,
    Lshr,
    Ashr,
    Udiv,
    Urem,
    Sdiv,
    Srem,
}

const BINS: [Bin; 13] = [
    Bin::And,
    Bin::Or,
    Bin::Xor,
    Bin::Add,
    Bin::Sub,
    Bin::Mul,
    Bin::Shl,
    Bin::Lshr,
    Bin::Ashr,
    Bin::Udiv,
    Bin::Urem,
    Bin::Sdiv,
    Bin::Srem,
];

#[derive(Clone, Debug)]
enum Term {
    Var(usize),
    Not(Box<Term>),
    Neg(Box<Term>),
    Bin(Bin, Box<Term>, Box<Term>),
}

impl Term {
    fn build(&self, g: &mut Aig, vars: &[BitVec]) -> BitVec {
        match self {
            Term::Var(i) => vars[*i].clone(),
            Term::Not(a) => bv::not(&a.build(g, vars)),
            Term::Neg(a) => {
                let a = a.build(g, vars);
                bv::neg(g, &a)
            }
            Term::Bin(op, a, b) => {
                let (a, b) = (a.build(g, vars), b.build(g, vars));
                match op {
                    Bin::And => bv::and(g, &a, &b),
                    Bin::Or => bv::or(g, &a, &b),
                    Bin::Xor => bv::xor(g, &a, &b),
                    Bin::Add => bv::add(g, &a, &b),
                    Bin::Sub => bv::sub(g, &a, &b),
                    Bin::Mul => bv::mul(g, &a, &b),
                    Bin::Shl => bv::shl(g, &a, &b),
                    Bin::Lshr => bv::lshr(g, &a, &b),
                    Bin::Ashr => bv::ashr(g, &a, &b),
                    Bin::Udiv => bv::udiv(g, &a, &b),
                    Bin::Urem => bv::urem(g, &a, &b),
                    Bin::Sdiv => bv::sdiv(g, &a, &b),
                    Bin::Srem => bv::srem(g, &a, &b),
                }
            }
        }
    }

    fn eval(&self, vals: &[u128], w: usize) -> u128 {
        match self {
            Term::Var(i) => vals[*i],
            Term::Not(a) => !a.eval(vals, w) & mask(w),
            Term::Neg(a) => a.eval(vals, w).wrapping_neg() & mask(w),
            Term::Bin(op, a, b) => {
                let (a, b) = (a.eval(vals, w), b.eval(vals, w));
                match op {
                    Bin::And => a & b,
                    Bin::Or => a | b,
                    Bin::Xor => a ^ b,
                    Bin::Add => a.wrapping_add(b) & mask(w),
                    Bin::Sub => a.wrapping_sub(b) & mask(w),
                    Bin::Mul => a.wrapping_mul(b) & mask(w),
                    Bin::Shl => shl_ref(a, b, w),
                    Bin::Lshr => lshr_ref(a, b, w),
                    Bin::Ashr => ashr_ref(a, b, w),
                    Bin::Udiv => udiv_ref(a, b, w),
                    Bin::Urem => urem_ref(a, b),
                    Bin::Sdiv => sdiv_ref(a, b, w),
                    Bin::Srem => srem_ref(a, b, w),
                }
            }
        }
    }
}

/// Every term of depth ≤ 1 over variables `x`, `y` (depth 0 = a variable).
fn depth_one() -> Vec<Term> {
    let leaves = [Term::Var(0), Term::Var(1)];
    let mut out: Vec<Term> = leaves.to_vec();
    for l in &leaves {
        out.push(Term::Not(Box::new(l.clone())));
        out.push(Term::Neg(Box::new(l.clone())));
    }
    for op in BINS {
        for a in &leaves {
            for b in &leaves {
                out.push(Term::Bin(op, Box::new(a.clone()), Box::new(b.clone())));
            }
        }
    }
    out
}

/// Every term of depth ≤ 2: a unary or binary operator over depth ≤ 1 terms.
fn depth_two() -> Vec<Term> {
    let d1 = depth_one();
    let mut out = d1.clone();
    for t in &d1 {
        out.push(Term::Not(Box::new(t.clone())));
        out.push(Term::Neg(Box::new(t.clone())));
    }
    for op in BINS {
        for a in &d1 {
            for b in &d1 {
                out.push(Term::Bin(op, Box::new(a.clone()), Box::new(b.clone())));
            }
        }
    }
    out
}

/// Every `stride`-th term of depth ≤ 2 (stride 1 = all 43 906), at widths
/// 2 and 3, every assignment each.
fn run_depth_two(stride: usize) -> (usize, Stats) {
    let terms: Vec<Term> = depth_two().into_iter().step_by(stride).collect();
    let mut stats = Stats::default();
    let mut count = 0usize;
    for w in [2usize, 3] {
        for (n, t) in terms.iter().enumerate() {
            // Certify a thin deterministic slice of the assignments; the
            // selector-based UNSAT check runs on every assignment.
            let stride = if n % 97 == 0 { 5 } else { 0 };
            let e = Effort { exhaustive_bits: 2 * w, sample: 0, certify_stride: stride };
            let t2 = t.clone();
            stats += check_op(&format!("{t:?}"), &[w, w], e, move |g, v| bits(t2.build(g, v)), |x| t.eval(x, w));
            count += 1;
        }
    }
    (count, stats)
}

/// A deterministic 1-in-25 slice, quick enough for every CI run.
#[test]
fn terms_of_depth_two_sampled() {
    let (count, stats) = run_depth_two(25);
    eprintln!("depth-2 sample: {count} (term, width) pairs, {} assignments, {} proofs", stats.assignments, stats.certified);
    assert!(count > 3_000 && stats.assignments > 100_000);
}

/// The full enumeration: 43 906 terms × widths 2, 3 = 87 812 circuits and
/// ≈ 3.5 M assignments (about 5 minutes in a debug build).
/// Run with `cargo test -p bit-blaster --test faithfulness -- --ignored`.
#[test]
#[ignore = "exhaustive tier, ~5 minutes unoptimised"]
fn every_term_of_depth_two_at_widths_two_and_three() {
    let (count, stats) = run_depth_two(1);
    eprintln!("depth-2 trees: {count} (term, width) pairs, {} assignments, {} proofs certified", stats.assignments, stats.certified);
    assert_eq!(count, 2 * depth_two().len());
}

/// Depth-2 predicates and ite over predicates: `cmp(t1, t2)` and
/// `ite(cmp(x, y), t1, t2)`, with `t1`, `t2` of depth ≤ 1.
#[test]
fn comparison_and_ite_trees() {
    type Cmp = (&'static str, fn(&mut Aig, &BitVec, &BitVec) -> Edge, fn(u128, u128, usize) -> bool);
    let cmps: [Cmp; 9] = [
        ("eq", bv::eq, |a, b, _| a == b),
        ("ult", bv::ult, |a, b, _| a < b),
        ("ule", bv::ule, |a, b, _| a <= b),
        ("ugt", bv::ugt, |a, b, _| a > b),
        ("uge", bv::uge, |a, b, _| a >= b),
        ("slt", bv::slt, |a, b, w| signed(a, w) < signed(b, w)),
        ("sle", bv::sle, |a, b, w| signed(a, w) <= signed(b, w)),
        ("sgt", bv::sgt, |a, b, w| signed(a, w) > signed(b, w)),
        ("sge", bv::sge, |a, b, w| signed(a, w) >= signed(b, w)),
    ];
    let d1 = depth_one();
    let w = 3usize;
    let e = Effort { exhaustive_bits: 2 * w, sample: 0, certify_stride: 0 };
    let mut n = 0;
    for (name, f, r) in cmps {
        for (i, a) in d1.iter().enumerate() {
            for (j, b) in d1.iter().enumerate() {
                // Thin the 9 × 50 × 50 product on the second axis only so the
                // test stays quick while every comparison meets every shape.
                if (i + j) % 3 != 0 {
                    continue;
                }
                let (a2, b2) = (a.clone(), b.clone());
                check_op(
                    name,
                    &[w, w],
                    e,
                    move |g, v| {
                        let (x, y) = (a2.build(g, v), b2.build(g, v));
                        one(f(g, &x, &y))
                    },
                    |x| r(a.eval(x, w), b.eval(x, w), w) as u128,
                );
                n += 1;
            }
        }
        // ite(cmp(x, y), t1, t2) for a sample of branch pairs.
        for (i, a) in d1.iter().enumerate().step_by(3) {
            for (j, b) in d1.iter().enumerate().step_by(5) {
                let _ = (i, j);
                let (a2, b2) = (a.clone(), b.clone());
                check_op(
                    name,
                    &[w, w],
                    e,
                    move |g, v| {
                        let c = f(g, &v[0], &v[1]);
                        let (t, el) = (a2.build(g, v), b2.build(g, v));
                        bits(bv::ite(g, c, &t, &el))
                    },
                    |x| if r(x[0], x[1], w) { a.eval(x, w) } else { b.eval(x, w) },
                );
                n += 1;
            }
        }
    }
    eprintln!("comparison/ite trees checked: {n}");
    assert!(n > 5_000);
}

// ---------------------------------------------------------------------------
// Equivalence queries: certified identities, refuted non-identities.
// ---------------------------------------------------------------------------

/// Decide `lhs == rhs` for all inputs by solving the miter. `Ok(())` means
/// UNSAT with a kernel-accepted proof; `Err(model_inputs)` is a SAT model's
/// input values, one per variable.
fn miter(
    widths: &[usize],
    build: impl Fn(&mut Aig, &[BitVec]) -> (BitVec, BitVec),
) -> Result<(), Vec<u128>> {
    let mut g = Aig::new();
    let vars: Vec<BitVec> = widths.iter().map(|&w| BitVec::input(&mut g, w)).collect();
    let (l, r) = build(&mut g, &vars);
    let same = bv::eq(&mut g, &l, &r);
    let mut cnf = Cnf::new(0);
    let mut t = Tseitin::new();
    t.assert_true(&g, !same, &mut cnf);
    // Encode every input too so a model assigns each of them.
    for i in 0..g.num_inputs() {
        t.lit(&g, g.input_edge(i).unwrap(), &mut cnf);
    }
    let mut s = Solver::from_cnf_with_proof(&cnf);
    match s.solve() {
        SolveResult::Unsat => {
            let proof = s.proof().unwrap().to_drat_text();
            drat_check::check_with_limits(&write_dimacs(&cnf), &proof, drat_check::Limits { max_work: u64::MAX / 4, ..Default::default() })
                .expect("kernel rejects the identity's proof");
            Ok(())
        }
        SolveResult::Sat => {
            let model = s.model().to_vec();
            let mut k = 0;
            let mut out = Vec::new();
            for v in &vars {
                let mut x = 0u128;
                for i in 0..v.width() {
                    let var = t.node_var(g.input_edge(k).unwrap().node()).unwrap();
                    x |= (model[var.index()] as u128) << i;
                    k += 1;
                }
                out.push(x);
            }
            Err(out)
        }
        SolveResult::Unknown => panic!("no budget was set, Unknown is impossible"),
    }
}

fn prove(widths: &[usize], build: impl Fn(&mut Aig, &[BitVec]) -> (BitVec, BitVec)) {
    if let Err(cex) = miter(widths, build) {
        panic!("the identity has a counterexample {cex:?} at widths {widths:?}");
    }
}

/// The identity must fail; the solver's model must really violate it under
/// the reference semantics.
fn refute(
    widths: &[usize],
    build: impl Fn(&mut Aig, &[BitVec]) -> (BitVec, BitVec),
    lhs: impl Fn(&[u128]) -> u128,
    rhs: impl Fn(&[u128]) -> u128,
) {
    match miter(widths, build) {
        Ok(()) => panic!("a non-identity was 'proved' at widths {widths:?}: UNSOUND ENCODING"),
        Err(cex) => assert_ne!(lhs(&cex), rhs(&cex), "model {cex:?} is not a real counterexample"),
    }
}

#[test]
fn identities_are_proved_with_certified_proofs() {
    for w in [1usize, 2, 3, 4, 6, 8, 16] {
        let all = vec![w, w];
        prove(&all, |g, v| (bv::add(g, &v[0], &v[1]), bv::add(g, &v[1], &v[0]))); // a+b = b+a
        prove(&all, |g, v| (bv::xor(g, &v[0], &v[0]), BitVec::constant(w, 0))); // x^x = 0
        prove(&all, |g, v| {
            let one = BitVec::constant(w, 1);
            (bv::shl(g, &v[0], &one), bv::add(g, &v[0], &v[0])) // a<<1 = a+a
        });
        prove(&all, |g, v| {
            let nb = bv::neg(g, &v[1]);
            (bv::sub(g, &v[0], &v[1]), bv::add(g, &v[0], &nb)) // a−b = a+(−b)
        });
        prove(&all, |g, v| {
            let (o, a) = (bv::or(g, &v[0], &v[1]), bv::and(g, &v[0], &v[1]));
            (bv::add(g, &o, &a), bv::add(g, &v[0], &v[1])) // (a|b)+(a&b) = a+b
        });
        prove(&all, |g, v| {
            let (o, a) = (bv::or(g, &v[0], &v[1]), bv::and(g, &v[0], &v[1]));
            (bv::xor(g, &v[0], &v[1]), bv::sub(g, &o, &a)) // a^b = (a|b)−(a&b)
        });
        prove(&all, |g, v| (bv::not(&bv::and(g, &v[0], &v[1])), bv::or(g, &bv::not(&v[0]), &bv::not(&v[1])))); // De Morgan
        prove(&all, |g, v| {
            let n = bv::neg(g, &v[0]);
            (bv::neg(g, &n), v[0].clone()) // −(−a) = a
        });
        prove(&all, |g, v| {
            let n = bv::not(&v[0]);
            let m = BitVec::constant(w, 1);
            (bv::neg(g, &v[0]), bv::add(g, &n, &m)) // −a = ¬a+1
        });
        prove(&all, |g, v| {
            let o = bv::or(g, &v[0], &v[1]);
            (bv::and(g, &v[0], &o), v[0].clone()) // absorption
        });
        // ite / comparison identities
        prove(&all, |g, v| {
            let c = bv::ult(g, &v[0], &v[1]);
            let m = bv::ite(g, c, &v[0], &v[1]);
            let c2 = bv::ule(g, &v[0], &v[1]);
            let m2 = bv::ite(g, c2, &v[0], &v[1]);
            (m, m2) // min via < equals min via ≤
        });
        prove(&all, |g, v| {
            // slt(a,b) = ult(a ^ MSB, b ^ MSB)
            let msb = BitVec::constant(w, 1 << (w - 1));
            let (x, y) = (bv::xor(g, &v[0], &msb), bv::xor(g, &v[1], &msb));
            let lhs = bv::slt(g, &v[0], &v[1]);
            let rhs = bv::ult(g, &x, &y);
            (BitVec { bits: vec![lhs] }, BitVec { bits: vec![rhs] })
        });
        prove(&all, |g, v| {
            // ult(a,b) = ¬uge(a,b)
            let (l, r) = (bv::ult(g, &v[0], &v[1]), bv::uge(g, &v[0], &v[1]));
            (BitVec { bits: vec![l] }, BitVec { bits: vec![!r] })
        });
        prove(&all, |g, v| {
            // sext to 2w preserves signed order
            let (x, y) = (bv::sext(&v[0], w), bv::sext(&v[1], w));
            let (l, r) = (bv::slt(g, &v[0], &v[1]), bv::slt(g, &x, &y));
            (BitVec { bits: vec![l] }, BitVec { bits: vec![r] })
        });
        // extract/concat round trip (needs w ≥ 2 to split)
        if w >= 2 {
            let k = w / 2;
            prove(&all, |_, v| {
                let hi = bv::extract(&v[0], w - 1, k);
                let lo = bv::extract(&v[0], k - 1, 0);
                (bv::concat(&hi, &lo), v[0].clone())
            });
        }
        // shifts: lshr(shl(a,1),1) = a & 0111…
        if w >= 2 {
            prove(&all, |g, v| {
                let one = BitVec::constant(w, 1);
                let s = bv::shl(g, &v[0], &one);
                let l = bv::lshr(g, &s, &one);
                let m = BitVec::constant(w, mask(w - 1));
                (l, bv::and(g, &v[0], &m))
            });
        }
        // ashr by w−1 is the sign smeared
        prove(&all, |g, v| {
            let s = BitVec::constant(w, w as u128 - 1);
            let a = bv::ashr(g, &v[0], &s);
            (a, BitVec { bits: vec![v[0].msb(); w] })
        });
    }
    // Multiplication identities, where the miters stay small.
    for w in [1usize, 2, 3, 4, 5] {
        let all = vec![w, w];
        prove(&all, |g, v| (bv::mul(g, &v[0], &v[1]), bv::mul(g, &v[1], &v[0]))); // commutative
        prove(&all, |g, v| {
            let two = BitVec::constant(w, 2);
            (bv::mul(g, &v[0], &two), bv::add(g, &v[0], &v[0]))
        });
        if w <= 3 {
        prove(&[w, w, w], |g, v| {
            // distributivity: a(b+c) = ab+ac
            let s = bv::add(g, &v[1], &v[2]);
            let (ab, ac) = (bv::mul(g, &v[0], &v[1]), bv::mul(g, &v[0], &v[2]));
            (bv::mul(g, &v[0], &s), bv::add(g, &ab, &ac))
        });
        }
    }
    // Division: a = (a/b)·b + a mod b, for every b (including 0: a = −1·0 + a).
    for w in [1usize, 2, 3, 4] {
        prove(&[w, w], |g, v| {
            let (q, r) = (bv::udiv(g, &v[0], &v[1]), bv::urem(g, &v[0], &v[1]));
            let qb = bv::mul(g, &q, &v[1]);
            (bv::add(g, &qb, &r), v[0].clone())
        });
        prove(&[w, w], |g, v| {
            let (q, r) = (bv::sdiv(g, &v[0], &v[1]), bv::srem(g, &v[0], &v[1]));
            let qb = bv::mul(g, &q, &v[1]);
            (bv::add(g, &qb, &r), v[0].clone())
        });
        // a mod b < b for b ≠ 0, expressed as a bit-vector identity on a flag.
        prove(&[w, w], |g, v| {
            let r = bv::urem(g, &v[0], &v[1]);
            let lt = bv::ult(g, &r, &v[1]);
            let zero = bv::eq(g, &v[1], &BitVec::constant(w, 0));
            let ok = g.or(lt, zero);
            (BitVec { bits: vec![ok] }, BitVec { bits: vec![Edge::TRUE] })
        });
    }
}

#[test]
fn non_identities_yield_real_counterexamples() {
    let w = 4usize;
    let two = vec![w, w];
    refute(&two, |g, v| (bv::add(g, &v[0], &v[1]), bv::sub(g, &v[0], &v[1])), |x| (x[0] + x[1]) & mask(w), |x| x[0].wrapping_sub(x[1]) & mask(w));
    refute(&two, |g, v| (bv::sub(g, &v[0], &v[1]), bv::sub(g, &v[1], &v[0])), |x| x[0].wrapping_sub(x[1]) & mask(w), |x| x[1].wrapping_sub(x[0]) & mask(w));
    refute(&two, |g, v| (bv::mul(g, &v[0], &v[1]), bv::add(g, &v[0], &v[1])), |x| (x[0] * x[1]) & mask(w), |x| (x[0] + x[1]) & mask(w));
    refute(
        &two,
        |g, v| {
            let one = BitVec::constant(w, 1);
            (bv::shl(g, &v[0], &one), v[0].clone())
        },
        |x| (x[0] << 1) & mask(w),
        |x| x[0],
    );
    refute(
        &two,
        |g, v| {
            let q = bv::udiv(g, &v[0], &v[1]);
            (bv::mul(g, &q, &v[1]), v[0].clone())
        },
        |x| udiv_ref(x[0], x[1], w).wrapping_mul(x[1]) & mask(w),
        |x| x[0],
    );
    refute(
        &two,
        |g, v| {
            let (l, r) = (bv::slt(g, &v[0], &v[1]), bv::ult(g, &v[0], &v[1]));
            (BitVec { bits: vec![l] }, BitVec { bits: vec![r] })
        },
        |x| (signed(x[0], w) < signed(x[1], w)) as u128,
        |x| (x[0] < x[1]) as u128,
    );
    refute(&two, |g, v| (bv::ashr(g, &v[0], &v[1]), bv::lshr(g, &v[0], &v[1])), |x| ashr_ref(x[0], x[1], w), |x| lshr_ref(x[0], x[1], w));
    refute(&two, |g, v| (bv::and(g, &v[0], &v[1]), bv::or(g, &v[0], &v[1])), |x| x[0] & x[1], |x| x[0] | x[1]);
    refute(&two, |g, v| (bv::sdiv(g, &v[0], &v[1]), bv::udiv(g, &v[0], &v[1])), |x| sdiv_ref(x[0], x[1], w), |x| udiv_ref(x[0], x[1], w));
    refute(&two, |g, v| (bv::srem(g, &v[0], &v[1]), bv::urem(g, &v[0], &v[1])), |x| srem_ref(x[0], x[1], w), |x| urem_ref(x[0], x[1]));
    // Off-by-one in the identity itself: −a = ¬a, not ¬a + 1.
    refute(&two, |g, v| (bv::neg(g, &v[0]), bv::not(&v[0])), |x| x[0].wrapping_neg() & mask(w), |x| !x[0] & mask(w));
    // a+b = b+a is true but a+b = b+a+1 is not.
    refute(
        &two,
        |g, v| {
            let s = bv::add(g, &v[1], &v[0]);
            (bv::add(g, &v[0], &v[1]), bv::add(g, &s, &BitVec::constant(w, 1)))
        },
        |x| (x[0] + x[1]) & mask(w),
        |x| (x[0] + x[1] + 1) & mask(w),
    );
    // Width 1 corner: a < b signed differs from unsigned on exactly (1, 0)/(0, 1).
    refute(
        &[1, 1],
        |g, v| {
            let (l, r) = (bv::slt(g, &v[0], &v[1]), bv::ult(g, &v[0], &v[1]));
            (BitVec { bits: vec![l] }, BitVec { bits: vec![r] })
        },
        |x| (signed(x[0], 1) < signed(x[1], 1)) as u128,
        |x| (x[0] < x[1]) as u128,
    );
}

// ---------------------------------------------------------------------------
// Structure: the AIG and Tseitin properties the contract relies on.
// ---------------------------------------------------------------------------

#[test]
fn structural_hashing_never_changes_a_function() {
    // Build the same random circuit twice: once through a graph that keeps
    // every gate (a fresh graph per gate, no sharing) and once with sharing;
    // all outputs must agree on every input assignment. The un-shared
    // oracle evaluates the gate list directly.
    let mut rng = Rng(0xFA17_F001);
    for round in 0..300 {
        let n_inputs = 2 + (rng.next() % 5) as usize;
        let mut g = Aig::new();
        let mut pool: Vec<Edge> = (0..n_inputs).map(|_| g.input()).collect();
        pool.push(Edge::TRUE);
        pool.push(Edge::FALSE);
        // gate list: (kind, a_idx, a_neg, b_idx, b_neg, s_idx); the oracle works on bools.
        let mut recipe = Vec::new();
        for _ in 0..(rng.next() % 40) as usize {
            let n = pool.len();
            let (ai, bi, si) = ((rng.next() % n as u64) as usize, (rng.next() % n as u64) as usize, (rng.next() % n as u64) as usize);
            let (an, bn) = (rng.next() & 1 == 1, rng.next() & 1 == 1);
            let kind = rng.next() % 5;
            let (a, b, s) = (if an { !pool[ai] } else { pool[ai] }, if bn { !pool[bi] } else { pool[bi] }, pool[si]);
            let e = match kind {
                0 => g.and(a, b),
                1 => g.or(a, b),
                2 => g.xor(a, b),
                3 => g.xnor(a, b),
                _ => g.mux(s, a, b),
            };
            pool.push(e);
            recipe.push((kind, ai, an, bi, bn, si));
        }
        for bits in 0..1u32 << n_inputs {
            let assignment: Vec<bool> = (0..n_inputs).map(|i| bits >> i & 1 == 1).collect();
            // Oracle: evaluate the recipe on plain bools.
            let mut vals: Vec<bool> = assignment.clone();
            vals.push(true);
            vals.push(false);
            for &(kind, ai, an, bi, bn, si) in &recipe {
                let (a, b, s) = (vals[ai] != an, vals[bi] != bn, vals[si]);
                vals.push(match kind {
                    0 => a && b,
                    1 => a || b,
                    2 => a ^ b,
                    3 => a == b,
                    _ => if s { a } else { b },
                });
            }
            for (i, &e) in pool.iter().enumerate() {
                assert_eq!(g.eval(e, &assignment), vals[i], "round {round} node {i} inputs {bits:b}");
            }
        }
    }
}

#[test]
fn tseitin_encodes_each_node_functionally() {
    // F2 directly on random AIGs: with the inputs fixed, the CNF has
    // exactly one model over its variables (counted by exhaustion of the
    // gate variables), and every node variable equals its simulated value.
    let mut rng = Rng(0x7531_7531);
    for _ in 0..120 {
        let n_inputs = 1 + (rng.next() % 4) as usize;
        let mut g = Aig::new();
        let mut pool: Vec<Edge> = (0..n_inputs).map(|_| g.input()).collect();
        for _ in 0..(rng.next() % 6) as usize {
            let (a, b) = (pool[(rng.next() % pool.len() as u64) as usize], pool[(rng.next() % pool.len() as u64) as usize]);
            let (a, b) = (if rng.next() & 1 == 1 { !a } else { a }, if rng.next() & 1 == 1 { !b } else { b });
            let e = g.and(a, b);
            pool.push(e);
        }
        pool.push(Edge::FALSE);
        let mut cnf = Cnf::new(0);
        let mut t = Tseitin::new();
        let lits: Vec<Lit> = pool.iter().map(|&e| t.lit(&g, e, &mut cnf)).collect();
        assert!(cnf.num_vars <= 16, "keep the brute force small");
        let mut models = 0usize;
        for code in 0..1u32 << cnf.num_vars {
            let m: Vec<bool> = (0..cnf.num_vars).map(|i| code >> i & 1 == 1).collect();
            if !cnf.is_satisfied_by(&m) {
                continue;
            }
            models += 1;
            let inputs: Vec<bool> = (0..n_inputs)
                .map(|i| m[t.node_var(g.input_edge(i).unwrap().node()).unwrap().index()])
                .collect();
            for (&e, &l) in pool.iter().zip(&lits) {
                assert_eq!(lit_value(&m, l), g.eval(e, &inputs), "gate variable disagrees with the circuit");
            }
        }
        // One model per input assignment, no more: the encoding is functional.
        assert_eq!(models, 1 << n_inputs, "extra or missing extensions");
    }
}

#[test]
fn incremental_encoding_equals_one_shot() {
    // Encode a circuit piece by piece into one solver (BMC style) and in
    // one go into a CNF; both must give the same outputs for every input.
    let mut g = Aig::new();
    let a = BitVec::input(&mut g, 4);
    let b = BitVec::input(&mut g, 4);
    let sum = bv::add(&mut g, &a, &b);
    let prod = bv::mul(&mut g, &a, &b);
    let mut solver = Solver::new();
    let mut inc = Tseitin::new();
    let sum_lits: Vec<Lit> = sum.bits.iter().map(|&e| inc.lit(&g, e, &mut solver)).collect();
    let in_lits: Vec<Lit> = (0..8).map(|i| inc.lit(&g, g.input_edge(i).unwrap(), &mut solver)).collect();
    // Only now, after solving once, encode the product.
    assert_eq!(solver.solve(), SolveResult::Sat);
    let prod_lits: Vec<Lit> = prod.bits.iter().map(|&e| inc.lit(&g, e, &mut solver)).collect();
    for x in 0..16u128 {
        for y in 0..16u128 {
            let fixed = fixing(&in_lits, &[4, 4], &[x, y]);
            assert_eq!(solver.solve_with_assumptions(&fixed), SolveResult::Sat);
            let m = solver.model();
            let s: u128 = sum_lits.iter().enumerate().map(|(i, &l)| (lit_value(m, l) as u128) << i).sum();
            let p: u128 = prod_lits.iter().enumerate().map(|(i, &l)| (lit_value(m, l) as u128) << i).sum();
            assert_eq!((s, p), ((x + y) & 15, (x * y) & 15));
        }
    }
}

#[test]
fn width_zero_and_bad_extract_are_refused_clearly() {
    let r = std::panic::catch_unwind(|| {
        let mut g = Aig::new();
        let a = BitVec::input(&mut g, 0);
        let b = BitVec::input(&mut g, 0);
        bv::slt(&mut g, &a, &b)
    });
    assert!(r.is_err());
    let r = std::panic::catch_unwind(|| {
        let mut g = Aig::new();
        let a = BitVec::input(&mut g, 4);
        bv::extract(&a, 4, 0)
    });
    assert!(r.is_err());
}
