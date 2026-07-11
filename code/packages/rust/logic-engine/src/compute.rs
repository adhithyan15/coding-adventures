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
    /// Truncate toward zero, `trunc(x)` — drop the fractional part, keeping the
    /// integer part with the operand's sign (`trunc(3.7) = 3`, `trunc(−3.7) = −3`).
    /// Contrast [`ComputeOp::Floor`] (toward −∞: `⌊−3.7⌋ = −4`) — they agree only
    /// for a non-negative operand. Like the rest of the rounding family it is
    /// **unary** and **dimension-preserving** (`trunc(3.7 mmol) = 3 mmol`), carried
    /// in a [`ComputeExpr::Unary`], and lowered from a LaTeX `\operatorname{trunc}(x)`
    /// — the operator-name juxtaposition surface (adj-lang's `latex "…"`). The exact
    /// sidecar stays exact: `trunc(num/den) = num / den` (Rust integer division
    /// truncates toward zero for `den > 0`, carried as `q/1`).
    Trunc,
    /// The named **transcendental** unary functions `sin`, `cos`, `tan`, `ln`
    /// (natural log), `log` (base-10), `exp`. Unlike the rounding unary ops these
    /// are **not** dimension-preserving: a transcendental is only defined on a
    /// pure number, so the operand must be dimensionless (`Scalar`) and the
    /// result is `Scalar` (`sin(3 dollars)` is a category error, rejected). They
    /// are irrational in general, so they drop the exact-rational sidecar. Each
    /// is carried in a [`ComputeExpr::Unary`] and lowered from a LaTeX
    /// `\sin(x)` / `\ln(x)` / `\exp(x)` … named-function call (adj-lang's
    /// `latex "…"` surface). Domain errors (`ln` of a non-positive number,
    /// `exp` overflow, `tan` at a pole) surface as the usual non-finite guard.
    Sin,
    Cos,
    Tan,
    Ln,
    Log,
    Exp,
    /// The rest of the standard trig family: the **inverse** functions `asin`,
    /// `acos`, `atan` (`\arcsin`/`\arccos`/`\arctan`), the **hyperbolic**
    /// functions `sinh`, `cosh`, `tanh`, and the **reciprocal** functions `cot`
    /// (cos/sin), `sec` (1/cos), `csc` (1/sin). Same contract as the other
    /// transcendentals — `Scalar → Scalar`, exact sidecar dropped, domain and
    /// pole errors (`asin`/`acos` outside [−1, 1]; `cot`/`csc` at a multiple of π;
    /// `sec` at an odd multiple of π/2) caught by the non-finite guard. They
    /// complete the trig set the LaTeX frontend already parses.
    Asin,
    Acos,
    Atan,
    Sinh,
    Cosh,
    Tanh,
    Cot,
    Sec,
    Csc,
    /// The **sign** function `sgn(x)` — `−1` for a negative operand, `0` for zero,
    /// `+1` for a positive one. **Unary**, but dimensionally in a category of its own:
    /// the sign of a quantity is a pure number (`sgn(−5 mmHg) = −1`, dimensionless),
    /// so — unlike the dimension-*preserving* rounding family (`|dollars| = dollars`)
    /// and unlike the transcendentals (which *reject* a dimensioned operand) — `sgn`
    /// **accepts any dimension and collapses the result to `Scalar`**. That makes the
    /// sign of a dimensioned difference (a net pressure, a net charge, a trend
    /// direction) computable: `sgn(pressure_a − pressure_b)` is a clean ±1. It is exact
    /// (`±1`/`0` is rational), so the exact sidecar is the sign of the numerator,
    /// carried as `q/1`. Lowered from a LaTeX `\operatorname{sgn}(x)` — the
    /// operator-name juxtaposition surface (like `\operatorname{trunc}`; adj-lang's
    /// `latex "…"`). Note this is the **mathematical** sign (`sgn(0) = 0`), NOT
    /// `f64::signum` (which returns `±1` for zero); a NaN operand is produced explicitly
    /// so the shared non-finite guard rejects it rather than laundering it to `0`.
    Sign,
    /// Binary **minimum** / **maximum** — `min(a, b)` / `max(a, b)` over exactly
    /// TWO operands. Unlike the aggregation [`ComputeOp::Min`]/[`ComputeOp::Max`]
    /// (which reduce *every* observation of a single slot), these are honest
    /// binary ops carried in a [`ComputeExpr::Bin`] with two sub-expressions —
    /// the first binary-`Call` lowering, from a LaTeX `\min(a, b)` / `\max(a, b)`
    /// (adj-lang's `latex "…"` surface). Dimensionally they behave like addition:
    /// both operands must share a dimension (`min(usd, days)` is a category
    /// error, exactly like `usd + days`) and the result carries that dimension.
    /// They *select* one operand unchanged, so the exact-rational sidecar is
    /// preserved from whichever operand won (no rounding, no new value). This is
    /// what makes a capped/floored clinical quantity (a dose capped at a ceiling,
    /// the worse of two labs) computable as a single native node.
    Min2,
    Max2,
    /// Binary **gcd** / **lcm** — `gcd(a, b)` / `lcm(a, b)` over exactly TWO
    /// operands, from a LaTeX `\gcd(a, b)` / `\lcm(a, b)`. Reuses the binary-`Call`
    /// path like `Min2`/`Max2`, but they are **integer number-theoretic** ops: both
    /// operands must be integer-valued (a non-integer like `2.5` is a
    /// `MalformedExpr`, not silently truncated), and dimensionally they combine
    /// like addition (a bare count / dimensionless integer in practice). `gcd` is
    /// Euclid on the magnitudes (`gcd(0, 0) = 0`); `lcm(a, b) = |a·b| / gcd(a, b)`
    /// with `lcm(_, 0) = 0`, and an overflow to non-finite is caught by the shared
    /// guard. The value is exact for realistic integer inputs, so the
    /// exact-rational sidecar is dropped (the f64 already carries it).
    Gcd,
    Lcm,
    /// Binary **modulo** — `a mod b`, the remainder of `a` divided by `b` carrying
    /// the **sign of the dividend** (`7 mod 3 = 1`, `−7 mod 3 = −1`, `7.5 mod 2 = 1.5`),
    /// matching Rust's `f64::%` (truncated division, C `fmod`). Lowered from a LaTeX
    /// `a \bmod b` / `a \pmod{b}` (adj-lang's `latex "…"` surface). Reuses the general
    /// binary path like [`ComputeOp::Div`]: dimensionally it combines like addition —
    /// both operands must share a dimension and the remainder carries it
    /// (`7 mmol mod 3 mmol = 1 mmol`, while `7 mmol mod 3` is a category error, exactly
    /// like `usd + days`) — and a zero divisor is a clean [`ComputeError::DivisionByZero`],
    /// never a silent `NaN`. Unlike [`ComputeOp::Gcd`]/[`ComputeOp::Lcm`] it does NOT
    /// require integer operands (it is the *real* remainder). The exact-rational sidecar
    /// is dropped — the `f64` remainder already carries the value for the realistic
    /// integer / short-decimal cases (like gcd/lcm).
    Mod,
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
            ComputeOp::Trunc => "trunc",
            ComputeOp::Sin => "sin",
            ComputeOp::Cos => "cos",
            ComputeOp::Tan => "tan",
            ComputeOp::Ln => "ln",
            ComputeOp::Log => "log",
            ComputeOp::Exp => "exp",
            ComputeOp::Asin => "asin",
            ComputeOp::Acos => "acos",
            ComputeOp::Atan => "atan",
            ComputeOp::Sinh => "sinh",
            ComputeOp::Cosh => "cosh",
            ComputeOp::Tanh => "tanh",
            ComputeOp::Cot => "cot",
            ComputeOp::Sec => "sec",
            ComputeOp::Csc => "csc",
            ComputeOp::Sign => "sgn",
            ComputeOp::Min2 => "min",
            ComputeOp::Max2 => "max",
            ComputeOp::Gcd => "gcd",
            ComputeOp::Lcm => "lcm",
            ComputeOp::Mod => "mod",
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
    /// A unary operation: the rounding family (`Abs`/`Floor`/`Ceil`/`Round`) and
    /// the transcendental family (`Sin`/`Cos`/`Tan`/`Ln`/`Log`/`Exp`). Kept
    /// distinct from [`ComputeExpr::Bin`] so the arity is honest (a unary op has
    /// one operand, not two) — it lowers to a [`DerivationNode::Op`] with a
    /// single-element `operands` vec.
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
    /// Provenance for the *formula* that produced this value, when it came from
    /// APPLYING a provenanced `formula` (ADJ-FORMULA-LIBRARIES rung-0): the
    /// formula's cited `source` / `locator` / `trust`. A plain `let` leaves this
    /// `None` — its audit trail is the derivation `tree` over observed facts, and
    /// there is no library claim to cite. This is the channel by which a computed
    /// answer carries **why** its formula is trustworthy, so an independent
    /// checker can re-verify the citation without the model.
    pub provenance: Option<crate::Provenance>,
}

impl Derived {
    /// Attach the applied formula's provenance (its cited `source`/`locator`/
    /// `trust`). Consumes and returns `self` so it composes with [`compute`]:
    /// `compute(name, expr, kb)?.with_provenance(prov)`.
    pub fn with_provenance(mut self, provenance: crate::Provenance) -> Self {
        self.provenance = Some(provenance);
        self
    }
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
        // A plain `let` carries no library-formula provenance; a formula
        // application attaches it afterward via [`Derived::with_provenance`].
        provenance: None,
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
        // Binary min/max behave dimensionally like addition: both operands must
        // share a dimension (`min(usd, days)` is the same category error as
        // `usd + days`) and the result carries that shared dimension. Reusing
        // `DimOp::Add` lets them flow through the general binary path with no
        // extra locals in the deeply-recursive `eval` frame.
        ComputeOp::Min2 | ComputeOp::Max2 => Some(DimOp::Add),
        // gcd/lcm are number-theoretic on (dimensionless) integers; combine like
        // addition so a bare count stays a bare count. The integer requirement is
        // enforced on the values, not the dimension.
        ComputeOp::Gcd | ComputeOp::Lcm => Some(DimOp::Add),
        // modulo `a mod b` combines like addition: both operands must share a
        // dimension and the remainder carries it (`7 mmol mod 3 mmol = 1 mmol`,
        // `7 mmol mod 3` a category error). Flows through the general binary path
        // with no extra `eval` locals.
        ComputeOp::Mod => Some(DimOp::Add),
        _ => None,
    }
}

/// Compute a binary integer `gcd`/`lcm` on two already-evaluated operand values.
/// A **leaf** helper (it does NOT call `eval`, so it never sits on the recursion
/// path) marked `#[inline(never)]` so its loop locals live in their own frame
/// rather than enlarging the deeply-recursive `eval`/`eval_binary` frame.
///
/// Contract: both operands must be **exact integers** in the exactly-representable
/// range (`|v| ≤ 2^53`); a non-integer (`2.5`), NaN/inf, or out-of-range value is a
/// clean error, never a silent truncation. `gcd` is Euclid on the magnitudes
/// (`gcd(0, 0) = 0`, `gcd(n, 0) = |n|`); `lcm(a, b) = |a·b| / gcd(a, b)` with
/// `lcm(_, 0) = 0`. Everything is done in `i128` so no intermediate overflows; the
/// caller's shared `is_finite` guard is the final backstop.
#[inline(never)]
fn int_gcd_lcm(op: ComputeOp, x: f64, y: f64) -> Result<f64, ComputeError> {
    // `2^53` is the largest integer every f64 represents exactly — beyond it an
    // f64 "integer" is already ambiguous, so gcd/lcm on it would be meaningless.
    const LIMIT: f64 = 9_007_199_254_740_992.0; // 2^53
    if x.fract() != 0.0 || y.fract() != 0.0 || x.abs() > LIMIT || y.abs() > LIMIT {
        return Err(ComputeError::MalformedExpr {
            detail: "gcd/lcm requires integer operands within the exact range",
        });
    }
    let a = (x as i128).unsigned_abs();
    let b = (y as i128).unsigned_abs();
    // Euclid's algorithm on the magnitudes.
    let (mut p, mut q) = (a, b);
    while q != 0 {
        let r = p % q;
        p = q;
        q = r;
    }
    let g = p; // gcd(a, b); 0 iff both operands are 0
    let value = match op {
        ComputeOp::Gcd => g as f64,
        // lcm: divide first (a/g is exact) then multiply, in i128 so the product
        // cannot overflow the integer type; the f64 cast + caller's finite guard
        // handle any magnitude concern.
        ComputeOp::Lcm if g == 0 => 0.0,
        ComputeOp::Lcm => ((a / g) * b) as f64,
        _ => unreachable!("int_gcd_lcm only handles Gcd/Lcm"),
    };
    Ok(value)
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
            // of the magnitudes. `dim_op` maps the additive/multiplicative ops AND
            // binary min/max (which combine like addition — see `dim_op`).
            let dimop = dim_op(*op).ok_or(ComputeError::MalformedExpr {
                detail: "aggregation operator in binary position",
            })?;
            let result_dim = Dimension::combine(dimop, &dim_l, &dim_r).map_err(|e| match e {
                crate::DimError::Mismatch { lhs, rhs, .. } => {
                    ComputeError::DimensionMismatch { op: *op, lhs, rhs }
                }
            })?;
            let (x, y) = (lhs.value(), rhs.value());
            // Binary min/max are folded into this general path (rather than a
            // separate block) so they add NO extra locals to `eval`'s frame — it
            // recurses up to `MAX_EVAL_DEPTH` levels, so every byte here is
            // multiplied 256× and a fatter frame can overflow a small (macOS ~2 MB)
            // thread stack before the depth guard trips. min/max SELECT one operand
            // (no arithmetic); a `NaN` operand would let `f64::min`/`max` silently
            // drop the NaN and return the finite side, so we produce `NaN`
            // explicitly and let the shared `is_finite` guard below reject it.
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
                // modulo `a mod b` — the remainder with the sign of the dividend
                // (Rust `%` / C `fmod`). Inline like `Div` (a single expression, no
                // extra `eval` locals on the deeply-recursive path); a zero divisor is
                // a clean error, never a `NaN`.
                ComputeOp::Mod => {
                    if y == 0.0 {
                        return Err(ComputeError::DivisionByZero);
                    }
                    x % y
                }
                ComputeOp::Min2 => {
                    if x.is_nan() || y.is_nan() {
                        f64::NAN
                    } else if x <= y {
                        x
                    } else {
                        y
                    }
                }
                ComputeOp::Max2 => {
                    if x.is_nan() || y.is_nan() {
                        f64::NAN
                    } else if x >= y {
                        x
                    } else {
                        y
                    }
                }
                // gcd/lcm are integer number-theoretic ops. Delegated to a leaf
                // `#[inline(never)]` helper (NOT on the recursion path) so its
                // Euclid-loop locals live in their own frame, not `eval`'s. A
                // non-integer operand is a clean `MalformedExpr`.
                ComputeOp::Gcd | ComputeOp::Lcm => int_gcd_lcm(*op, x, y)?,
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
                    // min/max select an operand UNCHANGED, so the winner's exact
                    // rational carries through verbatim (ties pick the left).
                    ComputeOp::Min2 => Some(if x <= y { a } else { b }),
                    ComputeOp::Max2 => Some(if x >= y { a } else { b }),
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

/// Evaluate a unary op — the rounding family (`Abs`/`Floor`/`Ceil`/`Round`) or a
/// transcendental (`Sin`/`Cos`/`Tan`/`Ln`/`Log`/`Exp`) — into a derivation node +
/// dimension.
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
    // The **transcendental** functions are only defined on a pure number, so — like
    // `Pow`'s exponent — the operand must be dimensionless. `sin(3 dollars)` is a
    // category error, rejected here with the same `DimensionMismatch` the binary
    // ops use (the operand's dimension vs the required `Scalar`).
    let transcendental = matches!(
        op,
        ComputeOp::Sin
            | ComputeOp::Cos
            | ComputeOp::Tan
            | ComputeOp::Ln
            | ComputeOp::Log
            | ComputeOp::Exp
            | ComputeOp::Asin
            | ComputeOp::Acos
            | ComputeOp::Atan
            | ComputeOp::Sinh
            | ComputeOp::Cosh
            | ComputeOp::Tanh
            | ComputeOp::Cot
            | ComputeOp::Sec
            | ComputeOp::Csc
    );
    if transcendental && !dim.is_scalar() {
        return Err(ComputeError::DimensionMismatch {
            op,
            lhs: dim.tag(),
            rhs: Dimension::Scalar.tag(),
        });
    }
    // The rounding family is **dimension-preserving**: the magnitude may change
    // (sign flip for `Abs`, snap to an integer for `Floor`/`Ceil`/`Round`) but the
    // unit does not (`|−3 dollars| = 3 dollars`, `⌊3.7 mmol⌋ = 3 mmol`). The
    // transcendentals map a pure number to a pure number (`Scalar → Scalar`).
    let value = operand.value();
    let result = match op {
        ComputeOp::Abs => value.abs(),
        ComputeOp::Floor => value.floor(),
        ComputeOp::Ceil => value.ceil(),
        ComputeOp::Round => value.round(),
        ComputeOp::Trunc => value.trunc(),
        ComputeOp::Sin => value.sin(),
        ComputeOp::Cos => value.cos(),
        ComputeOp::Tan => value.tan(),
        ComputeOp::Ln => value.ln(),
        ComputeOp::Log => value.log10(),
        ComputeOp::Exp => value.exp(),
        ComputeOp::Asin => value.asin(),
        ComputeOp::Acos => value.acos(),
        ComputeOp::Atan => value.atan(),
        ComputeOp::Sinh => value.sinh(),
        ComputeOp::Cosh => value.cosh(),
        ComputeOp::Tanh => value.tanh(),
        // Reciprocal trig defined from the primaries; a zero denominator (a pole)
        // becomes ±∞ and is caught by the finite guard below.
        ComputeOp::Cot => value.cos() / value.sin(),
        ComputeOp::Sec => 1.0 / value.cos(),
        ComputeOp::Csc => 1.0 / value.sin(),
        // The MATHEMATICAL sign: sgn(0) = 0 (NOT `f64::signum`, which returns ±1 for
        // zero). A NaN operand yields NaN explicitly so the non-finite guard below
        // rejects it, rather than the `else` branch laundering it to a clean `0`.
        ComputeOp::Sign => {
            if value.is_nan() {
                f64::NAN
            } else if value > 0.0 {
                1.0
            } else if value < 0.0 {
                -1.0
            } else {
                0.0
            }
        }
        _ => {
            return Err(ComputeError::MalformedExpr {
                detail: "non-unary operator in unary position",
            })
        }
    };
    // The rounding ops can't turn a finite operand non-finite; the transcendentals
    // CAN (`ln` of a non-positive number → `NaN`/`−∞`, `exp` overflow → `+∞`,
    // `tan` near a pole → a huge but finite value). The guard catches all of them —
    // a `NaN` would otherwise compare `false` against every threshold and silently
    // suppress a predicate, exactly the quiet wrong answer provenance-through-math
    // forbids.
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
        ComputeOp::Abs => r
            .num
            .checked_abs()
            .and_then(|n| ExactRational::new(n, r.den)),
        ComputeOp::Floor => ExactRational::new(r.num.div_euclid(r.den), 1),
        ComputeOp::Ceil => {
            let q = r.num.div_euclid(r.den);
            let q = if r.num.rem_euclid(r.den) != 0 {
                q.checked_add(1)?
            } else {
                q
            };
            ExactRational::new(q, 1)
        }
        ComputeOp::Round => {
            let q = r.num / r.den; // truncate toward zero (den > 0 ⇒ no overflow)
            let rem = r.num % r.den; // in (−den, den), sign of the numerator
            let arem = if rem >= 0 { rem } else { -rem }; // |rem| < den ⇒ no overflow
            let bump = if arem >= r.den - arem {
                // fractional part ≥ 1/2 → round away from zero (ties away from zero)
                if r.num >= 0 {
                    1
                } else {
                    -1
                }
            } else {
                0
            };
            ExactRational::new(q.checked_add(bump)?, 1)
        }
        // trunc(num/den) truncates toward zero — exactly Rust's integer division for
        // den > 0 (no `div_euclid`, which would floor toward −∞). The result is an
        // integer, carried as q/1.
        ComputeOp::Trunc => ExactRational::new(r.num / r.den, 1),
        // sgn(num/den) = sign of the numerator (den > 0 doesn't affect the sign);
        // `i64::signum` is the mathematical sign (0 → 0), carried as q/1.
        ComputeOp::Sign => ExactRational::new(r.num.signum(), 1),
        _ => None,
    });
    // Rounding preserves the operand's dimension; a transcendental collapses to a
    // pure number (`Scalar`); `sgn` also collapses to `Scalar` (a sign is
    // dimensionless) but — unlike the transcendentals — accepts a dimensioned operand.
    let result_dim = if transcendental || op == ComputeOp::Sign {
        Dimension::Scalar
    } else {
        dim
    };
    Ok((
        DerivationNode::Op {
            op,
            operands: vec![operand],
            result,
        },
        result_dim,
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
        let err = compute("p", &bin(ComputeOp::Pow, refexpr("x"), refexpr("e")), &kb).unwrap_err();
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
                Box::new(bin(
                    ComputeOp::Div,
                    ComputeExpr::Lit(7.0),
                    ComputeExpr::Lit(2.0),
                )),
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
                Box::new(bin(
                    ComputeOp::Div,
                    ComputeExpr::Lit(-7.0),
                    ComputeExpr::Lit(2.0),
                )),
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
                Box::new(bin(
                    ComputeOp::Div,
                    ComputeExpr::Lit(6.0),
                    ComputeExpr::Lit(2.0),
                )),
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
                Box::new(bin(
                    ComputeOp::Div,
                    ComputeExpr::Lit(5.0),
                    ComputeExpr::Lit(2.0),
                )),
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
                Box::new(bin(
                    ComputeOp::Div,
                    ComputeExpr::Lit(-5.0),
                    ComputeExpr::Lit(2.0),
                )),
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
    fn trunc_drops_the_fraction_toward_zero_and_preserves_dimension() {
        // trunc(7/2) = 3 (3.5 → drop the .5). Trunc is dimension-preserving:
        // trunc(dollars) is still dollars, NOT collapsed to Scalar.
        let kb = kb_with(vec![money("m", 7, "usd")]);
        let d = compute(
            "tr",
            &ComputeExpr::Unary(
                ComputeOp::Trunc,
                Box::new(bin(ComputeOp::Div, refexpr("m"), ComputeExpr::Lit(2.0))),
            ),
            &kb,
        )
        .unwrap();
        assert_eq!(d.value, 3.0);
        assert_eq!(d.exact, ExactRational::new(3, 1));
        assert_eq!(d.dim, kb.observed_dimensioned("m").unwrap().0.dim);
    }

    #[test]
    fn trunc_of_a_negative_value_goes_toward_zero_not_down() {
        // trunc(−7/2) = −3 (toward zero), NOT −4 the way Floor rounds toward −∞.
        // This is the whole point of adding Trunc beside Floor: (−7)/2 == −3 in
        // Rust integer division (truncation), vs (−7).div_euclid(2) == −4.
        let d = compute(
            "tr",
            &ComputeExpr::Unary(
                ComputeOp::Trunc,
                Box::new(bin(
                    ComputeOp::Div,
                    ComputeExpr::Lit(-7.0),
                    ComputeExpr::Lit(2.0),
                )),
            ),
            &kb_with(vec![]),
        )
        .unwrap();
        assert_eq!(d.value, -3.0);
        assert_eq!(d.exact, ExactRational::new(-3, 1));
    }

    #[test]
    fn modulo_returns_the_remainder_and_preserves_dimension() {
        // 7 mmol mod 3 mmol = 1 mmol. Modulo combines dimensionally like addition:
        // both operands share a dimension and the remainder carries it (NOT collapsed
        // to Scalar). The exact-rational sidecar is dropped (like gcd/lcm).
        let kb = kb_with(vec![quantity("a", 7, "mmol"), quantity("b", 3, "mmol")]);
        let d = compute(
            "m",
            &bin(ComputeOp::Mod, refexpr("a"), refexpr("b")),
            &kb,
        )
        .unwrap();
        assert_eq!(d.value, 1.0);
        assert_eq!(d.dim, kb.observed_dimensioned("a").unwrap().0.dim);
        assert_eq!(d.exact, None);
    }

    #[test]
    fn modulo_carries_the_sign_of_the_dividend() {
        // −7 mod 3 = −1 (sign of the DIVIDEND, Rust `%` / C fmod), NOT +2 the way a
        // Euclidean/floored modulo would give. 7.5 mod 2 = 1.5 (real operands allowed,
        // unlike gcd/lcm).
        let neg = compute(
            "m",
            &bin(ComputeOp::Mod, ComputeExpr::Lit(-7.0), ComputeExpr::Lit(3.0)),
            &kb_with(vec![]),
        )
        .unwrap();
        assert_eq!(neg.value, -1.0);
        let frac = compute(
            "m",
            &bin(ComputeOp::Mod, ComputeExpr::Lit(7.5), ComputeExpr::Lit(2.0)),
            &kb_with(vec![]),
        )
        .unwrap();
        assert_eq!(frac.value, 1.5);
    }

    #[test]
    fn modulo_by_zero_is_a_clean_error_not_a_nan() {
        // A zero divisor is a `DivisionByZero`, exactly like `Div` — never a silent NaN.
        let err = compute(
            "m",
            &bin(ComputeOp::Mod, ComputeExpr::Lit(7.0), ComputeExpr::Lit(0.0)),
            &kb_with(vec![]),
        )
        .unwrap_err();
        assert!(matches!(err, ComputeError::DivisionByZero));
    }

    #[test]
    fn modulo_of_mismatched_dimensions_is_a_category_error() {
        // 7 mmol mod 3 (scalar) shares no dimension → the same category error as
        // `usd + days` (modulo combines like addition in `dim_op`).
        let kb = kb_with(vec![quantity("a", 7, "mmol")]);
        let err = compute(
            "m",
            &bin(ComputeOp::Mod, refexpr("a"), ComputeExpr::Lit(3.0)),
            &kb,
        )
        .unwrap_err();
        assert!(matches!(err, ComputeError::DimensionMismatch { .. }));
    }

    #[test]
    fn sign_of_positive_negative_and_zero_scalars() {
        // sgn(5) = 1, sgn(−5) = −1, sgn(0) = 0. Zero maps to zero — the MATHEMATICAL
        // sign, NOT `f64::signum` (which returns +1 for zero). Result is a Scalar.
        for (input, want) in [(5.0, 1.0), (-5.0, -1.0), (0.0, 0.0)] {
            let d = compute(
                "s",
                &ComputeExpr::Unary(ComputeOp::Sign, Box::new(ComputeExpr::Lit(input))),
                &kb_with(vec![]),
            )
            .unwrap();
            assert_eq!(d.value, want, "sgn({input})");
            assert_eq!(d.dim, Dimension::Scalar);
        }
    }

    #[test]
    fn sign_accepts_a_dimensioned_operand_and_collapses_to_scalar() {
        // sgn(−3 mmol) = −1 (dimensionless). Unlike a transcendental, `sgn` does NOT
        // reject a dimensioned operand — the sign of a quantity is a pure number — so
        // the result is a Scalar −1, not a mmol and not a DimensionMismatch error.
        let kb = kb_with(vec![quantity("a", -3, "mmol")]);
        let d = compute(
            "s",
            &ComputeExpr::Unary(ComputeOp::Sign, Box::new(refexpr("a"))),
            &kb,
        )
        .unwrap();
        assert_eq!(d.value, -1.0);
        assert_eq!(d.dim, Dimension::Scalar);
    }

    #[test]
    fn sign_preserves_the_exact_sign_sidecar() {
        // sgn(−7/2) = −1 and sgn(7/2) = +1, both exact (a sign is ±1, rational).
        let neg = compute(
            "s",
            &ComputeExpr::Unary(
                ComputeOp::Sign,
                Box::new(bin(
                    ComputeOp::Div,
                    ComputeExpr::Lit(-7.0),
                    ComputeExpr::Lit(2.0),
                )),
            ),
            &kb_with(vec![]),
        )
        .unwrap();
        assert_eq!(neg.value, -1.0);
        assert_eq!(neg.exact, ExactRational::new(-1, 1));
        let pos = compute(
            "s",
            &ComputeExpr::Unary(
                ComputeOp::Sign,
                Box::new(bin(
                    ComputeOp::Div,
                    ComputeExpr::Lit(7.0),
                    ComputeExpr::Lit(2.0),
                )),
            ),
            &kb_with(vec![]),
        )
        .unwrap();
        assert_eq!(pos.value, 1.0);
        assert_eq!(pos.exact, ExactRational::new(1, 1));
    }

    #[test]
    fn sign_of_a_net_difference_gives_its_direction() {
        // sgn(a − b) with a = 3 mmol, b = 8 mmol is sgn(−5 mmol) = −1: the DIRECTION of
        // a net (dimensioned) quantity, computed as a single Scalar node. This is the
        // whole point of accepting a dimensioned operand.
        let kb = kb_with(vec![quantity("a", 3, "mmol"), quantity("b", 8, "mmol")]);
        let d = compute(
            "s",
            &ComputeExpr::Unary(
                ComputeOp::Sign,
                Box::new(bin(ComputeOp::Sub, refexpr("a"), refexpr("b"))),
            ),
            &kb,
        )
        .unwrap();
        assert_eq!(d.value, -1.0);
        assert_eq!(d.dim, Dimension::Scalar);
    }

    #[test]
    fn exp_and_ln_are_inverses_on_a_scalar_and_drop_exactness() {
        // exp(ln 5) = 5 (within float tolerance). Both are transcendental: the
        // operand is a pure number, the result is a pure number (Scalar), and the
        // exact-rational sidecar is dropped (a transcendental is irrational).
        let inner = ComputeExpr::Unary(ComputeOp::Ln, Box::new(ComputeExpr::Lit(5.0)));
        let d = compute(
            "e",
            &ComputeExpr::Unary(ComputeOp::Exp, Box::new(inner)),
            &kb_with(vec![]),
        )
        .unwrap();
        assert!((d.value - 5.0).abs() < 1e-9, "{}", d.value);
        assert_eq!(d.dim, Dimension::Scalar);
        assert_eq!(d.exact, None);
    }

    #[test]
    fn sin_of_zero_is_zero_and_cos_of_zero_is_one() {
        // Sanity anchors that don't depend on π: sin(0)=0, cos(0)=1.
        let s = compute(
            "s",
            &ComputeExpr::Unary(ComputeOp::Sin, Box::new(ComputeExpr::Lit(0.0))),
            &kb_with(vec![]),
        )
        .unwrap();
        assert_eq!(s.value, 0.0);
        let c = compute(
            "c",
            &ComputeExpr::Unary(ComputeOp::Cos, Box::new(ComputeExpr::Lit(0.0))),
            &kb_with(vec![]),
        )
        .unwrap();
        assert_eq!(c.value, 1.0);
        assert_eq!(c.dim, Dimension::Scalar);
    }

    #[test]
    fn a_transcendental_of_a_dimensioned_operand_is_a_category_error() {
        // `ln(4 dollars)` is meaningless — a transcendental is only defined on a
        // pure number. The engine rejects it with a DimensionMismatch (operand
        // dimension vs the required Scalar), NOT a silently-wrong number.
        let kb = kb_with(vec![money("m", 4, "usd")]);
        let err = compute(
            "l",
            &ComputeExpr::Unary(ComputeOp::Ln, Box::new(refexpr("m"))),
            &kb,
        )
        .unwrap_err();
        assert!(
            matches!(
                err,
                ComputeError::DimensionMismatch {
                    op: ComputeOp::Ln,
                    ..
                }
            ),
            "{err:?}"
        );
    }

    #[test]
    fn ln_of_a_non_positive_number_is_a_clean_nonfinite_error() {
        // ln(0) = −∞ and ln(−1) = NaN in IEEE — both are rejected by the finite
        // guard rather than flowing a non-finite value into a verdict.
        for x in [0.0, -1.0] {
            let err = compute(
                "l",
                &ComputeExpr::Unary(ComputeOp::Ln, Box::new(ComputeExpr::Lit(x))),
                &kb_with(vec![]),
            )
            .unwrap_err();
            assert!(
                matches!(err, ComputeError::NonFinite { op: ComputeOp::Ln }),
                "x={x}: {err:?}"
            );
        }
    }

    #[test]
    fn hyperbolic_and_inverse_trig_anchors_are_pi_free() {
        // π-free sanity anchors for the extended trig family: sinh(0)=0, cosh(0)=1,
        // tanh(0)=0, asin(0)=0, acos(1)=0, atan(0)=0. All Scalar, exact dropped.
        let cases = [
            (ComputeOp::Sinh, 0.0, 0.0),
            (ComputeOp::Cosh, 0.0, 1.0),
            (ComputeOp::Tanh, 0.0, 0.0),
            (ComputeOp::Asin, 0.0, 0.0),
            (ComputeOp::Acos, 1.0, 0.0),
            (ComputeOp::Atan, 0.0, 0.0),
        ];
        for (op, x, want) in cases {
            let d = compute(
                "t",
                &ComputeExpr::Unary(op, Box::new(ComputeExpr::Lit(x))),
                &kb_with(vec![]),
            )
            .unwrap();
            assert!(
                (d.value - want).abs() < 1e-12,
                "{op:?}({x}) = {} want {want}",
                d.value
            );
            assert_eq!(d.dim, Dimension::Scalar);
            assert_eq!(d.exact, None);
        }
    }

    #[test]
    fn reciprocal_trig_uses_the_primary_definitions() {
        // sec(0) = 1/cos(0) = 1 (a clean value); csc(0) = 1/sin(0) and cot(0) =
        // cos(0)/sin(0) are poles (sin 0 = 0 → ±∞) and are rejected by the finite
        // guard, never a silently-wrong number.
        let sec0 = compute(
            "s",
            &ComputeExpr::Unary(ComputeOp::Sec, Box::new(ComputeExpr::Lit(0.0))),
            &kb_with(vec![]),
        )
        .unwrap();
        assert_eq!(sec0.value, 1.0);
        assert_eq!(sec0.dim, Dimension::Scalar);
        for op in [ComputeOp::Csc, ComputeOp::Cot] {
            let err = compute(
                "p",
                &ComputeExpr::Unary(op, Box::new(ComputeExpr::Lit(0.0))),
                &kb_with(vec![]),
            )
            .unwrap_err();
            assert!(
                matches!(err, ComputeError::NonFinite { .. }),
                "{op:?}(0): {err:?}"
            );
        }
    }

    #[test]
    fn arcsine_outside_its_domain_is_a_clean_nonfinite_error() {
        // asin is only defined on [−1, 1]; asin(2) = NaN in IEEE → rejected, not
        // flowed into a verdict.
        let err = compute(
            "a",
            &ComputeExpr::Unary(ComputeOp::Asin, Box::new(ComputeExpr::Lit(2.0))),
            &kb_with(vec![]),
        )
        .unwrap_err();
        assert!(
            matches!(
                err,
                ComputeError::NonFinite {
                    op: ComputeOp::Asin
                }
            ),
            "{err:?}"
        );
    }

    #[test]
    fn an_overflowing_power_is_a_clean_nonfinite_error_not_inf() {
        // 10 ^ 400 overflows f64 → a NonFinite error, never a silent `inf`.
        let err = compute(
            "big",
            &bin(
                ComputeOp::Pow,
                ComputeExpr::Lit(10.0),
                ComputeExpr::Lit(400.0),
            ),
            &kb_with(vec![]),
        )
        .unwrap_err();
        assert!(matches!(
            err,
            ComputeError::NonFinite { op: ComputeOp::Pow }
        ));
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
    fn binary_min_max_select_the_extreme_operand() {
        // min(a, b) / max(a, b) as honest BINARY ops over two sub-expressions —
        // distinct from the slot-reducing aggregation Min/Max. Selection, not a
        // new value.
        let kb = kb_with(vec![
            Fact::certain(compound("a", vec![int(3)])),
            Fact::certain(compound("b", vec![int(8)])),
        ]);
        let lo = compute("lo", &bin(ComputeOp::Min2, refexpr("a"), refexpr("b")), &kb).unwrap();
        assert_eq!(lo.value, 3.0);
        let hi = compute("hi", &bin(ComputeOp::Max2, refexpr("a"), refexpr("b")), &kb).unwrap();
        assert_eq!(hi.value, 8.0);
        // The result node is a two-operand Op, not an aggregation over one slot.
        match &hi.tree {
            DerivationNode::Op { op, operands, .. } => {
                assert_eq!(*op, ComputeOp::Max2);
                assert_eq!(operands.len(), 2);
            }
            other => panic!("expected a binary Op node, got {other:?}"),
        }
    }

    #[test]
    fn binary_min_max_preserve_the_winning_operands_exact_rational() {
        // The winner is selected UNCHANGED, so its exact-rational sidecar carries
        // through verbatim — no rounding, no arithmetic. `min(3, 8) = 3` keeps 3/1.
        let kb = kb_with(vec![
            Fact::certain(compound("a", vec![int(3)])),
            Fact::certain(compound("b", vec![int(8)])),
        ]);
        let lo = compute("lo", &bin(ComputeOp::Min2, refexpr("a"), refexpr("b")), &kb).unwrap();
        assert_eq!(lo.exact, ExactRational::new(3, 1));
        let hi = compute("hi", &bin(ComputeOp::Max2, refexpr("a"), refexpr("b")), &kb).unwrap();
        assert_eq!(hi.exact, ExactRational::new(8, 1));
    }

    #[test]
    fn binary_min_carries_the_shared_dimension_and_rejects_a_mismatch() {
        // Like addition: both operands must share a dimension. `min(usd, usd)`
        // stays usd; `min(usd, days)` is the same category error as `usd + days`.
        let kb = kb_with(vec![
            money("cap", 200, "usd"),
            money("dose", 150, "usd"),
            Fact::certain(compound(
                "age",
                vec![compound("duration", vec![int(5), atom("days")])],
            )),
        ]);
        let capped = compute(
            "capped",
            &bin(ComputeOp::Min2, refexpr("dose"), refexpr("cap")),
            &kb,
        )
        .unwrap();
        assert_eq!(capped.value, 150.0);
        let err = compute(
            "bad",
            &bin(ComputeOp::Max2, refexpr("cap"), refexpr("age")),
            &kb,
        )
        .unwrap_err();
        assert!(
            matches!(err, ComputeError::DimensionMismatch { .. }),
            "{err:?}"
        );
    }

    #[test]
    fn binary_min_max_reject_a_non_finite_operand() {
        // A NaN/inf operand (an LLM-emitted `Lit(NaN)`) would make the comparison
        // meaningless, so it is a clean NonFinite error rather than a NaN silently
        // "winning".
        let kb = kb_with(vec![Fact::certain(compound("a", vec![int(3)]))]);
        let err = compute(
            "x",
            &bin(ComputeOp::Min2, refexpr("a"), ComputeExpr::Lit(f64::NAN)),
            &kb,
        )
        .unwrap_err();
        assert!(matches!(err, ComputeError::NonFinite { .. }), "{err:?}");
    }

    #[test]
    fn binary_gcd_lcm_compute_integer_results() {
        // gcd(12, 18) = 6; lcm(4, 6) = 12. Euclid + divide-then-multiply.
        let kb = kb_with(vec![
            Fact::certain(compound("a", vec![int(12)])),
            Fact::certain(compound("b", vec![int(18)])),
            Fact::certain(compound("c", vec![int(4)])),
            Fact::certain(compound("d", vec![int(6)])),
        ]);
        let g = compute("g", &bin(ComputeOp::Gcd, refexpr("a"), refexpr("b")), &kb).unwrap();
        assert_eq!(g.value, 6.0);
        let l = compute("l", &bin(ComputeOp::Lcm, refexpr("c"), refexpr("d")), &kb).unwrap();
        assert_eq!(l.value, 12.0);
    }

    #[test]
    fn binary_gcd_lcm_zero_edge_cases() {
        // gcd(0, 0) = 0; gcd(n, 0) = |n|; lcm(_, 0) = 0.
        let kb = kb_with(vec![
            Fact::certain(compound("z", vec![int(0)])),
            Fact::certain(compound("n", vec![int(7)])),
        ]);
        assert_eq!(
            compute("g0", &bin(ComputeOp::Gcd, refexpr("z"), refexpr("z")), &kb)
                .unwrap()
                .value,
            0.0
        );
        assert_eq!(
            compute("gn", &bin(ComputeOp::Gcd, refexpr("n"), refexpr("z")), &kb)
                .unwrap()
                .value,
            7.0
        );
        assert_eq!(
            compute("l0", &bin(ComputeOp::Lcm, refexpr("n"), refexpr("z")), &kb)
                .unwrap()
                .value,
            0.0
        );
    }

    #[test]
    fn binary_gcd_rejects_a_non_integer_operand() {
        // gcd(12, 2.5) — a non-integer operand is a clean MalformedExpr, never a
        // silent truncation.
        let kb = kb_with(vec![Fact::certain(compound("a", vec![int(12)]))]);
        let err = compute(
            "x",
            &bin(ComputeOp::Gcd, refexpr("a"), ComputeExpr::Lit(2.5)),
            &kb,
        )
        .unwrap_err();
        assert!(matches!(err, ComputeError::MalformedExpr { .. }), "{err:?}");
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
