//! # Deciding SMT-LIB `QF_BV` problems
//!
//! `smt-lib-format` reads an `.smt2` file into a
//! `constraint_instructions::Program`. This module decides it: every
//! declared bit-vector becomes AIG inputs, every assertion becomes a circuit
//! (via [`crate::bv`]), the circuits become clauses (via [`crate::tseitin`]),
//! and `cdcl-sat` answers each `(check-sat)`.
//!
//! ```text
//!   (declare-const x (_ BitVec 8))         x  → 8 AIG inputs
//!   (assert (= (bvadd x #x01) (_ bv5 8)))  ripple-carry adder + comparator
//!   (check-sat)                            Tseitin → cdcl-sat → sat, x = 4
//! ```
//!
//! **Every UNSAT answer carries its certificate**: the CNF that was solved
//! and the solver's DRAT proof, ready for `drat-check` (VER11). An UNSAT
//! here is only as trustworthy as that check.
//!
//! ## How `smt-lib-format` presents bit-vector syntax
//!
//! The reader is generic, so bit-vector literals arrive in generic shapes:
//!
//! | SMT-LIB | arrives as |
//! |---|---|
//! | `#b0101`, `#x1f` | `Predicate::Var("#b0101")`, `Var("#x1f")` |
//! | `(_ bv5 8)` | `Apply { f: "_", args: [Var("bv5"), Int(8)] }` |
//! | `(bvadd a b)` | `Apply { f: "bvadd", args: [a, b] }` |
//! | `(= a b)` / `(distinct a b)` | `Eq(a, b)` / `NEq(a, b)` |
//!
//! A user-declared name can never begin with `#` (SMT-LIB symbols cannot),
//! so treating `#b…` and `#x…` as literals is unambiguous.
//!
//! ## Hostile input
//!
//! A `.smt2` file may come from anywhere, so the front end bounds its work
//! ([`Limits`]): bit-vector widths are capped, the AIG is capped after every
//! operation, and predicates deeper than `constraint-core`'s depth limit
//! are refused before the recursive lowering sees them. Anything the module
//! does not support is an [`Unsupported`] error naming it, never a guess.

use std::collections::BTreeMap;
use std::fmt;

use cdcl_sat::{Cnf, DratProof, SolveResult, Solver};
use constraint_core::{depth_of, Predicate, Sort, MAX_PREDICATE_DEPTH};
use constraint_instructions::{ConstraintInstr, Program};

use crate::aig::{Aig, Edge};
use crate::bv::{self, BitVec};
use crate::tseitin::Tseitin;

/// Work bounds for hostile input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Widest bit-vector accepted, in bits.
    pub max_width: usize,
    /// Most AIG nodes the whole program may build.
    pub max_nodes: usize,
}

impl Default for Limits {
    /// 512-bit vectors and 4 million nodes: a 512-bit multiplier is about
    /// 1.8 million ANDs, so the defaults admit one, not dozens.
    fn default() -> Self {
        Limits { max_width: 512, max_nodes: 4_000_000 }
    }
}

/// A construct this module does not handle, or input that breaks a limit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unsupported(pub String);

impl fmt::Display for Unsupported {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unsupported QF_BV input: {}", self.0)
    }
}

impl std::error::Error for Unsupported {}

/// A value in a model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// A Boolean.
    Bool(bool),
    /// A bit-vector, least significant bit first.
    BitVec(Vec<bool>),
}

impl Value {
    /// A bit-vector's value as an unsigned integer, if it fits in 128 bits.
    pub fn as_u128(&self) -> Option<u128> {
        match self {
            Value::BitVec(bits) if bits.len() <= 128 => {
                Some(bits.iter().enumerate().map(|(i, &b)| (b as u128) << i).sum())
            }
            _ => None,
        }
    }
}

/// The answer to one `(check-sat)`.
#[derive(Debug, Clone)]
pub enum Answer {
    /// Satisfiable: a value for every variable in scope.
    Sat(BTreeMap<String, Value>),
    /// Unsatisfiable, with the CNF that was solved and the DRAT proof that
    /// refutes it.
    Unsat {
        /// The clauses given to the solver.
        cnf: Cnf,
        /// The solver's proof that `cnf` is unsatisfiable.
        proof: DratProof,
    },
}

/// A lowered term: a Boolean (one edge) or a bit-vector.
#[derive(Debug, Clone)]
enum Term {
    Bool(Edge),
    Bv(BitVec),
}

/// Decide every `(check-sat)` in `program` with the default limits.
pub fn check_program(program: &Program) -> Result<Vec<Answer>, Unsupported> {
    check_program_with(program, Limits::default())
}

/// Decide every `(check-sat)` in `program`, within `limits`.
pub fn check_program_with(program: &Program, limits: Limits) -> Result<Vec<Answer>, Unsupported> {
    let mut b = Blaster::new(limits);
    let mut answers = Vec::new();
    for instr in program.instructions() {
        match instr {
            ConstraintInstr::SetLogic { .. }
            | ConstraintInstr::SetOption { .. }
            | ConstraintInstr::Echo { .. }
            | ConstraintInstr::GetModel
            | ConstraintInstr::GetUnsatCore => {}
            ConstraintInstr::DeclareVar { name, sort } => b.declare(name, sort)?,
            ConstraintInstr::DeclareFn { name, .. } => {
                return Err(Unsupported(format!("uninterpreted function `{name}`")));
            }
            ConstraintInstr::Assert { pred } => {
                let e = b.lower_bool(pred)?;
                b.assertions.push(e);
            }
            ConstraintInstr::CheckSat => answers.push(b.decide()),
            ConstraintInstr::PushScope => b.scopes.push((b.assertions.len(), b.env.len())),
            ConstraintInstr::PopScope => {
                // `Program::new` guarantees balance; an unbalanced pop from
                // an unvalidated program is refused rather than trusted.
                let (a, e) = b.scopes.pop().ok_or_else(|| Unsupported("pop without push".into()))?;
                b.assertions.truncate(a);
                b.env.truncate(e);
            }
            ConstraintInstr::Reset => b = Blaster::new(limits),
            other => return Err(Unsupported(format!("command `{}`", other.mnemonic()))),
        }
    }
    Ok(answers)
}

/// Lowering state: the AIG, the variables in scope, the assertions.
struct Blaster {
    limits: Limits,
    aig: Aig,
    /// Declared variables, newest last (a later declaration shadows).
    env: Vec<(String, Term)>,
    assertions: Vec<Edge>,
    /// `(assertions.len(), env.len())` at each `push`.
    scopes: Vec<(usize, usize)>,
}

impl Blaster {
    fn new(limits: Limits) -> Blaster {
        Blaster { limits, aig: Aig::new(), env: Vec::new(), assertions: Vec::new(), scopes: Vec::new() }
    }

    fn declare(&mut self, name: &str, sort: &Sort) -> Result<(), Unsupported> {
        let term = match sort {
            Sort::Bool => Term::Bool(self.aig.input()),
            Sort::BitVec(w) => {
                let w = self.width(*w as usize)?;
                Term::Bv(BitVec::input(&mut self.aig, w))
            }
            other => return Err(Unsupported(format!("sort {other} of `{name}`"))),
        };
        self.env.push((name.to_owned(), term));
        self.check_size()
    }

    fn width(&self, w: usize) -> Result<usize, Unsupported> {
        if w == 0 || w > self.limits.max_width {
            return Err(Unsupported(format!("bit-vector width {w} (allowed: 1..={})", self.limits.max_width)));
        }
        Ok(w)
    }

    fn check_size(&self) -> Result<(), Unsupported> {
        if self.aig.num_nodes() > self.limits.max_nodes {
            return Err(Unsupported(format!("circuit exceeds {} AIG nodes", self.limits.max_nodes)));
        }
        Ok(())
    }

    /// Encode the assertions in scope and solve.
    fn decide(&self) -> Answer {
        let mut cnf = Cnf::new(0);
        let mut t = Tseitin::new();
        for &e in &self.assertions {
            t.assert_true(&self.aig, e, &mut cnf);
        }
        let mut solver = Solver::from_cnf_with_proof(&cnf);
        match solver.solve() {
            SolveResult::Sat => {
                let model = solver.model();
                let bit = |e: Edge| t.input_value(&self.aig, input_index(&self.aig, e), model).unwrap_or(false);
                let mut values = BTreeMap::new();
                for (name, term) in &self.env {
                    let v = match term {
                        Term::Bool(e) => Value::Bool(bit(*e)),
                        Term::Bv(b) => Value::BitVec(b.bits.iter().map(|&e| bit(e)).collect()),
                    };
                    values.insert(name.clone(), v); // later declarations shadow
                }
                Answer::Sat(values)
            }
            // No budget is set, so the solver always finishes.
            _ => Answer::Unsat { cnf, proof: solver.take_proof().expect("logging was enabled") },
        }
    }

    // -----------------------------------------------------------------------
    // Lowering
    // -----------------------------------------------------------------------

    fn lower_bool(&mut self, p: &Predicate) -> Result<Edge, Unsupported> {
        if depth_of(p) > MAX_PREDICATE_DEPTH {
            return Err(Unsupported(format!("assertion deeper than {MAX_PREDICATE_DEPTH}")));
        }
        match self.term(p)? {
            Term::Bool(e) => Ok(e),
            Term::Bv(_) => Err(Unsupported("asserted a bit-vector, not a Boolean".into())),
        }
    }

    fn boolean(&mut self, p: &Predicate) -> Result<Edge, Unsupported> {
        match self.term(p)? {
            Term::Bool(e) => Ok(e),
            Term::Bv(_) => Err(Unsupported("bit-vector where a Boolean is required".into())),
        }
    }

    fn bitvec(&mut self, p: &Predicate) -> Result<BitVec, Unsupported> {
        match self.term(p)? {
            Term::Bv(b) => Ok(b),
            Term::Bool(_) => Err(Unsupported("Boolean where a bit-vector is required".into())),
        }
    }

    /// Two bit-vector operands of equal width.
    fn pair(&mut self, op: &str, args: &[Predicate]) -> Result<(BitVec, BitVec), Unsupported> {
        let [a, b] = args else {
            return Err(Unsupported(format!("`{op}` takes 2 arguments, got {}", args.len())));
        };
        let (a, b) = (self.bitvec(a)?, self.bitvec(b)?);
        if a.width() != b.width() {
            return Err(Unsupported(format!("`{op}` on widths {} and {}", a.width(), b.width())));
        }
        Ok((a, b))
    }

    fn term(&mut self, p: &Predicate) -> Result<Term, Unsupported> {
        let t = match p {
            Predicate::Bool(b) => Term::Bool(Edge::constant(*b)),
            Predicate::Var(name) if name.starts_with('#') => Term::Bv(self.literal(name)?),
            Predicate::Var(name) => self
                .env
                .iter()
                .rev()
                .find(|(n, _)| n == name)
                .map(|(_, t)| t.clone())
                .ok_or_else(|| Unsupported(format!("undeclared `{name}`")))?,
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
                    _ => return Err(Unsupported("`=` on operands of different sorts or widths".into())),
                };
                Term::Bool(if matches!(p, Predicate::Eq(..)) { same } else { !same })
            }
            Predicate::Ite(c, t, e) => {
                let c = self.boolean(c)?;
                match (self.term(t)?, self.term(e)?) {
                    (Term::Bool(x), Term::Bool(y)) => Term::Bool(self.aig.mux(c, x, y)),
                    (Term::Bv(x), Term::Bv(y)) if x.width() == y.width() => Term::Bv(bv::ite(&mut self.aig, c, &x, &y)),
                    _ => return Err(Unsupported("`ite` arms of different sorts or widths".into())),
                }
            }
            Predicate::Apply { f, args } => self.apply(f, args)?,
            other => return Err(Unsupported(format!("term {other:?}"))),
        };
        self.check_size()?;
        Ok(t)
    }

    fn apply(&mut self, f: &str, args: &[Predicate]) -> Result<Term, Unsupported> {
        Ok(match f {
            "_" => Term::Bv(self.indexed_literal(args)?),
            "bvnot" | "bvneg" => {
                let [a] = args else {
                    return Err(Unsupported(format!("`{f}` takes 1 argument, got {}", args.len())));
                };
                let a = self.bitvec(a)?;
                Term::Bv(if f == "bvnot" { bv::not(&a) } else { bv::neg(&mut self.aig, &a) })
            }
            // Left-associative n-ary operators (SMT-LIB 2.6 :left-assoc).
            "bvand" | "bvor" | "bvxor" | "bvadd" | "bvsub" | "bvmul" => {
                if args.len() < 2 {
                    return Err(Unsupported(format!("`{f}` takes at least 2 arguments")));
                }
                let mut acc = self.bitvec(&args[0])?;
                for q in &args[1..] {
                    let x = self.bitvec(q)?;
                    if x.width() != acc.width() {
                        return Err(Unsupported(format!("`{f}` on widths {} and {}", acc.width(), x.width())));
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
            _ => return Err(Unsupported(format!("function `{f}`"))),
        })
    }

    /// `#b0101` (binary, one bit per digit) or `#x1f` (hex, four bits per
    /// digit). Digits are written most significant first.
    fn literal(&self, text: &str) -> Result<BitVec, Unsupported> {
        let bad = || Unsupported(format!("malformed bit-vector literal `{text}`"));
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
        self.width(digits.len().saturating_mul(radix_bits))?;
        let mut bits = Vec::with_capacity(digits.len() * radix_bits);
        for c in digits.chars().rev() {
            let v = c.to_digit(1 << radix_bits).ok_or_else(bad)?;
            bits.extend((0..radix_bits).map(|i| v >> i & 1 == 1));
        }
        Ok(BitVec::from_bools(&bits))
    }

    /// `(_ bvN w)`: the decimal constant `N` in `w` bits. `N` must fit in
    /// 128 bits and in `w` bits.
    fn indexed_literal(&self, args: &[Predicate]) -> Result<BitVec, Unsupported> {
        let bad = || Unsupported("indexed term other than `(_ bvN w)`".into());
        let [Predicate::Var(name), Predicate::Int(w)] = args else {
            return Err(bad());
        };
        let digits = name.strip_prefix("bv").ok_or_else(bad)?;
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return Err(bad());
        }
        let value: u128 = digits
            .parse()
            .map_err(|_| Unsupported(format!("`(_ {name} {w})`: constant beyond 128 bits")))?;
        let w = usize::try_from(*w).map_err(|_| Unsupported(format!("bit-vector width {w}")))?;
        let w = self.width(w)?;
        if w < 128 && value >> w != 0 {
            return Err(Unsupported(format!("`(_ {name} {w})`: {value} does not fit in {w} bits")));
        }
        Ok(BitVec::constant(w, value))
    }
}

/// The input index of an input edge (every declared bit is one).
fn input_index(aig: &Aig, e: Edge) -> usize {
    match aig.node(e.node()) {
        crate::aig::Node::Input(i) => i,
        _ => unreachable!("declared variables are inputs"),
    }
}
