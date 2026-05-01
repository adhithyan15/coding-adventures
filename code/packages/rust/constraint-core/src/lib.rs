//! # `constraint-core` — predicate AST + sort/logic/theory enums + normalisation.
//!
//! **LANG24 PR 24-A**.  The data-crate foundation for the generic
//! Constraint-VM.  Pure data + algorithms; no solver state, no I/O.
//! Future PRs (`constraint-instructions`, `constraint-engine`,
//! `constraint-vm`, every consumer) build against this.
//!
//! Mirrors the Logic-VM stack's layering: `logic-core` → `logic-
//! instructions` → `logic-engine` → `logic-vm`.  This crate is to
//! Constraint-VM what `logic-core` is to Logic-VM.
//!
//! ## What lives here
//!
//! - [`Predicate`] — the recursive constraint-language AST.  Linear
//!   integer arithmetic, boolean logic, ITE, quantifiers, arrays —
//!   covers everything LANG24 §"Theories supported" lists for v1
//!   (SAT + LIA).  v2/v3 theories (LRA, BV, EUF, strings, …) plug
//!   in by adding variants — the enum is `#[non_exhaustive]`.
//! - [`Sort`] — the type system for predicates: `Bool`, `Int`,
//!   `Real`, `BitVec(width)`, `Array { idx, val }`, `Uninterpreted`.
//! - [`Logic`] — declares which theories a constraint program uses
//!   (QF_LIA, QF_LRA, QF_BV, QF_AUFLIA, …) so an engine can reject
//!   programs whose logic isn't supported up-front.
//! - [`Theory`] — what a single tactic handles.  Tactics declare
//!   their `Theory`s; the engine composes them via Nelson-Oppen.
//! - [`Rational`] — minimal `num/den` rational for the `Real`
//!   variant.  Hand-rolled to keep this crate dependency-free.
//! - **Normalisation passes** — [`Predicate::to_nnf`] (negation
//!   normal form via De Morgan), [`Predicate::to_cnf`] (conjunctive
//!   normal form via distribution), [`Predicate::simplify`]
//!   (constant folding + identity eliminations + dedup), and the
//!   smart-constructor methods [`Predicate::and`] / [`Predicate::or`]
//!   / [`Predicate::not`] that simplify on construction.
//! - **Free-variable extraction** — [`Predicate::free_vars`] returns
//!   the names referenced by `Var(name)` minus any bound by
//!   `Forall`/`Exists`.  Bounded by the natural quantifier scope.
//! - **Sort inference** — [`infer_sort`] checks a predicate against
//!   a `SortEnv` and returns the inferred sort or a typed
//!   `SortError`.
//!
//! ## Display format
//!
//! Predicates `Display` as Lisp-style s-expressions:
//! `(and (>= x 1) (<= x 100))`.  Useful for debugging and as the
//! basis for an eventual SMT-LIB exporter (a separate crate; see
//! LANG24 PR 24-E).
//!
//! ## Non-guarantees (caller responsibilities)
//!
//! - **Predicate depth.**  All recursive operations
//!   ([`Predicate::to_nnf`], [`Predicate::to_cnf`],
//!   [`Predicate::simplify`], [`Predicate::free_vars`],
//!   [`infer_sort`], `Display::fmt`) recurse on the AST without an
//!   explicit depth guard.  A maliciously deep `Predicate` can
//!   overflow the thread stack.  Consumers ingesting predicates
//!   from untrusted sources (refinement-type checkers reading
//!   user code, SMT-LIB importers) must enforce a depth limit at
//!   the boundary.  `constraint-engine` (PR 24-C) will provide
//!   the canonical guarded entry point.
//! - **CNF blow-up.**  [`Predicate::to_cnf`] uses naive
//!   distribution, which is exponential in the worst case
//!   (`O(2^n)` clauses for an `Or` of `n` `And`s).  Acceptable
//!   for the small predicates that arise in refinement-type
//!   checking; consumers handling untrusted predicates should
//!   prefer Tseitin encoding (also forthcoming in
//!   `constraint-engine`).
//! - **`Rational` range.**  [`Rational::new`] rejects `i128::MIN`
//!   for either numerator or denominator (its magnitude can't be
//!   represented as a positive `i128`).  Callers building
//!   `Rational`s from external numeric literals must clamp or
//!   pre-validate.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use std::collections::BTreeSet;
use std::collections::HashMap;
use std::fmt;

// ---------------------------------------------------------------------------
// Sort
// ---------------------------------------------------------------------------

/// Sorts (types) for variables, function results, and predicate
/// sub-expressions.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Sort {
    /// Boolean — `true` or `false`.
    Bool,
    /// Integer (arbitrary range; the engine uses i128 internally).
    Int,
    /// Real number — represented by an arbitrary-precision rational.
    Real,
    /// Bit-vector of a fixed width.  The width is in bits.
    BitVec(u32),
    /// Array sort.  `Array { idx, val }` reads as
    /// "array indexed by `idx`, holding `val`".
    Array {
        /// Index sort.
        idx: Box<Sort>,
        /// Value sort.
        val: Box<Sort>,
    },
    /// User-declared uninterpreted sort.  String identifies it.
    Uninterpreted(String),
}

impl fmt::Display for Sort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Sort::Bool => write!(f, "Bool"),
            Sort::Int => write!(f, "Int"),
            Sort::Real => write!(f, "Real"),
            Sort::BitVec(w) => write!(f, "(_ BitVec {w})"),
            Sort::Array { idx, val } => write!(f, "(Array {idx} {val})"),
            Sort::Uninterpreted(name) => write!(f, "{name}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Logic
// ---------------------------------------------------------------------------

/// Logic family — declares which theories a program uses.  Engine
/// uses this to pick tactics + reject unsupported features
/// up-front.  Names match SMT-LIB's standard logic identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
#[allow(non_camel_case_types)] // matches SMT-LIB convention
pub enum Logic {
    /// Boolean only — pure SAT.
    QF_Bool,
    /// Quantifier-free linear integer arithmetic.
    QF_LIA,
    /// Quantifier-free linear real arithmetic.
    QF_LRA,
    /// Quantifier-free bit-vectors.
    QF_BV,
    /// Quantifier-free arrays + uninterpreted functions + LIA.
    QF_AUFLIA,
    /// LIA with quantifiers (Presburger arithmetic).
    LIA,
    /// All theories; engine picks the heaviest tactic.
    ALL,
}

impl fmt::Display for Logic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Logic::QF_Bool => write!(f, "QF_Bool"),
            Logic::QF_LIA => write!(f, "QF_LIA"),
            Logic::QF_LRA => write!(f, "QF_LRA"),
            Logic::QF_BV => write!(f, "QF_BV"),
            Logic::QF_AUFLIA => write!(f, "QF_AUFLIA"),
            Logic::LIA => write!(f, "LIA"),
            Logic::ALL => write!(f, "ALL"),
        }
    }
}

// ---------------------------------------------------------------------------
// Theory
// ---------------------------------------------------------------------------

/// Theory enum — used by tactics to declare what they handle.
/// `Engine` composes tactics across theories via Nelson-Oppen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Theory {
    /// Pure boolean logic (SAT).
    Bool,
    /// Linear integer arithmetic.
    LIA,
    /// Linear real arithmetic.
    LRA,
    /// Theory of arrays (read/write).
    Arrays,
    /// Bit-vectors.
    BitVectors,
    /// Equality with uninterpreted functions.
    EUF,
    /// Strings (future).
    Strings,
    /// Non-linear real arithmetic (future).
    NRA,
    /// Floating-point (future).
    FP,
}

// ---------------------------------------------------------------------------
// Rational
// ---------------------------------------------------------------------------

/// Minimal rational number for the [`Predicate::Real`] variant.
///
/// Stored as `num / den` with `den > 0` and `gcd(|num|, den) == 1`.
/// Construction normalises both invariants.
///
/// Hand-rolled (no `num-rational` dep) to keep the crate
/// dependency-free.  All operations needed by the engine
/// (comparison, addition, sign extraction) work on these directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rational {
    /// Numerator (any sign).
    pub num: i128,
    /// Denominator (always > 0 after normalisation).
    pub den: i128,
}

impl Rational {
    /// Construct a rational `num / den`.  Panics if `den == 0`, or if
    /// either operand equals `i128::MIN` (whose magnitude `2^127`
    /// can't be represented as a positive `i128`, so neither sign
    /// normalisation nor GCD reduction can preserve the invariants).
    /// Otherwise normalises sign onto the numerator and reduces by
    /// the GCD.
    pub fn new(num: i128, den: i128) -> Self {
        assert!(den != 0, "Rational: division by zero");
        // `i128::MIN` cannot be safely negated (its magnitude doesn't
        // fit in `i128`).  Caller must clamp or pre-validate inputs
        // sourced from external predicates.
        assert!(num != i128::MIN, "Rational: numerator must not be i128::MIN");
        assert!(den != i128::MIN, "Rational: denominator must not be i128::MIN");
        let (mut n, mut d) = (num, den);
        if d < 0 {
            n = -n;
            d = -d;
        }
        // After the asserts above, both magnitudes fit in
        // `i128::MAX as u128`, so the `as i128` cast is lossless.
        let g = gcd_i128(n.unsigned_abs(), d.unsigned_abs()) as i128;
        Rational { num: n / g, den: d / g }
    }

    /// Return the rational `n / 1`.
    pub fn from_int(n: i128) -> Self {
        Rational { num: n, den: 1 }
    }
}

impl fmt::Display for Rational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.den == 1 {
            write!(f, "{}", self.num)
        } else {
            write!(f, "{}/{}", self.num, self.den)
        }
    }
}

fn gcd_i128(a: u128, b: u128) -> u128 {
    let (mut a, mut b) = (a, b);
    if a == 0 && b == 0 {
        return 1;
    }
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a.max(1)
}

// ---------------------------------------------------------------------------
// Predicate AST
// ---------------------------------------------------------------------------

/// The recursive constraint-language AST.
///
/// Mirrors LANG24 §"ConstraintIR shape" verbatim.  Variants cover
/// the v1 vocabulary (SAT + LIA + boolean combinators + ITE +
/// quantifiers + arrays) — v2/v3 add to this enum without removing
/// anything (it's `#[non_exhaustive]`).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Predicate {
    /// Boolean literal.
    Bool(bool),
    /// Variable reference (must be declared via DeclareVar in
    /// constraint-instructions).
    Var(String),
    /// Integer literal.
    Int(i128),
    /// Real literal — see [`Rational`].
    Real(Rational),
    /// Application of an uninterpreted function (declared via
    /// DeclareFn in constraint-instructions).
    Apply {
        /// Function name (matches a prior DeclareFn).
        f: String,
        /// Arguments in declared-parameter order.
        args: Vec<Predicate>,
    },
    /// Conjunction.  Empty `And` is `Bool(true)`; flattened by
    /// smart constructors.
    And(Vec<Predicate>),
    /// Disjunction.  Empty `Or` is `Bool(false)`; flattened by
    /// smart constructors.
    Or(Vec<Predicate>),
    /// Negation.
    Not(Box<Predicate>),
    /// `a ⇒ b`.
    Implies(Box<Predicate>, Box<Predicate>),
    /// `a ⇔ b`.
    Iff(Box<Predicate>, Box<Predicate>),
    /// Equality.
    Eq(Box<Predicate>, Box<Predicate>),
    /// Disequality.
    NEq(Box<Predicate>, Box<Predicate>),
    /// `a + b + c + …`.  Empty `Add` is `Int(0)`.
    Add(Vec<Predicate>),
    /// `a - b`.
    Sub(Box<Predicate>, Box<Predicate>),
    /// Linear scaling: `coef * term`.  Restricted to keep LIA
    /// linear; non-linear (var × var) is out of v1 scope.
    Mul {
        /// Integer coefficient.
        coef: i128,
        /// Sub-predicate being scaled.
        term: Box<Predicate>,
    },
    /// `a ≤ b`.
    Le(Box<Predicate>, Box<Predicate>),
    /// `a < b`.
    Lt(Box<Predicate>, Box<Predicate>),
    /// `a ≥ b`.
    Ge(Box<Predicate>, Box<Predicate>),
    /// `a > b`.
    Gt(Box<Predicate>, Box<Predicate>),
    /// If-then-else: `if c then t else e`.  Both arms must have
    /// the same sort.
    Ite(Box<Predicate>, Box<Predicate>, Box<Predicate>),
    /// `∀x: Sort. body`.
    Forall {
        /// Bound variable name.
        var: String,
        /// Sort of the bound variable.
        sort: Sort,
        /// Body in which `var` is bound.
        body: Box<Predicate>,
    },
    /// `∃x: Sort. body`.
    Exists {
        /// Bound variable name.
        var: String,
        /// Sort of the bound variable.
        sort: Sort,
        /// Body in which `var` is bound.
        body: Box<Predicate>,
    },
    /// Array select: `arr[idx]`.
    Select {
        /// Array predicate.
        arr: Box<Predicate>,
        /// Index predicate.
        idx: Box<Predicate>,
    },
    /// Array store: `arr[idx ↦ val]` — produces a new array.
    Store {
        /// Original array.
        arr: Box<Predicate>,
        /// Index to overwrite.
        idx: Box<Predicate>,
        /// New value at that index.
        val: Box<Predicate>,
    },
}

// ---------------------------------------------------------------------------
// Smart constructors — simplify on construction
// ---------------------------------------------------------------------------

impl Predicate {
    /// Build an `And` predicate, applying these simplifications:
    /// - empty Vec → `Bool(true)`
    /// - single element → return it unwrapped
    /// - flatten nested `And` (one level)
    /// - drop `Bool(true)` operands
    /// - any `Bool(false)` operand → `Bool(false)`
    pub fn and(parts: Vec<Predicate>) -> Predicate {
        let mut out = Vec::with_capacity(parts.len());
        for p in parts {
            match p {
                Predicate::Bool(true) => continue,
                Predicate::Bool(false) => return Predicate::Bool(false),
                Predicate::And(inner) => out.extend(inner),
                other => out.push(other),
            }
        }
        match out.len() {
            0 => Predicate::Bool(true),
            1 => out.into_iter().next().unwrap(),
            _ => Predicate::And(out),
        }
    }

    /// Build an `Or` predicate, applying mirror simplifications to
    /// [`Predicate::and`].
    pub fn or(parts: Vec<Predicate>) -> Predicate {
        let mut out = Vec::with_capacity(parts.len());
        for p in parts {
            match p {
                Predicate::Bool(false) => continue,
                Predicate::Bool(true) => return Predicate::Bool(true),
                Predicate::Or(inner) => out.extend(inner),
                other => out.push(other),
            }
        }
        match out.len() {
            0 => Predicate::Bool(false),
            1 => out.into_iter().next().unwrap(),
            _ => Predicate::Or(out),
        }
    }

    /// Build a `Not` predicate, applying:
    /// - `Not(Bool(b))` → `Bool(!b)`
    /// - `Not(Not(p))` → `p`
    #[allow(clippy::should_implement_trait)]
    pub fn not(p: Predicate) -> Predicate {
        match p {
            Predicate::Bool(b) => Predicate::Bool(!b),
            Predicate::Not(inner) => *inner,
            other => Predicate::Not(Box::new(other)),
        }
    }
}

// ---------------------------------------------------------------------------
// Normalisation passes
// ---------------------------------------------------------------------------

impl Predicate {
    /// Convert to **negation normal form** (NNF): all `Not`s
    /// pushed down to atoms via De Morgan + double-negation
    /// elimination.  Does NOT touch quantifier bodies' bound
    /// variables.
    pub fn to_nnf(self) -> Predicate {
        match self {
            // Atoms — already NNF
            Predicate::Bool(_) | Predicate::Var(_) | Predicate::Int(_)
            | Predicate::Real(_) => self,
            Predicate::Eq(_, _) | Predicate::NEq(_, _)
            | Predicate::Le(_, _) | Predicate::Lt(_, _)
            | Predicate::Ge(_, _) | Predicate::Gt(_, _) => self,
            // Compound logical operators
            Predicate::And(parts) => {
                Predicate::and(parts.into_iter().map(Self::to_nnf).collect())
            }
            Predicate::Or(parts) => {
                Predicate::or(parts.into_iter().map(Self::to_nnf).collect())
            }
            Predicate::Not(inner) => match *inner {
                // Double negation
                Predicate::Not(p) => p.to_nnf(),
                // De Morgan
                Predicate::And(parts) => Predicate::or(
                    parts.into_iter().map(|p| Predicate::not(p).to_nnf()).collect(),
                ),
                Predicate::Or(parts) => Predicate::and(
                    parts.into_iter().map(|p| Predicate::not(p).to_nnf()).collect(),
                ),
                Predicate::Implies(a, b) => Predicate::and(vec![
                    a.to_nnf(),
                    Predicate::not(*b).to_nnf(),
                ]),
                // Negate atomic comparisons (an LIA optimisation;
                // ¬(a < b) ≡ a ≥ b, etc.)
                Predicate::Lt(a, b) => Predicate::Ge(a, b),
                Predicate::Le(a, b) => Predicate::Gt(a, b),
                Predicate::Gt(a, b) => Predicate::Le(a, b),
                Predicate::Ge(a, b) => Predicate::Lt(a, b),
                Predicate::Eq(a, b) => Predicate::NEq(a, b),
                Predicate::NEq(a, b) => Predicate::Eq(a, b),
                Predicate::Bool(b) => Predicate::Bool(!b),
                // Other unary cases keep a Not wrapper.
                other => Predicate::Not(Box::new(other.to_nnf())),
            },
            // Implies / Iff — translate to And/Or/Not so NNF can act
            Predicate::Implies(a, b) => {
                Predicate::or(vec![Predicate::not(*a).to_nnf(), b.to_nnf()])
            }
            Predicate::Iff(a, b) => {
                let a_nnf = a.to_nnf();
                let b_nnf = b.to_nnf();
                Predicate::and(vec![
                    Predicate::or(vec![Predicate::not(a_nnf.clone()).to_nnf(), b_nnf.clone()]),
                    Predicate::or(vec![Predicate::not(b_nnf).to_nnf(), a_nnf]),
                ])
            }
            // Recurse into structural arms.
            Predicate::Apply { f, args } => Predicate::Apply {
                f,
                args: args.into_iter().map(Self::to_nnf).collect(),
            },
            Predicate::Add(parts) => Predicate::Add(parts.into_iter().map(Self::to_nnf).collect()),
            Predicate::Sub(a, b) => Predicate::Sub(Box::new(a.to_nnf()), Box::new(b.to_nnf())),
            Predicate::Mul { coef, term } => {
                Predicate::Mul { coef, term: Box::new(term.to_nnf()) }
            }
            Predicate::Ite(c, t, e) => Predicate::Ite(
                Box::new(c.to_nnf()),
                Box::new(t.to_nnf()),
                Box::new(e.to_nnf()),
            ),
            Predicate::Forall { var, sort, body } => Predicate::Forall {
                var,
                sort,
                body: Box::new(body.to_nnf()),
            },
            Predicate::Exists { var, sort, body } => Predicate::Exists {
                var,
                sort,
                body: Box::new(body.to_nnf()),
            },
            Predicate::Select { arr, idx } => Predicate::Select {
                arr: Box::new(arr.to_nnf()),
                idx: Box::new(idx.to_nnf()),
            },
            Predicate::Store { arr, idx, val } => Predicate::Store {
                arr: Box::new(arr.to_nnf()),
                idx: Box::new(idx.to_nnf()),
                val: Box::new(val.to_nnf()),
            },
        }
    }

    /// Convert to **conjunctive normal form** (CNF) via NNF + naive
    /// distribution of `Or` over `And`.
    ///
    /// **Warning: exponential in the worst case.**  Naive
    /// distribution of an OR-of-N-ANDs blows up factorially.  For
    /// large predicates, use Tseitin-style rewriting (introduces
    /// fresh bool vars) — out of v1 scope.  Callers should keep CNF
    /// inputs small or arrange them in NNF and let the SAT tactic
    /// drive its own clause learning.
    pub fn to_cnf(self) -> Predicate {
        let nnf = self.to_nnf();
        cnf_distribute(nnf)
    }

    /// Apply local simplifications — constant folding, identity
    /// elimination, dedup of And/Or operands.  Idempotent.
    ///
    /// Doesn't change semantics; returns a smaller equivalent
    /// predicate (or the same one if no rule fires).
    pub fn simplify(self) -> Predicate {
        match self {
            Predicate::And(parts) => {
                let simplified: Vec<_> = parts
                    .into_iter()
                    .map(|p| p.simplify())
                    .collect();
                Predicate::and(dedup_keeping_order(simplified))
            }
            Predicate::Or(parts) => {
                let simplified: Vec<_> = parts
                    .into_iter()
                    .map(|p| p.simplify())
                    .collect();
                Predicate::or(dedup_keeping_order(simplified))
            }
            Predicate::Not(inner) => Predicate::not(inner.simplify()),
            Predicate::Implies(a, b) => {
                Predicate::Implies(Box::new(a.simplify()), Box::new(b.simplify()))
            }
            Predicate::Iff(a, b) => {
                Predicate::Iff(Box::new(a.simplify()), Box::new(b.simplify()))
            }
            Predicate::Eq(a, b) => Predicate::Eq(Box::new(a.simplify()), Box::new(b.simplify())),
            Predicate::NEq(a, b) => Predicate::NEq(Box::new(a.simplify()), Box::new(b.simplify())),
            Predicate::Add(parts) => {
                Predicate::Add(parts.into_iter().map(|p| p.simplify()).collect())
            }
            Predicate::Sub(a, b) => Predicate::Sub(Box::new(a.simplify()), Box::new(b.simplify())),
            Predicate::Mul { coef, term } => Predicate::Mul {
                coef,
                term: Box::new(term.simplify()),
            },
            Predicate::Le(a, b) => Predicate::Le(Box::new(a.simplify()), Box::new(b.simplify())),
            Predicate::Lt(a, b) => Predicate::Lt(Box::new(a.simplify()), Box::new(b.simplify())),
            Predicate::Ge(a, b) => Predicate::Ge(Box::new(a.simplify()), Box::new(b.simplify())),
            Predicate::Gt(a, b) => Predicate::Gt(Box::new(a.simplify()), Box::new(b.simplify())),
            Predicate::Ite(c, t, e) => {
                let c_s = c.simplify();
                match c_s {
                    Predicate::Bool(true) => t.simplify(),
                    Predicate::Bool(false) => e.simplify(),
                    other => Predicate::Ite(
                        Box::new(other),
                        Box::new(t.simplify()),
                        Box::new(e.simplify()),
                    ),
                }
            }
            // Atoms and quantifiers / arrays / apply: recurse into structure
            // but no simplification rule fires.
            Predicate::Apply { f, args } => Predicate::Apply {
                f,
                args: args.into_iter().map(|a| a.simplify()).collect(),
            },
            Predicate::Forall { var, sort, body } => Predicate::Forall {
                var,
                sort,
                body: Box::new(body.simplify()),
            },
            Predicate::Exists { var, sort, body } => Predicate::Exists {
                var,
                sort,
                body: Box::new(body.simplify()),
            },
            Predicate::Select { arr, idx } => Predicate::Select {
                arr: Box::new(arr.simplify()),
                idx: Box::new(idx.simplify()),
            },
            Predicate::Store { arr, idx, val } => Predicate::Store {
                arr: Box::new(arr.simplify()),
                idx: Box::new(idx.simplify()),
                val: Box::new(val.simplify()),
            },
            atom => atom,
        }
    }

    /// Return the set of free variable names referenced by this
    /// predicate.  Variables bound by `Forall` / `Exists` are
    /// excluded.
    pub fn free_vars(&self) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        free_vars_rec(self, &mut out, &mut BTreeSet::new());
        out
    }
}

fn free_vars_rec(p: &Predicate, out: &mut BTreeSet<String>, bound: &mut BTreeSet<String>) {
    match p {
        Predicate::Bool(_) | Predicate::Int(_) | Predicate::Real(_) => {}
        Predicate::Var(name) => {
            if !bound.contains(name) {
                out.insert(name.clone());
            }
        }
        Predicate::Apply { args, .. } => {
            for a in args {
                free_vars_rec(a, out, bound);
            }
        }
        Predicate::And(parts) | Predicate::Or(parts) | Predicate::Add(parts) => {
            for p in parts {
                free_vars_rec(p, out, bound);
            }
        }
        Predicate::Not(p) => free_vars_rec(p, out, bound),
        Predicate::Implies(a, b)
        | Predicate::Iff(a, b)
        | Predicate::Eq(a, b)
        | Predicate::NEq(a, b)
        | Predicate::Sub(a, b)
        | Predicate::Le(a, b)
        | Predicate::Lt(a, b)
        | Predicate::Ge(a, b)
        | Predicate::Gt(a, b) => {
            free_vars_rec(a, out, bound);
            free_vars_rec(b, out, bound);
        }
        Predicate::Mul { term, .. } => free_vars_rec(term, out, bound),
        Predicate::Ite(c, t, e) => {
            free_vars_rec(c, out, bound);
            free_vars_rec(t, out, bound);
            free_vars_rec(e, out, bound);
        }
        Predicate::Forall { var, body, .. } | Predicate::Exists { var, body, .. } => {
            let inserted = bound.insert(var.clone());
            free_vars_rec(body, out, bound);
            if inserted {
                bound.remove(var);
            }
        }
        Predicate::Select { arr, idx } => {
            free_vars_rec(arr, out, bound);
            free_vars_rec(idx, out, bound);
        }
        Predicate::Store { arr, idx, val } => {
            free_vars_rec(arr, out, bound);
            free_vars_rec(idx, out, bound);
            free_vars_rec(val, out, bound);
        }
    }
}

fn cnf_distribute(p: Predicate) -> Predicate {
    // Walk the NNF tree.  At an Or node: distribute over any And
    // child.  At an And node: just recurse and re-and the parts.
    // No top-level rewriting needed for atoms, comparisons, or
    // already-cnf-shaped subtrees.
    match p {
        Predicate::And(parts) => {
            Predicate::and(parts.into_iter().map(cnf_distribute).collect())
        }
        Predicate::Or(parts) => {
            // Fully distribute by repeatedly cross-producting And children.
            let parts: Vec<Predicate> = parts.into_iter().map(cnf_distribute).collect();
            // Find the first And in parts (if any).  If none, this Or is
            // already in CNF (it's a clause).
            if let Some(and_idx) = parts.iter().position(|p| matches!(p, Predicate::And(_))) {
                let and_parts = match &parts[and_idx] {
                    Predicate::And(v) => v.clone(),
                    _ => unreachable!(),
                };
                let rest: Vec<Predicate> = parts
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| *i != and_idx)
                    .map(|(_, p)| p.clone())
                    .collect();
                // (a ∧ b) ∨ rest  →  (a ∨ rest) ∧ (b ∨ rest)
                let clauses: Vec<Predicate> = and_parts
                    .into_iter()
                    .map(|a| {
                        let mut new_or = vec![a];
                        new_or.extend(rest.clone());
                        cnf_distribute(Predicate::or(new_or))
                    })
                    .collect();
                Predicate::and(clauses)
            } else {
                Predicate::or(parts)
            }
        }
        // Atoms and other constructors stay as-is (they're either
        // atomic or contain no boolean structure to distribute over
        // in v1's scope).
        other => other,
    }
}

fn dedup_keeping_order(parts: Vec<Predicate>) -> Vec<Predicate> {
    let mut seen: Vec<Predicate> = Vec::with_capacity(parts.len());
    for p in parts {
        if !seen.iter().any(|q| q == &p) {
            seen.push(p);
        }
    }
    seen
}

// ---------------------------------------------------------------------------
// Display
// ---------------------------------------------------------------------------

impl fmt::Display for Predicate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Predicate::Bool(b) => write!(f, "{}", if *b { "true" } else { "false" }),
            Predicate::Var(name) => write!(f, "{name}"),
            Predicate::Int(n) => write!(f, "{n}"),
            Predicate::Real(r) => write!(f, "{r}"),
            Predicate::Apply { f: name, args } => {
                write!(f, "({name}")?;
                for a in args {
                    write!(f, " {a}")?;
                }
                write!(f, ")")
            }
            Predicate::And(parts) => write_nary(f, "and", parts),
            Predicate::Or(parts) => write_nary(f, "or", parts),
            Predicate::Add(parts) => write_nary(f, "+", parts),
            Predicate::Not(p) => write!(f, "(not {p})"),
            Predicate::Implies(a, b) => write!(f, "(=> {a} {b})"),
            Predicate::Iff(a, b) => write!(f, "(= {a} {b})"),
            Predicate::Eq(a, b) => write!(f, "(= {a} {b})"),
            Predicate::NEq(a, b) => write!(f, "(distinct {a} {b})"),
            Predicate::Sub(a, b) => write!(f, "(- {a} {b})"),
            Predicate::Mul { coef, term } => write!(f, "(* {coef} {term})"),
            Predicate::Le(a, b) => write!(f, "(<= {a} {b})"),
            Predicate::Lt(a, b) => write!(f, "(< {a} {b})"),
            Predicate::Ge(a, b) => write!(f, "(>= {a} {b})"),
            Predicate::Gt(a, b) => write!(f, "(> {a} {b})"),
            Predicate::Ite(c, t, e) => write!(f, "(ite {c} {t} {e})"),
            Predicate::Forall { var, sort, body } => {
                write!(f, "(forall (({var} {sort})) {body})")
            }
            Predicate::Exists { var, sort, body } => {
                write!(f, "(exists (({var} {sort})) {body})")
            }
            Predicate::Select { arr, idx } => write!(f, "(select {arr} {idx})"),
            Predicate::Store { arr, idx, val } => write!(f, "(store {arr} {idx} {val})"),
        }
    }
}

fn write_nary(f: &mut fmt::Formatter<'_>, op: &str, parts: &[Predicate]) -> fmt::Result {
    write!(f, "({op}")?;
    for p in parts {
        write!(f, " {p}")?;
    }
    write!(f, ")")
}

// ---------------------------------------------------------------------------
// Sort inference
// ---------------------------------------------------------------------------

/// Mapping from variable names to declared sorts.  Built up by the
/// caller before invoking [`infer_sort`].
pub type SortEnv = HashMap<String, Sort>;

/// Errors that sort inference can surface.
#[allow(missing_docs)] // field documentation lives on each variant.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SortError {
    /// Variable referenced but not present in the SortEnv.
    UnknownVar(String),
    /// Sub-predicates have incompatible sorts (e.g. `Add` of an
    /// `Int` and a `Bool`).  `expected` and `got` describe the
    /// mismatch.
    Mismatch { expected: Sort, got: Sort, op: &'static str },
    /// Sort cannot be inferred (e.g. polymorphic Apply with no
    /// declared signature).  Out of v1 scope.
    Indeterminate(&'static str),
}

impl fmt::Display for SortError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SortError::UnknownVar(name) => write!(f, "unknown variable: {name}"),
            SortError::Mismatch { expected, got, op } => {
                write!(f, "{op}: expected {expected}, got {got}")
            }
            SortError::Indeterminate(reason) => write!(f, "indeterminate sort: {reason}"),
        }
    }
}

impl std::error::Error for SortError {}

/// Infer the sort of `p` against `env`.  Returns the sort or a
/// typed error.
pub fn infer_sort(p: &Predicate, env: &SortEnv) -> Result<Sort, SortError> {
    match p {
        Predicate::Bool(_) => Ok(Sort::Bool),
        Predicate::Int(_) => Ok(Sort::Int),
        Predicate::Real(_) => Ok(Sort::Real),
        Predicate::Var(name) => env
            .get(name)
            .cloned()
            .ok_or_else(|| SortError::UnknownVar(name.clone())),
        // Boolean operators always return Bool; their sub-preds
        // must also be Bool.
        Predicate::And(parts) | Predicate::Or(parts) => {
            for sub in parts {
                let s = infer_sort(sub, env)?;
                expect_sort(&s, &Sort::Bool, "and/or")?;
            }
            Ok(Sort::Bool)
        }
        Predicate::Not(inner) => {
            let s = infer_sort(inner, env)?;
            expect_sort(&s, &Sort::Bool, "not")?;
            Ok(Sort::Bool)
        }
        Predicate::Implies(a, b) | Predicate::Iff(a, b) => {
            let sa = infer_sort(a, env)?;
            let sb = infer_sort(b, env)?;
            expect_sort(&sa, &Sort::Bool, "implies/iff")?;
            expect_sort(&sb, &Sort::Bool, "implies/iff")?;
            Ok(Sort::Bool)
        }
        Predicate::Eq(a, b) | Predicate::NEq(a, b) => {
            let sa = infer_sort(a, env)?;
            let sb = infer_sort(b, env)?;
            if sa != sb {
                return Err(SortError::Mismatch {
                    expected: sa,
                    got: sb,
                    op: "eq/neq",
                });
            }
            Ok(Sort::Bool)
        }
        // Arithmetic: Int + Int → Int; Real + Real → Real; mixed → error
        Predicate::Add(parts) => {
            if parts.is_empty() {
                return Ok(Sort::Int);
            }
            let first = infer_sort(&parts[0], env)?;
            for sub in &parts[1..] {
                let s = infer_sort(sub, env)?;
                if s != first {
                    return Err(SortError::Mismatch {
                        expected: first,
                        got: s,
                        op: "add",
                    });
                }
            }
            Ok(first)
        }
        Predicate::Sub(a, b) => {
            let sa = infer_sort(a, env)?;
            let sb = infer_sort(b, env)?;
            if sa != sb {
                return Err(SortError::Mismatch {
                    expected: sa,
                    got: sb,
                    op: "sub",
                });
            }
            Ok(sa)
        }
        Predicate::Mul { term, .. } => {
            let s = infer_sort(term, env)?;
            // The coef is i128; result is the term's sort.
            Ok(s)
        }
        // Comparisons: take Int/Real, return Bool.
        Predicate::Le(a, b)
        | Predicate::Lt(a, b)
        | Predicate::Ge(a, b)
        | Predicate::Gt(a, b) => {
            let sa = infer_sort(a, env)?;
            let sb = infer_sort(b, env)?;
            if sa != sb {
                return Err(SortError::Mismatch {
                    expected: sa,
                    got: sb,
                    op: "comparison",
                });
            }
            if !matches!(sa, Sort::Int | Sort::Real | Sort::BitVec(_)) {
                return Err(SortError::Mismatch {
                    expected: Sort::Int,
                    got: sa,
                    op: "comparison",
                });
            }
            Ok(Sort::Bool)
        }
        // ITE: condition is Bool; arms must agree.
        Predicate::Ite(c, t, e) => {
            let sc = infer_sort(c, env)?;
            expect_sort(&sc, &Sort::Bool, "ite-cond")?;
            let st = infer_sort(t, env)?;
            let se = infer_sort(e, env)?;
            if st != se {
                return Err(SortError::Mismatch {
                    expected: st,
                    got: se,
                    op: "ite-arms",
                });
            }
            Ok(st)
        }
        // Quantifiers: extend env, infer body, return Bool.
        Predicate::Forall { var, sort, body } | Predicate::Exists { var, sort, body } => {
            let mut new_env = env.clone();
            new_env.insert(var.clone(), sort.clone());
            let s = infer_sort(body, &new_env)?;
            expect_sort(&s, &Sort::Bool, "quantifier-body")?;
            Ok(Sort::Bool)
        }
        // Apply: indeterminate without a declared signature in v1.
        Predicate::Apply { .. } => {
            Err(SortError::Indeterminate("Apply requires a function-signature env (v2)"))
        }
        // Arrays: select returns the value sort; store returns the array sort.
        Predicate::Select { arr, .. } => {
            let s = infer_sort(arr, env)?;
            match s {
                Sort::Array { val, .. } => Ok(*val),
                other => Err(SortError::Mismatch {
                    expected: Sort::Array {
                        idx: Box::new(Sort::Int),
                        val: Box::new(Sort::Int),
                    },
                    got: other,
                    op: "select",
                }),
            }
        }
        Predicate::Store { arr, .. } => {
            let s = infer_sort(arr, env)?;
            match s {
                Sort::Array { .. } => Ok(s),
                other => Err(SortError::Mismatch {
                    expected: Sort::Array {
                        idx: Box::new(Sort::Int),
                        val: Box::new(Sort::Int),
                    },
                    got: other,
                    op: "store",
                }),
            }
        }
    }
}

fn expect_sort(got: &Sort, want: &Sort, op: &'static str) -> Result<(), SortError> {
    if got != want {
        return Err(SortError::Mismatch {
            expected: want.clone(),
            got: got.clone(),
            op,
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn vx() -> Predicate { Predicate::Var("x".into()) }
    fn vy() -> Predicate { Predicate::Var("y".into()) }
    fn lit(n: i128) -> Predicate { Predicate::Int(n) }

    // ---------- Sort + Logic + Theory + Display ----------

    #[test]
    fn sort_display() {
        assert_eq!(format!("{}", Sort::Bool), "Bool");
        assert_eq!(format!("{}", Sort::Int), "Int");
        assert_eq!(format!("{}", Sort::Real), "Real");
        assert_eq!(format!("{}", Sort::BitVec(8)), "(_ BitVec 8)");
        assert_eq!(
            format!("{}", Sort::Array { idx: Box::new(Sort::Int), val: Box::new(Sort::Bool) }),
            "(Array Int Bool)",
        );
        assert_eq!(format!("{}", Sort::Uninterpreted("MyType".into())), "MyType");
    }

    #[test]
    fn logic_display() {
        for (l, expected) in [
            (Logic::QF_Bool, "QF_Bool"),
            (Logic::QF_LIA, "QF_LIA"),
            (Logic::QF_LRA, "QF_LRA"),
            (Logic::QF_BV, "QF_BV"),
            (Logic::QF_AUFLIA, "QF_AUFLIA"),
            (Logic::LIA, "LIA"),
            (Logic::ALL, "ALL"),
        ] {
            assert_eq!(format!("{l}"), expected);
        }
    }

    #[test]
    fn theory_set_membership() {
        // Just smoke-test: each variant is constructible and Hash + Eq work.
        use std::collections::HashSet;
        let mut s: HashSet<Theory> = HashSet::new();
        for t in [Theory::Bool, Theory::LIA, Theory::LRA, Theory::Arrays,
                  Theory::BitVectors, Theory::EUF, Theory::Strings,
                  Theory::NRA, Theory::FP] {
            s.insert(t);
        }
        assert_eq!(s.len(), 9);
    }

    // ---------- Rational ----------

    #[test]
    fn rational_reduces_via_gcd() {
        let r = Rational::new(4, 6);
        assert_eq!(r, Rational { num: 2, den: 3 });
    }

    #[test]
    fn rational_normalises_negative_denominator() {
        let r = Rational::new(1, -2);
        assert_eq!(r, Rational { num: -1, den: 2 });
    }

    #[test]
    fn rational_zero_numerator() {
        let r = Rational::new(0, 5);
        // 0/5 reduces to 0/1.
        assert_eq!(r, Rational { num: 0, den: 1 });
    }

    #[test]
    #[should_panic(expected = "division by zero")]
    fn rational_panics_on_zero_denominator() {
        let _ = Rational::new(1, 0);
    }

    #[test]
    #[should_panic(expected = "numerator must not be i128::MIN")]
    fn rational_panics_on_min_numerator() {
        // i128::MIN can't be safely negated; reject explicitly.
        let _ = Rational::new(i128::MIN, 1);
    }

    #[test]
    #[should_panic(expected = "denominator must not be i128::MIN")]
    fn rational_panics_on_min_denominator() {
        let _ = Rational::new(1, i128::MIN);
    }

    #[test]
    fn rational_handles_extreme_but_safe_values() {
        // i128::MAX and i128::MIN+1 are both safe (their magnitudes fit).
        let r = Rational::new(i128::MAX, 1);
        assert_eq!(r, Rational { num: i128::MAX, den: 1 });
        let r = Rational::new(i128::MIN + 1, 1);
        assert_eq!(r, Rational { num: i128::MIN + 1, den: 1 });
        // Negative denominator near the boundary.
        let r = Rational::new(1, -(i128::MAX));
        assert_eq!(r, Rational { num: -1, den: i128::MAX });
    }

    #[test]
    fn rational_display() {
        assert_eq!(format!("{}", Rational::new(3, 1)), "3");
        assert_eq!(format!("{}", Rational::new(1, 2)), "1/2");
        assert_eq!(format!("{}", Rational::from_int(7)), "7");
    }

    // ---------- Smart constructors ----------

    #[test]
    fn and_empty_is_true() {
        assert_eq!(Predicate::and(vec![]), Predicate::Bool(true));
    }

    #[test]
    fn and_single_unwraps() {
        assert_eq!(Predicate::and(vec![vx()]), vx());
    }

    #[test]
    fn and_flattens_nested_ands() {
        let inner = Predicate::And(vec![vx(), vy()]);
        let outer = Predicate::and(vec![inner, lit(0)]);
        match outer {
            Predicate::And(parts) => assert_eq!(parts, vec![vx(), vy(), lit(0)]),
            _ => panic!("expected And"),
        }
    }

    #[test]
    fn and_drops_true_operands_and_short_circuits_false() {
        assert_eq!(Predicate::and(vec![Predicate::Bool(true), vx()]), vx());
        assert_eq!(
            Predicate::and(vec![Predicate::Bool(false), vx()]),
            Predicate::Bool(false),
        );
    }

    #[test]
    fn or_empty_is_false() {
        assert_eq!(Predicate::or(vec![]), Predicate::Bool(false));
    }

    #[test]
    fn or_drops_false_operands_and_short_circuits_true() {
        assert_eq!(Predicate::or(vec![Predicate::Bool(false), vx()]), vx());
        assert_eq!(
            Predicate::or(vec![Predicate::Bool(true), vx()]),
            Predicate::Bool(true),
        );
    }

    #[test]
    fn not_double_negation_eliminates() {
        let p = Predicate::not(Predicate::not(vx()));
        assert_eq!(p, vx());
    }

    #[test]
    fn not_constant_folds() {
        assert_eq!(Predicate::not(Predicate::Bool(true)), Predicate::Bool(false));
        assert_eq!(Predicate::not(Predicate::Bool(false)), Predicate::Bool(true));
    }

    // ---------- Display ----------

    #[test]
    fn display_simple_predicates() {
        assert_eq!(format!("{}", lit(42)), "42");
        assert_eq!(format!("{}", vx()), "x");
        assert_eq!(format!("{}", Predicate::Bool(true)), "true");
        assert_eq!(format!("{}", Predicate::Bool(false)), "false");
        let p = Predicate::and(vec![
            Predicate::Ge(Box::new(vx()), Box::new(lit(1))),
            Predicate::Le(Box::new(vx()), Box::new(lit(100))),
        ]);
        assert_eq!(format!("{p}"), "(and (>= x 1) (<= x 100))");
    }

    #[test]
    fn display_quantifiers() {
        let p = Predicate::Forall {
            var: "x".into(),
            sort: Sort::Int,
            body: Box::new(Predicate::Ge(Box::new(vx()), Box::new(lit(0)))),
        };
        assert_eq!(format!("{p}"), "(forall ((x Int)) (>= x 0))");
    }

    // ---------- NNF ----------

    #[test]
    fn nnf_pushes_not_through_and() {
        let p = Predicate::not(Predicate::and(vec![vx(), vy()]));
        let nnf = p.to_nnf();
        // Expected: (or (not x) (not y))
        match nnf {
            Predicate::Or(parts) => {
                assert_eq!(parts.len(), 2);
                assert_eq!(parts[0], Predicate::not(vx()));
                assert_eq!(parts[1], Predicate::not(vy()));
            }
            _ => panic!("expected Or after NNF, got {nnf:?}"),
        }
    }

    #[test]
    fn nnf_pushes_not_through_or() {
        let p = Predicate::not(Predicate::or(vec![vx(), vy()]));
        let nnf = p.to_nnf();
        match nnf {
            Predicate::And(parts) => assert_eq!(parts.len(), 2),
            _ => panic!("expected And after NNF, got {nnf:?}"),
        }
    }

    #[test]
    fn nnf_eliminates_double_negation() {
        let p = Predicate::not(Predicate::not(vx()));
        assert_eq!(p.to_nnf(), vx());
    }

    #[test]
    fn nnf_negates_atomic_comparisons() {
        // ¬(x < y) ≡ x ≥ y
        let lt = Predicate::Lt(Box::new(vx()), Box::new(vy()));
        let nnf = Predicate::Not(Box::new(lt)).to_nnf();
        assert_eq!(nnf, Predicate::Ge(Box::new(vx()), Box::new(vy())));
    }

    #[test]
    fn nnf_translates_implies() {
        // (=> x y) ≡ (or (not x) y)
        let p = Predicate::Implies(Box::new(vx()), Box::new(vy()));
        let nnf = p.to_nnf();
        match nnf {
            Predicate::Or(parts) => {
                assert_eq!(parts.len(), 2);
                assert_eq!(parts[0], Predicate::not(vx()));
                assert_eq!(parts[1], vy());
            }
            _ => panic!("expected Or after Implies NNF, got {nnf:?}"),
        }
    }

    // ---------- CNF ----------

    #[test]
    fn cnf_distributes_or_over_and() {
        // (or (and a b) c) → (and (or a c) (or b c))
        let a = Predicate::Var("a".into());
        let b = Predicate::Var("b".into());
        let c = Predicate::Var("c".into());
        let p = Predicate::or(vec![Predicate::and(vec![a.clone(), b.clone()]), c.clone()]);
        let cnf = p.to_cnf();
        match cnf {
            Predicate::And(clauses) => {
                assert_eq!(clauses.len(), 2);
                assert!(clauses.contains(&Predicate::or(vec![a.clone(), c.clone()])));
                assert!(clauses.contains(&Predicate::or(vec![b, c])));
            }
            _ => panic!("expected And-of-Ors after CNF, got {cnf:?}"),
        }
    }

    #[test]
    fn cnf_idempotent_on_already_cnf_input() {
        let p = Predicate::and(vec![
            Predicate::or(vec![vx(), vy()]),
            Predicate::not(vy()),
        ]);
        let cnf = p.clone().to_cnf();
        // Must remain shape-equivalent (And-of-Ors-or-atoms).
        match cnf {
            Predicate::And(parts) => {
                for part in parts {
                    assert!(
                        matches!(part, Predicate::Or(_) | Predicate::Var(_) | Predicate::Not(_)),
                        "CNF clause should be Or/atom/literal, got {part:?}",
                    );
                }
            }
            _ => panic!("expected And"),
        }
    }

    // ---------- Simplify ----------

    #[test]
    fn simplify_dedups_and_operands() {
        let p = Predicate::And(vec![vx(), vx(), vy(), vx()]);
        let s = p.simplify();
        match s {
            Predicate::And(parts) => assert_eq!(parts.len(), 2),
            other => panic!("expected dedup'd And, got {other:?}"),
        }
    }

    #[test]
    fn simplify_collapses_ite_with_constant_condition() {
        let p = Predicate::Ite(
            Box::new(Predicate::Bool(true)),
            Box::new(vx()),
            Box::new(vy()),
        );
        assert_eq!(p.simplify(), vx());
        let p = Predicate::Ite(
            Box::new(Predicate::Bool(false)),
            Box::new(vx()),
            Box::new(vy()),
        );
        assert_eq!(p.simplify(), vy());
    }

    // ---------- free_vars ----------

    #[test]
    fn free_vars_empty_for_constants() {
        assert!(lit(7).free_vars().is_empty());
        assert!(Predicate::Bool(true).free_vars().is_empty());
    }

    #[test]
    fn free_vars_collects_from_atoms() {
        let p = Predicate::Add(vec![vx(), vy()]);
        let fv: Vec<String> = p.free_vars().into_iter().collect();
        assert_eq!(fv, vec!["x".to_string(), "y".to_string()]);
    }

    #[test]
    fn free_vars_excludes_quantifier_bound() {
        // (forall ((x Int)) (>= x y))  →  free vars: {y} only
        let body = Predicate::Ge(Box::new(vx()), Box::new(vy()));
        let p = Predicate::Forall {
            var: "x".into(),
            sort: Sort::Int,
            body: Box::new(body),
        };
        let fv: Vec<String> = p.free_vars().into_iter().collect();
        assert_eq!(fv, vec!["y".to_string()]);
    }

    // ---------- Sort inference ----------

    #[test]
    fn infer_int_literal() {
        let env = SortEnv::new();
        assert_eq!(infer_sort(&lit(42), &env).unwrap(), Sort::Int);
    }

    #[test]
    fn infer_var_from_env() {
        let mut env = SortEnv::new();
        env.insert("x".into(), Sort::Int);
        assert_eq!(infer_sort(&vx(), &env).unwrap(), Sort::Int);
    }

    #[test]
    fn infer_var_unknown_errors() {
        let env = SortEnv::new();
        let err = infer_sort(&vx(), &env).unwrap_err();
        assert!(matches!(err, SortError::UnknownVar(name) if name == "x"));
    }

    #[test]
    fn infer_and_of_bools_is_bool() {
        let mut env = SortEnv::new();
        env.insert("a".into(), Sort::Bool);
        env.insert("b".into(), Sort::Bool);
        let p = Predicate::and(vec![Predicate::Var("a".into()), Predicate::Var("b".into())]);
        assert_eq!(infer_sort(&p, &env).unwrap(), Sort::Bool);
    }

    #[test]
    fn infer_and_of_int_and_bool_errors() {
        let mut env = SortEnv::new();
        env.insert("x".into(), Sort::Int);
        let p = Predicate::And(vec![vx(), Predicate::Bool(true)]);
        let err = infer_sort(&p, &env).unwrap_err();
        assert!(matches!(err, SortError::Mismatch { .. }));
    }

    #[test]
    fn infer_comparison_returns_bool() {
        let mut env = SortEnv::new();
        env.insert("x".into(), Sort::Int);
        let p = Predicate::Lt(Box::new(vx()), Box::new(lit(5)));
        assert_eq!(infer_sort(&p, &env).unwrap(), Sort::Bool);
    }

    #[test]
    fn infer_eq_requires_matching_sorts() {
        let env = SortEnv::new();
        let p = Predicate::Eq(Box::new(lit(0)), Box::new(Predicate::Bool(false)));
        let err = infer_sort(&p, &env).unwrap_err();
        assert!(matches!(err, SortError::Mismatch { op: "eq/neq", .. }));
    }

    #[test]
    fn infer_quantifier_extends_env_for_body() {
        let env = SortEnv::new();
        let p = Predicate::Forall {
            var: "x".into(),
            sort: Sort::Int,
            body: Box::new(Predicate::Ge(Box::new(vx()), Box::new(lit(0)))),
        };
        assert_eq!(infer_sort(&p, &env).unwrap(), Sort::Bool);
    }

    #[test]
    fn infer_select_returns_value_sort() {
        let mut env = SortEnv::new();
        env.insert(
            "arr".into(),
            Sort::Array { idx: Box::new(Sort::Int), val: Box::new(Sort::Bool) },
        );
        env.insert("i".into(), Sort::Int);
        let p = Predicate::Select {
            arr: Box::new(Predicate::Var("arr".into())),
            idx: Box::new(Predicate::Var("i".into())),
        };
        assert_eq!(infer_sort(&p, &env).unwrap(), Sort::Bool);
    }
}
