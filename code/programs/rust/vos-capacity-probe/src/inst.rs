//! # `inst`: an instance, its CNF, and the validity guards G1 and G2
//!
//! An [`Instance`] is one *obligation shape* ready to solve: an AIG, the miter output (true
//! exactly when the two sides differ), the premises that must hold (cut hypotheses, path
//! conditions), the declared input list, each side's output bits, and a replay closure that
//! re-checks a counterexample on concrete machines.
//!
//! ```text
//!   asserted CNF  =  Tseitin( premises  AND  miter )          UNSAT  <=>  the sides agree
//! ```
//!
//! ## G1: collapse accounting, measured from the AIG
//!
//! [`Instance::cone_ands`] walks the AIG from the miter output, so `ands` is a measurement and
//! not the generator's claim.  [`Instance::collapse_reason`] classifies a trivially-refuted
//! miter by the closed enum of VOS30 section 4.5, and [`Instance::verify_collapse_reason`]
//! re-derives the reason structurally (the checker never reads the reason as a claim).
//!
//! ## G2: liveness, measured on behaviour
//!
//! [`sensitive_bits`] flips each *declared* input bit and simulates the side's own output
//! cone on 64 random assignments at a time, four rounds (256 assignments).  A bit is
//! *sensitive* if any flip ever changes any output bit.  Sensitivity is measured on each
//! side's output cone separately (never on the miter, which is constant 0 for an UNSAT
//! instance and would mark every row collapsed).

use crate::cpu::{input_index, Declared};
use crate::prog::Rng;
use bit_blaster::aig::{edge_value, Aig, Edge, Node};
use bit_blaster::tseitin::Tseitin;
use cdcl_sat::{write_dimacs, Cnf};
use std::collections::HashMap;

/// A counterexample replay: given the value of every declared input (by name), run the code
/// side on the concrete simulator and the reference side on the independent evaluator, and
/// say whether they differ (for a leak control, whether the two runs differ in the
/// observable).  `Err` is a replay failure, which fails the control.
pub type Replay = Box<dyn Fn(&HashMap<String, u32>) -> Result<bool, String>>;

/// One obligation shape.
pub struct Instance {
    /// The circuit.
    pub aig: Aig,
    /// True exactly when the sides differ.
    pub miter: Edge,
    /// Conditions asserted with the miter.
    pub premises: Vec<Edge>,
    /// The declared input groups.
    pub declared: Vec<Declared>,
    /// The code side's output bits.
    pub code_out: Vec<Edge>,
    /// The reference side's output bits.
    pub ref_out: Vec<Edge>,
    /// Concrete replay of a model.
    pub replay: Replay,
    /// The two cones G2 measures, when they are not `code_out` and `ref_out`: a
    /// self-composition's *output* is secret-independent by design, so liveness is measured on
    /// the two sides of the equivalence that cancels the secret instead.
    pub live: Option<(Vec<Edge>, Vec<Edge>)>,
}

/// The G2 measurement of one instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entropy {
    /// Sensitive declared bits (the smaller of the two sides).
    pub sensitive: usize,
    /// Declared bits.
    pub declared: usize,
    /// Whether the count was estimated from a sample of the bits.
    pub sampled: bool,
}

impl Entropy {
    /// Whether fewer than half of the declared bits are sensitive (G2).
    pub fn below_half(&self) -> bool {
        self.declared > 0 && self.sensitive * 2 < self.declared
    }
}

/// The closed enum of VOS30 section 4.5.
pub const COLLAPSE_REASONS: [&str; 4] = [
    "identical-structure",
    "constant-fold",
    "frame-untouched",
    "propagation-only",
];

impl Instance {
    /// The CNF asserting the premises and the miter, with its Tseitin map.
    pub fn cnf(&self) -> (Cnf, Tseitin) {
        let mut cnf = Cnf::new(0);
        let mut ts = Tseitin::new();
        for p in &self.premises {
            ts.assert_true(&self.aig, *p, &mut cnf);
        }
        ts.assert_true(&self.aig, self.miter, &mut cnf);
        (cnf, ts)
    }

    /// AND nodes in the cone of the miter output, walked from the output (G1).
    pub fn cone_ands(&self) -> usize {
        let mut seen = vec![false; self.aig.num_nodes()];
        let mut stack = vec![self.miter.node()];
        let mut n = 0;
        while let Some(i) = stack.pop() {
            if std::mem::replace(&mut seen[i], true) {
                continue;
            }
            if let Node::And(a, b) = self.aig.node(i) {
                n += 1;
                stack.push(a.node());
                stack.push(b.node());
            }
        }
        n
    }

    /// Total declared bits.
    pub fn declared_bits(&self) -> usize {
        self.declared.iter().map(|d| d.bits.len()).sum()
    }

    /// Classify a trivially-refuted miter (G1).  `conflicts` is what the solver used.
    pub fn collapse_reason(&self, cnf_clauses: usize, conflicts: u64) -> Option<&'static str> {
        if let Some(reason) = self.structural_collapse_reason(cnf_clauses) {
            return Some(reason);
        }
        if conflicts == 0 {
            return Some("propagation-only");
        }
        None
    }

    /// The part of [`Instance::collapse_reason`] that needs no solver: the instance alone (and
    /// its clause count) decides it.  `check`'s structural rebuild uses it to hold a recorded
    /// `collapsed` row to what the rebuilt instance says (round 3, finding 1).
    pub fn structural_collapse_reason(&self, cnf_clauses: usize) -> Option<&'static str> {
        if self.pairs_equal() {
            return Some(if self.code_out.iter().all(|e| e.is_constant()) {
                "constant-fold"
            } else {
                "identical-structure"
            });
        }
        if cnf_clauses <= 2 {
            return Some("propagation-only");
        }
        None
    }

    fn pairs_equal(&self) -> bool {
        !self.code_out.is_empty() && self.code_out == self.ref_out
    }

    /// Re-derive a recorded `collapsed_reason` from the instance, by the allowed-when rules of
    /// VOS30 section 4.5 (`check` never trusts the recorded string).
    pub fn verify_collapse_reason(
        &self,
        reason: &str,
        cnf_clauses: usize,
        conflicts: u64,
    ) -> Result<(), String> {
        // Every collapse requires symbolic, bound input bits: a program that read a constant
        // instead of a symbolic value is exactly the vacuous collapse the guard exists for.
        if self.declared_bits() == 0 {
            return Err("a collapse with no declared symbolic input is vacuous".into());
        }
        for d in &self.declared {
            if d.bits.iter().any(|e| input_index(&self.aig, *e).is_none()) {
                return Err(format!("declared input {} is not an AIG input", d.name));
            }
        }
        match reason {
            "identical-structure"
                if self.pairs_equal() && !self.code_out.iter().all(|e| e.is_constant()) =>
            {
                Ok(())
            }
            "constant-fold"
                if self.pairs_equal() && self.code_out.iter().all(|e| e.is_constant()) =>
            {
                Ok(())
            }
            "propagation-only" if !self.pairs_equal() && (cnf_clauses <= 2 || conflicts == 0) => {
                Ok(())
            }
            "frame-untouched" => {
                Err("frame-untouched needs a write set the probe does not have".into())
            }
            other => Err(format!(
                "collapsed_reason {other:?} does not hold structurally on the regenerated instance"
            )),
        }
    }

    /// The value of every declared group and every AIG input under a solver model.
    pub fn assignment(&self, ts: &Tseitin, model: &[bool]) -> (Vec<bool>, HashMap<String, u32>) {
        let mut inputs = vec![false; self.aig.num_inputs()];
        let mut by_name = HashMap::new();
        for d in &self.declared {
            let mut v = 0u32;
            for (b, e) in d.bits.iter().enumerate() {
                if let Some(i) = input_index(&self.aig, *e) {
                    let bit = ts.input_value(&self.aig, i, model).unwrap_or(false);
                    inputs[i] = bit;
                    if bit && b < 32 {
                        v |= 1 << b;
                    }
                }
            }
            by_name.insert(d.name.clone(), v);
        }
        (inputs, by_name)
    }

    /// Evaluate edges on a full input assignment (independent of the solver and of Tseitin).
    pub fn eval(&self, inputs: &[bool], edges: &[Edge]) -> Vec<bool> {
        let words: Vec<u64> = inputs.iter().map(|&b| b as u64).collect();
        let values = self.aig.simulate(&words);
        edges
            .iter()
            .map(|e| edge_value(&values, *e) & 1 == 1)
            .collect()
    }
}

/// SHA-256 of the DIMACS text of a CNF.
pub fn dimacs_sha(cnf: &Cnf) -> (String, String) {
    let text = write_dimacs(cnf);
    let sha = sha_hex(text.as_bytes());
    (text, sha)
}

/// Hex SHA-256 of a byte string.
pub fn sha_hex(bytes: &[u8]) -> String {
    coding_adventures_sha256::sha256_hex(bytes)
}

/// How many node-evaluations the sensitivity measurement may spend on one side (about a few
/// seconds); above it the bits are sampled and the count scaled, and the row says so.
pub const SENSITIVITY_BUDGET: u64 = 1_500_000_000;

/// Count the declared bits that are sensitive on the cone of `outs` (G2).
pub fn sensitive_bits(aig: &Aig, outs: &[Edge], declared: &[Declared], seed: u64) -> Entropy {
    let total: usize = declared.iter().map(|d| d.bits.len()).sum();
    // The cone of the outputs, in node order (which is topological).
    let mut in_cone = vec![false; aig.num_nodes()];
    let mut stack: Vec<usize> = outs.iter().map(|e| e.node()).collect();
    while let Some(i) = stack.pop() {
        if std::mem::replace(&mut in_cone[i], true) {
            continue;
        }
        if let Node::And(a, b) = aig.node(i) {
            stack.push(a.node());
            stack.push(b.node());
        }
    }
    let cone: Vec<usize> = (0..aig.num_nodes()).filter(|&i| in_cone[i]).collect();
    // Candidate bits: the declared bits that are in the cone at all (the rest are insensitive).
    let mut cand: Vec<usize> = Vec::new();
    for d in declared {
        for e in &d.bits {
            if in_cone[e.node()] {
                if let Some(i) = input_index(aig, *e) {
                    cand.push(i);
                }
            }
        }
    }
    let rounds = 4u64;
    let cost = |n: usize| {
        (n as u64)
            .saturating_mul(rounds)
            .saturating_mul(cone.len() as u64 + 1)
    };
    let mut rng = Rng::new(seed, 0x6E7);
    let mut sampled = false;
    let in_cone_total = cand.len();
    if cost(cand.len()) > SENSITIVITY_BUDGET {
        let keep = ((SENSITIVITY_BUDGET / (rounds * (cone.len() as u64 + 1))) as usize)
            .max(64)
            .min(cand.len());
        rng.shuffle(&mut cand);
        cand.truncate(keep);
        sampled = true;
    }
    let n_inputs = aig.num_inputs();
    let mut values = vec![0u64; aig.num_nodes()];
    let simulate = |values: &mut Vec<u64>, inw: &[u64]| {
        for &i in &cone {
            values[i] = match aig.node(i) {
                Node::False => 0,
                Node::Input(k) => inw[k],
                Node::And(a, b) => edge_value(values, a) & edge_value(values, b),
            };
        }
    };
    let out_words =
        |values: &Vec<u64>| -> Vec<u64> { outs.iter().map(|e| edge_value(values, *e)).collect() };
    let mut sensitive = vec![false; n_inputs];
    for _ in 0..rounds {
        let mut inw: Vec<u64> = (0..n_inputs).map(|_| rng.next_u64()).collect();
        simulate(&mut values, &inw);
        let base = out_words(&values);
        for &i in &cand {
            if sensitive[i] {
                continue;
            }
            inw[i] = !inw[i];
            simulate(&mut values, &inw);
            if out_words(&values) != base {
                sensitive[i] = true;
            }
            inw[i] = !inw[i];
        }
    }
    // Stage 2, exact where the instance is small (VOS30 section 9.4): a bit the sampling did not
    // find sensitive is decided by a SAT query "two inputs differing only in this bit give
    // different outputs".  A priority function such as `pick` depends on a bit only when no
    // higher bit is set, which random sampling sees for about nine bits of any width, so the
    // sampling alone would call every wide `pick` dead.
    let undecided: Vec<usize> = cand.iter().copied().filter(|&i| !sensitive[i]).collect();
    let mut exact = true;
    let and_count = cone
        .iter()
        .filter(|&&i| matches!(aig.node(i), Node::And(..)))
        .count();
    if !undecided.is_empty() {
        if and_count <= EXACT_MAX_ANDS
            && (undecided.len() as u64) * (and_count as u64) <= EXACT_MAX_WORK
        {
            for &i in &undecided {
                if differs_by_flipping(aig, outs, &cone, i) {
                    sensitive[i] = true;
                }
            }
        } else {
            exact = false;
        }
    }
    let found = cand.iter().filter(|&&i| sensitive[i]).count();
    let est = if sampled && !cand.is_empty() {
        ((found as f64) * (in_cone_total as f64) / (cand.len() as f64)).round() as usize
    } else {
        found
    };
    Entropy {
        sensitive: est.min(total),
        declared: total,
        sampled: sampled || !exact,
    }
}

/// The largest cone (in AND nodes) for which the exact per-bit query is attempted.
pub const EXACT_MAX_ANDS: usize = 4_000;
/// The largest `undecided bits x cone ANDs` for the exact stage.
pub const EXACT_MAX_WORK: u64 = 400_000;

/// Is there an input assignment where flipping input `flip` changes some output bit?  A SAT
/// query on the cone and a twin of it with that one input inverted (a refused or capped query
/// counts as "no", so the measure fails closed).
fn differs_by_flipping(aig: &Aig, outs: &[Edge], cone: &[usize], flip: usize) -> bool {
    let mut g = aig.clone();
    let mut map: Vec<Option<Edge>> = vec![None; aig.num_nodes()];
    let remap = |map: &Vec<Option<Edge>>, e: Edge| -> Edge {
        let base = map[e.node()].expect("operands precede their gate");
        if e.is_complemented() {
            !base
        } else {
            base
        }
    };
    for &i in cone {
        let m = match aig.node(i) {
            Node::False => Edge::FALSE,
            Node::Input(k) => {
                let e = aig.input_edge(k).expect("input exists");
                if k == flip {
                    !e
                } else {
                    e
                }
            }
            Node::And(a, b) => {
                let (x, y) = (remap(&map, a), remap(&map, b));
                g.and(x, y)
            }
        };
        map[i] = Some(m);
    }
    let diffs: Vec<Edge> = outs.iter().map(|&o| g.xor(o, remap(&map, o))).collect();
    let miter = g.or_all(&diffs);
    let mut cnf = Cnf::new(0);
    Tseitin::new().assert_true(&g, miter, &mut cnf);
    let o = crate::solve::SolveOpts {
        deadline: std::time::Instant::now() + std::time::Duration::from_secs(1),
        proof: false,
        conflict_cap: Some(20_000),
    };
    matches!(
        crate::solve::solve_cnf(&cnf, &o).verdict,
        crate::solve::Verdict::Sat(_)
    )
}

/// The G2 measure of an instance: the worse of the two sides.
///
/// In the cut form (inputs named `...@c` and `...@r`, one per side) each side is measured
/// against *its own* declared inputs, so a side that uses every one of its inputs scores 100%
/// and not 50%.
pub fn entropy(inst: &Instance, seed: u64) -> Entropy {
    let (ca, cb, da, db): (&Vec<Edge>, &Vec<Edge>, Vec<Declared>, Vec<Declared>) = match &inst.live
    {
        Some((a, b)) => (a, b, inst.declared.clone(), inst.declared.clone()),
        None => (
            &inst.code_out,
            &inst.ref_out,
            inst.declared
                .iter()
                .filter(|d| !d.name.ends_with("@r"))
                .cloned()
                .collect(),
            inst.declared
                .iter()
                .filter(|d| !d.name.ends_with("@c"))
                .cloned()
                .collect(),
        ),
    };
    let a = sensitive_bits(&inst.aig, ca, &da, seed);
    let b = sensitive_bits(&inst.aig, cb, &db, seed ^ 1);
    // The worse *fraction*.
    if (a.sensitive as u128) * (b.declared.max(1) as u128)
        <= (b.sensitive as u128) * (a.declared.max(1) as u128)
    {
        a
    } else {
        b
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::Pool;
    use bit_blaster::bv::{self, BitVec};

    fn tiny(identical: bool) -> Instance {
        let mut aig = Aig::new();
        let mut pool = Pool::new();
        let a = pool.word(&mut aig, "a");
        let b = pool.word(&mut aig, "b");
        let l = bv::add(&mut aig, &a, &b);
        let r = if identical {
            bv::add(&mut aig, &a, &b)
        } else {
            crate::sem::alternative(&mut aig, crate::sem::Bin::Add, 0, &a, &b).unwrap()
        };
        let eq = bv::eq(&mut aig, &l, &r);
        Instance {
            aig,
            miter: !eq,
            premises: vec![],
            declared: pool.declared,
            code_out: l.bits,
            ref_out: r.bits,
            replay: Box::new(|_| Ok(true)),
            live: None,
        }
    }

    #[test]
    fn an_identical_pair_collapses_with_the_identical_structure_reason() {
        let i = tiny(true);
        assert_eq!(
            i.cone_ands(),
            0,
            "the miter is the constant false: no AND in its cone"
        );
        let (cnf, _) = i.cnf();
        assert!(cnf.clauses.len() <= 2);
        assert_eq!(
            i.collapse_reason(cnf.clauses.len(), 0),
            Some("identical-structure")
        );
        assert!(i
            .verify_collapse_reason("identical-structure", cnf.clauses.len(), 0)
            .is_ok());
        assert!(i
            .verify_collapse_reason("constant-fold", cnf.clauses.len(), 0)
            .is_err());
        assert!(i.verify_collapse_reason("frame-untouched", 2, 0).is_err());
    }

    #[test]
    fn a_real_miter_is_not_collapsed_and_has_ands() {
        let i = tiny(false);
        assert!(i.cone_ands() > 100);
        let (cnf, _) = i.cnf();
        assert!(cnf.clauses.len() > 2);
        assert_eq!(i.collapse_reason(cnf.clauses.len(), 17), None);
        assert_eq!(
            i.collapse_reason(cnf.clauses.len(), 0),
            Some("propagation-only")
        );
        assert!(i
            .verify_collapse_reason("propagation-only", cnf.clauses.len(), 0)
            .is_ok());
        assert!(i
            .verify_collapse_reason("propagation-only", cnf.clauses.len(), 5)
            .is_err());
        assert!(i.verify_collapse_reason("bogus", 9, 0).is_err());
    }

    #[test]
    fn a_collapse_without_symbolic_inputs_is_rejected_as_vacuous() {
        let mut aig = Aig::new();
        let k = BitVec::constant(32, 5);
        let eq = bv::eq(&mut aig, &k, &k);
        let i = Instance {
            aig,
            miter: !eq,
            premises: vec![],
            declared: vec![],
            code_out: k.bits.clone(),
            ref_out: k.bits,
            replay: Box::new(|_| Ok(true)),
            live: None,
        };
        assert!(i.verify_collapse_reason("constant-fold", 2, 0).is_err());
    }

    #[test]
    fn sensitivity_counts_the_bits_an_output_depends_on() {
        // out = a + b depends on every bit of both; out = a & 0xFF depends on 8 bits of a only.
        let mut aig = Aig::new();
        let mut pool = Pool::new();
        let a = pool.word(&mut aig, "a");
        let b = pool.word(&mut aig, "b");
        let sum = bv::add(&mut aig, &a, &b);
        let e = sensitive_bits(&aig, &sum.bits, &pool.declared, 1);
        assert_eq!((e.sensitive, e.declared, e.sampled), (64, 64, false));
        let low = bv::and(&mut aig, &a, &BitVec::constant(32, 0xFF));
        let e = sensitive_bits(&aig, &low.bits, &pool.declared, 1);
        assert_eq!(
            e.sensitive, 8,
            "only the eight low bits of a reach the output"
        );
        assert!(e.below_half());
        // A cone that mentions an input bit and ignores it: x & !x folds, but (x & y) | (x & !y) does not.
        let x = a.bits[0];
        let y = b.bits[0];
        let t1 = aig.and(x, y);
        let t2 = aig.and(x, !y);
        let same_as_x = aig.or(t1, t2);
        let e = sensitive_bits(&aig, &[same_as_x], &pool.declared, 1);
        assert_eq!(
            e.sensitive, 1,
            "y is in the cone but no flip of it changes the output"
        );
    }

    #[test]
    fn assignment_reads_back_the_declared_values() {
        let i = tiny(false);
        let (cnf, ts) = i.cnf();
        let mut s = cdcl_sat::Solver::from_cnf(&cnf);
        // alt 0 of add is correct, so the miter is UNSAT: no model.  Use a forced model instead.
        assert_eq!(s.solve(), cdcl_sat::SolveResult::Unsat);
        let model = vec![true; cnf.num_vars];
        let (inputs, by_name) = i.assignment(&ts, &model);
        assert_eq!(by_name["a"], u32::MAX);
        assert!(inputs.iter().all(|&b| b));
        assert_eq!(
            i.eval(&inputs, &[i.miter]),
            vec![false],
            "the AIG itself says the sides agree"
        );
    }
}
