//! # Computation with provenance-through-math (ADJ language expansion, step 3a).
//!
//! Adjudication is full of arithmetic: *sum these line items; compute the
//! CSF:serum ratio; prorate the bonus.* If the **model** does that math, the
//! answer is un-auditable and wrong-by-arithmetic. The fix is the standing
//! principle of this framework: **the model only extracts typed values and
//! writes the formula; the CPU engine computes**, and every derived value
//! carries a **derivation tree** back to the source facts. A reviewer audits
//! the tree; the model is never in the arithmetic loop.
//!
//! This module is the engine half (no surface syntax yet — that is step 3b in
//! [`code/specs/data/adj-language-expansion/STEP3-let-arithmetic-PLAN.md`]). It
//! provides:
//!
//! - [`ComputeExpr`] — the formula IR the lowerer will build (`a / b`,
//!   `sum(line_item)`, …). Deliberately tiny and `Term`-native: we evaluate
//!   over [`logic_core::Term`] magnitudes via [`crate::numeric_magnitude`], so
//!   a typed value `quantity(40, mg_dl)` participates directly. (We do **not**
//!   bridge to symbolic-vm: it offers no derivation-capture channel, so the
//!   tree would have to be hand-built either way — see the step-3 plan.)
//! - [`compute`] — the deterministic evaluator. It returns a [`Derived`]: the
//!   numeric `value` **plus** the [`DerivationNode`] tree recording every
//!   operation and citing each leaf's [`FactId`].
//!
//! A `Derived` is then bound into the [`KnowledgeBase`](crate::KnowledgeBase)
//! by name; [`observed_value`](crate::KnowledgeBase::observed_value) falls back
//! to the derived table, so a predicate-gated contribution
//! (`from csf_ratio <= 0.4 to bacterial`) fires over a **computed** value
//! exactly as it would over an observed one — one engine, no new verdict logic.
//!
//! ## Worked example (what the tree looks like)
//!
//! ```text
//! observe csf_glucose = quantity(40, mg_dl)     % FactId(3)
//! observe serum_glucose = quantity(100, mg_dl)  % FactId(4)
//! let csf_ratio = csf_glucose / serum_glucose
//!
//!   Derived { name: "csf_ratio", value: 0.4, tree:
//!     Op { op: Div, result: 0.4, operands: [
//!       Leaf { slot: "csf_glucose",   value: 40.0,  fact_id: FactId(3) },
//!       Leaf { slot: "serum_glucose", value: 100.0, fact_id: FactId(4) },
//!     ] } }
//! ```
//!
//! Every number in the answer (0.4) is reconstructable from the tree without
//! the model: 40 / 100, each operand cited to the byte-grounded fact that
//! produced it.

use crate::dimension::{DimOp, Dimension};
use crate::{FactId, KnowledgeBase};

/// An exact rational sidecar for CPU arithmetic whose operands are exact
/// integers/rationals. The public engine still exposes `f64` magnitudes for
/// compatibility, but equality-sensitive consumers can use this when present.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExactRational {
    pub num: i128,
    pub den: i128,
}

impl ExactRational {
    pub fn new(num: i128, den: i128) -> Option<Self> {
        if den == 0 || num == i128::MIN || den == i128::MIN {
            return None;
        }
        let (mut n, mut d) = (num, den);
        if d < 0 {
            n = n.checked_neg()?;
            d = d.checked_neg()?;
        }
        let g = gcd_i128(n.unsigned_abs(), d.unsigned_abs()) as i128;
        Some(Self {
            num: n / g,
            den: d / g,
        })
    }

    pub fn from_i128(n: i128) -> Self {
        Self { num: n, den: 1 }
    }

    pub fn from_integer_f64(value: f64) -> Option<Self> {
        if value.is_finite()
            && value.fract() == 0.0
            && value >= i64::MIN as f64
            && value <= i64::MAX as f64
        {
            Some(Self::from_i128(value as i64 as i128))
        } else {
            None
        }
    }

    pub fn add(self, rhs: Self) -> Option<Self> {
        let left = self.num.checked_mul(rhs.den)?;
        let right = rhs.num.checked_mul(self.den)?;
        let num = left.checked_add(right)?;
        let den = self.den.checked_mul(rhs.den)?;
        Self::new(num, den)
    }

    pub fn sub(self, rhs: Self) -> Option<Self> {
        let left = self.num.checked_mul(rhs.den)?;
        let right = rhs.num.checked_mul(self.den)?;
        let num = left.checked_sub(right)?;
        let den = self.den.checked_mul(rhs.den)?;
        Self::new(num, den)
    }

    pub fn mul(self, rhs: Self) -> Option<Self> {
        Self::new(
            self.num.checked_mul(rhs.num)?,
            self.den.checked_mul(rhs.den)?,
        )
    }

    pub fn div(self, rhs: Self) -> Option<Self> {
        if rhs.num == 0 {
            return None;
        }
        Self::new(
            self.num.checked_mul(rhs.den)?,
            self.den.checked_mul(rhs.num)?,
        )
    }

    pub fn to_f64(self) -> f64 {
        self.num as f64 / self.den as f64
    }

    /// Raise to a **non-negative integer** power, exactly, by repeated
    /// multiplication (`x^0 = 1`). This keeps the exact sidecar precise for the
    /// common `x^n` case — a rational raised to a whole power is itself rational,
    /// so `(3/2)^2 = 9/4` stays exact rather than collapsing to the `f64` 2.25.
    ///
    /// Returns `None` when the exponent is negative (a reciprocal power — the
    /// caller keeps the `f64` result instead), when the exponent exceeds
    /// [`MAX_EXACT_POW`] (a guard so a pathological exponent can't spin the loop
    /// for an unbounded time — the `f64` result still stands), or on `i128`
    /// overflow. `None` is never *wrong*: it only means "no exact sidecar here".
    pub fn powi(self, exp: i128) -> Option<Self> {
        if !(0..=MAX_EXACT_POW).contains(&exp) {
            return None;
        }
        let mut acc = Self::from_i128(1);
        for _ in 0..exp {
            acc = acc.mul(self)?;
        }
        Some(acc)
    }
}

/// The largest exponent for which the exact-rational sidecar is computed by
/// repeated multiplication (see [`ExactRational::powi`]). Beyond this the `f64`
/// magnitude is authoritative; the cap bounds the loop so an adversarial program
/// (`base^{10^18}`) cannot make the engine spin — an algorithmic-DoS guard.
const MAX_EXACT_POW: i128 = 1024;

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

/// A computation operator. Binary ops (`Add`/`Sub`/`Mul`/`Div`/`Pow`) take two
/// operands; aggregation ops (`Sum`/`Count`/`Min`/`Max`/`Avg`) reduce a list
/// of same-slot observations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComputeOp {
    Add,
    Sub,
    Mul,
    Div,
    /// Exponentiation, `base ^ exponent`. Unlike the other binary ops it is
    /// **not** a symmetric dimensional combine: the exponent must be
    /// dimensionless (`Scalar`) and the result dimension is the *base* raised to
    /// the exponent (`x^0 = scalar`, `x^2 = x·x`), so it is evaluated on its own
    /// path rather than through [`dim_op`] + [`Dimension::combine`]. This is what
    /// makes a LaTeX `x^n` (adj-lang's `latex "…"` surface) computable as a
    /// single native node instead of an expanded `x*x*…*x` chain.
    Pow,
    /// Absolute value, `|x|`. Unlike the other operators it is **unary** (one
    /// operand) and **dimension-preserving**: `|dollars|` is still dollars (a
    /// magnitude flips sign, its unit does not), so it neither combines two
    /// dimensions like the additive/multiplicative ops nor collapses to `Scalar`.
    /// It is carried in a [`ComputeExpr::Unary`] and is what makes a LaTeX
    /// `|x|`/`\left|x\right|` (adj-lang's `latex "…"` surface) computable instead
    /// of being silently dropped to a bare `x`.
    Abs,
    /// Floor, `⌊x⌋` — the greatest integer ≤ x. Like [`ComputeOp::Abs`] it is
    /// **unary** and **dimension-preserving** (⌊3.7 mmol⌋ = 3 mmol: the magnitude
    /// snaps down to an integer, the unit is untouched). It is carried in a
    /// [`ComputeExpr::Unary`] and makes a LaTeX `\left\lfloor x\right\rfloor`
    /// (adj-lang's `latex "…"` surface) computable as a single native node.
    Floor,
    /// Ceiling, `⌈x⌉` — the least integer ≥ x. The exact mirror of
    /// [`ComputeOp::Floor`]: unary, dimension-preserving (⌈3.2 mmol⌉ = 4 mmol),
    /// carried in a [`ComputeExpr::Unary`], lowered from a LaTeX
    /// `\left\lceil x\right\rceil`.
    Ceil,
    /// Round to the nearest integer, `⌊x⌉`, with **ties away from zero**
    /// (matching Rust's `f64::round`: `⌊2.5⌉ = 3`, `⌊−2.5⌉ = −3`). Like
    /// [`ComputeOp::Floor`]/[`ComputeOp::Ceil`] it is unary and
    /// dimension-preserving (`⌊3.6 mmol⌉ = 4 mmol`), carried in a
    /// [`ComputeExpr::Unary`], and lowered from the standard nearest-integer
    /// LaTeX fence `\left\lfloor x\right\rceil` (floor-left, ceil-right).
    Round,
    Sum,
    Count,
    Min,
    Max,
    Avg,
}

impl ComputeOp {
    /// A short symbol/name for audit rendering.
    pub fn symbol(&self) -> &'static str {
        match self {
            ComputeOp::Add => "+",
            ComputeOp::Sub => "-",
            ComputeOp::Mul => "*",
            ComputeOp::Div => "/",
            ComputeOp::Pow => "^",
            ComputeOp::Abs => "abs",
            ComputeOp::Floor => "floor",
            ComputeOp::Ceil => "ceil",
            ComputeOp::Round => "round",
            ComputeOp::Sum => "sum",
            ComputeOp::Count => "count",
            ComputeOp::Min => "min",
            ComputeOp::Max => "max",
            ComputeOp::Avg => "avg",
        }
    }
}

/// The formula IR — what `let <name> = <expr>` lowers to. Tiny on purpose;
/// step 3b's adapter builds it from the surface grammar.
#[derive(Debug, Clone, PartialEq)]
pub enum ComputeExpr {
    /// A reference to a slot — resolves to an observed valued fact `slot(V)`
    /// (a [`DerivationNode::Leaf`]) or, failing that, to a previously-bound
    /// derived value (a [`DerivationNode::DerivedRef`]).
    Ref(String),
    /// A numeric literal in the formula. The **no-magic-numbers** gate (step
    /// 3d) will require each of these to be a declared structural constant.
    Lit(f64),
    /// A binary operation: `Add`/`Sub`/`Mul`/`Div`/`Pow` only.
    Bin(ComputeOp, Box<ComputeExpr>, Box<ComputeExpr>),
    /// A unary operation: `Abs`/`Floor`/`Ceil`/`Round`. Kept distinct from
    /// [`ComputeExpr::Bin`] so the arity is honest (a unary op has one operand,
    /// not two) — it lowers to a [`DerivationNode::Op`] with a single-element
    /// `operands` vec.
    Unary(ComputeOp, Box<ComputeExpr>),
    /// An aggregation over **every** observation of a slot:
    /// `Sum`/`Count`/`Min`/`Max`/`Avg`.
    Agg(ComputeOp, String),
}

/// A node in the derivation tree — the provenance-through-math record.
#[derive(Debug, Clone, PartialEq)]
pub enum DerivationNode {
    /// A leaf grounded in an observed fact: the magnitude `value` came from
    /// the valued fact `slot(...)` identified by `fact_id`. The audit descends
    /// from here into that fact's [`Provenance`](crate::Provenance) → bytes.
    Leaf {
        slot: String,
        value: f64,
        fact_id: FactId,
    },
    /// A reference to another derived value (a `let` over a `let`). Its own
    /// tree lives in the KB's derived table, reachable by `name`.
    DerivedRef { name: String, value: f64 },
    /// A literal constant written into the formula.
    Lit { value: f64 },
    /// An operation applied to its operands, with the computed `result`.
    Op {
        op: ComputeOp,
        operands: Vec<DerivationNode>,
        result: f64,
    },
}

impl DerivationNode {
    /// The numeric value this node evaluates to.
    pub fn value(&self) -> f64 {
        match self {
            DerivationNode::Leaf { value, .. } => *value,
            DerivationNode::DerivedRef { value, .. } => *value,
            DerivationNode::Lit { value } => *value,
            DerivationNode::Op { result, .. } => *result,
        }
    }
}

/// A computed value bound to a name, with its full derivation tree and the
/// [`Dimension`] the engine inferred for it (so a predicate firing over a
/// derived value — `csf_ratio <= 0.4` — knows `csf_ratio` is a dimensionless
/// `Scalar`, and the faithfulness gate has rejected any unit-mismatched op).
#[derive(Debug, Clone, PartialEq)]
pub struct Derived {
    pub name: String,
    pub value: f64,
    /// Exact value when the expression stayed inside integer/rational arithmetic.
    pub exact: Option<ExactRational>,
    pub dim: Dimension,
    pub tree: DerivationNode,
}

/// Why a computation could not be carried out. These are clean errors — the
/// engine never panics on a malformed formula; the caller renders the
/// diagnostic (the CLI as `{"error": ...}`).
#[derive(Debug, Clone, PartialEq)]
pub enum ComputeError {
    /// A `Ref(slot)` matched neither an observed fact nor a derived value.
    UnknownSlot { slot: String },
    /// An aggregation (`sum`/`min`/`max`/`avg`) found no observations of the
    /// slot. (`count` of zero is fine — it returns 0.)
    EmptyAggregation { slot: String },
    /// Division by zero.
    DivisionByZero,
    /// An aggregation operator was used in a binary position or vice versa.
    /// Should not occur if [`ComputeExpr`] is built correctly, but guarded so
    /// a hand-built expression can't panic.
    MalformedExpr { detail: &'static str },
    /// The expression nests deeper than [`MAX_EVAL_DEPTH`]. Bounds the
    /// recursion so a pathologically deep formula returns a clean error
    /// instead of overflowing the stack (an unrecoverable abort). A real
    /// adjudication formula is a handful of levels deep; this limit is a
    /// safety backstop, not a modelling constraint.
    TooDeep { limit: usize },
    /// An operation produced a non-finite result (`NaN` or `±∞`) — e.g.
    /// overflow, or `∞ − ∞`. We reject it rather than let it flow into a
    /// verdict: a `NaN` compares `false` against every threshold, so it would
    /// silently make a predicate not fire (a quiet wrong answer). The whole
    /// point of provenance-through-math is that no number is silently wrong.
    NonFinite { op: ComputeOp },
    /// A binary operation mixed incompatible dimensions — `usd + days`,
    /// `usd + eur` without a conversion. The faithfulness gate (track A4): the
    /// engine, not the model, decides this is a category error. Carries the two
    /// dimension tags so the audit reader sees exactly which units clashed.
    DimensionMismatch {
        op: ComputeOp,
        lhs: String,
        rhs: String,
    },
}

/// Maximum nesting depth for a computation expression. A genuine adjudication
/// formula is only a few levels deep; this is a backstop against an
/// adversarially deep formula (once step 3b feeds parsed input to [`eval`])
/// blowing the call stack.
pub const MAX_EVAL_DEPTH: usize = 256;

/// Evaluate `expr` against `kb`, binding the result to `name`. Pure and
/// deterministic: the same `(name, expr, kb)` always yields the same
/// [`Derived`]. Every numeric result is reconstructable from the returned
/// tree without consulting the model.
pub fn compute(
    name: impl Into<String>,
    expr: &ComputeExpr,
    kb: &KnowledgeBase,
) -> Result<Derived, ComputeError> {
    let (tree, dim, exact) = eval(expr, kb, 0)?;
    let value = tree.value();
    Ok(Derived {
        name: name.into(),
        value,
        exact,
        dim,
        tree,
    })
}

/// Map a binary [`ComputeOp`] to the dimensional [`DimOp`]. Aggregation
/// operators have no binary dimensional rule (their result dimension is handled
/// in the `Agg` arm), so they return `None`.
fn dim_op(op: ComputeOp) -> Option<DimOp> {
    match op {
        ComputeOp::Add => Some(DimOp::Add),
        ComputeOp::Sub => Some(DimOp::Sub),
        ComputeOp::Mul => Some(DimOp::Mul),
        ComputeOp::Div => Some(DimOp::Div),
        _ => None,
    }
}

/// Recursively evaluate a sub-expression into a derivation node **and its
/// dimension**. `depth` bounds the recursion at [`MAX_EVAL_DEPTH`]. The
/// dimension is checked at each binary op via [`Dimension::combine`], so a
/// unit-mismatched formula (`usd + days`) is a clean
/// [`ComputeError::DimensionMismatch`], not a silently-wrong number.
fn eval(
    expr: &ComputeExpr,
    kb: &KnowledgeBase,
    depth: usize,
) -> Result<(DerivationNode, Dimension, Option<ExactRational>), ComputeError> {
    if depth >= MAX_EVAL_DEPTH {
        return Err(ComputeError::TooDeep {
            limit: MAX_EVAL_DEPTH,
        });
    }
    match expr {
        // A literal is dimensionless (Scalar). The no-magic-numbers gate (3d)
        // will check it's a declared constant; dimensionally it's the identity.
        ComputeExpr::Lit(x) => Ok((
            DerivationNode::Lit { value: *x },
            Dimension::Scalar,
            ExactRational::from_integer_f64(*x),
        )),

        ComputeExpr::Ref(slot) => {
            // Observed fact first (carries a FactId for byte provenance + its
            // dimension); then a previously-bound derived value (with its dim).
            if let Some((d, fact_id)) = kb.observed_dimensioned(slot) {
                let exact = kb.observed_exact_value_with_fact(slot).and_then(|(x, id)| {
                    if id == fact_id {
                        Some(x)
                    } else {
                        None
                    }
                });
                Ok((
                    DerivationNode::Leaf {
                        slot: slot.clone(),
                        value: d.magnitude,
                        fact_id,
                    },
                    d.dim,
                    exact,
                ))
            } else if let Some(derived) = kb.derived_for(slot) {
                Ok((
                    DerivationNode::DerivedRef {
                        name: slot.clone(),
                        value: derived.value,
                    },
                    derived.dim.clone(),
                    derived.exact,
                ))
            } else {
                Err(ComputeError::UnknownSlot { slot: slot.clone() })
            }
        }

        ComputeExpr::Bin(op, a, b) => {
            let (lhs, dim_l, exact_l) = eval(a, kb, depth + 1)?;
            let (rhs, dim_r, exact_r) = eval(b, kb, depth + 1)?;
            // Power is special: not a symmetric combine. The exponent must be
            // dimensionless and the result dimension is `base ^ exponent`
            // (`x^0 = scalar`, `x^2 = x·x`), so it bypasses the `dim_op` +
            // `Dimension::combine` path the additive/multiplicative ops share.
            if *op == ComputeOp::Pow {
                // An exponent with a dimension (`x ^ money(…)`) is a category
                // error — you cannot raise to a "3 dollars" power.
                if !dim_r.is_scalar() {
                    return Err(ComputeError::DimensionMismatch {
                        op: *op,
                        lhs: dim_l.tag(),
                        rhs: dim_r.tag(),
                    });
                }
                let (base, exponent) = (lhs.value(), rhs.value());
                // Guard the *inputs* before `powf`, not just the result: `powf`
                // special-cases `1.0.powf(NaN) == 1.0` and `1.0.powf(inf) == 1.0`,
                // so a non-finite exponent (a `Lit(NaN)` in an LLM-emitted IR) with
                // a unit base would otherwise launder into a clean `1.0`, violating
                // the "no silently-wrong number" contract. (`base` is already
                // finite-checked upstream; re-checking is cheap defense-in-depth.)
                if !base.is_finite() || !exponent.is_finite() {
                    return Err(ComputeError::NonFinite { op: *op });
                }
                let result_dim = dim_l.pow(exponent).map_err(|e| match e {
                    crate::DimError::Mismatch { lhs, rhs, .. } => {
                        ComputeError::DimensionMismatch { op: *op, lhs, rhs }
                    }
                })?;
                let result = base.powf(exponent);
                if !result.is_finite() {
                    return Err(ComputeError::NonFinite { op: *op });
                }
                // Exact sidecar only for a non-negative integer exponent of an
                // exact base — `(3/2)^2 = 9/4` stays exact; anything else keeps
                // just the `f64` result.
                let exact = match (exact_l, exact_r) {
                    (Some(a), Some(b)) if b.den == 1 => a.powi(b.num),
                    _ => None,
                };
                return Ok((
                    DerivationNode::Op {
                        op: *op,
                        operands: vec![lhs, rhs],
                        result,
                    },
                    result_dim,
                    exact,
                ));
            }
            // Dimensional check FIRST: usd + days is a category error regardless
            // of the magnitudes.
            let dimop = dim_op(*op).ok_or(ComputeError::MalformedExpr {
                detail: "aggregation operator in binary position",
            })?;
            let result_dim = Dimension::combine(dimop, &dim_l, &dim_r).map_err(|e| match e {
                crate::DimError::Mismatch { lhs, rhs, .. } => {
                    ComputeError::DimensionMismatch { op: *op, lhs, rhs }
                }
            })?;
            let (x, y) = (lhs.value(), rhs.value());
            let result = match op {
                ComputeOp::Add => x + y,
                ComputeOp::Sub => x - y,
                ComputeOp::Mul => x * y,
                ComputeOp::Div => {
                    if y == 0.0 {
                        return Err(ComputeError::DivisionByZero);
                    }
                    x / y
                }
                _ => unreachable!("dim_op already rejected non-binary ops"),
            };
            if !result.is_finite() {
                return Err(ComputeError::NonFinite { op: *op });
            }
            let exact = match (exact_l, exact_r) {
                (Some(a), Some(b)) => match op {
                    ComputeOp::Add => a.add(b),
                    ComputeOp::Sub => a.sub(b),
                    ComputeOp::Mul => a.mul(b),
                    ComputeOp::Div => a.div(b),
                    _ => None,
                },
                _ => None,
            };
            Ok((
                DerivationNode::Op {
                    op: *op,
                    operands: vec![lhs, rhs],
                    result,
                },
                result_dim,
                exact,
            ))
        }

        // Delegated to an `#[inline(never)]` helper so the unary arm's locals do
        // NOT enlarge `eval`'s own stack frame. `eval` recurses up to
        // `MAX_EVAL_DEPTH` levels deep, so a bigger frame here would multiply
        // across 256 frames and can overflow a small (macOS ~2 MB) test-thread
        // stack before the depth guard trips — keeping the frame lean preserves
        // the "clean `TooDeep`, never a stack overflow" contract.
        ComputeExpr::Unary(op, a) => eval_unary(*op, a, kb, depth),

        ComputeExpr::Agg(op, slot) => {
            let observations = kb.observed_values_all(slot);
            // `count` is defined even when there are no observations (it's 0);
            // every other aggregation over an empty set is an error, not 0/NaN.
            if observations.is_empty() && *op != ComputeOp::Count {
                return Err(ComputeError::EmptyAggregation { slot: slot.clone() });
            }
            let operands: Vec<DerivationNode> = observations
                .iter()
                .map(|(value, fact_id)| DerivationNode::Leaf {
                    slot: slot.clone(),
                    value: *value,
                    fact_id: *fact_id,
                })
                .collect();
            let values: Vec<f64> = operands.iter().map(|n| n.value()).collect();
            let result = match op {
                ComputeOp::Sum => values.iter().sum(),
                ComputeOp::Count => values.len() as f64,
                ComputeOp::Min => values.iter().cloned().fold(f64::INFINITY, f64::min),
                ComputeOp::Max => values.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
                ComputeOp::Avg => values.iter().sum::<f64>() / (values.len() as f64),
                _ => {
                    return Err(ComputeError::MalformedExpr {
                        detail: "binary operator in aggregation position",
                    })
                }
            };
            if !result.is_finite() {
                return Err(ComputeError::NonFinite { op: *op });
            }
            // `count` is a dimensionless tally; sum/min/max/avg keep the slot's
            // dimension (the magnitudes share it). Read it from the slot, or
            // Scalar if the slot has no dimensioned observation.
            let result_dim = if *op == ComputeOp::Count {
                Dimension::Scalar
            } else {
                kb.observed_dimensioned(slot)
                    .map(|(d, _)| d.dim)
                    .unwrap_or(Dimension::Scalar)
            };
            Ok((
                DerivationNode::Op {
                    op: *op,
                    operands,
                    result,
                },
                result_dim,
                None,
            ))
        }
    }
}

/// Evaluate a unary op (`Abs`/`Floor`/`Ceil`) into a derivation node + dimension.
/// Split out of [`eval`] and marked `#[inline(never)]` so its locals live in their
/// own stack frame rather than enlarging every one of `eval`'s up-to-`MAX_EVAL_DEPTH`
/// recursive frames — a fatter `eval` frame multiplied across 256 levels can
/// overflow a small (macOS ~2 MB) thread stack *before* the depth guard trips,
/// which would turn the "clean `TooDeep`, never a stack overflow" guarantee into
/// exactly the abort it promises to prevent.
#[inline(never)]
fn eval_unary(
    op: ComputeOp,
    a: &ComputeExpr,
    kb: &KnowledgeBase,
    depth: usize,
) -> Result<(DerivationNode, Dimension, Option<ExactRational>), ComputeError> {
    let (operand, dim, exact) = eval(a, kb, depth + 1)?;
    // Every unary op here is **dimension-preserving**: the magnitude may change
    // (sign flip for `Abs`, snap to an integer for `Floor`/`Ceil`/`Round`) but the
    // unit does not (`|−3 dollars| = 3 dollars`, `⌊3.7 mmol⌋ = 3 mmol`), so — unlike
    // `Pow`, which recomputes the dimension — the operand's dimension flows
    // straight through.
    let value = operand.value();
    let result = match op {
        ComputeOp::Abs => value.abs(),
        ComputeOp::Floor => value.floor(),
        ComputeOp::Ceil => value.ceil(),
        ComputeOp::Round => value.round(),
        _ => {
            return Err(ComputeError::MalformedExpr {
                detail: "non-unary operator in unary position",
            })
        }
    };
    // None of these can introduce a non-finite value from a finite operand
    // (|±∞|/⌊±∞⌋/⌈±∞⌉/⌊±∞⌉ were already non-finite; NaN stays NaN), and the operand
    // is finite-checked at its own producing op. Guard anyway — cheap
    // defense-in-depth, same contract as the binary ops.
    if !result.is_finite() {
        return Err(ComputeError::NonFinite { op });
    }
    // The exact sidecar stays exact. `ExactRational` keeps `den > 0`, so:
    //   • |num/den| = |num|/den (abs of the numerator);
    //   • ⌊num/den⌋ = num.div_euclid(den) (Euclidean division floors for den > 0);
    //   • ⌈num/den⌉ = that quotient plus one when the division leaves a remainder;
    //   • ⌊num/den⌉ = round to nearest with TIES AWAY FROM ZERO (matching
    //     `f64::round`): truncate toward zero, then bump one step outward when the
    //     fractional part reaches a half (2·|rem| ≥ den). The `den − arem` compare
    //     avoids the overflow a bare `2·arem` could hit; `arem = |rem| < den`.
    // Each result is an integer, carried as `q/1`.
    let exact = exact.and_then(|r| match op {
        ComputeOp::Abs => r.num.checked_abs().and_then(|n| ExactRational::new(n, r.den)),
        ComputeOp::Floor => ExactRational::new(r.num.div_euclid(r.den), 1),
        ComputeOp::Ceil => {
            let q = r.num.div_euclid(r.den);
            let q = if r.num.rem_euclid(r.den) != 0 { q.checked_add(1)? } else { q };
            ExactRational::new(q, 1)
        }
        ComputeOp::Round => {
            let q = r.num / r.den; // truncate toward zero (den > 0 ⇒ no overflow)
            let rem = r.num % r.den; // in (−den, den), sign of the numerator
            let arem = if rem >= 0 { rem } else { -rem }; // |rem| < den ⇒ no overflow
            let bump = if arem >= r.den - arem {
                // fractional part ≥ 1/2 → round away from zero (ties away from zero)
                if r.num >= 0 { 1 } else { -1 }
            } else {
                0
            };
            ExactRational::new(q.checked_add(bump)?, 1)
        }
        _ => None,
    });
    Ok((
        DerivationNode::Op {
            op,
            operands: vec![operand],
            result,
        },
        dim,
        exact,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Fact, KnowledgeBase};
    use logic_core::{atom, compound, int};

    fn kb_with(facts: Vec<crate::Fact>) -> KnowledgeBase {
        let mut kb = KnowledgeBase::new();
        for f in facts {
            kb.add_fact(f);
        }
        kb
    }

    // ---- the dimensional faithfulness gate (track A4) ----

    fn money(slot: &str, amount: i64, ccy: &str) -> crate::Fact {
        crate::Fact::certain(compound(
            slot,
            vec![compound("money", vec![int(amount), atom(ccy)])],
        ))
    }
    fn refexpr(slot: &str) -> ComputeExpr {
        ComputeExpr::Ref(slot.into())
    }
    fn bin(op: ComputeOp, a: ComputeExpr, b: ComputeExpr) -> ComputeExpr {
        ComputeExpr::Bin(op, Box::new(a), Box::new(b))
    }

    #[test]
    fn same_currency_add_is_allowed_and_keeps_the_dimension() {
        let kb = kb_with(vec![money("a", 100, "usd"), money("b", 50, "usd")]);
        let d = compute(
            "total",
            &bin(ComputeOp::Add, refexpr("a"), refexpr("b")),
            &kb,
        )
        .unwrap();
        assert_eq!(d.value, 150.0);
        assert_eq!(d.dim, Dimension::Money("usd".into()));
    }

    #[test]
    fn mixed_currency_add_is_a_dimension_mismatch() {
        let kb = kb_with(vec![money("a", 100, "usd"), money("b", 50, "eur")]);
        let err = compute("x", &bin(ComputeOp::Add, refexpr("a"), refexpr("b")), &kb).unwrap_err();
        assert!(matches!(
            err,
            ComputeError::DimensionMismatch {
                op: ComputeOp::Add,
                ..
            }
        ));
    }

    #[test]
    fn money_plus_days_is_a_category_error() {
        let kb = kb_with(vec![
            money("price", 100, "usd"),
            Fact::certain(compound(
                "age",
                vec![compound("duration", vec![int(5), atom("days")])],
            )),
        ]);
        let err = compute(
            "x",
            &bin(ComputeOp::Add, refexpr("price"), refexpr("age")),
            &kb,
        )
        .unwrap_err();
        assert!(matches!(err, ComputeError::DimensionMismatch { .. }));
    }

    #[test]
    fn money_over_money_is_a_dimensionless_ratio() {
        let kb = kb_with(vec![
            money("debt", 3000, "usd"),
            money("income", 10000, "usd"),
        ]);
        let d = compute(
            "dti",
            &bin(ComputeOp::Div, refexpr("debt"), refexpr("income")),
            &kb,
        )
        .unwrap();
        assert!((d.value - 0.3).abs() < 1e-12);
        assert_eq!(
            d.dim,
            Dimension::Scalar,
            "a ratio of like dimensions is dimensionless"
        );
    }

    #[test]
    fn money_scaled_by_a_scalar_literal_stays_money() {
        let kb = kb_with(vec![money("base", 1000, "usd")]);
        // base * 2 → money(usd). (Mul with a Scalar literal is transparent.)
        let d = compute(
            "scaled",
            &bin(ComputeOp::Mul, refexpr("base"), ComputeExpr::Lit(2.0)),
            &kb,
        )
        .unwrap();
        assert_eq!(d.value, 2000.0);
        assert_eq!(d.dim, Dimension::Money("usd".into()));
    }

    // ---- exponentiation (ComputeOp::Pow) ----

    fn quantity(slot: &str, amount: i64, unit: &str) -> crate::Fact {
        crate::Fact::certain(compound(
            slot,
            vec![compound("quantity", vec![int(amount), atom(unit)])],
        ))
    }

    #[test]
    fn scalar_base_to_an_integer_power_computes_and_stays_scalar() {
        // 3 ^ 4 = 81, dimensionless, and exact (81/1).
        let kb = kb_with(vec![quantity("x", 3, "index")]);
        let d = compute(
            "p",
            &bin(ComputeOp::Pow, refexpr("x"), ComputeExpr::Lit(4.0)),
            &kb,
        )
        .unwrap();
        assert_eq!(d.value, 81.0);
        // `quantity(3, index)` is a Unit dim; cubing a Unit is composite, but here
        // the base's own dimension is a Unit — see the dimensioned test below. For
        // a *scalar* base we use a literal:
        let kb2 = kb_with(vec![]);
        let d2 = compute(
            "p",
            &bin(ComputeOp::Pow, ComputeExpr::Lit(3.0), ComputeExpr::Lit(4.0)),
            &kb2,
        )
        .unwrap();
        assert_eq!(d2.value, 81.0);
        assert_eq!(d2.dim, Dimension::Scalar);
        assert_eq!(d2.exact, Some(ExactRational::new(81, 1).unwrap()));
        // the derivation tree records a single `^` node, not an expanded chain.
        if let DerivationNode::Op { op, operands, .. } = &d2.tree {
            assert_eq!(*op, ComputeOp::Pow);
            assert_eq!(operands.len(), 2);
        } else {
            panic!("expected a Pow op node");
        }
    }

    #[test]
    fn power_zero_is_one_and_dimensionless_even_for_a_dimensioned_base() {
        // (money)^0 = 1, scalar. x^0 discards the base's dimension.
        let kb = kb_with(vec![money("m", 500, "usd")]);
        let d = compute(
            "p",
            &bin(ComputeOp::Pow, refexpr("m"), ComputeExpr::Lit(0.0)),
            &kb,
        )
        .unwrap();
        assert_eq!(d.value, 1.0);
        assert_eq!(d.dim, Dimension::Scalar);
    }

    #[test]
    fn squaring_a_dimensioned_base_composes_its_dimension() {
        // quantity(4, mg_dl) ^ 2 = 16, dim mg_dl·mg_dl — same as mg_dl * mg_dl.
        let kb = kb_with(vec![quantity("c", 4, "mg_dl")]);
        let d = compute(
            "sq",
            &bin(ComputeOp::Pow, refexpr("c"), ComputeExpr::Lit(2.0)),
            &kb,
        )
        .unwrap();
        assert_eq!(d.value, 16.0);
        assert_eq!(d.dim, Dimension::Unit("mg_dl·mg_dl".into()));
    }

    #[test]
    fn power_of_a_dimensioned_base_by_one_keeps_its_dimension() {
        let kb = kb_with(vec![money("m", 7, "usd")]);
        let d = compute(
            "p",
            &bin(ComputeOp::Pow, refexpr("m"), ComputeExpr::Lit(1.0)),
            &kb,
        )
        .unwrap();
        assert_eq!(d.value, 7.0);
        assert_eq!(d.dim, Dimension::Money("usd".into()));
    }

    #[test]
    fn a_dimensioned_exponent_is_a_category_error() {
        // x ^ (money) is meaningless — the exponent must be dimensionless.
        let kb = kb_with(vec![quantity("x", 2, "index"), money("e", 3, "usd")]);
        let err = compute(
            "p",
            &bin(ComputeOp::Pow, refexpr("x"), refexpr("e")),
            &kb,
        )
        .unwrap_err();
        assert!(matches!(
            err,
            ComputeError::DimensionMismatch {
                op: ComputeOp::Pow,
                ..
            }
        ));
    }

    #[test]
    fn a_fractional_power_of_a_dimensioned_base_has_no_dimension() {
        // (money)^0.5 — a square root of dollars — has no representable dim.
        let kb = kb_with(vec![money("m", 4, "usd")]);
        let err = compute(
            "root",
            &bin(ComputeOp::Pow, refexpr("m"), ComputeExpr::Lit(0.5)),
            &kb,
        )
        .unwrap_err();
        assert!(matches!(err, ComputeError::DimensionMismatch { .. }));
    }

    #[test]
    fn a_fractional_power_of_a_scalar_base_is_fine() {
        // 9 ^ 0.5 = 3, scalar (a scalar base is closed under any power). No exact
        // sidecar (the exponent isn't a whole number), but the f64 is correct.
        let d = compute(
            "root",
            &bin(ComputeOp::Pow, ComputeExpr::Lit(9.0), ComputeExpr::Lit(0.5)),
            &kb_with(vec![]),
        )
        .unwrap();
        assert!((d.value - 3.0).abs() < 1e-12);
        assert_eq!(d.dim, Dimension::Scalar);
        assert_eq!(d.exact, None);
    }

    #[test]
    fn absolute_value_flips_a_negative_scalar_and_stays_exact() {
        // |−7| = 7, scalar, and the exact sidecar stays exact (|−7/1| = 7/1) —
        // the absolute value of an integer/rational operand keeps its exactness.
        let d = compute(
            "abs",
            &ComputeExpr::Unary(ComputeOp::Abs, Box::new(ComputeExpr::Lit(-7.0))),
            &kb_with(vec![]),
        )
        .unwrap();
        assert_eq!(d.value, 7.0);
        assert_eq!(d.dim, Dimension::Scalar);
        assert_eq!(d.exact, ExactRational::new(7, 1));
    }

    #[test]
    fn absolute_value_preserves_the_operand_dimension() {
        // |−4 dollars| = 4 dollars — a magnitude flips sign but the UNIT does not
        // (unlike a square root, which has no representable half-dimension). The
        // result carries the operand's money dimension unchanged.
        let kb = kb_with(vec![money("m", -4, "usd")]);
        let d = compute(
            "abs",
            &ComputeExpr::Unary(ComputeOp::Abs, Box::new(refexpr("m"))),
            &kb,
        )
        .unwrap();
        assert_eq!(d.value, 4.0);
        // Same dimension as the operand `m` (money/usd), NOT collapsed to Scalar.
        assert_eq!(d.dim, kb.observed_dimensioned("m").unwrap().0.dim);
    }

    #[test]
    fn floor_rounds_down_toward_negative_infinity_and_stays_exact() {
        // ⌊7/2⌋ = 3 (the greatest integer ≤ 3.5), and the exact sidecar snaps to
        // the integer 3/1. Euclidean division floors: 7.div_euclid(2) == 3.
        let d = compute(
            "fl",
            &ComputeExpr::Unary(
                ComputeOp::Floor,
                Box::new(bin(ComputeOp::Div, ComputeExpr::Lit(7.0), ComputeExpr::Lit(2.0))),
            ),
            &kb_with(vec![]),
        )
        .unwrap();
        assert_eq!(d.value, 3.0);
        assert_eq!(d.dim, Dimension::Scalar);
        assert_eq!(d.exact, ExactRational::new(3, 1));
    }

    #[test]
    fn floor_of_a_negative_value_rounds_down_not_toward_zero() {
        // ⌊−7/2⌋ = −4 (NOT −3): floor rounds toward −∞, so a negative
        // non-integer snaps DOWN. (−7).div_euclid(2) == −4, the Euclidean floor.
        let d = compute(
            "fl",
            &ComputeExpr::Unary(
                ComputeOp::Floor,
                Box::new(bin(ComputeOp::Div, ComputeExpr::Lit(-7.0), ComputeExpr::Lit(2.0))),
            ),
            &kb_with(vec![]),
        )
        .unwrap();
        assert_eq!(d.value, -4.0);
        assert_eq!(d.exact, ExactRational::new(-4, 1));
    }

    #[test]
    fn ceil_rounds_up_and_preserves_the_operand_dimension() {
        // ⌈7/2⌉ = 4 (the least integer ≥ 3.5). Ceil is dimension-preserving:
        // ⌈dollars⌉ is still dollars, NOT collapsed to Scalar.
        let kb = kb_with(vec![money("m", 7, "usd")]);
        let d = compute(
            "ce",
            &ComputeExpr::Unary(
                ComputeOp::Ceil,
                Box::new(bin(ComputeOp::Div, refexpr("m"), ComputeExpr::Lit(2.0))),
            ),
            &kb,
        )
        .unwrap();
        assert_eq!(d.value, 4.0);
        assert_eq!(d.exact, ExactRational::new(4, 1));
        // Same dimension as the operand `m` (money/usd).
        assert_eq!(d.dim, kb.observed_dimensioned("m").unwrap().0.dim);
    }

    #[test]
    fn ceil_of_an_exact_integer_is_the_integer_itself() {
        // ⌈6/2⌉ = 3 — an exact integer has no remainder, so ceil leaves it be
        // (the `+1` fires only when the Euclidean division leaves a remainder).
        let d = compute(
            "ce",
            &ComputeExpr::Unary(
                ComputeOp::Ceil,
                Box::new(bin(ComputeOp::Div, ComputeExpr::Lit(6.0), ComputeExpr::Lit(2.0))),
            ),
            &kb_with(vec![]),
        )
        .unwrap();
        assert_eq!(d.value, 3.0);
        assert_eq!(d.exact, ExactRational::new(3, 1));
    }

    #[test]
    fn round_ties_go_away_from_zero_and_stay_exact() {
        // ⌊5/2⌉ = 3, not 2 — a half rounds AWAY from zero (matching f64::round),
        // and the exact sidecar snaps to the integer 3/1.
        let d = compute(
            "rd",
            &ComputeExpr::Unary(
                ComputeOp::Round,
                Box::new(bin(ComputeOp::Div, ComputeExpr::Lit(5.0), ComputeExpr::Lit(2.0))),
            ),
            &kb_with(vec![]),
        )
        .unwrap();
        assert_eq!(d.value, 3.0);
        assert_eq!(d.dim, Dimension::Scalar);
        assert_eq!(d.exact, ExactRational::new(3, 1));
    }

    #[test]
    fn round_of_a_negative_half_goes_away_from_zero_too() {
        // ⌊−5/2⌉ = −3 (NOT −2): ties away from zero is symmetric, so a negative
        // half rounds down. Matches f64::round((-2.5)) == -3.0.
        let d = compute(
            "rd",
            &ComputeExpr::Unary(
                ComputeOp::Round,
                Box::new(bin(ComputeOp::Div, ComputeExpr::Lit(-5.0), ComputeExpr::Lit(2.0))),
            ),
            &kb_with(vec![]),
        )
        .unwrap();
        assert_eq!(d.value, -3.0);
        assert_eq!(d.exact, ExactRational::new(-3, 1));
    }

    #[test]
    fn round_below_a_half_stays_down_and_preserves_dimension() {
        // ⌊7/3⌉ = 2 (2.33… rounds down, fractional part < ½). Round is
        // dimension-preserving: ⌊dollars⌉ is still dollars, NOT collapsed to Scalar.
        let kb = kb_with(vec![money("m", 7, "usd")]);
        let d = compute(
            "rd",
            &ComputeExpr::Unary(
                ComputeOp::Round,
                Box::new(bin(ComputeOp::Div, refexpr("m"), ComputeExpr::Lit(3.0))),
            ),
            &kb,
        )
        .unwrap();
        assert_eq!(d.value, 2.0);
        assert_eq!(d.exact, ExactRational::new(2, 1));
        // Same dimension as the operand `m` (money/usd).
        assert_eq!(d.dim, kb.observed_dimensioned("m").unwrap().0.dim);
    }

    #[test]
    fn an_overflowing_power_is_a_clean_nonfinite_error_not_inf() {
        // 10 ^ 400 overflows f64 → a NonFinite error, never a silent `inf`.
        let err = compute(
            "big",
            &bin(ComputeOp::Pow, ComputeExpr::Lit(10.0), ComputeExpr::Lit(400.0)),
            &kb_with(vec![]),
        )
        .unwrap_err();
        assert!(matches!(err, ComputeError::NonFinite { op: ComputeOp::Pow }));
    }

    #[test]
    fn a_non_finite_exponent_is_rejected_not_laundered() {
        // `1.0.powf(NaN) == 1.0` and `1.0.powf(inf) == 1.0` in IEEE — a unit base
        // would otherwise turn a NaN/inf exponent into a clean 1.0. The input guard
        // rejects it instead, upholding "no silently-wrong number".
        for exp in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let err = compute(
                "p",
                &bin(ComputeOp::Pow, ComputeExpr::Lit(1.0), ComputeExpr::Lit(exp)),
                &kb_with(vec![]),
            )
            .unwrap_err();
            assert!(
                matches!(err, ComputeError::NonFinite { op: ComputeOp::Pow }),
                "exponent {exp} should be rejected, got {err:?}"
            );
        }
    }

    #[test]
    fn bare_number_formulas_are_scalar_and_unaffected() {
        // Regression: the pre-A4 numeric behaviour is unchanged for Scalars.
        let kb = kb_with(vec![
            Fact::certain(compound("a", vec![int(2)])),
            Fact::certain(compound("b", vec![int(3)])),
        ]);
        let d = compute("s", &bin(ComputeOp::Add, refexpr("a"), refexpr("b")), &kb).unwrap();
        assert_eq!(d.value, 5.0);
        assert_eq!(d.dim, Dimension::Scalar);
    }

    #[test]
    fn ratio_of_two_observed_facts_builds_a_cited_tree() {
        let kb = kb_with(vec![
            Fact::certain(compound("csf_glucose", vec![int(40)])),
            Fact::certain(compound("serum_glucose", vec![int(100)])),
        ]);
        let expr = ComputeExpr::Bin(
            ComputeOp::Div,
            Box::new(ComputeExpr::Ref("csf_glucose".into())),
            Box::new(ComputeExpr::Ref("serum_glucose".into())),
        );
        let d = compute("csf_ratio", &expr, &kb).unwrap();
        assert_eq!(d.name, "csf_ratio");
        assert!((d.value - 0.4).abs() < 1e-12);
        // The tree cites both leaves with their FactIds.
        match &d.tree {
            DerivationNode::Op {
                op,
                operands,
                result,
            } => {
                assert_eq!(*op, ComputeOp::Div);
                assert!((result - 0.4).abs() < 1e-12);
                assert_eq!(operands.len(), 2);
                assert!(
                    matches!(&operands[0], DerivationNode::Leaf { slot, value, .. }
                    if slot == "csf_glucose" && (*value - 40.0).abs() < 1e-12)
                );
                assert!(
                    matches!(&operands[1], DerivationNode::Leaf { slot, value, .. }
                    if slot == "serum_glucose" && (*value - 100.0).abs() < 1e-12)
                );
            }
            other => panic!("expected an Op node, got {other:?}"),
        }
    }

    #[test]
    fn integer_fraction_arithmetic_carries_exact_rational_sidecar() {
        let kb = KnowledgeBase::new();
        let expr = ComputeExpr::Bin(
            ComputeOp::Add,
            Box::new(ComputeExpr::Bin(
                ComputeOp::Div,
                Box::new(ComputeExpr::Lit(1.0)),
                Box::new(ComputeExpr::Lit(10.0)),
            )),
            Box::new(ComputeExpr::Bin(
                ComputeOp::Div,
                Box::new(ComputeExpr::Lit(2.0)),
                Box::new(ComputeExpr::Lit(10.0)),
            )),
        );
        let d = compute("answer", &expr, &kb).unwrap();
        assert!((d.value - 0.3).abs() < 1e-12);
        assert_eq!(d.exact, ExactRational::new(3, 10));
    }

    #[test]
    fn sum_aggregates_every_observation_of_a_slot() {
        let kb = kb_with(vec![
            Fact::certain(compound("line_item", vec![int(12000)])),
            Fact::certain(compound("line_item", vec![int(6000)])),
            Fact::certain(compound("line_item", vec![int(2000)])),
        ]);
        let d = compute(
            "total",
            &ComputeExpr::Agg(ComputeOp::Sum, "line_item".into()),
            &kb,
        )
        .unwrap();
        assert!((d.value - 20000.0).abs() < 1e-9);
        match &d.tree {
            DerivationNode::Op { op, operands, .. } => {
                assert_eq!(*op, ComputeOp::Sum);
                assert_eq!(operands.len(), 3, "every line_item should be a cited leaf");
            }
            other => panic!("expected Op, got {other:?}"),
        }
    }

    #[test]
    fn count_min_max_avg_reduce_correctly() {
        let kb = kb_with(vec![
            Fact::certain(compound("score", vec![int(10)])),
            Fact::certain(compound("score", vec![int(20)])),
            Fact::certain(compound("score", vec![int(30)])),
        ]);
        let c = compute(
            "n",
            &ComputeExpr::Agg(ComputeOp::Count, "score".into()),
            &kb,
        )
        .unwrap();
        assert_eq!(c.value, 3.0);
        let mn = compute("lo", &ComputeExpr::Agg(ComputeOp::Min, "score".into()), &kb).unwrap();
        assert_eq!(mn.value, 10.0);
        let mx = compute("hi", &ComputeExpr::Agg(ComputeOp::Max, "score".into()), &kb).unwrap();
        assert_eq!(mx.value, 30.0);
        let avg = compute(
            "mean",
            &ComputeExpr::Agg(ComputeOp::Avg, "score".into()),
            &kb,
        )
        .unwrap();
        assert!((avg.value - 20.0).abs() < 1e-12);
    }

    #[test]
    fn reads_magnitude_of_typed_value_operands() {
        // quantity(40, mg_dl) — the leading magnitude participates.
        let kb = kb_with(vec![
            Fact::certain(compound(
                "csf_glucose",
                vec![compound("quantity", vec![int(40), atom("mg_dl")])],
            )),
            Fact::certain(compound(
                "serum_glucose",
                vec![compound("quantity", vec![int(100), atom("mg_dl")])],
            )),
        ]);
        let expr = ComputeExpr::Bin(
            ComputeOp::Div,
            Box::new(ComputeExpr::Ref("csf_glucose".into())),
            Box::new(ComputeExpr::Ref("serum_glucose".into())),
        );
        assert!((compute("r", &expr, &kb).unwrap().value - 0.4).abs() < 1e-12);
    }

    #[test]
    fn unknown_slot_is_a_clean_error() {
        let kb = KnowledgeBase::new();
        let err = compute("x", &ComputeExpr::Ref("nope".into()), &kb).unwrap_err();
        assert_eq!(
            err,
            ComputeError::UnknownSlot {
                slot: "nope".into()
            }
        );
    }

    #[test]
    fn division_by_zero_is_a_clean_error() {
        let kb = kb_with(vec![
            Fact::certain(compound("a", vec![int(5)])),
            Fact::certain(compound("b", vec![int(0)])),
        ]);
        let expr = ComputeExpr::Bin(
            ComputeOp::Div,
            Box::new(ComputeExpr::Ref("a".into())),
            Box::new(ComputeExpr::Ref("b".into())),
        );
        assert_eq!(
            compute("x", &expr, &kb).unwrap_err(),
            ComputeError::DivisionByZero
        );
    }

    #[test]
    fn empty_aggregation_errors_except_count() {
        let kb = KnowledgeBase::new();
        assert_eq!(
            compute("s", &ComputeExpr::Agg(ComputeOp::Sum, "none".into()), &kb).unwrap_err(),
            ComputeError::EmptyAggregation {
                slot: "none".into()
            }
        );
        // count of an unobserved slot is a well-defined 0.
        assert_eq!(
            compute("n", &ComputeExpr::Agg(ComputeOp::Count, "none".into()), &kb)
                .unwrap()
                .value,
            0.0
        );
    }

    #[test]
    fn deeply_nested_expression_is_a_clean_error_not_a_stack_overflow() {
        // Build a formula nested far past MAX_EVAL_DEPTH: 1 + (1 + (1 + ...)).
        let kb = KnowledgeBase::new();
        let mut e = ComputeExpr::Lit(1.0);
        for _ in 0..(MAX_EVAL_DEPTH + 50) {
            e = ComputeExpr::Bin(ComputeOp::Add, Box::new(ComputeExpr::Lit(1.0)), Box::new(e));
        }
        assert_eq!(
            compute("deep", &e, &kb).unwrap_err(),
            ComputeError::TooDeep {
                limit: MAX_EVAL_DEPTH
            }
        );
    }

    #[test]
    fn non_finite_result_is_rejected_not_propagated() {
        // overflow to +inf via multiplication of two huge magnitudes.
        let kb = kb_with(vec![
            Fact::certain(compound("a", vec![logic_core::float(1e308)])),
            Fact::certain(compound("b", vec![logic_core::float(1e308)])),
        ]);
        let expr = ComputeExpr::Bin(
            ComputeOp::Mul,
            Box::new(ComputeExpr::Ref("a".into())),
            Box::new(ComputeExpr::Ref("b".into())),
        );
        assert_eq!(
            compute("x", &expr, &kb).unwrap_err(),
            ComputeError::NonFinite { op: ComputeOp::Mul }
        );
    }

    #[test]
    fn let_over_let_references_a_bound_derived_value() {
        let mut kb = kb_with(vec![
            Fact::certain(compound("a", vec![int(3)])),
            Fact::certain(compound("b", vec![int(4)])),
        ]);
        let sum = compute(
            "s",
            &ComputeExpr::Bin(
                ComputeOp::Add,
                Box::new(ComputeExpr::Ref("a".into())),
                Box::new(ComputeExpr::Ref("b".into())),
            ),
            &kb,
        )
        .unwrap();
        kb.add_derived(sum);
        // A later formula can reference the bound derived value by name.
        let doubled = compute(
            "d",
            &ComputeExpr::Bin(
                ComputeOp::Mul,
                Box::new(ComputeExpr::Ref("s".into())),
                Box::new(ComputeExpr::Lit(2.0)),
            ),
            &kb,
        )
        .unwrap();
        assert_eq!(doubled.value, 14.0);
        match &doubled.tree {
            DerivationNode::Op { operands, .. } => {
                assert!(
                    matches!(&operands[0], DerivationNode::DerivedRef { name, value }
                    if name == "s" && *value == 7.0)
                );
                assert!(matches!(&operands[1], DerivationNode::Lit { value } if *value == 2.0));
            }
            other => panic!("expected Op, got {other:?}"),
        }
    }
}
