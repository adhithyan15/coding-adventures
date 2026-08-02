// `add`/`sub`/`mul`/`div` here are fallible checked-arithmetic methods returning
// `Option<Self>`, not the infallible `std::ops` traits (which must return `Self`);
// implementing those traits would be the wrong contract.
#![allow(clippy::should_implement_trait)]
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
use bignum_core::{BigDecimal, BigInteger, BigRational, RoundingMode};

/// An exact rational value for CPU arithmetic — a [`BigRational`] from `bignum-core`, so it is
/// **unbounded** (no `i128` overflow) and every `+ − × ÷` of rationals stays exact forever.
/// This is the engine's exactness carrier (NUM-5): the public magnitude is still exported as
/// `f64` for compatibility ([`to_f64`](Self::to_f64), a *labeled lossy* narrowing), but the
/// exact `BigRational` is the ground truth an audit reconstructs. Where the old `i128` sidecar
/// dropped to `None` on overflow, this simply stays exact.
///
/// `BigRational` is canonical (reduced, positive denominator), so the derived `PartialEq`/`Eq`
/// is value equality — `160/7 == 320/14` — exactly as the equality-sensitive LR gate relies on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactRational(BigRational);

impl ExactRational {
    /// Build from an `i128` numerator/denominator (gcd-normalized, denominator made positive).
    /// Returns `None` **only** on a zero denominator — never on overflow, since the parts are
    /// arbitrary-precision.
    pub fn new(num: i128, den: i128) -> Option<Self> {
        BigRational::checked_new(BigInteger::from_i128(num), BigInteger::from_i128(den)).map(Self)
    }

    /// A whole number.
    pub fn from_i128(n: i128) -> Self {
        Self(BigRational::from(n))
    }

    /// Wrap an already-canonical [`BigRational`].
    pub fn from_ratio(r: BigRational) -> Self {
        Self(r)
    }

    /// The underlying exact rational.
    pub fn as_ratio(&self) -> &BigRational {
        &self.0
    }

    /// The numerator (sign-carrying) as an arbitrary-precision integer.
    pub fn numerator(&self) -> &BigInteger {
        self.0.numerator()
    }

    /// The denominator (always positive) as an arbitrary-precision integer.
    pub fn denominator(&self) -> &BigInteger {
        self.0.denominator()
    }

    /// An exact value from an **integer-valued** `f64` (finite, no fractional part, within
    /// `i64` range). A non-integer `f64` is deliberately *not* captured here: the intended
    /// exact form of a decimal literal like `0.1` is `1/10`, not the binary `f64`'s
    /// `0.1000…0555`, so decimal exactness must enter through the base-10 literal string, not
    /// through this float bridge (a later NUM-5 rung).
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

    /// Wrap an exact result, **but drop the exact sidecar (return `None`) if it would exceed the
    /// size budget** [`MAX_EXACT_POW_BITS`]. `None` is never *wrong* — the `f64` magnitude still
    /// stands; it only means "no exact sidecar this far out".
    ///
    /// This is a DoS guard. Without it, a *linear* formula could grow an *exponential* exact
    /// value: a chain of `let a_{k} = a_{k-1} * a_{k-1}` doubles the denominator's bit length each
    /// step (`a_k = 1 / 2^(2^k)`). The `f64` finiteness guard in the evaluator does not stop this,
    /// because such a value **underflows** the `f64` to `0.0`, which is still *finite* — only the
    /// overflow-to-`∞` direction is caught there. Bounding each result's bit length stops the
    /// accumulation (the old `i128` sidecar bounded it implicitly via `checked_mul` overflow).
    fn bounded(r: BigRational) -> Option<Self> {
        if r.numerator().bit_len().max(r.denominator().bit_len()) > MAX_EXACT_POW_BITS {
            None
        } else {
            Some(Self(r))
        }
    }

    /// Exact sum / difference / product — defined for all inputs (unbounded arithmetic), but the
    /// result is size-guarded (see [`bounded`](Self::bounded)); `Some`/`None` keeps
    /// signature-compatibility with the previously-fallible `i128` sidecar.
    pub fn add(&self, rhs: &Self) -> Option<Self> {
        Self::bounded(&self.0 + &rhs.0)
    }
    pub fn sub(&self, rhs: &Self) -> Option<Self> {
        Self::bounded(&self.0 - &rhs.0)
    }
    pub fn mul(&self, rhs: &Self) -> Option<Self> {
        Self::bounded(&self.0 * &rhs.0)
    }

    /// Exact quotient — `None` when dividing by zero, or when the result exceeds the size budget.
    pub fn div(&self, rhs: &Self) -> Option<Self> {
        self.0.checked_div(&rhs.0).and_then(Self::bounded)
    }

    /// The **labeled lossy** `f64` export (see [`BigRational::to_f64`]).
    pub fn to_f64(&self) -> f64 {
        self.0.to_f64()
    }

    /// The **exact** base-10 expansion as a string, when the value terminates — the rendering
    /// side of ADJ-EXACT-NUMBERS NX-4. `1/4 → "0.25"`; a stored 39-digit π doubled → all 39
    /// fractional digits; `1/3 → None` (a repeating expansion no finite decimal can hold).
    ///
    /// This is the compute-result analogue of NX-2's `Number::Exact` recall rendering: an exact
    /// value is shown *exactly* by default, and the `f64` from [`to_f64`](Self::to_f64) is used
    /// only as the labeled-lossy fallback for the repeating case. See
    /// [`BigDecimal::from_rational_exact`] for the terminating-vs-repeating test.
    pub fn to_exact_decimal_string(&self) -> Option<String> {
        bignum_core::BigDecimal::from_rational_exact(&self.0).map(|d| d.to_string())
    }

    /// Raise to a **non-negative integer** power, exactly (`x^0 = 1`). A rational to a whole
    /// power is itself rational, so `(3/2)^2 = 9/4` stays exact rather than collapsing to the
    /// `f64` `2.25`.
    ///
    /// Returns `None` for a negative or out-of-range exponent (the caller keeps the `f64`
    /// result). The result-size guard [`MAX_EXACT_POW_BITS`] — checked in O(1) *before*
    /// allocating, via [`BigRational::try_pow`] — refuses a pathologically large result, so a
    /// large base with a legal exponent still cannot exhaust memory. This replaces the old
    /// bounded `i128` multiply loop.
    pub fn powi(&self, exp: i128) -> Option<Self> {
        if !(0..=MAX_EXACT_POW).contains(&exp) {
            return None;
        }
        self.0
            .try_pow(exp as i32, MAX_EXACT_POW_BITS)
            .ok()
            .map(Self)
    }
}

/// The largest exponent [`ExactRational::powi`] accepts; beyond it the `f64` magnitude is
/// authoritative. With [`MAX_EXACT_POW_BITS`] it bounds the work an untrusted exponent can
/// request — an algorithmic-DoS guard.
const MAX_EXACT_POW: i128 = 1024;

/// The largest result size (bits of numerator or denominator) [`ExactRational::powi`] will
/// materialize; [`BigRational::try_pow`] refuses in O(1) above it, so even a large base with a
/// legal exponent cannot exhaust memory.
const MAX_EXACT_POW_BITS: u64 = 1_000_000;

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
    /// A **precision narrowing** — `round_to(x, n)` (NUM-6a). Unlike the unary
    /// rounding family in [`ComputeExpr::Unary`] (which snaps to an *integer*),
    /// this rounds `expr` to a stated precision (`spec`) under a stated `mode`,
    /// and is evaluated on the **exact** rational path so the audit records both
    /// the exact source value and the rounded rendering (ADJ-NUMERIC-SUBSTRATE
    /// §4.1–§4.4). It is dimension-preserving, like the unary round family
    /// (`round_to(3.14159 mmol, 2) = 3.14 mmol`). A distinct node — not a
    /// [`ComputeOp`] in [`ComputeExpr::Unary`] — because it carries a precision
    /// and a mode that a bare unary op has nowhere to hold.
    Round {
        spec: RoundSpec,
        mode: RoundingMode,
        expr: Box<ComputeExpr>,
    },
    /// A **scientific-notation formatting** — `to_scientific(x [, figures])`
    /// (NUM-6c). Unlike [`ComputeExpr::Round`] (which narrows to a *number*), this
    /// is a **rendering** op: it narrows `expr` to `figures` significant figures on
    /// the exact path (reusing the 6a/6b `round_sig` machinery), then produces the
    /// normalized `d.ddde±E` string alongside the narrowed numeric value. Both the
    /// exact source and the rendered string land in the audit (ADJ-NUMERIC-SUBSTRATE
    /// §4.1, §4.3), so a checker can re-derive the rendering from the exact value.
    /// `figures ≥ 1`; the default when the surface omits it is resolved at lowering.
    /// Dimension-preserving (the magnitude is reformatted; its unit is untouched).
    ToScientific {
        figures: u32,
        mode: RoundingMode,
        expr: Box<ComputeExpr>,
    },
    /// A **percentage formatting** — `to_percent(x [, places])` (NUM-6c). A rendering
    /// op like [`ComputeExpr::ToScientific`]: it takes `x` as a dimensionless *ratio*
    /// (`0.5 → "50%"`), scales it by 100, rounds to `places` decimal places on the
    /// exact path under `mode`, and renders the fixed-point string with a `%` suffix
    /// (`to_percent(1/3, 2) = "33.33%"`). The narrowed numeric value is the *fraction*
    /// the string denotes (`"33.33%"` → `3333/10000`), so a downstream predicate over
    /// the binding still sees the ratio, and the audit carries both the exact source
    /// and the rendered form (ADJ-NUMERIC-SUBSTRATE §4.1, §4.3). `places ≥ 0`
    /// (`to_percent(x, 0) = "50%"`); the default when the surface omits it is resolved
    /// at lowering. Dimension-preserving.
    ToPercent {
        places: u32,
        mode: RoundingMode,
        expr: Box<ComputeExpr>,
    },
    /// A **currency formatting** — `to_currency(x, code [, places])` (NUM-6c). A rendering
    /// op like [`ComputeExpr::ToPercent`], but it carries a currency **code** string (not
    /// just a numeric precision), so it is a distinct node shape: it renders the money
    /// amount `x` to `places` base-10-exact decimal places under `mode` and prefixes the
    /// stated code (`to_currency(1234.5, USD, 2) = "USD 1234.50"`). The narrowed numeric
    /// value is the rounded amount (`"USD 1234.50"` → `246900/200 = 1234.5`), so a
    /// downstream predicate over the binding still sees the money magnitude, and the audit
    /// carries both the exact source and the rendered form (ADJ-NUMERIC-SUBSTRATE §4.1,
    /// §4.3). `places ≥ 0`; the default (2, the common minor-unit precision) is resolved at
    /// lowering. Dimension-preserving (the `code` is a rendering label, not a re-typing).
    ToCurrency {
        code: String,
        places: u32,
        mode: RoundingMode,
        expr: Box<ComputeExpr>,
    },
}

/// *What* precision a [`ComputeExpr::Round`] narrows to. NUM-6a ships the
/// decimal-**places** form (`round_to(x, n)`); NUM-6b adds a `SigFigures(u32)`
/// variant for `round_sig`, reusing the same node and eval path (only the target
/// scale is derived differently). Kept a named enum, per ADJ-NUMERIC-SUBSTRATE
/// §4.4, so that later variant is an additive change, not a node reshaping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundSpec {
    /// Round to exactly `n` digits after the decimal point. `round_to(x, 0)` is
    /// the precision-parameterized generalization of the integer `Round`.
    Places(u32),
    /// Round to `n` **significant figures** — `round_sig(x, n)` (NUM-6b). Rounding
    /// to `n` sig-figs is rounding to `n − 1 − e` decimal *places*, where `e` is the
    /// base-10 exponent of `x`'s most-significant digit (`e = ⌊log₁₀|x|⌋`); that
    /// place count is derived exactly from the operand's magnitude and handed to the
    /// same exact rounding path as [`RoundSpec::Places`] (the place count may be
    /// negative — `round_sig(31_459, 3) = 31_500`, rounding to the hundreds). `n ≥ 1`.
    SigFigures(u32),
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
    /// A **precision narrowing** node (NUM-6a) — the audit record for a
    /// `round_to(x, n)`. Carries the `spec`/`mode` it rounded under and its single
    /// `operand` subtree (the exact source), so `adj-verify` can re-round the
    /// operand's exact value and confirm the rendered `result`
    /// (ADJ-NUMERIC-SUBSTRATE §4.3: rounding is a first-class, checkable step,
    /// never a silent lossy coercion).
    ///
    /// `operand_exact` is the operand's **exact** source rational (the
    /// [`ExactRational`] the narrowing consumed), captured so an independent checker
    /// can re-round it under the recorded `spec`/`mode` without re-running the whole
    /// formula — the value [`recheck_narrowing`] re-rounds. `None` when the operand
    /// was genuinely inexact (a transcendental with no exact sidecar): the narrowing
    /// then rounded an already-approximate `f64`, so there is no exact source to
    /// re-round and the re-check is honestly [`NarrowingCheck::Unverifiable`].
    Round {
        spec: RoundSpec,
        mode: RoundingMode,
        operand: Box<DerivationNode>,
        operand_exact: Option<ExactRational>,
        result: f64,
    },
    /// A **scientific-notation rendering** node (NUM-6c) — the audit record for a
    /// `to_scientific(x, figures)`. Carries the `figures`/`mode` it narrowed under,
    /// the `rendered` `d.ddde±E` string, and its single `operand` subtree (the exact
    /// source), so `adj-verify` can re-narrow the operand's exact value to `figures`
    /// significant figures and confirm the `rendered` form (ADJ-NUMERIC-SUBSTRATE
    /// §4.3). `result` is the narrowed numeric value (so a downstream predicate over
    /// the binding still sees a number); `rendered` is the boundary form.
    ToScientific {
        figures: u32,
        mode: RoundingMode,
        rendered: String,
        operand: Box<DerivationNode>,
        /// The operand's exact source rational (see [`DerivationNode::Round`]'s
        /// `operand_exact`), re-narrowed by [`recheck_narrowing`] to confirm both
        /// `rendered` and `result`. `None` for a genuinely-inexact operand.
        operand_exact: Option<ExactRational>,
        result: f64,
    },
    /// A **percentage rendering** node (NUM-6c) — the audit record for a
    /// `to_percent(x, places)`. Carries the `places`/`mode` it rounded under, the
    /// `rendered` `d.dd%` string, and its single `operand` subtree (the exact source
    /// ratio), so `adj-verify` can re-scale and re-round the operand's exact value and
    /// confirm the `rendered` form (ADJ-NUMERIC-SUBSTRATE §4.3). `result` is the
    /// narrowed numeric value — the *fraction* the percentage denotes.
    ToPercent {
        places: u32,
        mode: RoundingMode,
        rendered: String,
        operand: Box<DerivationNode>,
        /// The operand's exact source ratio (see [`DerivationNode::Round`]'s
        /// `operand_exact`), re-scaled and re-rounded by [`recheck_narrowing`] to
        /// confirm both `rendered` and `result`. `None` for an inexact operand.
        operand_exact: Option<ExactRational>,
        result: f64,
    },
    /// A **currency rendering** node (NUM-6c) — the audit record for a
    /// `to_currency(x, code, places)`. Carries the `code`/`places`/`mode` it rendered
    /// under, the `rendered` `CODE d.dd` string, and its single `operand` subtree (the
    /// exact source amount), so `adj-verify` can re-round the operand's exact value and
    /// confirm the `rendered` form (ADJ-NUMERIC-SUBSTRATE §4.3). `result` is the narrowed
    /// numeric value — the rounded money amount.
    ToCurrency {
        code: String,
        places: u32,
        mode: RoundingMode,
        rendered: String,
        operand: Box<DerivationNode>,
        /// The operand's exact source amount (see [`DerivationNode::Round`]'s
        /// `operand_exact`), re-rounded by [`recheck_narrowing`] to confirm both
        /// `rendered` and `result`. `None` for an inexact operand.
        operand_exact: Option<ExactRational>,
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
            DerivationNode::Round { result, .. } => *result,
            DerivationNode::ToScientific { result, .. } => *result,
            DerivationNode::ToPercent { result, .. } => *result,
            DerivationNode::ToCurrency { result, .. } => *result,
        }
    }
}

/// A computed value bound to a name, with its full derivation tree and the
/// [`Dimension`] the engine inferred for it (so a predicate firing over a
/// derived value — `csf_ratio <= 0.4` — knows `csf_ratio` is a dimensionless
/// `Scalar`, and the faithfulness gate has rejected any unit-mismatched op).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComputationScope {
    /// Fact ids below this exclusive limit were visible when evaluation ran.
    pub fact_limit: u64,
    /// Derived bindings below this exclusive index were visible.
    pub derived_limit: usize,
}

/// Stable identity assigned when a computed artifact enters a knowledge base.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ComputationId(pub usize);

/// Compiler-owned plan kept separately from the result artifact under audit.
#[derive(Debug, Clone)]
pub(crate) struct ComputationPlan {
    pub expr: ComputeExpr,
    pub scope: ComputationScope,
    pub formula_sources: Vec<crate::Provenance>,
    pub is_query_answer: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Derived {
    /// Assigned by [`KnowledgeBase::add_derived`](crate::KnowledgeBase::add_derived).
    pub(crate) computation_id: Option<ComputationId>,
    pub name: String,
    pub value: f64,
    /// Exact value when the expression stayed inside integer/rational arithmetic.
    pub exact: Option<ExactRational>,
    pub dim: Dimension,
    pub tree: DerivationNode,
    /// The compiler-produced expression that the verifier independently
    /// evaluates. Replaying only `tree` would trust the very operator and
    /// operand structure under audit.
    pub expr: ComputeExpr,
    /// The exact fact/derived prefix visible at the original evaluation point.
    /// This makes latest-observation and rebinding resolution reproducible.
    pub scope: ComputationScope,
    /// Provenance for the *formula* that produced this value, when it came from
    /// APPLYING a provenanced `formula` (ADJ-FORMULA-LIBRARIES rung-0): the
    /// formula's cited `source` / `locator` / `trust`. A plain `let` leaves this
    /// `None` — its audit trail is the derivation `tree` over observed facts, and
    /// there is no library claim to cite. This is the channel by which a computed
    /// answer carries **why** its formula is trustworthy, so an independent
    /// checker can re-verify the citation without the model.
    pub provenance: Option<crate::Provenance>,
    /// Every formula applied to produce this value, outermost first. Unlike
    /// display corroborations, these retain their exact quote and snapshot pins.
    pub formula_sources: Vec<crate::Provenance>,
    /// True only when this binding is the answer to an explicit formula query.
    pub is_query_answer: bool,
}

impl Derived {
    /// Attach the applied formula's provenance (its cited `source`/`locator`/
    /// `trust`). Consumes and returns `self` so it composes with [`compute`]:
    /// `compute(name, expr, kb)?.with_provenance(prov)`.
    pub fn with_provenance(mut self, provenance: crate::Provenance) -> Self {
        self.formula_sources = vec![provenance.clone()];
        self.provenance = Some(provenance);
        self
    }

    /// Attach display provenance and the lossless applied-formula source chain.
    pub fn with_formula_sources(
        mut self,
        provenance: crate::Provenance,
        formula_sources: Vec<crate::Provenance>,
    ) -> Self {
        self.provenance = Some(provenance);
        self.formula_sources = formula_sources;
        self
    }

    /// Mark this derived binding as the answer to an explicit formula query.
    pub fn as_query_answer(mut self) -> Self {
        self.is_query_answer = true;
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
///
/// NUM-5 note: the exact value each `eval` frame carries is now a heap-backed
/// [`ExactRational`] (a `BigRational`) rather than a pair of `i128`s, so each
/// recursive frame is larger. The depth cap is set below the old 256 to keep the
/// same "clean `TooDeep`, never a stack overflow" guarantee on the smaller stacks
/// spawned test threads (and worst-case embedders) use — 128 is still far deeper
/// than any real formula nests.
pub const MAX_EVAL_DEPTH: usize = 128;

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
        computation_id: None,
        name: name.into(),
        value,
        exact,
        dim,
        tree,
        expr: expr.clone(),
        scope: kb.computation_scope(),
        // A plain `let` carries no library-formula provenance; a formula
        // application attaches it afterward via [`Derived::with_provenance`].
        provenance: None,
        formula_sources: Vec::new(),
        is_query_answer: false,
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
                    derived.exact.clone(),
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
                    (Some(a), Some(b)) if b.denominator() == &BigInteger::one() => b
                        .numerator()
                        .to_string()
                        .parse::<i128>()
                        .ok()
                        .and_then(|e| a.powi(e)),
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
                    ComputeOp::Add => a.add(&b),
                    ComputeOp::Sub => a.sub(&b),
                    ComputeOp::Mul => a.mul(&b),
                    ComputeOp::Div => a.div(&b),
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

        ComputeExpr::Round { spec, mode, expr } => eval_round(*spec, *mode, expr, kb, depth),

        ComputeExpr::ToScientific {
            figures,
            mode,
            expr,
        } => eval_to_scientific(*figures, *mode, expr, kb, depth),

        ComputeExpr::ToPercent { places, mode, expr } => {
            eval_to_percent(*places, *mode, expr, kb, depth)
        }

        ComputeExpr::ToCurrency {
            code,
            places,
            mode,
            expr,
        } => eval_to_currency(code, *places, *mode, expr, kb, depth),

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
    // The exact result stays exact — now over unbounded `BigInteger`s. `BigRational` keeps
    // `den > 0`, and `BigInteger::div_rem` truncates toward zero with a remainder that takes
    // the numerator's sign (`num = q·den + rem`, `|rem| < den`), so from that one primitive:
    //   • |num/den|      = the rational's own `abs`;
    //   • ⌊num/den⌋      = q, minus one when `rem < 0` (truncation rounded up for a negative);
    //   • ⌈num/den⌉      = q, plus one when `rem > 0`  (truncation rounded down for a positive);
    //   • ⌊num/den⌉      = round to nearest, TIES AWAY FROM ZERO (matching `f64::round`):
    //                      bump one step outward when `2·|rem| ≥ den`;
    //   • trunc(num/den) = q itself;
    //   • sgn(num/den)   = the numerator's sign.
    // Each non-abs result is an integer, carried as `q/1`.
    let exact = exact.and_then(|r| {
        let n = r.numerator();
        let d = r.denominator(); // always positive
        let int = |i: BigInteger| Some(ExactRational::from_ratio(BigRational::from_integer(i)));
        match op {
            ComputeOp::Abs => Some(ExactRational::from_ratio(r.as_ratio().abs())),
            ComputeOp::Floor => {
                let (q, rem) = n.div_rem(d);
                int(if rem.is_negative() {
                    &q - &BigInteger::one()
                } else {
                    q
                })
            }
            ComputeOp::Ceil => {
                let (q, rem) = n.div_rem(d);
                int(if rem.is_positive() {
                    &q + &BigInteger::one()
                } else {
                    q
                })
            }
            ComputeOp::Round => {
                let (q, rem) = n.div_rem(d);
                let twice = {
                    let a = rem.abs();
                    &a + &a
                };
                if twice >= *d {
                    // fractional part ≥ 1/2 → round away from zero (ties away from zero)
                    int(if n.is_negative() {
                        &q - &BigInteger::one()
                    } else {
                        &q + &BigInteger::one()
                    })
                } else {
                    int(q)
                }
            }
            ComputeOp::Trunc => int(n.div_rem(d).0),
            ComputeOp::Sign => Some(ExactRational::from_i128(n.signum() as i128)),
            _ => None,
        }
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

/// Evaluate a **precision narrowing** — `round_to(x, n)` (NUM-6a). The rounding
/// is done on the **exact** rational path so the audit keeps both the exact
/// source value and its rounded rendering (ADJ-NUMERIC-SUBSTRATE §4.3).
///
/// The exact rounding is *uniform* over terminating and repeating operands. An
/// [`ExactRational`] is `n / d` with `d > 0`; rounding it to `p` decimal places
/// under `mode` is exactly `round(n / d)` carried out by [`BigDecimal::div_round`]
/// — dividing the integer numerator by the integer denominator to scale `p` with
/// the stated rounding — whose result is an exact `BigDecimal` we hand straight
/// back to a `BigRational`. So `1/3` rounds to `0.33` and `2.54` stays `2.54`
/// through the same one path, with no `f64` hop deciding a tie.
///
/// Split out and `#[inline(never)]` for the same stack-frame reason as
/// [`eval_unary`] — see there.
#[inline(never)]
fn eval_round(
    spec: RoundSpec,
    mode: RoundingMode,
    expr: &ComputeExpr,
    kb: &KnowledgeBase,
    depth: usize,
) -> Result<(DerivationNode, Dimension, Option<ExactRational>), ComputeError> {
    let (operand, dim, exact) = eval(expr, kb, depth + 1)?;
    // `round_to` is **dimension-preserving**, exactly like the unary round family:
    // narrowing `3.14159 mmol` to 2 places is `3.14 mmol`, still an amount.
    let exact_out = exact.as_ref().map(|r| round_rational(r, spec, mode));
    // The rendered `f64` is derived FROM the exact rounded value when we have one,
    // so the labeled-lossy export and the exact audit value never disagree. Only a
    // genuinely-inexact operand (a transcendental result with no exact sidecar)
    // falls back to rounding the `f64` directly — itself already approximate.
    let result = match &exact_out {
        Some(r) => r.to_f64(),
        None => round_f64(operand.value(), spec, mode),
    };
    // Rounding a finite value can never produce a non-finite one, but a
    // non-finite operand (an `exp` overflow upstream) must not slip through the
    // narrowing as a clean number — the same "no silently-wrong number" guard the
    // other ops apply.
    if !result.is_finite() {
        return Err(ComputeError::NonFinite {
            op: ComputeOp::Round,
        });
    }
    Ok((
        DerivationNode::Round {
            spec,
            mode,
            operand: Box::new(operand),
            // The operand's exact source — the value `round_rational` above narrowed —
            // captured so `adj-verify` can re-round it independently (§4.3).
            operand_exact: exact,
            result,
        },
        dim,
        exact_out,
    ))
}

/// Round an exact rational to `spec`'s precision under `mode`, staying exact.
///
/// Both forms reduce to "round `n / d` to `places` decimal places" via
/// [`BigDecimal::div_round`] — `Places` uses the stated count directly;
/// `SigFigures` derives it from the operand's magnitude ([`msd_exponent`]). `d` is
/// a rational denominator, always > 0, so the division is total (never the
/// divide-by-zero `div_round` guards against). `places` may be **negative** for a
/// significant-figures rounding of a large number (`round_sig(31_459, 3) → 31_500`,
/// `places = 3 − 1 − 4 = −2`), which `div_round` and `BigDecimal::to_rational`
/// both handle.
fn round_rational(r: &ExactRational, spec: RoundSpec, mode: RoundingMode) -> ExactRational {
    let places: i64 = match spec {
        RoundSpec::Places(p) => p as i64,
        RoundSpec::SigFigures(sig) => {
            // Zero has no significant figures — `round_sig(0, n)` is exactly 0
            // (any other place count would still round 0/d to 0, but short-circuit
            // to avoid a meaningless `msd_exponent(0)`).
            if r.numerator().is_zero() {
                return ExactRational::from_i128(0);
            }
            // n sig-figs = round to (n − 1 − e) decimal places, e = ⌊log₁₀|x|⌋.
            (sig as i64) - 1 - msd_exponent(&r.numerator().abs(), r.denominator())
        }
    };
    let num = BigDecimal::from_integer(r.numerator().clone());
    let den = BigDecimal::from_integer(r.denominator().clone());
    let rounded = num.div_round(&den, places, mode);
    ExactRational::from_ratio(rounded.to_rational())
}

/// `⌊log₁₀(num / den)⌋` for positive integers `num, den` — the base-10 exponent of
/// the most-significant digit of the value. Exact and allocation-cheap: from the
/// decimal digit counts `dn, dd` we have `num/den ∈ (10^(dn−dd−1), 10^(dn−dd+1))`,
/// so the exponent is either `dn − dd` or one less; a single big-integer comparison
/// against `10^e` (moved to the numerator side when `e < 0` to stay in integers)
/// picks the right one. Assumes `num > 0` (the caller short-circuits zero).
fn msd_exponent(num: &BigInteger, den: &BigInteger) -> i64 {
    let e0 = num.to_string().len() as i64 - den.to_string().len() as i64;
    let ten = BigInteger::from_i64(10);
    // Is `num/den ≥ 10^e0`?  ⟺  `num ≥ den·10^e0` (e0 ≥ 0)  or  `num·10^(−e0) ≥ den` (e0 < 0).
    let at_least_e0 = if e0 >= 0 {
        *num >= &den.clone() * &ten.pow(e0 as u32)
    } else {
        &num.clone() * &ten.pow((-e0) as u32) >= *den
    };
    if at_least_e0 {
        e0
    } else {
        e0 - 1
    }
}

/// Evaluate `to_scientific(x, figures)` (NUM-6c): narrow `x` to `figures` significant
/// figures on the exact path and render the normalized scientific-notation string.
/// A **rendering** op — it produces a boundary string (`"6.022e23"`) while keeping the
/// narrowed numeric value for `value()` and the exact sidecar for the audit, so
/// `adj-verify` can re-derive the string from the exact source (ADJ-NUMERIC-SUBSTRATE
/// §4.1, §4.3). Dimension-preserving, like the round family (the magnitude is
/// reformatted; its unit is untouched).
#[inline(never)]
fn eval_to_scientific(
    figures: u32,
    mode: RoundingMode,
    expr: &ComputeExpr,
    kb: &KnowledgeBase,
    depth: usize,
) -> Result<(DerivationNode, Dimension, Option<ExactRational>), ComputeError> {
    let (operand, dim, exact) = eval(expr, kb, depth + 1)?;
    // With an exact sidecar (every rational formula), the rendering and the narrowed
    // exact value are derived TOGETHER from one rounding, so the string and the audit
    // number can never disagree. A genuinely-inexact operand (a transcendental with no
    // exact sidecar) falls back to formatting the already-approximate `f64`.
    let (rendered, exact_out, result) = match &exact {
        Some(r) => {
            let (s, narrowed) = scientific(r, figures, mode);
            let value = narrowed.to_f64();
            (s, Some(narrowed), value)
        }
        None => {
            let value = operand.value();
            // `{:e}` with `figures − 1` fractional digits yields `d.ddde±E`; the
            // operand is already approximate, so this is the labeled-lossy path.
            let s = format!("{:.*e}", (figures.saturating_sub(1)) as usize, value);
            (s, None, value)
        }
    };
    // A non-finite operand (an `exp` overflow upstream) must not render as a clean
    // scientific number — the same "no silently-wrong number" guard the other ops apply.
    // (The op label reuses `Round`: `to_scientific` is a rounding-based narrowing.)
    if !result.is_finite() {
        return Err(ComputeError::NonFinite {
            op: ComputeOp::Round,
        });
    }
    Ok((
        DerivationNode::ToScientific {
            figures,
            mode,
            rendered,
            operand: Box::new(operand),
            // The operand's exact source — re-narrowed by `adj-verify` to reproduce
            // both the rendered string and the numeric result (§4.3).
            operand_exact: exact,
            result,
        },
        dim,
        exact_out,
    ))
}

/// Render an exact rational in normalized scientific notation `d.ddde±E` with exactly
/// `figures` significant figures, and return the **narrowed exact value** alongside — both
/// derived from one rounding so they always agree. `figures ≥ 1`.
///
/// The significant coefficient `C` is `round(|r| · 10^(figures−1−e))` under `mode`, where
/// `e = ⌊log₁₀|r|⌋` ([`msd_exponent`]); `C` has exactly `figures` digits, except when a
/// carry (`9.99 → 10.0`) makes it `10^figures`, which bumps the exponent and resets `C`
/// to `10^(figures−1)`. The mantissa is `C`'s digit string with a point after the first
/// digit (`"6.022"`); the narrowed value is `±C · 10^(e−figures+1)`. Zero renders `"0e0"`.
/// All arithmetic is exact big-integer/-decimal, so no `f64` log or tie-break is involved.
fn scientific(r: &ExactRational, figures: u32, mode: RoundingMode) -> (String, ExactRational) {
    if r.numerator().is_zero() {
        return ("0e0".to_string(), ExactRational::from_i128(0));
    }
    let neg = r.numerator().is_negative();
    let num = r.numerator().abs();
    let den = r.denominator().clone(); // always > 0 (canonical rational)
    let mut e = msd_exponent(&num, &den);

    // C = round(|r| · 10^(figures−1−e)) to the nearest integer under `mode`, computed as an
    // exact integer division via `div_round` to scale 0 (so half-even etc. are honored).
    let p: i64 = figures as i64 - 1 - e;
    let (dividend, divisor) = if p >= 0 {
        (&num * &ten_to(p as u32), den)
    } else {
        (num, &den * &ten_to((-p) as u32))
    };
    let c_bd =
        BigDecimal::from_integer(dividend).div_round(&BigDecimal::from_integer(divisor), 0, mode);
    let mut c = bigdecimal_to_integer(&c_bd);

    // Rounding carry: `9.99…` at 3 figs rounds to `10.0…` = `10^figures`. Bump the
    // exponent and collapse the coefficient back to `figures` digits (`10^(figures−1)`).
    if c >= ten_to(figures) {
        c = ten_to(figures - 1);
        e += 1;
    }

    let digits = c.to_string(); // exactly `figures` digits (`c ≥ 0`, in `[10^(f−1), 10^f)`)
    let mantissa = if figures == 1 {
        digits.clone()
    } else {
        format!("{}.{}", &digits[..1], &digits[1..])
    };
    let sign = if neg { "-" } else { "" };
    let rendered = format!("{sign}{mantissa}e{e}");

    // Narrowed exact value = ±C · 10^(e − figures + 1) (the place value of the last digit).
    let place = e - figures as i64 + 1;
    let signed_c = if neg { -&c } else { c };
    let narrowed = if place >= 0 {
        ExactRational::from_ratio(BigRational::from_integer(&signed_c * &ten_to(place as u32)))
    } else {
        ExactRational::from_ratio(BigRational::new(signed_c, ten_to((-place) as u32)))
    };
    (rendered, narrowed)
}

/// `10^n` as a [`BigInteger`]. A thin wrapper over [`BigInteger::pow`] used by the
/// scientific-notation renderer; `n` is bounded by the operand's own magnitude (the same
/// materialization `round_sig` already performs), not by any new user-controlled amplifier.
fn ten_to(n: u32) -> BigInteger {
    BigInteger::from_i64(10).pow(n)
}

/// The integer value of a `BigDecimal` known to be integral (produced by a `div_round` to
/// scale 0). `value = mant · 10^(−scale)`, and an integral value normalizes to `scale ≤ 0`,
/// so this is `mant · 10^|scale|`.
fn bigdecimal_to_integer(bd: &BigDecimal) -> BigInteger {
    let scale = bd.scale();
    if scale < 0 {
        bd.mantissa() * &ten_to((-scale) as u32)
    } else {
        bd.mantissa().clone()
    }
}

/// Evaluate `to_percent(x, places)` (NUM-6c): render the dimensionless ratio `x` as a
/// percentage to `places` decimal places (`0.5 → "50%"`, `1/3 → "33.33%"` at 2 places).
/// A **rendering** op — the narrowed numeric value is the *fraction* the string denotes
/// (so a downstream predicate over the binding still sees the ratio) and the exact sidecar
/// backs the audit, per ADJ-NUMERIC-SUBSTRATE §4.1, §4.3. Dimension-preserving.
#[inline(never)]
fn eval_to_percent(
    places: u32,
    mode: RoundingMode,
    expr: &ComputeExpr,
    kb: &KnowledgeBase,
    depth: usize,
) -> Result<(DerivationNode, Dimension, Option<ExactRational>), ComputeError> {
    let (operand, dim, exact) = eval(expr, kb, depth + 1)?;
    let (rendered, exact_out, result) = match &exact {
        Some(r) => {
            let (s, narrowed) = percent(r, places, mode);
            let value = narrowed.to_f64();
            (s, Some(narrowed), value)
        }
        None => {
            // Already-inexact operand (a transcendental ratio): format the lossy `f64`
            // scaled to a percentage. `result` stays the fraction the string denotes.
            let value = operand.value();
            let s = format!("{:.*}%", places as usize, value * 100.0);
            (s, None, value)
        }
    };
    // A non-finite operand must not render as a clean percentage — same guard as the
    // other ops (op label reuses `Round`: `to_percent` is a rounding-based narrowing).
    if !result.is_finite() {
        return Err(ComputeError::NonFinite {
            op: ComputeOp::Round,
        });
    }
    Ok((
        DerivationNode::ToPercent {
            places,
            mode,
            rendered,
            operand: Box::new(operand),
            // The operand's exact source ratio — re-scaled and re-rounded by
            // `adj-verify` to reproduce the percentage string and result (§4.3).
            operand_exact: exact,
            result,
        },
        dim,
        exact_out,
    ))
}

/// Render an exact ratio as a fixed-point percentage string with exactly `places` decimal
/// places and a `%` suffix, and return the **narrowed fraction** alongside — both from one
/// rounding so they always agree. The percentage magnitude is `x · 100`, so the scaled
/// integer is `C = round(x · 10^(places+2))` under `mode`; the string places the decimal
/// point `places` from `C`'s right (`"33.33%"`), and the narrowed fraction is `C / 10^(places+2)`
/// (`= "33.33%" / 100`). Zero and `places = 0` are handled by the same padding path. All
/// arithmetic is exact big-integer/-decimal.
fn percent(r: &ExactRational, places: u32, mode: RoundingMode) -> (String, ExactRational) {
    // `C = round(r · 10^(places+2))` — the percentage's digits scaled to an integer.
    let scale_pow = places + 2;
    let dividend = r.numerator() * &ten_to(scale_pow);
    let c_bd = BigDecimal::from_integer(dividend).div_round(
        &BigDecimal::from_integer(r.denominator().clone()),
        0,
        mode,
    );
    let c = bigdecimal_to_integer(&c_bd);
    let neg = c.is_negative();
    let body = fixed_decimal_body(c.abs().to_string(), places);
    let sign = if neg { "-" } else { "" };
    let rendered = format!("{sign}{body}%");

    // Narrowed fraction = C / 10^(places+2) (the percentage magnitude divided back by 100).
    let narrowed = ExactRational::from_ratio(BigRational::new(c, ten_to(scale_pow)));
    (rendered, narrowed)
}

/// Format a non-negative integer's digit string `mag` as a fixed-point decimal with exactly
/// `places` fractional digits: it places the decimal point `places` from the right, padding on
/// the left so there is always at least one integer digit (`"0.05"`, not `".05"`). `places = 0`
/// returns the integer digits unchanged. Shared by [`percent`] and [`currency`] — the only
/// difference between those renderers is the scale factor and the suffix/prefix around this body.
fn fixed_decimal_body(mag: String, places: u32) -> String {
    if places == 0 {
        return mag;
    }
    let p = places as usize;
    let mag = if mag.len() <= p {
        format!("{}{}", "0".repeat(p + 1 - mag.len()), mag)
    } else {
        mag
    };
    let split = mag.len() - p;
    format!("{}.{}", &mag[..split], &mag[split..])
}

/// Evaluate `to_currency(x, code, places)` (NUM-6c): render the money amount `x` to `places`
/// base-10-exact decimal places, prefixed with the currency `code` (`"USD 1234.50"`). A
/// **rendering** op — the narrowed numeric value is the rounded amount (so a downstream
/// predicate over the binding still sees the money magnitude) and the exact sidecar backs the
/// audit (ADJ-NUMERIC-SUBSTRATE §4.1, §4.3). Dimension-preserving.
#[inline(never)]
fn eval_to_currency(
    code: &str,
    places: u32,
    mode: RoundingMode,
    expr: &ComputeExpr,
    kb: &KnowledgeBase,
    depth: usize,
) -> Result<(DerivationNode, Dimension, Option<ExactRational>), ComputeError> {
    let (operand, dim, exact) = eval(expr, kb, depth + 1)?;
    let (rendered, exact_out, result) = match &exact {
        Some(r) => {
            let (amount, narrowed) = currency(r, places, mode);
            let value = narrowed.to_f64();
            (format!("{code} {amount}"), Some(narrowed), value)
        }
        None => {
            // Already-inexact operand: format the lossy `f64` to `places` decimals.
            let value = operand.value();
            (format!("{code} {:.*}", places as usize, value), None, value)
        }
    };
    // A non-finite operand must not render as a clean money string — same guard as the
    // other ops (op label reuses `Round`: `to_currency` is a rounding-based narrowing).
    if !result.is_finite() {
        return Err(ComputeError::NonFinite {
            op: ComputeOp::Round,
        });
    }
    Ok((
        DerivationNode::ToCurrency {
            code: code.to_string(),
            places,
            mode,
            rendered,
            operand: Box::new(operand),
            // The operand's exact source amount — re-rounded by `adj-verify` to
            // reproduce the money string and result (§4.3).
            operand_exact: exact,
            result,
        },
        dim,
        exact_out,
    ))
}

/// Render an exact money amount as a fixed-point decimal string with exactly `places` decimal
/// places (no currency code — the caller prefixes it), and return the **narrowed amount**
/// alongside — both from one rounding so they always agree. The scaled integer is
/// `C = round(x · 10^places)` under `mode`; the string places the decimal point `places` from
/// `C`'s right (`1234.5 → "1234.50"` at 2 places), and the narrowed amount is `C / 10^places`.
/// Zero and `places = 0` are handled by the shared padding path. All arithmetic is exact
/// big-integer/-decimal — base-10-exact money, no `f64` hop.
fn currency(r: &ExactRational, places: u32, mode: RoundingMode) -> (String, ExactRational) {
    // `C = round(r · 10^places)` — the amount's digits scaled to an integer.
    let dividend = r.numerator() * &ten_to(places);
    let c_bd = BigDecimal::from_integer(dividend).div_round(
        &BigDecimal::from_integer(r.denominator().clone()),
        0,
        mode,
    );
    let c = bigdecimal_to_integer(&c_bd);
    let neg = c.is_negative();
    let body = fixed_decimal_body(c.abs().to_string(), places);
    let sign = if neg { "-" } else { "" };
    let amount = format!("{sign}{body}");

    // Narrowed amount = C / 10^places.
    let narrowed = ExactRational::from_ratio(BigRational::new(c, ten_to(places)));
    (amount, narrowed)
}

// ---------------------------------------------------------------------------
// NUM-6 audit re-check — rounding/formatting is a first-class, checkable step
// ---------------------------------------------------------------------------

/// The verdict of independently re-checking one NUM-6 narrowing node
/// (`round_to` / `round_sig` / `to_scientific` / `to_percent` / `to_currency`)
/// against the exact source it recorded (ADJ-NUMERIC-SUBSTRATE §4.3).
///
/// A trail is **testimony**: the engine describing its own rounding. The audit
/// records the exact source, the target precision, the rounding mode, and the
/// rendered result — but a confidently-wrong (or since-tampered) artifact prints
/// a plausible rounded number just as fluently. [`recheck_narrowing`] turns that
/// testimony into **evidence**: it re-runs the *same* exact narrowing on the
/// recorded exact source under the recorded spec/mode and confirms the recorded
/// result (and rendered string, for the formatters) reproduces.
#[derive(Debug, Clone, PartialEq)]
pub enum NarrowingCheck {
    /// The node is not a narrowing (`Leaf`/`Lit`/`Op`/…): nothing to re-round.
    NotANarrowing,
    /// Re-narrowing the recorded exact source reproduced the recorded result and
    /// rendered form exactly — the narrowing is sound.
    ReChecked,
    /// The recorded result/rendering did **not** reproduce from the exact source
    /// under the recorded spec/mode: a drifted or tampered narrowing. Carries a
    /// stable machine-readable reason and both the `recorded` and `recomputed`
    /// forms so a reviewer can see the disagreement.
    Mismatch {
        why: &'static str,
        recorded: String,
        recomputed: String,
    },
    /// No exact source was carried (`operand_exact == None`): the operand was
    /// genuinely inexact (a transcendental with no exact sidecar), so the engine
    /// rounded an already-approximate `f64` and there is nothing exact to re-round.
    /// Honest — never counted as a pass, never a hard failure (§4.3 can only make
    /// the promise it can keep: an exact source is re-checkable, an approximate one
    /// is labeled unverifiable).
    Unverifiable,
}

impl NarrowingCheck {
    /// Whether this node's narrowing was affirmatively re-derived. `false` for a
    /// non-narrowing node, an unverifiable (inexact-source) narrowing, or a
    /// mismatch — so a caller counting "how many narrowings did I confirm" only
    /// counts the ones actually re-rounded.
    pub fn is_rechecked(&self) -> bool {
        matches!(self, NarrowingCheck::ReChecked)
    }

    /// Whether this is a hard failure — a narrowing whose recorded output does not
    /// reproduce from its exact source. A non-narrowing / unverifiable / rechecked
    /// verdict is not a failure.
    pub fn is_mismatch(&self) -> bool {
        matches!(self, NarrowingCheck::Mismatch { .. })
    }
}

/// Re-check a **single** [`DerivationNode`] if it is a NUM-6 narrowing: re-round /
/// re-format its recorded exact source ([`operand_exact`](DerivationNode::Round))
/// under the recorded `spec`/`mode` and confirm the recorded `result` (and
/// `rendered` string, for the formatters) reproduces (ADJ-NUMERIC-SUBSTRATE §4.3).
///
/// This calls the *same* exact narrowing primitives the engine used to produce the
/// node ([`round_rational`], [`scientific`], [`percent`], [`currency`]) — the point
/// is a second, independent evaluation from the recorded exact source, not a second
/// copy of the arithmetic. Because those primitives are total and deterministic, a
/// [`NarrowingCheck::Mismatch`] can only mean the recorded output did not come from
/// the recorded source under the recorded mode — exactly the "valid-looking number
/// from an invented rounding" this re-check exists to catch.
///
/// Does **not** recurse — see [`recheck_narrowings`] for the whole-tree walk.
pub fn recheck_narrowing(node: &DerivationNode) -> NarrowingCheck {
    match node {
        DerivationNode::Round {
            spec,
            mode,
            operand_exact,
            result,
            ..
        } => match operand_exact {
            None => NarrowingCheck::Unverifiable,
            Some(src) => {
                let recomputed = round_rational(src, *spec, *mode).to_f64();
                // The recorded `result` is `round_rational(src, …).to_f64()` from the
                // emit path; re-running the same exact rounding reproduces the same
                // `f64` bit-for-bit, so equality (not tolerance) is the honest test.
                if recomputed == *result {
                    NarrowingCheck::ReChecked
                } else {
                    NarrowingCheck::Mismatch {
                        why: "result_differs",
                        recorded: format!("{result}"),
                        recomputed: format!("{recomputed}"),
                    }
                }
            }
        },
        DerivationNode::ToScientific {
            figures,
            mode,
            rendered,
            operand_exact,
            result,
            ..
        } => match operand_exact {
            None => NarrowingCheck::Unverifiable,
            Some(src) => {
                let (s, narrowed) = scientific(src, *figures, *mode);
                check_rendered(&s, narrowed.to_f64(), rendered, *result)
            }
        },
        DerivationNode::ToPercent {
            places,
            mode,
            rendered,
            operand_exact,
            result,
            ..
        } => match operand_exact {
            None => NarrowingCheck::Unverifiable,
            Some(src) => {
                let (s, narrowed) = percent(src, *places, *mode);
                check_rendered(&s, narrowed.to_f64(), rendered, *result)
            }
        },
        DerivationNode::ToCurrency {
            code,
            places,
            mode,
            rendered,
            operand_exact,
            result,
            ..
        } => match operand_exact {
            None => NarrowingCheck::Unverifiable,
            Some(src) => {
                let (amount, narrowed) = currency(src, *places, *mode);
                // The node's `rendered` is the code-prefixed form; reconstruct it the
                // same way the emit path did (`"{code} {amount}"`).
                check_rendered(&format!("{code} {amount}"), narrowed.to_f64(), rendered, *result)
            }
        },
        _ => NarrowingCheck::NotANarrowing,
    }
}

/// Shared verdict for the three **formatter** narrowings: confirm both the
/// re-derived rendered string and the re-derived numeric result reproduce what the
/// node recorded. The rendered string is checked first because it is the boundary
/// artifact a consumer reads; a value that reproduces under a *different* string
/// (or vice versa) is still a mismatch, so both must hold for `ReChecked`.
fn check_rendered(
    recomputed_rendered: &str,
    recomputed_result: f64,
    recorded_rendered: &str,
    recorded_result: f64,
) -> NarrowingCheck {
    if recomputed_rendered != recorded_rendered {
        NarrowingCheck::Mismatch {
            why: "rendered_differs",
            recorded: recorded_rendered.to_string(),
            recomputed: recomputed_rendered.to_string(),
        }
    } else if recomputed_result != recorded_result {
        NarrowingCheck::Mismatch {
            why: "result_differs",
            recorded: format!("{recorded_result}"),
            recomputed: format!("{recomputed_result}"),
        }
    } else {
        NarrowingCheck::ReChecked
    }
}

/// Walk a derivation `tree` in pre-order and re-check **every** NUM-6 narrowing node
/// it contains, returning one `(depth, verdict)` per narrowing found (a narrowing can
/// be nested inside another node's operand — `round_to(to_percent(x))` — so the walk
/// recurses through operands). Non-narrowing nodes contribute nothing. The `depth` is
/// the node's nesting level under `tree` (root = 0), so a caller can point a reviewer
/// at *which* narrowing in a multi-step formula failed.
pub fn recheck_narrowings(tree: &DerivationNode) -> Vec<(usize, NarrowingCheck)> {
    let mut out = Vec::new();
    collect_narrowings(tree, 0, &mut out);
    out
}

/// Pre-order helper for [`recheck_narrowings`]: re-check `node` if it is a narrowing,
/// then recurse into its children. Depth-bounded implicitly by the tree the engine
/// built (a compute expression is capped at [`MAX_EVAL_DEPTH`] when produced), so this
/// cannot recurse deeper than a value the engine already accepted.
fn collect_narrowings(node: &DerivationNode, depth: usize, out: &mut Vec<(usize, NarrowingCheck)>) {
    let verdict = recheck_narrowing(node);
    if !matches!(verdict, NarrowingCheck::NotANarrowing) {
        out.push((depth, verdict));
    }
    match node {
        DerivationNode::Op { operands, .. } => {
            for child in operands {
                collect_narrowings(child, depth + 1, out);
            }
        }
        DerivationNode::Round { operand, .. }
        | DerivationNode::ToScientific { operand, .. }
        | DerivationNode::ToPercent { operand, .. }
        | DerivationNode::ToCurrency { operand, .. } => {
            collect_narrowings(operand, depth + 1, out);
        }
        DerivationNode::Leaf { .. }
        | DerivationNode::DerivedRef { .. }
        | DerivationNode::Lit { .. } => {}
    }
}

/// Round an `f64` to `spec`'s precision — the fallback for an operand that carried
/// no exact value (already inexact, e.g. a transcendental result). It scales,
/// rounds to the nearest integer, and unscales; on this already-lossy path the tie
/// rule is `f64::round`'s ties-away rather than `mode`, which is acceptable because
/// the value is approximate to begin with (the exact path, which every rational
/// formula takes, honors `mode` precisely).
fn round_f64(value: f64, spec: RoundSpec, _mode: RoundingMode) -> f64 {
    let places: i32 = match spec {
        RoundSpec::Places(p) => p as i32,
        RoundSpec::SigFigures(sig) => {
            if value == 0.0 || !value.is_finite() {
                return value;
            }
            (sig as i32) - 1 - value.abs().log10().floor() as i32
        }
    };
    let factor = 10f64.powi(places);
    (value * factor).round() / factor
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
        let d = compute("m", &bin(ComputeOp::Mod, refexpr("a"), refexpr("b")), &kb).unwrap();
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
            &bin(
                ComputeOp::Mod,
                ComputeExpr::Lit(-7.0),
                ComputeExpr::Lit(3.0),
            ),
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
    fn exact_arithmetic_is_size_bounded_against_a_squaring_chain() {
        // Regression guard for the NUM-5 BigRational swap: `a_k = a_{k-1} · a_{k-1}` doubles the
        // exact value's bit length each step, so a *linear* number of multiplications would grow
        // an *exponential* exact rational (`3^(2^k)`). The f64 finiteness guard does NOT stop a
        // shrinking variant (it underflows to a finite 0.0), so the size guard on the exact
        // result must drop the sidecar to `None` before it explodes. Without the guard this loop
        // would allocate a `3^(2^40)`-bit integer and OOM; with it, it terminates quickly.
        let mut a = ExactRational::from_i128(3);
        let mut dropped_at = None;
        for k in 0..40 {
            match a.mul(&a) {
                Some(next) => a = next,
                None => {
                    dropped_at = Some(k);
                    break;
                }
            }
        }
        assert!(
            dropped_at.is_some(),
            "exact squaring chain must be size-bounded (dropped to None), not grow unboundedly"
        );
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
    fn stored_exact_decimal_pi_doubles_exactly_with_no_f64_hop() {
        // The proof of NX-3: a high-precision constant stored EXACTLY survives arithmetic.
        // The stdlib ships pi to 39 digits. If the compute engine ingested it through an
        // `f64` hop, `pi + pi` would collapse to the ~16-digit `6.283185307179586`. NX-3
        // ingests the `BigDecimal` as its true `mantissa / 10^scale` rational, so all 39
        // fractional digits survive doubling.
        use logic_core::{Number, Term};
        use std::str::FromStr;

        let pi_str = "3.141592653589793238462643383279502884197";
        let pi_decimal = bignum_core::BigDecimal::from_str(pi_str).unwrap();

        // Store pi as an EXACT valued fact `pi(3.14159…197)` — a `Number::Exact`, not an f64.
        let kb = kb_with(vec![crate::Fact::certain(compound(
            "pi",
            vec![Term::Num(Number::Exact(pi_decimal.clone()))],
        ))]);

        // Double it through the deterministic compute engine: pi + pi.
        let d = compute(
            "two_pi",
            &bin(ComputeOp::Add, refexpr("pi"), refexpr("pi")),
            &kb,
        )
        .unwrap();

        // The human-readable exact expectation: 2·pi rendered to its full 40-digit decimal
        // via BigDecimal's own exact addition — no rounding anywhere.
        let doubled_decimal = &pi_decimal + &pi_decimal;
        assert_eq!(
            doubled_decimal.to_string(),
            "6.283185307179586476925286766559005768394",
            "BigDecimal exact doubling must keep every digit"
        );

        // The engine's exact sidecar must equal that exact value — NOT the f64-rounded one.
        let exact = d
            .exact
            .expect("a stored Number::Exact fact must populate the exact sidecar");
        assert_eq!(
            exact,
            ExactRational::from_ratio(doubled_decimal.to_rational()),
            "compute must ingest the stored decimal exactly (no f64 hop)"
        );

        // And in lowest terms: 2·(N/10^39) = N / (5·10^38), so the numerator is pi's exact
        // 40-digit mantissa and the denominator is 5·10^38 (39 digits). This pins the precise
        // ratio the engine now holds.
        assert_eq!(
            exact.numerator().to_string(),
            "3141592653589793238462643383279502884197"
        );
        assert_eq!(
            exact.denominator().to_string(),
            "500000000000000000000000000000000000000" // 5 × 10^38 (39 digits)
        );

        // Guard the regression this PR fixes: the exact value is strictly better than the
        // f64 hop. The old NX-2 path folded through `to_f64()`, losing everything past the
        // 16th significant digit; the exact denominator here has 39 digits.
        assert_ne!(
            exact,
            ExactRational::from_integer_f64((pi_decimal.to_f64() * 2.0).trunc())
                .unwrap_or_else(|| ExactRational::from_i128(0)),
            "the exact sidecar must not degrade to the truncated f64 value"
        );

        // NX-4 — the RENDERING half: the exact sidecar prints all 39 fractional digits, where
        // `to_f64` would collapse to the ~16 a binary float carries. This is the string the CLI
        // now emits for a computed exact result.
        assert_eq!(
            exact.to_exact_decimal_string().as_deref(),
            Some("6.283185307179586476925286766559005768394"),
            "a computed exact result renders with every digit, not the f64 export"
        );
        assert_ne!(
            exact.to_exact_decimal_string().as_deref(),
            Some(format!("{}", exact.to_f64()).as_str()),
            "the exact rendering is strictly richer than the lossy f64 rendering"
        );
    }

    #[test]
    fn exact_decimal_string_is_none_for_repeating_expansions() {
        // A quotient like 1/3 has no finite decimal; the render path must fall back to the
        // labeled-lossy f64 rather than loop forever or fabricate a truncation.
        let third = ExactRational::new(1, 3).unwrap();
        assert_eq!(third.to_exact_decimal_string(), None);
        // But a terminating quotient (3/4) renders exactly.
        let three_quarters = ExactRational::new(3, 4).unwrap();
        assert_eq!(
            three_quarters.to_exact_decimal_string().as_deref(),
            Some("0.75")
        );
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

    // ---- NUM-6a: round_to(x, n) — the precision narrowing ----

    fn round_places(n: u32, inner: ComputeExpr) -> ComputeExpr {
        ComputeExpr::Round {
            spec: RoundSpec::Places(n),
            mode: RoundingMode::HalfEven,
            expr: Box::new(inner),
        }
    }
    fn frac(a: i64, b: i64) -> ComputeExpr {
        ComputeExpr::Bin(
            ComputeOp::Div,
            Box::new(ComputeExpr::Lit(a as f64)),
            Box::new(ComputeExpr::Lit(b as f64)),
        )
    }
    fn round_sig(n: u32, inner: ComputeExpr) -> ComputeExpr {
        ComputeExpr::Round {
            spec: RoundSpec::SigFigures(n),
            mode: RoundingMode::HalfEven,
            expr: Box::new(inner),
        }
    }

    // ---- NUM-6b: round_sig — the significant-figures narrowing ----

    #[test]
    fn msd_exponent_is_exact_across_magnitudes() {
        // ⌊log₁₀(num/den)⌋ for a spread of values, incl. the boundary cases (exact
        // powers of ten, values just under a power, sub-1 values).
        let e = |num: i64, den: i64| {
            super::msd_exponent(&BigInteger::from_i64(num), &BigInteger::from_i64(den))
        };
        assert_eq!(e(314159, 1000), 2); // 314.159 → MSD at 10^2
        assert_eq!(e(314, 100), 0); // 3.14 → 10^0
        assert_eq!(e(1, 1), 0); // 1 → 10^0
        assert_eq!(e(999, 100), 0); // 9.99 → 10^0 (just under 10^1)
        assert_eq!(e(1000, 1), 3); // 1000 → 10^3 (exact power)
        assert_eq!(e(1, 2), -1); // 0.5 → 10^-1
        assert_eq!(e(1, 1000), -3); // 0.001 → 10^-3 (exact power)
        assert_eq!(e(9, 10000), -4); // 0.0009 → 10^-4
        assert_eq!(e(314, 100000), -3); // 0.00314 → 10^-3
    }

    #[test]
    fn round_sig_rounds_a_large_integer_to_the_hundreds_exactly() {
        let kb = KnowledgeBase::new();
        // 31459 to 3 significant figures = 31500 (place count −2 — rounding to the
        // hundreds — which the exact path handles). Held exactly as 31500/1.
        let d = compute("r", &round_sig(3, ComputeExpr::Lit(31459.0)), &kb).unwrap();
        assert_eq!(d.value, 31500.0);
        assert_eq!(d.exact, ExactRational::new(31500, 1));
    }

    #[test]
    fn round_sig_rounds_fractional_values_exactly_across_scales() {
        let kb = KnowledgeBase::new();
        // 3.14159 (314159/100000) to 3 sig-figs = 3.14 = 157/50.
        let a = compute("r", &round_sig(3, frac(314159, 100000)), &kb).unwrap();
        assert_eq!(a.exact, ExactRational::new(157, 50));
        // 0.00314159 to 2 sig-figs = 0.0031 = 31/10000 (leading zeros don't count).
        let b = compute("r", &round_sig(2, frac(314159, 100_000_000)), &kb).unwrap();
        assert_eq!(b.exact, ExactRational::new(31, 10000));
    }

    #[test]
    fn round_sig_of_zero_is_zero() {
        let kb = KnowledgeBase::new();
        let d = compute("r", &round_sig(3, ComputeExpr::Lit(0.0)), &kb).unwrap();
        assert_eq!(d.exact, ExactRational::new(0, 1));
        assert_eq!(d.value, 0.0);
    }

    #[test]
    fn round_to_places_rounds_a_repeating_rational_and_stays_exact() {
        let kb = KnowledgeBase::new();
        // 1/3 = 0.333… → 2 places = 0.33 = 33/100, EXACTLY (no f64 hop). The whole
        // point: the audit value is the exact fraction, not a lossy 0.33000000004.
        let d = compute("r", &round_places(2, frac(1, 3)), &kb).unwrap();
        assert!((d.value - 0.33).abs() < 1e-12);
        assert_eq!(d.exact, ExactRational::new(33, 100));
        // 2/3 = 0.666… → 0.67 = 67/100 (rounds up, away from the truncation).
        let d2 = compute("r", &round_places(2, frac(2, 3)), &kb).unwrap();
        assert_eq!(d2.exact, ExactRational::new(67, 100));
    }

    #[test]
    fn round_to_breaks_ties_to_even_not_away_from_zero() {
        let kb = KnowledgeBase::new();
        // 5/2 = 2.5 → nearest EVEN = 2. Ties-away (`f64::round`) would give 3, so
        // this pins the half-even default distinct from the legacy integer Round.
        let a = compute("r", &round_places(0, frac(5, 2)), &kb).unwrap();
        assert_eq!(a.exact, ExactRational::new(2, 1));
        assert_eq!(a.value, 2.0);
        // 7/2 = 3.5 → nearest even = 4.
        let b = compute("r", &round_places(0, frac(7, 2)), &kb).unwrap();
        assert_eq!(b.exact, ExactRational::new(4, 1));
    }

    #[test]
    fn round_to_is_exact_on_an_already_terminating_value() {
        let kb = KnowledgeBase::new();
        // 15/4 = 3.75 → 3 places is unchanged (adding places is exact); the value
        // stays 15/4, not a re-parsed 3.75.
        let d = compute("r", &round_places(3, frac(15, 4)), &kb).unwrap();
        assert_eq!(d.exact, ExactRational::new(15, 4));
        assert_eq!(d.value, 3.75);
    }

    #[test]
    fn round_to_preserves_dimension_and_records_precision_mode_and_operand() {
        // Round a dimensioned money value: 10/3 usd → 2 places = 3.33 usd. The
        // unit must survive (rounding narrows the magnitude, not the dimension),
        // and the audit node must carry the precision, mode, and operand subtree.
        let kb = kb_with(vec![money("bal", 10, "usd")]);
        let expr = round_places(
            2,
            ComputeExpr::Bin(
                ComputeOp::Div,
                Box::new(refexpr("bal")),
                Box::new(ComputeExpr::Lit(3.0)),
            ),
        );
        let d = compute("r", &expr, &kb).unwrap();
        assert_eq!(d.exact, ExactRational::new(333, 100));
        assert_eq!(d.dim, Dimension::Money("usd".into()));
        match &d.tree {
            DerivationNode::Round {
                spec,
                mode,
                result,
                operand,
                operand_exact,
            } => {
                assert_eq!(*spec, RoundSpec::Places(2));
                assert_eq!(*mode, RoundingMode::HalfEven);
                assert!((*result - 3.33).abs() < 1e-12);
                // The operand subtree is the exact source the narrowing rounded —
                // 10/3 usd, a division node — so a checker can re-round it.
                assert!(matches!(operand.as_ref(), DerivationNode::Op { .. }));
                // And the exact source is captured on the node itself (NUM-6v), so
                // `adj-verify` can re-round 10/3 without re-walking the subtree.
                assert_eq!(*operand_exact, ExactRational::new(10, 3));
            }
            other => panic!("expected Round node, got {other:?}"),
        }
    }

    // ---- NUM-6c: to_scientific(x, figures) — the scientific-notation rendering ----

    fn to_sci(figures: u32, inner: ComputeExpr) -> ComputeExpr {
        ComputeExpr::ToScientific {
            figures,
            mode: RoundingMode::HalfEven,
            expr: Box::new(inner),
        }
    }
    fn rendered_of(d: &Derived) -> String {
        match &d.tree {
            DerivationNode::ToScientific { rendered, .. } => rendered.clone(),
            other => panic!("expected ToScientific node, got {other:?}"),
        }
    }

    #[test]
    fn to_scientific_renders_and_narrows_across_scales() {
        let kb = KnowledgeBase::new();
        // A large integer: 31459 to 3 sig-figs = 31500 = 3.15e4 (the 59 rounds the 4 up).
        let a = compute("r", &to_sci(3, ComputeExpr::Lit(31459.0)), &kb).unwrap();
        assert_eq!(rendered_of(&a), "3.15e4");
        assert_eq!(a.exact, ExactRational::new(31500, 1));
        // A repeating rational: 1/3 to 4 sig-figs = 0.3333 = 3.333e-1, EXACTLY.
        let b = compute("r", &to_sci(4, frac(1, 3)), &kb).unwrap();
        assert_eq!(rendered_of(&b), "3.333e-1");
        assert_eq!(b.exact, ExactRational::new(3333, 10000));
        // A sub-1 terminating value: 3.14159 to 3 sig-figs = 3.14 = 157/50 = 3.14e0.
        let c = compute("r", &to_sci(3, frac(314159, 100000)), &kb).unwrap();
        assert_eq!(rendered_of(&c), "3.14e0");
        assert_eq!(c.exact, ExactRational::new(157, 50));
    }

    #[test]
    fn to_scientific_handles_rounding_carry_into_a_new_exponent() {
        let kb = KnowledgeBase::new();
        // 999 to 2 sig-figs rounds 9.99e2 UP to 1.0e3 — the carry must bump the
        // exponent and keep exactly `figures` mantissa digits (`"1.0"`, not `"10"`).
        let d = compute("r", &to_sci(2, ComputeExpr::Lit(999.0)), &kb).unwrap();
        assert_eq!(rendered_of(&d), "1.0e3");
        assert_eq!(d.exact, ExactRational::new(1000, 1));
    }

    #[test]
    fn to_scientific_handles_sign_single_figure_and_zero() {
        let kb = KnowledgeBase::new();
        // Negative, 3 figs: −1/8 = −0.125 → −1.25e−1, narrowed value exactly −1/8.
        let neg = compute("r", &to_sci(3, frac(-1, 8)), &kb).unwrap();
        assert_eq!(rendered_of(&neg), "-1.25e-1");
        assert_eq!(neg.exact, ExactRational::new(-1, 8));
        // A single significant figure has no decimal point: 602 → 6e2.
        let one = compute("r", &to_sci(1, ComputeExpr::Lit(602.0)), &kb).unwrap();
        assert_eq!(rendered_of(&one), "6e2");
        assert_eq!(one.exact, ExactRational::new(600, 1));
        // Zero has no significant digits — rendered "0e0", exact 0.
        let z = compute("r", &to_sci(4, ComputeExpr::Lit(0.0)), &kb).unwrap();
        assert_eq!(rendered_of(&z), "0e0");
        assert_eq!(z.exact, ExactRational::new(0, 1));
    }

    #[test]
    fn to_scientific_preserves_dimension() {
        // Rendering a dimensioned value reformats the magnitude; the unit survives.
        let kb = kb_with(vec![money("bal", 10, "usd")]);
        let expr = to_sci(
            3,
            ComputeExpr::Bin(
                ComputeOp::Div,
                Box::new(refexpr("bal")),
                Box::new(ComputeExpr::Lit(3.0)),
            ),
        );
        let d = compute("r", &expr, &kb).unwrap();
        assert_eq!(rendered_of(&d), "3.33e0"); // 10/3 usd → 3.33e0
        assert_eq!(d.dim, Dimension::Money("usd".into()));
        assert_eq!(d.exact, ExactRational::new(333, 100));
    }

    // ---- NUM-6c: to_percent(x, places) — the percentage rendering ----

    fn to_pct(places: u32, inner: ComputeExpr) -> ComputeExpr {
        ComputeExpr::ToPercent {
            places,
            mode: RoundingMode::HalfEven,
            expr: Box::new(inner),
        }
    }
    fn pct_rendered_of(d: &Derived) -> String {
        match &d.tree {
            DerivationNode::ToPercent { rendered, .. } => rendered.clone(),
            other => panic!("expected ToPercent node, got {other:?}"),
        }
    }

    #[test]
    fn to_percent_renders_a_ratio_to_the_stated_places() {
        let kb = KnowledgeBase::new();
        // 1/3 = 0.333… → 2 places = "33.33%", the narrowed FRACTION held as 3333/10000.
        let a = compute("r", &to_pct(2, frac(1, 3)), &kb).unwrap();
        assert_eq!(pct_rendered_of(&a), "33.33%");
        assert_eq!(a.exact, ExactRational::new(3333, 10000));
        // 1/2 = 0.5 → 2 places pads the trailing zeros: "50.00%", value exactly 1/2.
        let b = compute("r", &to_pct(2, frac(1, 2)), &kb).unwrap();
        assert_eq!(pct_rendered_of(&b), "50.00%");
        assert_eq!(b.exact, ExactRational::new(1, 2));
    }

    #[test]
    fn to_percent_zero_places_drops_the_decimal_point() {
        let kb = KnowledgeBase::new();
        // 1/2 → 0 places = "50%" (no decimal point), value exactly 1/2.
        let d = compute("r", &to_pct(0, frac(1, 2)), &kb).unwrap();
        assert_eq!(pct_rendered_of(&d), "50%");
        assert_eq!(d.exact, ExactRational::new(1, 2));
    }

    #[test]
    fn to_percent_handles_sub_one_percent_sign_and_zero() {
        let kb = KnowledgeBase::new();
        // 1/2000 = 0.0005 → 2 places = "0.05%" (integer part padded to a leading 0).
        let small = compute("r", &to_pct(2, frac(1, 2000)), &kb).unwrap();
        assert_eq!(pct_rendered_of(&small), "0.05%");
        assert_eq!(small.exact, ExactRational::new(5, 10000)); // 0.0005
                                                               // Negative: −1/4 = −0.25 → 1 place = "-25.0%", value exactly −1/4.
        let neg = compute("r", &to_pct(1, frac(-1, 4)), &kb).unwrap();
        assert_eq!(pct_rendered_of(&neg), "-25.0%");
        assert_eq!(neg.exact, ExactRational::new(-1, 4));
        // Zero → "0.00%", exact 0.
        let z = compute("r", &to_pct(2, ComputeExpr::Lit(0.0)), &kb).unwrap();
        assert_eq!(pct_rendered_of(&z), "0.00%");
        assert_eq!(z.exact, ExactRational::new(0, 1));
    }

    #[test]
    fn to_percent_preserves_dimension_and_is_exact_on_a_percentage_point_case() {
        // The "$100M-per-point" case: an exact ratio, rounded only at render. A budget
        // fraction 1/7 of a dimensioned quantity narrows the magnitude, keeps the unit.
        let kb = kb_with(vec![money("share", 1, "usd")]);
        let expr = to_pct(
            3,
            ComputeExpr::Bin(
                ComputeOp::Div,
                Box::new(refexpr("share")),
                Box::new(ComputeExpr::Lit(7.0)),
            ),
        );
        let d = compute("r", &expr, &kb).unwrap();
        // 1/7 = 0.142857… → 3 places = "14.286%" (half-even on the 4th place: …57→6).
        assert_eq!(pct_rendered_of(&d), "14.286%");
        assert_eq!(d.exact, ExactRational::new(14286, 100000)); // 0.14286
        assert_eq!(d.dim, Dimension::Money("usd".into()));
    }

    // ---- NUM-6c: to_currency(x, code, places) — the money rendering ----

    fn to_cur(code: &str, places: u32, inner: ComputeExpr) -> ComputeExpr {
        ComputeExpr::ToCurrency {
            code: code.to_string(),
            places,
            mode: RoundingMode::HalfEven,
            expr: Box::new(inner),
        }
    }
    fn cur_rendered_of(d: &Derived) -> String {
        match &d.tree {
            DerivationNode::ToCurrency { rendered, .. } => rendered.clone(),
            other => panic!("expected ToCurrency node, got {other:?}"),
        }
    }

    #[test]
    fn to_currency_renders_amount_with_code_and_padded_places() {
        let kb = KnowledgeBase::new();
        // 2469/2 = 1234.5 → 2 places pads the trailing zero: "USD 1234.50", value exactly 2469/2.
        let a = compute("r", &to_cur("USD", 2, frac(2469, 2)), &kb).unwrap();
        assert_eq!(cur_rendered_of(&a), "USD 1234.50");
        assert_eq!(a.exact, ExactRational::new(2469, 2));
        // A repeating amount 10/3 = 3.333… → 2 places (half-even) = "EUR 3.33", value 333/100.
        let b = compute("r", &to_cur("EUR", 2, frac(10, 3)), &kb).unwrap();
        assert_eq!(cur_rendered_of(&b), "EUR 3.33");
        assert_eq!(b.exact, ExactRational::new(333, 100));
    }

    #[test]
    fn to_currency_zero_places_sub_one_sign_and_zero() {
        let kb = KnowledgeBase::new();
        // 0 places drops the decimal point: 7/2 = 3.5 → "JPY 4" (half-even to nearest even).
        let z = compute("r", &to_cur("JPY", 0, frac(7, 2)), &kb).unwrap();
        assert_eq!(cur_rendered_of(&z), "JPY 4");
        assert_eq!(z.exact, ExactRational::new(4, 1));
        // Sub-one amount 1/20 = 0.05 → 2 places = "USD 0.05" (leading zero preserved).
        let small = compute("r", &to_cur("USD", 2, frac(1, 20)), &kb).unwrap();
        assert_eq!(cur_rendered_of(&small), "USD 0.05");
        assert_eq!(small.exact, ExactRational::new(1, 20));
        // Negative: −5/4 = −1.25 → "USD -1.25", value exactly −5/4.
        let neg = compute("r", &to_cur("USD", 2, frac(-5, 4)), &kb).unwrap();
        assert_eq!(cur_rendered_of(&neg), "USD -1.25");
        assert_eq!(neg.exact, ExactRational::new(-5, 4));
        // Zero → "USD 0.00", exact 0.
        let zero = compute("r", &to_cur("USD", 2, ComputeExpr::Lit(0.0)), &kb).unwrap();
        assert_eq!(cur_rendered_of(&zero), "USD 0.00");
        assert_eq!(zero.exact, ExactRational::new(0, 1));
    }

    #[test]
    fn to_currency_is_base_ten_exact_and_preserves_dimension() {
        // The exactness point: a bill split three ways stays exact until the render. $100
        // / 3 = 33.333… → "USD 33.33" at 2 places; the money dimension survives.
        let kb = kb_with(vec![money("bill", 100, "usd")]);
        let expr = to_cur(
            "USD",
            2,
            ComputeExpr::Bin(
                ComputeOp::Div,
                Box::new(refexpr("bill")),
                Box::new(ComputeExpr::Lit(3.0)),
            ),
        );
        let d = compute("r", &expr, &kb).unwrap();
        assert_eq!(cur_rendered_of(&d), "USD 33.33");
        assert_eq!(d.exact, ExactRational::new(3333, 100)); // 33.33 exactly
        assert_eq!(d.dim, Dimension::Money("usd".into()));
    }

    // ---- NUM-6v: adj-verify re-check of the narrowing nodes (ADJ-NUMERIC §4.3) ----

    #[test]
    fn recheck_confirms_a_genuine_rounding() {
        let kb = KnowledgeBase::new();
        // 10/3 → 2 places = 3.33; re-rounding the recorded exact source reproduces it.
        let d = compute("r", &round_places(2, frac(10, 3)), &kb).unwrap();
        assert_eq!(recheck_narrowing(&d.tree), NarrowingCheck::ReChecked);
    }

    #[test]
    fn recheck_confirms_every_formatter() {
        let kb = KnowledgeBase::new();
        // to_scientific, to_percent, to_currency all re-derive their rendered string
        // AND their numeric result from the recorded exact source.
        let sci = compute("r", &to_sci(4, frac(1, 3)), &kb).unwrap();
        assert_eq!(recheck_narrowing(&sci.tree), NarrowingCheck::ReChecked);
        let pct = compute("r", &to_pct(2, frac(1, 3)), &kb).unwrap();
        assert_eq!(recheck_narrowing(&pct.tree), NarrowingCheck::ReChecked);
        let cur = compute("r", &to_cur("usd", 2, frac(100, 3)), &kb).unwrap();
        assert_eq!(recheck_narrowing(&cur.tree), NarrowingCheck::ReChecked);
    }

    #[test]
    fn recheck_catches_a_tampered_result() {
        let kb = KnowledgeBase::new();
        // Compute a genuine rounding, then tamper the recorded `result` (the kind of
        // drift a since-edited or fabricated artifact carries): the re-check must
        // catch it, because re-rounding the still-honest exact source disagrees.
        let mut d = compute("r", &round_places(2, frac(10, 3)), &kb).unwrap();
        if let DerivationNode::Round { result, .. } = &mut d.tree {
            *result = 9.99; // was 3.33
        }
        assert!(recheck_narrowing(&d.tree).is_mismatch());
    }

    #[test]
    fn recheck_catches_a_tampered_rendered_string() {
        let kb = KnowledgeBase::new();
        // Tamper only the boundary STRING of a currency rendering, leaving the exact
        // source and numeric result intact: still a mismatch, because the string a
        // consumer reads no longer matches what the exact source renders to.
        let mut d = compute("r", &to_cur("usd", 2, frac(100, 3)), &kb).unwrap();
        if let DerivationNode::ToCurrency { rendered, .. } = &mut d.tree {
            *rendered = "USD 99.99".to_string(); // was "USD 33.33"
        }
        match recheck_narrowing(&d.tree) {
            NarrowingCheck::Mismatch { why, .. } => assert_eq!(why, "rendered_differs"),
            other => panic!("expected a rendered_differs mismatch, got {other:?}"),
        }
    }

    #[test]
    fn recheck_is_unverifiable_without_an_exact_source() {
        // A narrowing node whose operand carried no exact value (a transcendental
        // result) cannot be re-rounded exactly — honestly reported, never a pass.
        let node = DerivationNode::Round {
            spec: RoundSpec::Places(2),
            mode: RoundingMode::HalfEven,
            operand: Box::new(DerivationNode::Lit { value: 3.333 }),
            operand_exact: None,
            result: 3.33,
        };
        assert_eq!(recheck_narrowing(&node), NarrowingCheck::Unverifiable);
    }

    #[test]
    fn recheck_a_non_narrowing_node_is_inert() {
        let node = DerivationNode::Lit { value: 1.0 };
        assert_eq!(recheck_narrowing(&node), NarrowingCheck::NotANarrowing);
    }

    #[test]
    fn recheck_narrowings_walks_a_nested_tree() {
        let kb = KnowledgeBase::new();
        // round_to(to_percent(1/3, 2), 2): a narrowing whose operand is itself a
        // narrowing — the walk must find and re-check BOTH, at depths 0 and 1.
        let inner = to_pct(2, frac(1, 3));
        let expr = round_places(2, inner);
        let d = compute("r", &expr, &kb).unwrap();
        let checks = recheck_narrowings(&d.tree);
        assert_eq!(checks.len(), 2, "expected the outer round and inner to_percent");
        assert_eq!(checks[0].0, 0); // outer round at the root
        assert_eq!(checks[1].0, 1); // inner to_percent one level down
        assert!(checks.iter().all(|(_, c)| c.is_rechecked()));
    }

    #[test]
    fn recheck_narrowings_is_empty_for_a_plain_formula() {
        let kb = kb_with(vec![money("a", 10, "usd"), money("b", 3, "usd")]);
        // A plain division carries no narrowing node, so there is nothing to re-check.
        let expr = ComputeExpr::Bin(
            ComputeOp::Div,
            Box::new(refexpr("a")),
            Box::new(refexpr("b")),
        );
        let d = compute("r", &expr, &kb).unwrap();
        assert!(recheck_narrowings(&d.tree).is_empty());
    }
}
