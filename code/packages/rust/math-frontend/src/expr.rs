//! The **neutral math AST** every frontend produces and every consumer lowers.
//!
//! This tree is deliberately *notation-agnostic*: it is not LaTeX-shaped, not
//! AsciiMath-shaped. Two source strings that mean the same mathematics — `a \times b`
//! and `a \cdot b` and `ab`, or `\frac{1}{2}` and `1/2` — produce the **same**
//! `MathExpr`. That is what lets a consumer (a rule engine, a CAS, a renderer) treat
//! every input notation uniformly: write the lowering once, get every frontend free.
//!
//! Presentation-only distinctions are normalized away on purpose (see [`BinOp`]).
//! Meaning-bearing structure is kept faithfully.

/// A numeric literal that **preserves exactness**.
///
/// A frontend must never silently turn `0.1` into the nearest `f64` — lossy float
/// conversion is a *consumer's* decision at lowering time, made explicitly via
/// [`Number::to_f64`]. So a `Number` keeps the literal's exact decimal value as a
/// normalized `(negative, digits, exponent)` triple (value = `±digits × 10^exponent`),
/// where `digits` has no leading or trailing zeros. This is exact for every decimal
/// numeral a notation can write, needs no big-integer dependency, and gives a correct
/// `==` (so `1.0`, `1`, `01`, and `1e0` all compare equal).
#[derive(Debug, Clone)]
pub struct Number {
    negative: bool,
    /// Significant digits, no leading/trailing zeros. Empty string means the value 0.
    digits: String,
    /// Power-of-ten exponent applied to `digits` interpreted as an integer.
    exponent: i64,
    /// The literal exactly as written by the source notation (for round-tripping).
    raw: String,
}

impl Number {
    /// Parse a decimal numeral: optional sign, integer and/or fraction part, optional
    /// `e`/`E` exponent (`-12`, `3.14`, `.5`, `6.022e23`, `1E-3`). Returns `None` if the
    /// text is not a well-formed decimal numeral.
    pub fn parse(raw: &str) -> Option<Number> {
        let s = raw.trim();
        if s.is_empty() {
            return None;
        }
        let bytes = s.as_bytes();
        let mut i = 0;
        let negative = match bytes.first() {
            Some(b'+') => { i += 1; false }
            Some(b'-') => { i += 1; true }
            _ => false,
        };
        let int_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        let int_part = &s[int_start..i];
        let mut frac_part = "";
        if i < bytes.len() && bytes[i] == b'.' {
            i += 1;
            let frac_start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            frac_part = &s[frac_start..i];
        }
        // need at least one digit overall
        if int_part.is_empty() && frac_part.is_empty() {
            return None;
        }
        let mut exp: i64 = 0;
        if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
            i += 1;
            let neg_exp = match bytes.get(i) {
                Some(b'+') => { i += 1; false }
                Some(b'-') => { i += 1; true }
                _ => false,
            };
            let exp_start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if exp_start == i {
                return None; // `e` with no exponent digits
            }
            let mag: i64 = s[exp_start..i].parse().ok()?;
            exp = if neg_exp { -mag } else { mag };
        }
        if i != bytes.len() {
            return None; // trailing junk
        }

        // Combine into digits × 10^exponent: the fraction shifts the exponent down.
        // All exponent arithmetic is CHECKED so an adversarial literal (e.g. an
        // exponent near i64::MAX) returns None rather than overflow-panicking — the
        // parser must stay total, never panic.
        let mut all_digits = String::with_capacity(int_part.len() + frac_part.len());
        all_digits.push_str(int_part);
        all_digits.push_str(frac_part);
        let mut exponent = exp.checked_sub(frac_part.len() as i64)?;

        // Normalize: strip trailing zeros (raising the exponent) then leading zeros.
        let trimmed_trailing = all_digits.trim_end_matches('0');
        exponent = exponent.checked_add((all_digits.len() - trimmed_trailing.len()) as i64)?;
        let normalized: String = trimmed_trailing.trim_start_matches('0').to_string();

        let (digits, exponent, negative) = if normalized.is_empty() {
            (String::new(), 0, false) // canonical zero (never "-0")
        } else {
            (normalized, exponent, negative)
        };

        Some(Number {
            negative,
            digits,
            exponent,
            raw: raw.to_string(),
        })
    }

    /// True if this number is exactly zero.
    pub fn is_zero(&self) -> bool {
        self.digits.is_empty()
    }

    /// The literal exactly as the source notation wrote it.
    pub fn as_written(&self) -> &str {
        &self.raw
    }

    /// **Lossy** conversion to `f64` — an explicit choice the consumer makes when it is
    /// willing to leave exact arithmetic. Returns `None` only if the magnitude is so
    /// extreme it cannot be represented.
    pub fn to_f64(&self) -> Option<f64> {
        if self.is_zero() {
            return Some(0.0);
        }
        let mantissa: f64 = self.digits.parse().ok()?;
        // Range-check the exponent rather than truncating with `as i32`: a magnitude
        // too extreme for i32 is exactly the "cannot be represented" case → None,
        // never a silently-wrong finite value.
        let exp = i32::try_from(self.exponent).ok()?;
        let v = mantissa * 10f64.powi(exp);
        if v.is_finite() {
            Some(if self.negative { -v } else { v })
        } else {
            None
        }
    }

    /// Construct directly from an integer (convenience for tests and consumers).
    pub fn from_i64(n: i64) -> Number {
        Number::parse(&n.to_string()).expect("integer is a valid numeral")
    }
}

/// Two numbers are equal iff they denote the same exact value (normalized form), so
/// `1`, `1.0`, `01`, and `1e0` all compare equal. The `raw` spelling is *not* compared.
impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        self.negative == other.negative
            && self.digits == other.digits
            && self.exponent == other.exponent
    }
}
impl Eq for Number {}

/// Binary operators. **`Mul` and `Div` carry no surface style** — `\times`, `\cdot`,
/// and juxtaposition all become `Mul`; `\frac`, `\dfrac`, `/` all become `Div` (or the
/// intent-carrying [`MathExpr::Frac`]). Presentation, not meaning, is dropped here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    /// `a ± b` — denotes the *pair* {a+b, a−b}. A meaning-bearing operator (not presentation),
    /// kept binary so a consumer can interpret both branches.
    PlusMinus,
    /// `a ∓ b` — the opposite pairing to [`BinOp::PlusMinus`] ({a−b, a+b}).
    MinusPlus,
}

/// Unary prefix operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Pos,
}

/// Relational operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Approx,
    Equiv,
}

/// Named functions. The common ones are closed variants (so consumers can `match`
/// exhaustively on what they support); anything else is preserved by name in `Other`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Func {
    Sin, Cos, Tan, Cot, Sec, Csc,
    Asin, Acos, Atan,
    Sinh, Cosh, Tanh,
    Ln, Log, Exp,
    Min, Max, Gcd, Lcm, Det,
    Other(String),
}

/// Big operators that take optional lower/upper bounds and a body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BigOp {
    Sum, Prod, Int, Oint, Coprod, Lim,
    Other(String),
}

/// A node in the neutral math AST. See the module docs for the normalization contract.
#[derive(Debug, Clone, PartialEq)]
pub enum MathExpr {
    /// An exact numeric literal.
    Number(Number),
    /// A variable or named constant: `"x"`, `"pi"`, `"alpha"`.
    Symbol(String),
    /// A binary operation.
    Bin(BinOp, Box<MathExpr>, Box<MathExpr>),
    /// A unary operation.
    Unary(UnaryOp, Box<MathExpr>),
    /// A fraction `numerator / denominator` (a `Div` that the source wrote as a built-up
    /// fraction; kept distinct so a renderer can reproduce it, but it means division).
    Frac(Box<MathExpr>, Box<MathExpr>),
    /// A binomial coefficient `C(n, k)` = "n choose k" — `Binom(n, k)`. Distinct from `Frac`
    /// (no division bar) and meaning-bearing, so consumers can evaluate or render it.
    Binom(Box<MathExpr>, Box<MathExpr>),
    /// An nth root: `degree` is `None` for a square root.
    Root {
        degree: Option<Box<MathExpr>>,
        radicand: Box<MathExpr>,
    },
    /// A named-function application: `sin(x)`, `ln(x)`.
    Call { func: Func, arg: Box<MathExpr> },
    /// A big operator with optional bounds: `\sum_{i=1}^{n} body`.
    BigOp {
        op: BigOp,
        lower: Option<Box<MathExpr>>,
        upper: Option<Box<MathExpr>>,
        body: Box<MathExpr>,
    },
    /// Indexing (subscript), kept distinct from `Pow`: `a_i`.
    Subscript(Box<MathExpr>, Box<MathExpr>),
    /// A relation: `a = b`, `x <= y`.
    Rel(RelOp, Box<MathExpr>, Box<MathExpr>),
    /// Explicit grouping (parentheses/braces) — preserved so precedence intent is exact.
    Group(Box<MathExpr>),
    /// Prose embedded in math (`\text{…}`): units, labels.
    Text(String),
    /// A matrix as rows of cells.
    Matrix(Vec<Vec<MathExpr>>),
}

/// Drop a `MathExpr` **iteratively** so freeing a deeply-nested tree cannot overflow the
/// stack.
///
/// A frontend can legitimately produce a very deep tree from small input: a left-
/// associative chain `a + a + a + …` (or juxtaposition `aaa…`, or `1/1/1/…`) parses — by
/// design, with loops not recursion — into `Bin(Add, Bin(Add, …))` nested N deep. The
/// compiler's *default* destructor for a recursive `Box`-owning enum is itself recursive,
/// so dropping such a tree would recurse N frames and abort the process (an uncatchable
/// stack overflow) on adversarial-but-tiny input. Since every frontend hands these trees
/// back through the panic-free `MathFrontend` contract, the neutral AST must be safe to
/// drop at any depth — so we override `Drop` to dismantle the tree with an explicit heap
/// worklist instead of the call stack.
///
/// How it stays O(1) in stack depth: we move each node's boxed children onto a `Vec`
/// worklist (replacing them in place with a cheap leaf), then pop and repeat. By the time
/// any node is finally dropped, its children are leaves, so the compiler-generated
/// destructor recurses at most one trivial level.
impl Drop for MathExpr {
    fn drop(&mut self) {
        let mut stack: Vec<MathExpr> = Vec::new();
        take_children(self, &mut stack);
        while let Some(mut node) = stack.pop() {
            take_children(&mut node, &mut stack);
            // `node` now owns only leaf children, so dropping it here is shallow.
        }
    }
}

/// Move every boxed child of `e` onto `out`, leaving `e` holding cheap leaves in their place.
/// A leaf (`Number`/`Symbol`/`Text`) contributes nothing. Used only by [`MathExpr`]'s `Drop`.
fn take_children(e: &mut MathExpr, out: &mut Vec<MathExpr>) {
    // Swap a boxed child out for a leaf (no allocation: `String::new()` doesn't allocate).
    fn take(b: &mut Box<MathExpr>, out: &mut Vec<MathExpr>) {
        out.push(std::mem::replace(b.as_mut(), MathExpr::Symbol(String::new())));
    }
    fn take_opt(b: &mut Option<Box<MathExpr>>, out: &mut Vec<MathExpr>) {
        if let Some(boxed) = b.take() {
            out.push(*boxed);
        }
    }
    match e {
        MathExpr::Number(_) | MathExpr::Symbol(_) | MathExpr::Text(_) => {}
        MathExpr::Bin(_, a, b)
        | MathExpr::Frac(a, b)
        | MathExpr::Binom(a, b)
        | MathExpr::Subscript(a, b)
        | MathExpr::Rel(_, a, b) => {
            take(a, out);
            take(b, out);
        }
        MathExpr::Unary(_, a) | MathExpr::Group(a) => take(a, out),
        MathExpr::Root { degree, radicand } => {
            take_opt(degree, out);
            take(radicand, out);
        }
        MathExpr::Call { arg, .. } => take(arg, out),
        MathExpr::BigOp { lower, upper, body, .. } => {
            take_opt(lower, out);
            take_opt(upper, out);
            take(body, out);
        }
        MathExpr::Matrix(rows) => {
            for row in std::mem::take(rows) {
                out.extend(row);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plusminus_and_binom_are_constructible_and_compare() {
        let one = || Box::new(MathExpr::Number(Number::from_i64(1)));
        let pm = MathExpr::Bin(BinOp::PlusMinus, one(), one());
        let mp = MathExpr::Bin(BinOp::MinusPlus, one(), one());
        // ± and ∓ are distinct operators.
        assert_ne!(pm, mp);
        assert_eq!(pm, MathExpr::Bin(BinOp::PlusMinus, one(), one()));
        // Binom is distinct from Frac with the same operands.
        let binom = MathExpr::Binom(one(), one());
        assert_eq!(binom, MathExpr::Binom(one(), one()));
        assert_ne!(binom, MathExpr::Frac(one(), one()));
    }

    #[test]
    fn dropping_a_very_deep_tree_does_not_overflow() {
        // A frontend can build a left-associative chain `a + a + … ` this deep from tiny
        // input. With the default recursive destructor this drop would overflow the stack
        // and abort the process; the iterative `Drop` impl must free it on the heap instead.
        // 300_000 nodes is far beyond any per-frame recursive-drop budget on a 2 MiB stack.
        let mut e = MathExpr::Number(Number::from_i64(0));
        for _ in 0..300_000 {
            e = MathExpr::Bin(
                BinOp::Add,
                Box::new(e),
                Box::new(MathExpr::Number(Number::from_i64(1))),
            );
        }
        drop(e); // must return normally, not abort with a stack overflow
    }

    #[test]
    fn dropping_a_deep_matrix_and_options_does_not_overflow() {
        // Exercise the Option/Vec child paths of the iterative drop (Root degree, BigOp
        // bounds, Matrix rows) nested deeply through a Root chain.
        let mut e = MathExpr::Symbol("x".to_string());
        for _ in 0..100_000 {
            e = MathExpr::Root { degree: Some(Box::new(MathExpr::Number(Number::from_i64(2)))), radicand: Box::new(e) };
        }
        e = MathExpr::Matrix(vec![vec![e]]);
        drop(e);
    }

    #[test]
    fn number_normalizes_equal_values() {
        assert_eq!(Number::parse("1").unwrap(), Number::parse("1.0").unwrap());
        assert_eq!(Number::parse("1").unwrap(), Number::parse("01").unwrap());
        assert_eq!(Number::parse("1").unwrap(), Number::parse("1e0").unwrap());
        assert_eq!(Number::parse("100").unwrap(), Number::parse("1e2").unwrap());
        assert_eq!(Number::parse("0.5").unwrap(), Number::parse(".5").unwrap());
        assert_eq!(Number::parse("0.5").unwrap(), Number::parse("5e-1").unwrap());
    }

    #[test]
    fn number_distinguishes_different_values() {
        assert_ne!(Number::parse("1").unwrap(), Number::parse("2").unwrap());
        assert_ne!(Number::parse("0.1").unwrap(), Number::parse("0.10001").unwrap());
        assert_ne!(Number::parse("1").unwrap(), Number::parse("-1").unwrap());
    }

    #[test]
    fn zero_is_canonical_and_never_negative() {
        assert_eq!(Number::parse("0").unwrap(), Number::parse("-0").unwrap());
        assert_eq!(Number::parse("0").unwrap(), Number::parse("0.000").unwrap());
        assert!(Number::parse("0").unwrap().is_zero());
    }

    #[test]
    fn number_preserves_the_written_form() {
        assert_eq!(Number::parse("3.140").unwrap().as_written(), "3.140");
    }

    #[test]
    fn number_to_f64_is_explicit_and_lossy() {
        assert_eq!(Number::parse("0.5").unwrap().to_f64(), Some(0.5));
        assert_eq!(Number::parse("100").unwrap().to_f64(), Some(100.0));
        assert_eq!(Number::parse("-2.5").unwrap().to_f64(), Some(-2.5));
        assert_eq!(Number::parse("0").unwrap().to_f64(), Some(0.0));
    }

    #[test]
    fn number_rejects_non_numerals() {
        assert!(Number::parse("").is_none());
        assert!(Number::parse("x").is_none());
        assert!(Number::parse("1.2.3").is_none());
        assert!(Number::parse("1e").is_none());
        assert!(Number::parse("1e+").is_none());
        assert!(Number::parse("0x10").is_none());
    }

    #[test]
    fn adversarial_exponents_stay_total_and_honest() {
        // A near-i64::MAX exponent with trailing-zero normalization must NOT overflow-
        // panic in parse — it returns None (the value isn't a representable numeral here).
        assert!(Number::parse("100e9223372036854775807").is_none());
        // A huge but parseable exponent: to_f64 must return None (too extreme), never a
        // truncated finite lie.
        if let Some(n) = Number::parse("1e3000000000") {
            assert_eq!(n.to_f64(), None);
        }
        // Sanity: a normal large-ish exponent still works.
        assert_eq!(Number::parse("1e3").unwrap().to_f64(), Some(1000.0));
    }

    #[test]
    fn from_i64_round_trips() {
        assert_eq!(Number::from_i64(42), Number::parse("42").unwrap());
        assert_eq!(Number::from_i64(-7).to_f64(), Some(-7.0));
    }
}
