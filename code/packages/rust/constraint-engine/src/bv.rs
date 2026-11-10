//! # BV tactic: fixed-width bit-vectors by bit-blasting (VER10)
//!
//! The engine's third tactic, beside SAT (CDCL) and LIA (Cooper). It decides
//! conjunctions over `Bool` and `(_ BitVec w)` variables, the SMT-LIB
//! `QF_BV` logic, by turning every bit-vector operation into a circuit and
//! asking the CDCL solver:
//!
//! ```text
//!   [Predicate] over Bool / BitVec(w)
//!       │  lower (this module): every w-bit value → w AIG signals,
//!       │  every bvadd / bvult / … → its circuit (bit-blaster::bv)
//!       ▼
//!   and-inverter graph ── Tseitin (bit-blaster) ──► CNF
//!       │
//!       ▼
//!   cdcl-sat, with proof logging and a conflict budget
//!       │
//!       ├─ SAT   → model: every declared variable's bits
//!       └─ UNSAT → DRAT proof ──► drat-check (the trusted kernel)
//!                   accepted → Unsat;  rejected → Unknown, never Unsat
//! ```
//!
//! **Every UNSAT this tactic reports has passed the trusted kernel.** A
//! solver bug cannot surface as a false `Unsat`; at worst it surfaces as
//! `Unknown`. (The certificate proves the *CNF* unsatisfiable. That the CNF
//! means what the predicates mean is the job of the lowering below, which
//! the tests check exhaustively against concrete arithmetic.)
//!
//! ## How SMT-LIB bit-vector syntax arrives
//!
//! `smt-lib-format` is generic, so bit-vector syntax reaches the engine in
//! generic shapes:
//!
//! | SMT-LIB | `Predicate` |
//! |---|---|
//! | `#b0101`, `#x1f` | `Var("#b0101")`, `Var("#x1f")` |
//! | `(_ bv5 8)` | `Apply { f: "_", args: [Var("bv5"), Int(8)] }` |
//! | `(bvadd a b)` | `Apply { f: "bvadd", args: [a, b] }` |
//! | `(= a b)` / `(distinct a b)` | `Eq(a, b)` / `NEq(a, b)` |
//!
//! SMT-LIB symbols cannot begin with `#`, so reading `#b…` and `#x…` as
//! literals is unambiguous. That is enforced, not assumed: a *declared*
//! variable whose name begins with `#` is refused (`Unknown`), because
//! otherwise `(declare-const #b1 (_ BitVec 1))` would be silently read as
//! the constant 1 and could produce a false, certified `Unsat`.
//!
//! ## Hostile input
//!
//! Predicates may come from user-written annotations or `.smt2` files, so
//! every kind of work is bounded, and running out answers `Unknown`:
//!
//! | Work | Bound |
//! |---|---|
//! | bit-vector width | [`MAX_BV_WIDTH`] |
//! | circuit size (memory) | [`MAX_AIG_NODES`], checked after every operation |
//! | circuit construction (time; repeated subterms cost work even when structural hashing makes them free in size) | [`MAX_GATE_CALLS`] |
//! | search | [`BV_CONFLICT_BUDGET`] conflicts and [`BV_WORK_BUDGET`] propagation steps |
//! | certificate check | `drat-check`'s default limits |
//!
//! Anything outside the supported fragment is `Unknown` with a reason, never
//! a guess.

use std::collections::HashMap;

use bit_blaster::aig::{Aig, Edge, Node};
use bit_blaster::bv::{self, BitVec};
use bit_blaster::tseitin::Tseitin;
use cdcl_sat::{write_dimacs, Cnf, SolveResult, Solver};
use constraint_core::{depth_of, Predicate, Sort, MAX_PREDICATE_DEPTH};

use crate::{Model, SolverResult, SortEnv, Value};

/// The widest bit-vector the tactic accepts.
pub const MAX_BV_WIDTH: usize = 512;

/// The most AIG nodes one query may build. Each node costs a few hundred
/// bytes once encoded and solved (about 150 MB at this cap). A 128-bit
/// multiplier (about 115 000 gates) fits; a 256-bit one does not.
pub const MAX_AIG_NODES: usize = 250_000;

/// The most gate constructions ([`Aig::and_calls`]) one query may make.
/// Structural hashing makes a repeated subterm free in size but not in
/// time, so time is bounded separately.
pub const MAX_GATE_CALLS: u64 = 2_000_000;

/// Conflicts the CDCL search may spend on one query.
pub const BV_CONFLICT_BUDGET: u64 = 1_000_000;

/// Propagation steps (watch-list visits) the search may spend on one query:
/// seconds of work. A conflict budget alone does not bound time on large
/// formulas.
pub const BV_WORK_BUDGET: u64 = 1 << 30;

/// The bit-vector tactic.
pub struct BvTactic;

impl BvTactic {
    /// Decide the conjunction of `assertions` over the variables in
    /// `sort_env`, which must all be `Bool` or `BitVec`.
    pub fn solve(assertions: &[Predicate], sort_env: &SortEnv) -> SolverResult {
        match Lowering::new(sort_env).and_then(|mut l| l.lower_all(assertions).map(|edges| (l, edges))) {
            Ok((l, edges)) => l.decide(&edges, drat_check::Limits::default()),
            Err(reason) => SolverResult::Unknown(reason),
        }
    }
}

/// A lowered term: a Boolean (one edge) or a bit-vector.
#[derive(Debug, Clone)]
enum Term {
    Bool(Edge),
    Bv(BitVec),
}

/// The circuit being built for one query.
struct Lowering {
    aig: Aig,
    /// Each declared variable's signals, in name order (deterministic).
    vars: Vec<(String, Term)>,
    index: HashMap<String, usize>,
}

type Lowered<T> = Result<T, String>;

impl Lowering {
    /// Create inputs for every declared variable.
    fn new(sort_env: &SortEnv) -> Lowered<Lowering> {
        let mut names: Vec<&String> = sort_env.keys().collect();
        names.sort();
        let mut l = Lowering { aig: Aig::new(), vars: Vec::new(), index: HashMap::new() };
        for name in names {
            if name.starts_with('#') {
                return Err(format!(
                    "variable `{name}` begins with `#`, which is reserved for bit-vector literals"
                ));
            }
            let term = match &sort_env[name] {
                Sort::Bool => Term::Bool(l.aig.input()),
                Sort::BitVec(w) => {
                    let w = width(*w as usize)?;
                    Term::Bv(BitVec::input(&mut l.aig, w))
                }
                other => return Err(format!("sort {other} of `{name}` is not supported by the BV tactic")),
            };
            l.index.insert(name.clone(), l.vars.len());
            l.vars.push((name.clone(), term));
            l.check_size()?;
        }
        Ok(l)
    }

    fn lower_all(&mut self, assertions: &[Predicate]) -> Lowered<Vec<Edge>> {
        assertions
            .iter()
            .map(|p| {
                if depth_of(p) > MAX_PREDICATE_DEPTH {
                    return Err(format!("predicate deeper than MAX_PREDICATE_DEPTH ({MAX_PREDICATE_DEPTH})"));
                }
                self.boolean(p)
            })
            .collect()
    }

    /// The work counterpart of [`Self::reserve`]: refuse before building when
    /// `estimate` more gate constructions would pass [`MAX_GATE_CALLS`]. Node
    /// count alone is not enough, because a repeated subterm is free in size
    /// (structural hashing returns the existing node) but every construction
    /// still costs work.
    fn reserve_work(&self, estimate: u64) -> Lowered<()> {
        if self.aig.and_calls().saturating_add(estimate) > MAX_GATE_CALLS {
            return Err(format!("bit-blasting work exceeds MAX_GATE_CALLS ({MAX_GATE_CALLS})"));
        }
        Ok(())
    }

    fn check_size(&self) -> Lowered<()> {
        if self.aig.num_nodes() > MAX_AIG_NODES {
            return Err(format!("bit-blasted circuit exceeds MAX_AIG_NODES ({MAX_AIG_NODES})"));
        }
        if self.aig.and_calls() > MAX_GATE_CALLS {
            return Err(format!("bit-blasting work exceeds MAX_GATE_CALLS ({MAX_GATE_CALLS})"));
        }
        Ok(())
    }

    /// Encode, solve, and either read back a model or certify the UNSAT.
    ///
    /// `drat_limits` bounds the certificate check. [`BvTactic::solve`]
    /// always passes `drat-check`'s defaults; only the test module passes a
    /// larger, documented work budget for a few wide miters, so the public
    /// tactic's limits are never raised.
    fn decide(&self, asserted: &[Edge], drat_limits: drat_check::Limits) -> SolverResult {
        let mut cnf = Cnf::new(0);
        let mut t = Tseitin::new();
        for &e in asserted {
            t.assert_true(&self.aig, e, &mut cnf);
        }
        // The node cap keeps the CNF inside the kernel's variable limit, so
        // every UNSAT can be checked. Should the caps ever drift apart, refuse
        // before spending a solve whose UNSAT could never be certified.
        if cnf.num_vars > drat_check::DEFAULT_MAX_VARS {
            return SolverResult::Unknown(format!(
                "CNF has {} variables, above drat-check's limit of {}",
                cnf.num_vars,
                drat_check::DEFAULT_MAX_VARS
            ));
        }
        let mut solver = Solver::from_cnf_with_proof(&cnf);
        solver.set_conflict_budget(Some(BV_CONFLICT_BUDGET));
        solver.set_work_budget(Some(BV_WORK_BUDGET));
        match solver.solve() {
            SolveResult::Sat => {
                let model = solver.model();
                // An input outside every asserted cone is unconstrained:
                // any value works, and false is as good as any.
                let bit = |e: Edge| match self.aig.node(e.node()) {
                    Node::Input(i) => t.input_value(&self.aig, i, model).unwrap_or(false),
                    _ => unreachable!("declared variables are AIG inputs"),
                };
                let mut m = Model::new();
                for (name, term) in &self.vars {
                    let v = match term {
                        Term::Bool(e) => Value::Bool(bit(*e)),
                        Term::Bv(b) => Value::BitVec(b.bits.iter().map(|&e| bit(e)).collect()),
                    };
                    m.insert(name.clone(), v);
                }
                SolverResult::Sat(m)
            }
            SolveResult::Unsat => {
                let proof = solver.proof().expect("proof logging was enabled").to_drat_text();
                match drat_check::check_with_limits(&write_dimacs(&cnf), &proof, drat_limits) {
                    Ok(_) => SolverResult::Unsat,
                    Err(why) => SolverResult::Unknown(format!(
                        "UNSAT not certified: drat-check rejected the proof ({why})"
                    )),
                }
            }
            SolveResult::Unknown => SolverResult::Unknown(format!(
                "BV search budget exhausted ({BV_CONFLICT_BUDGET} conflicts / {BV_WORK_BUDGET} propagation steps)"
            )),
        }
    }

    // -----------------------------------------------------------------------
    // Lowering predicates to circuits
    // -----------------------------------------------------------------------

    fn boolean(&mut self, p: &Predicate) -> Lowered<Edge> {
        match self.term(p)? {
            Term::Bool(e) => Ok(e),
            Term::Bv(_) => Err("bit-vector where a Boolean is required".into()),
        }
    }

    fn bitvec(&mut self, p: &Predicate) -> Lowered<BitVec> {
        match self.term(p)? {
            Term::Bv(b) => Ok(b),
            Term::Bool(_) => Err("Boolean where a bit-vector is required".into()),
        }
    }

    /// Two bit-vector operands of equal width.
    fn pair(&mut self, op: &str, args: &[Predicate]) -> Lowered<(BitVec, BitVec)> {
        let [a, b] = args else {
            return Err(format!("`{op}` takes 2 arguments, got {}", args.len()));
        };
        let (a, b) = (self.bitvec(a)?, self.bitvec(b)?);
        if a.width() != b.width() {
            return Err(format!("`{op}` on widths {} and {}", a.width(), b.width()));
        }
        Ok((a, b))
    }

    fn term(&mut self, p: &Predicate) -> Lowered<Term> {
        let t = match p {
            Predicate::Bool(b) => Term::Bool(Edge::constant(*b)),
            Predicate::Var(name) if name.starts_with('#') => Term::Bv(literal(name)?),
            Predicate::Var(name) => {
                let i = *self.index.get(name).ok_or_else(|| format!("undeclared `{name}`"))?;
                self.vars[i].1.clone()
            }
            Predicate::Not(a) => Term::Bool(!self.boolean(a)?),
            Predicate::And(ps) | Predicate::Or(ps) => {
                let mut edges = Vec::with_capacity(ps.len());
                for q in ps {
                    edges.push(self.boolean(q)?);
                }
                Term::Bool(if matches!(p, Predicate::And(_)) {
                    self.aig.and_all(&edges)
                } else {
                    self.aig.or_all(&edges)
                })
            }
            Predicate::Implies(a, b) => {
                let (a, b) = (self.boolean(a)?, self.boolean(b)?);
                Term::Bool(self.aig.implies(a, b))
            }
            Predicate::Iff(a, b) => {
                let (a, b) = (self.boolean(a)?, self.boolean(b)?);
                Term::Bool(self.aig.xnor(a, b))
            }
            Predicate::Eq(a, b) | Predicate::NEq(a, b) => {
                let same = match (self.term(a)?, self.term(b)?) {
                    (Term::Bool(x), Term::Bool(y)) => self.aig.xnor(x, y),
                    (Term::Bv(x), Term::Bv(y)) if x.width() == y.width() => bv::eq(&mut self.aig, &x, &y),
                    _ => return Err("`=` on operands of different sorts or widths".into()),
                };
                Term::Bool(if matches!(p, Predicate::Eq(..)) { same } else { !same })
            }
            Predicate::Ite(c, t, e) => {
                let c = self.boolean(c)?;
                match (self.term(t)?, self.term(e)?) {
                    (Term::Bool(x), Term::Bool(y)) => Term::Bool(self.aig.mux(c, x, y)),
                    (Term::Bv(x), Term::Bv(y)) if x.width() == y.width() => {
                        Term::Bv(bv::ite(&mut self.aig, c, &x, &y))
                    }
                    _ => return Err("`ite` arms of different sorts or widths".into()),
                }
            }
            Predicate::Apply { f, args } => self.apply(f, args)?,
            other => return Err(format!("term not supported by the BV tactic: {other:?}")),
        };
        self.check_size()?;
        Ok(t)
    }

    /// Refuse before building a circuit whose estimated size would push the
    /// query past [`MAX_AIG_NODES`]. [`Lowering::check_size`] only runs after
    /// an operation finishes; a division at width 512 would first build
    /// millions of gates.
    fn reserve(&self, estimate: usize) -> Lowered<()> {
        if self.aig.num_nodes().saturating_add(estimate) > MAX_AIG_NODES {
            return Err(format!("bit-blasted circuit exceeds MAX_AIG_NODES ({MAX_AIG_NODES})"));
        }
        Ok(())
    }

    /// Indexed identifiers, `(_ name i…)`, with the operands (if any)
    /// following the indices. `smt-lib-format` cannot yet read the head
    /// `((_ extract 7 4) x)`, so the tactic accepts the flattened form
    /// `Apply { f: "_", args: [Var("extract"), Int(7), Int(4), x] }`.
    ///
    /// | Form | Meaning |
    /// |---|---|
    /// | `(_ bvN w)` | constant `N` in `w` bits |
    /// | `(_ extract hi lo) a` | bits `hi..=lo` of `a` |
    /// | `(_ zero_extend k) a` | `a` with `k` zero bits above |
    /// | `(_ sign_extend k) a` | `a` with `k` copies of its sign bit above |
    fn indexed(&mut self, args: &[Predicate]) -> Lowered<BitVec> {
        let Some(Predicate::Var(name)) = args.first() else {
            return Err("indexed term other than `(_ bvN w)`".into());
        };
        let index = |p: &Predicate| match p {
            Predicate::Int(i) if *i >= 0 => Ok(usize::try_from(*i).unwrap_or(usize::MAX)),
            _ => Err(format!("`(_ {name} …)`: index must be a non-negative integer")),
        };
        match (name.as_str(), &args[1..]) {
            ("extract", [hi, lo, a]) => {
                let (hi, lo) = (index(hi)?, index(lo)?);
                let a = self.bitvec(a)?;
                if lo > hi || hi >= a.width() {
                    return Err(format!(
                        "`(_ extract {hi} {lo})` on a {}-bit vector (needs lo <= hi < width)",
                        a.width()
                    ));
                }
                Ok(bv::extract(&a, hi, lo))
            }
            ("zero_extend" | "sign_extend", [k, a]) => {
                let k = index(k)?;
                let a = self.bitvec(a)?;
                width(a.width().saturating_add(k))?;
                Ok(if name == "zero_extend" { bv::zext(&a, k) } else { bv::sext(&a, k) })
            }
            _ => indexed_literal(args),
        }
    }

    fn apply(&mut self, f: &str, args: &[Predicate]) -> Lowered<Term> {
        Ok(match f {
            "_" => Term::Bv(self.indexed(args)?),
            "concat" => {
                let [hi, lo] = args else {
                    return Err(format!("`concat` takes 2 arguments, got {}", args.len()));
                };
                let (hi, lo) = (self.bitvec(hi)?, self.bitvec(lo)?);
                width(hi.width() + lo.width())?;
                Term::Bv(bv::concat(&hi, &lo))
            }
            "bvudiv" | "bvurem" | "bvsdiv" | "bvsrem" | "bvsmod" => {
                let (a, b) = self.pair(f, args)?;
                self.reserve(division_gate_estimate(a.width()))?;
                let g = &mut self.aig;
                Term::Bv(match f {
                    "bvudiv" => bv::udiv(g, &a, &b),
                    "bvurem" => bv::urem(g, &a, &b),
                    "bvsdiv" => bv::sdiv(g, &a, &b),
                    "bvsrem" => bv::srem(g, &a, &b),
                    _ => smod(g, &a, &b),
                })
            }
            "bvnot" | "bvneg" => {
                let [a] = args else {
                    return Err(format!("`{f}` takes 1 argument, got {}", args.len()));
                };
                let a = self.bitvec(a)?;
                Term::Bv(if f == "bvnot" { bv::not(&a) } else { bv::neg(&mut self.aig, &a) })
            }
            // Left-associative n-ary operators (SMT-LIB 2.6 :left-assoc).
            "bvand" | "bvor" | "bvxor" | "bvadd" | "bvsub" | "bvmul" => {
                if args.len() < 2 {
                    return Err(format!("`{f}` takes at least 2 arguments"));
                }
                let mut acc = self.bitvec(&args[0])?;
                for q in &args[1..] {
                    let x = self.bitvec(q)?;
                    if x.width() != acc.width() {
                        return Err(format!("`{f}` on widths {} and {}", acc.width(), x.width()));
                    }
                    if f == "bvmul" {
                        self.reserve(multiplication_gate_estimate(acc.width()))?;
                        self.reserve_work(multiplication_and_call_estimate(acc.width()))?;
                    }
                    let g = &mut self.aig;
                    acc = match f {
                        "bvand" => bv::and(g, &acc, &x),
                        "bvor" => bv::or(g, &acc, &x),
                        "bvxor" => bv::xor(g, &acc, &x),
                        "bvadd" => bv::add(g, &acc, &x),
                        "bvsub" => bv::sub(g, &acc, &x),
                        _ => bv::mul(g, &acc, &x),
                    };
                    self.check_size()?;
                }
                Term::Bv(acc)
            }
            "bvshl" | "bvlshr" | "bvashr" => {
                let (a, s) = self.pair(f, args)?;
                let g = &mut self.aig;
                Term::Bv(match f {
                    "bvshl" => bv::shl(g, &a, &s),
                    "bvlshr" => bv::lshr(g, &a, &s),
                    _ => bv::ashr(g, &a, &s),
                })
            }
            "bvult" | "bvule" | "bvugt" | "bvuge" | "bvslt" | "bvsle" | "bvsgt" | "bvsge" => {
                let (a, b) = self.pair(f, args)?;
                let g = &mut self.aig;
                Term::Bool(match f {
                    "bvult" => bv::ult(g, &a, &b),
                    "bvule" => bv::ule(g, &a, &b),
                    "bvugt" => bv::ugt(g, &a, &b),
                    "bvuge" => bv::uge(g, &a, &b),
                    "bvslt" => bv::slt(g, &a, &b),
                    "bvsle" => bv::sle(g, &a, &b),
                    "bvsgt" => bv::sgt(g, &a, &b),
                    _ => bv::sge(g, &a, &b),
                })
            }
            _ => return Err(format!("function `{f}` is not supported by the BV tactic")),
        })
    }
}

/// Conservative gates per bit² of a restoring divider (measured: see
/// `division_cost_estimate_is_conservative`). Used to refuse a division
/// before building it.
const DIV_GATES_PER_BIT_SQUARED: usize = 16;

/// The linear part: `bvsmod` also builds two negations, two selections, two
/// additions and an equality on top of the divider (about 30 gates a bit).
const DIV_GATES_PER_BIT: usize = 64;

/// Upper bound on the gates a division-family operation builds at width `w`.
fn division_gate_estimate(w: usize) -> usize {
    DIV_GATES_PER_BIT_SQUARED * w * w + DIV_GATES_PER_BIT * w
}

/// Upper bound on the gates one `bvmul` builds at width `w`, so that a
/// multiplier too big for [`MAX_AIG_NODES`] is refused before it is built.
///
/// `bit_blaster::bv::mul` is shift-and-add: row `i` is `a << i` ANDed with
/// `b[i]`, and the `w` rows are summed by `w` ripple-carry adders. Shifted-in
/// zeros and the all-zero first accumulator fold away, so about half of each
/// adder is constant; what is left is a little under `4 w²` AND nodes. Measured
/// at the time of writing (new nodes for `bvmul x y`, `w`-bit inputs):
///
/// ```text
///   w      actual    4w² + 2w    ratio
///   1           1           6     6.0
///   2           6          20     3.3
///   3          19          42     2.2
///   8         204         272     1.3
///   16        916       1 056     1.15
///   32      3 876       4 160     1.07
///   64     15 940      16 512     1.04
///  128     64 644      66 048     1.02
/// ```
///
/// The actual count is `4 w² - O(w)`, so `4 w² + 2 w` stays above it with a
/// ratio that tends to 1. Structural hashing can only make a real multiplier
/// smaller (never larger), so the estimate also bounds a multiplier built
/// next to existing circuitry. Work is bounded separately by
/// [`multiplication_and_call_estimate`]: a repeated `bvmul x y` adds no nodes
/// but still makes every gate construction again. See
/// `multiplication_cost_estimate_is_conservative`.
fn multiplication_gate_estimate(w: usize) -> usize {
    MUL_GATES_PER_BIT_SQUARED
        .saturating_mul(w)
        .saturating_mul(w)
        .saturating_add(MUL_GATES_PER_BIT.saturating_mul(w))
}

/// Upper bound on the gate constructions ([`Aig::and_calls`]) one `bvmul`
/// makes at width `w`, counted whether or not structural hashing finds the
/// gate already built. Each of the `w` rows is `w` ANDs, and each ripple-carry
/// adder costs about 9 constructions per bit, so `10 w²` covers both; the
/// conservativeness test measures it.
fn multiplication_and_call_estimate(w: usize) -> u64 {
    (w as u64).saturating_mul(w as u64).saturating_mul(MUL_CALLS_PER_BIT_SQUARED)
}

/// Gate constructions per bit² of the shift-and-add multiplier.
const MUL_CALLS_PER_BIT_SQUARED: u64 = 10;

/// Gates per bit² of the shift-and-add multiplier (see above).
const MUL_GATES_PER_BIT_SQUARED: usize = 4;

/// The linear slack on top, which also covers widths 1 to 3.
const MUL_GATES_PER_BIT: usize = 2;

/// SMT-LIB `bvsmod`: the remainder whose sign follows the **divisor**.
///
/// SMT-LIB defines it in terms of `bvurem` on magnitudes, then fixes the
/// sign by cases (`s` dividend, `t` divisor, `u = |s| urem |t|`):
///
/// ```text
///   u = 0                 →  0          (exact division: no sign to fix)
///   s ≥ 0, t ≥ 0          →  u
///   s <  0, t ≥ 0         →  t − u      (lands in [0, t): sign of t)
///   s ≥ 0, t <  0         →  u + t      (lands in (t, 0]: sign of t)
///   s <  0, t <  0        →  −u
/// ```
///
/// Worked example, 4 bits, `s = −7`, `t = 3`: `u = 7 mod 3 = 1`, result
/// `3 − 1 = 2`, and indeed `−7 = 3·(−3) + 2`. For `t = 0`, `|t| = 0` gives
/// `u = |s|` (see [`bv::urem`]), so the result is `s` itself, as SMT-LIB
/// requires.
fn smod(aig: &mut Aig, s: &BitVec, t: &BitVec) -> BitVec {
    let (neg_s, neg_t) = (s.msb(), t.msb());
    let abs_s = {
        let n = bv::neg(aig, s);
        bv::ite(aig, neg_s, &n, s)
    };
    let abs_t = {
        let n = bv::neg(aig, t);
        bv::ite(aig, neg_t, &n, t)
    };
    let u = bv::urem(aig, &abs_s, &abs_t);
    let neg_u = bv::neg(aig, &u);
    let t_minus_u = bv::add(aig, &neg_u, t); // (−u) + t
    let u_plus_t = bv::add(aig, &u, t);
    let zero = BitVec::constant(s.width(), 0);
    let u_is_zero = bv::eq(aig, &u, &zero);
    // Fold the case split from the inside out.
    let s_pos_t_neg = bv::ite(aig, neg_t, &u_plus_t, &u); // s ≥ 0
    let s_neg_t = bv::ite(aig, neg_t, &neg_u, &t_minus_u); // s < 0
    let by_sign = bv::ite(aig, neg_s, &s_neg_t, &s_pos_t_neg);
    bv::ite(aig, u_is_zero, &zero, &by_sign)
}

/// Check a width against [`MAX_BV_WIDTH`].
fn width(w: usize) -> Lowered<usize> {
    if w == 0 || w > MAX_BV_WIDTH {
        return Err(format!("bit-vector width {w} (allowed: 1..={MAX_BV_WIDTH})"));
    }
    Ok(w)
}

/// `#b0101` (binary, one bit per digit) or `#x1f` (hex, four bits per
/// digit). Digits are written most significant first.
fn literal(text: &str) -> Lowered<BitVec> {
    let bad = || format!("malformed bit-vector literal `{text}`");
    let (radix_bits, digits) = if let Some(d) = text.strip_prefix("#b") {
        (1, d)
    } else if let Some(d) = text.strip_prefix("#x") {
        (4, d)
    } else {
        return Err(bad());
    };
    if digits.is_empty() {
        return Err(bad());
    }
    width(digits.len().saturating_mul(radix_bits))?;
    let mut bits = Vec::with_capacity(digits.len() * radix_bits);
    for c in digits.chars().rev() {
        let v = c.to_digit(1 << radix_bits).ok_or_else(bad)?;
        bits.extend((0..radix_bits).map(|i| v >> i & 1 == 1));
    }
    Ok(BitVec::from_bools(&bits))
}

/// `(_ bvN w)`: the decimal constant `N` in `w` bits. `N` must fit in 128
/// bits and in `w` bits.
fn indexed_literal(args: &[Predicate]) -> Lowered<BitVec> {
    let bad = || "indexed term other than `(_ bvN w)`".to_string();
    let [Predicate::Var(name), Predicate::Int(w)] = args else {
        return Err(bad());
    };
    let digits = name.strip_prefix("bv").ok_or_else(bad)?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad());
    }
    let value: u128 = digits.parse().map_err(|_| format!("`(_ {name} {w})`: constant beyond 128 bits"))?;
    let w = usize::try_from(*w).map_err(|_| format!("bit-vector width {w}"))?;
    let w = width(w)?;
    if w < 128 && value >> w != 0 {
        return Err(format!("`(_ {name} {w})`: {value} does not fit in {w} bits"));
    }
    Ok(BitVec::constant(w, value))
}

#[cfg(test)]
#[path = "bv_faithfulness.rs"]
mod bv_faithfulness;

#[cfg(test)]
mod tests {
    use super::*;

    fn env(decls: &[(&str, Sort)]) -> SortEnv {
        decls.iter().map(|(n, s)| (n.to_string(), s.clone())).collect()
    }

    fn var(n: &str) -> Predicate {
        Predicate::Var(n.into())
    }

    fn app(f: &str, args: Vec<Predicate>) -> Predicate {
        Predicate::Apply { f: f.into(), args }
    }

    #[test]
    fn sat_with_a_model() {
        // x + 1 = 5 over 8 bits.
        let e = env(&[("x", Sort::BitVec(8)), ("p", Sort::Bool)]);
        let a = Predicate::Eq(Box::new(app("bvadd", vec![var("x"), var("#x01")])), Box::new(var("#x05")));
        let r = BvTactic::solve(&[a], &e);
        let m = r.model().expect("sat");
        assert_eq!(m.get("x").unwrap().as_bv_u128(), Some(4));
        assert_eq!(m.get("p"), Some(&Value::Bool(false)), "unconstrained variables still get a value");
    }

    #[test]
    fn unsat_is_certified() {
        // a - b ≠ a + (-b) over 16 bits: UNSAT, and the kernel must agree.
        let e = env(&[("a", Sort::BitVec(16)), ("b", Sort::BitVec(16))]);
        let a = Predicate::NEq(
            Box::new(app("bvsub", vec![var("a"), var("b")])),
            Box::new(app("bvadd", vec![var("a"), app("bvneg", vec![var("b")])])),
        );
        assert_eq!(BvTactic::solve(&[a], &e), SolverResult::Unsat);
    }

    #[test]
    fn refusals_are_unknown() {
        let unknown = |e: &SortEnv, p: Predicate| match BvTactic::solve(&[p], e) {
            SolverResult::Unknown(why) => why,
            other => panic!("expected Unknown, got {other:?}"),
        };
        let e = env(&[("x", Sort::BitVec(4))]);
        assert!(unknown(&env(&[("n", Sort::Int)]), Predicate::Bool(true)).contains("sort Int"));
        assert!(unknown(&env(&[("x", Sort::BitVec(513))]), Predicate::Bool(true)).contains("width 513"));
        let deep = (0..600).fold(Predicate::Bool(true), |acc, _| Predicate::Not(Box::new(acc)));
        assert!(unknown(&e, deep).contains("deeper"));
        assert!(unknown(&e, var("x")).contains("bit-vector where a Boolean"));
        assert!(unknown(&e, Predicate::Le(Box::new(Predicate::Int(1)), Box::new(Predicate::Int(2)))).contains("not supported"));
        // A declared name that looks like a literal would be misread: refuse.
        let tricky = env(&[("#b1", Sort::BitVec(1))]);
        let p = Predicate::Eq(Box::new(var("#b1")), Box::new(var("#b0")));
        assert!(unknown(&tricky, p).contains("reserved for bit-vector literals"));
    }

    #[test]
    fn size_and_work_caps() {
        let unknown = |e: &SortEnv, p: Predicate| match BvTactic::solve(&[p], e) {
            SolverResult::Unknown(why) => why,
            other => panic!("expected Unknown, got {other:?}"),
        };
        // One 512-bit multiplier is far beyond the node cap.
        let e = env(&[("x", Sort::BitVec(512)), ("y", Sort::BitVec(512))]);
        let mul = app("bvmul", vec![var("x"), var("y")]);
        assert!(unknown(&e, Predicate::Eq(Box::new(mul.clone()), Box::new(var("x")))).contains("MAX_AIG_NODES"));
        // A 64-bit multiplier repeated many times: free in size (structural
        // hashing), not in work.
        let e = env(&[("x", Sort::BitVec(64)), ("y", Sort::BitVec(64))]);
        let mul = app("bvmul", vec![var("x"), var("y")]);
        let same = Predicate::Eq(Box::new(mul.clone()), Box::new(mul));
        let many = Predicate::And(vec![same; 200]);
        assert!(unknown(&e, many).contains("MAX_GATE_CALLS"));
    }

    #[test]
    fn literal_forms() {
        assert_eq!(literal("#b10").unwrap(), BitVec::constant(2, 2));
        assert_eq!(literal("#x1f").unwrap(), BitVec::constant(8, 0x1f));
        for bad in ["#b", "#q1", "#b102", "#xg"] {
            assert!(literal(bad).is_err(), "{bad}");
        }
        assert!(literal(&format!("#x{}", "f".repeat(200))).unwrap_err().contains("width 800"));
        let lit = |n: &str, w: i128| indexed_literal(&[var(n), Predicate::Int(w)]);
        assert_eq!(lit("bv5", 8).unwrap(), BitVec::constant(8, 5));
        assert!(lit("bv16", 4).unwrap_err().contains("does not fit"));
        assert!(lit("bvx", 4).is_err());
        assert!(lit("foo", 4).is_err());
        assert!(lit("bv", 4).is_err());
        assert!(lit("bv1", -4).unwrap_err().contains("width -4"));
        assert!(lit("bv1", 0).unwrap_err().contains("width 0"));
        assert!(lit("bv340282366920938463463374607431768211456", 4).unwrap_err().contains("beyond 128"));
        assert!(indexed_literal(&[var("bv1")]).is_err());
    }
}
