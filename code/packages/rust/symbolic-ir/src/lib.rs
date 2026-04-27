//! # symbolic-ir
//!
//! The universal symbolic expression IR — the shared tree representation
//! that every computer-algebra-system frontend compiles to and every CAS
//! backend consumes.
//!
//! ## Design
//!
//! Every node is one of six variants of the [`IRNode`] enum.  All nodes
//! implement [`Clone`], [`PartialEq`], [`Eq`], and [`Hash`] so they can be
//! used as `HashMap` keys.
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────┐
//! │                         IRNode                               │
//! ├──────────────────┬───────────────────────────────────────────┤
//! │ Symbol(name)     │ Named atom: variable, constant, op head   │
//! │ Integer(i64)     │ Arbitrary-precision integer (i64 for now) │
//! │ Rational(n, d)   │ Exact fraction, always reduced            │
//! │ Float(f64)       │ Double-precision float                     │
//! │ Str(String)      │ String literal                            │
//! │ Apply(head,args) │ Compound: head(arg0, arg1, …)             │
//! └──────────────────┴───────────────────────────────────────────┘
//! ```
//!
//! The single compound form `Apply` covers everything from `x + y` to
//! `Integrate(f(x), x, 0, 1)`.  Head and args are `IRNode` values, so
//! higher-order applications work naturally.
//!
//! ## Standard heads
//!
//! A small vocabulary of named string constants (`ADD`, `MUL`, …) represents
//! the head symbols every backend is expected to handle.  They are just
//! `&'static str` values — no singleton objects needed, because `Symbol`
//! equality compares the string content.
//!
//! ## Example
//!
//! ```rust
//! use symbolic_ir::{IRNode, sym, int, apply, ADD, MUL, POW};
//!
//! // Build  x^2 + 2*x + 1
//! let x = sym("x");
//! let one = int(1);
//! let two = int(2);
//! let x_sq = apply(sym(POW), vec![x.clone(), two.clone()]);
//! let two_x = apply(sym(MUL), vec![two.clone(), x.clone()]);
//! let expr = apply(sym(ADD), vec![x_sq, two_x, one]);
//! println!("{expr}");  // Add(Pow(x, 2), Mul(2, x), 1)
//! ```

use std::fmt;
use std::hash::{Hash, Hasher};

// ---------------------------------------------------------------------------
// Core node types
// ---------------------------------------------------------------------------

/// A compound apply expression — `head(arg0, arg1, …)`.
///
/// Boxed inside [`IRNode::Apply`] to keep the enum size small.  Every
/// higher-level construct (arithmetic, calculus, data structures) is an
/// `IRApply`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IRApply {
    /// The operator or function head.  Almost always an `IRNode::Symbol`,
    /// but arbitrary heads are permitted for higher-order expressions.
    pub head: IRNode,
    /// The operands, in order.
    pub args: Vec<IRNode>,
}

impl fmt::Display for IRApply {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}(", self.head)?;
        for (i, arg) in self.args.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{arg}")?;
        }
        write!(f, ")")
    }
}

/// The six IR node variants.
///
/// # Equality and hashing for floats
///
/// IEEE-754 `f64` does not implement `Eq` or `Hash` because `NaN ≠ NaN`.
/// Here we define equality and hashing over the raw bit pattern
/// (`f64::to_bits`), which means two `NaN`s with identical bit patterns
/// compare equal.  This is acceptable for a CAS IR where `NaN` values
/// should never appear in well-formed expressions; the degenerate case is
/// preserved deterministically rather than silently dropped.
///
/// # Integers
///
/// The Python reference implementation uses Python's arbitrary-precision
/// integers.  This port uses `i64` (64-bit signed).  Values that exceed
/// `i64::MAX` are not yet supported.  A big-integer crate can be
/// substituted later by changing `Integer(i64)` to `Integer(BigInt)`.
#[derive(Debug, Clone)]
pub enum IRNode {
    /// A named atom: variable, constant, or operation head.
    ///
    /// Examples: `IRNode::Symbol("x")`, `IRNode::Symbol("Pi")`,
    /// `IRNode::Symbol("Add")`.  The name is case-sensitive, matching
    /// the MACSYMA / Mathematica convention.
    Symbol(String),

    /// A 64-bit integer literal.  Negative values are stored directly
    /// (not wrapped in `IRNode::Apply(sym("Neg"), …)`).
    Integer(i64),

    /// An exact rational number in lowest terms.
    ///
    /// Two invariants always hold:
    /// 1. `denom > 0` — the sign is carried by `numer`.
    /// 2. `gcd(|numer|, denom) == 1` — the fraction is fully reduced.
    ///
    /// Construct with [`IRNode::rational`] or the [`rat`] helper to
    /// guarantee the invariants.  The tuple fields are public for pattern
    /// matching, but direct construction bypasses normalization.
    Rational(i64, i64),

    /// A double-precision floating-point literal.
    Float(f64),

    /// A string literal.  Rare in CAS expressions but present in MACSYMA
    /// for `print("label", value)` and some rewrite-rule conditions.
    Str(String),

    /// A compound expression: `head` applied to zero or more `args`.
    ///
    /// Every arithmetic, calculus, or structural operation in the IR is
    /// an `Apply`.  Using a uniform compound form keeps all tree-walking
    /// code simple — there is only one compound pattern to handle.
    Apply(Box<IRApply>),
}

// ---------------------------------------------------------------------------
// Constructors and factory helpers
// ---------------------------------------------------------------------------

impl IRNode {
    /// Build a `Rational(numer, denom)` node in fully reduced form.
    ///
    /// Normalises sign (denominator always positive) and divides by GCD.
    /// Collapses to `Integer(n)` when the denominator reduces to 1.
    ///
    /// # Panics
    ///
    /// Panics if `denom == 0`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use symbolic_ir::IRNode;
    ///
    /// assert_eq!(IRNode::rational(2, 4), IRNode::Rational(1, 2));
    /// assert_eq!(IRNode::rational(6, 3), IRNode::Integer(2));   // collapses
    /// assert_eq!(IRNode::rational(1, -2), IRNode::Rational(-1, 2)); // sign → numer
    /// ```
    pub fn rational(numer: i64, denom: i64) -> Self {
        assert!(denom != 0, "IRNode::rational: denominator cannot be zero");
        // Move sign to numerator.
        let (numer, denom) = if denom < 0 {
            (-numer, -denom)
        } else {
            (numer, denom)
        };
        let g = gcd(numer.unsigned_abs(), denom.unsigned_abs()) as i64;
        let (n, d) = (numer / g, denom / g);
        if d == 1 {
            IRNode::Integer(n)
        } else {
            IRNode::Rational(n, d)
        }
    }
}

// ---------------------------------------------------------------------------
// Convenience free-function constructors
// ---------------------------------------------------------------------------

/// Build a `Symbol` node from any `&str` or `String`.
///
/// ```rust
/// use symbolic_ir::{sym, IRNode};
/// assert_eq!(sym("x"), IRNode::Symbol("x".to_string()));
/// ```
#[inline]
pub fn sym(name: impl Into<String>) -> IRNode {
    IRNode::Symbol(name.into())
}

/// Build an `Integer` node.
///
/// ```rust
/// use symbolic_ir::{int, IRNode};
/// assert_eq!(int(42), IRNode::Integer(42));
/// ```
#[inline]
pub fn int(n: i64) -> IRNode {
    IRNode::Integer(n)
}

/// Build a `Rational` node in reduced form.  Delegates to
/// [`IRNode::rational`]; collapses to `Integer` when `denom == 1` after
/// reduction.
///
/// ```rust
/// use symbolic_ir::{rat, IRNode};
/// assert_eq!(rat(1, 2), IRNode::Rational(1, 2));
/// assert_eq!(rat(4, 2), IRNode::Integer(2));
/// ```
#[inline]
pub fn rat(numer: i64, denom: i64) -> IRNode {
    IRNode::rational(numer, denom)
}

/// Build a `Float` node.
///
/// ```rust
/// use symbolic_ir::{flt, IRNode};
/// assert_eq!(flt(3.14), IRNode::Float(3.14));
/// ```
#[inline]
pub fn flt(v: f64) -> IRNode {
    IRNode::Float(v)
}

/// Build a `Str` node.
///
/// ```rust
/// use symbolic_ir::{str_node, IRNode};
/// assert_eq!(str_node("hello"), IRNode::Str("hello".to_string()));
/// ```
#[inline]
pub fn str_node(s: impl Into<String>) -> IRNode {
    IRNode::Str(s.into())
}

/// Build an `Apply` node from a head and a `Vec<IRNode>` of arguments.
///
/// ```rust
/// use symbolic_ir::{apply, sym, int, ADD};
///
/// // Add(x, 1)
/// let expr = apply(sym(ADD), vec![sym("x"), int(1)]);
/// assert!(matches!(expr, symbolic_ir::IRNode::Apply(_)));
/// ```
#[inline]
pub fn apply(head: IRNode, args: Vec<IRNode>) -> IRNode {
    IRNode::Apply(Box::new(IRApply { head, args }))
}

// ---------------------------------------------------------------------------
// PartialEq, Eq, Hash — manual impl needed for Float
// ---------------------------------------------------------------------------

impl PartialEq for IRNode {
    fn eq(&self, other: &Self) -> bool {
        use IRNode::*;
        match (self, other) {
            (Symbol(a), Symbol(b)) => a == b,
            (Integer(a), Integer(b)) => a == b,
            (Rational(n1, d1), Rational(n2, d2)) => n1 == n2 && d1 == d2,
            // Compare floats by bit pattern so NaNs with the same bits are
            // considered equal and the impl is consistent with Hash.
            (Float(a), Float(b)) => a.to_bits() == b.to_bits(),
            (Str(a), Str(b)) => a == b,
            (Apply(a), Apply(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for IRNode {}

impl Hash for IRNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Mix in the discriminant first so different variants with the same
        // payload hash differently.
        std::mem::discriminant(self).hash(state);
        use IRNode::*;
        match self {
            Symbol(s) => s.hash(state),
            Integer(n) => n.hash(state),
            Rational(n, d) => {
                n.hash(state);
                d.hash(state);
            }
            Float(f) => f.to_bits().hash(state),
            Str(s) => s.hash(state),
            Apply(a) => a.hash(state),
        }
    }
}

// ---------------------------------------------------------------------------
// Display
// ---------------------------------------------------------------------------

impl fmt::Display for IRNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use IRNode::*;
        match self {
            Symbol(s) => write!(f, "{s}"),
            Integer(n) => write!(f, "{n}"),
            Rational(n, d) => write!(f, "{n}/{d}"),
            // Use `{:?}` (Rust's default float repr) which is round-trip safe.
            Float(v) => write!(f, "{v:?}"),
            Str(s) => write!(f, "\"{s}\""),
            Apply(a) => write!(f, "{a}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Standard head-name constants
// ---------------------------------------------------------------------------
//
// These are `&'static str` values, not singleton objects.  Equality between
// symbol nodes works by string comparison, so there is no need for pointer
// identity.  Using constants avoids typo-induced bugs and makes it easy to
// discover the standard vocabulary.

// Arithmetic
pub const ADD: &str = "Add";
pub const SUB: &str = "Sub";
pub const MUL: &str = "Mul";
pub const DIV: &str = "Div";
pub const POW: &str = "Pow";
pub const NEG: &str = "Neg";
pub const INV: &str = "Inv";

// Elementary functions
pub const EXP: &str = "Exp";
pub const LOG: &str = "Log";
pub const SIN: &str = "Sin";
pub const COS: &str = "Cos";
pub const TAN: &str = "Tan";
pub const SQRT: &str = "Sqrt";
pub const ATAN: &str = "Atan";
pub const ASIN: &str = "Asin";
pub const ACOS: &str = "Acos";

// Hyperbolic functions
pub const SINH: &str = "Sinh";
pub const COSH: &str = "Cosh";
pub const TANH: &str = "Tanh";
pub const ASINH: &str = "Asinh";
pub const ACOSH: &str = "Acosh";
pub const ATANH: &str = "Atanh";

// Calculus
pub const D: &str = "D";
pub const INTEGRATE: &str = "Integrate";

// Relations
pub const EQUAL: &str = "Equal";
pub const NOT_EQUAL: &str = "NotEqual";
pub const LESS: &str = "Less";
pub const GREATER: &str = "Greater";
pub const LESS_EQUAL: &str = "LessEqual";
pub const GREATER_EQUAL: &str = "GreaterEqual";

// Logic
pub const AND: &str = "And";
pub const OR: &str = "Or";
pub const NOT: &str = "Not";
pub const IF: &str = "If";

// Containers and binding
pub const LIST: &str = "List";
pub const ASSIGN: &str = "Assign";
pub const DEFINE: &str = "Define";
pub const RULE: &str = "Rule";

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Greatest common divisor via Euclidean algorithm.
///
/// Returns `0` when both inputs are `0` (GCD(0,0) = 0 by convention).
fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- IRNode::rational ---------------------------------------------------

    #[test]
    fn rational_reduces_fraction() {
        assert_eq!(IRNode::rational(2, 4), IRNode::Rational(1, 2));
    }

    #[test]
    fn rational_collapses_to_integer_when_denom_one_after_reduction() {
        assert_eq!(IRNode::rational(6, 3), IRNode::Integer(2));
        assert_eq!(IRNode::rational(10, 5), IRNode::Integer(2));
    }

    #[test]
    fn rational_normalises_negative_denominator() {
        // Sign must end up in numerator.
        assert_eq!(IRNode::rational(1, -2), IRNode::Rational(-1, 2));
        assert_eq!(IRNode::rational(-3, -4), IRNode::Rational(3, 4));
    }

    #[test]
    fn rational_keeps_numerator_zero() {
        // 0/anything == 0 (integer zero)
        assert_eq!(IRNode::rational(0, 5), IRNode::Integer(0));
    }

    #[test]
    #[should_panic(expected = "denominator cannot be zero")]
    fn rational_panics_on_zero_denominator() {
        let _ = IRNode::rational(1, 0);
    }

    // --- PartialEq / Eq -----------------------------------------------------

    #[test]
    fn symbol_equality_is_case_sensitive() {
        assert_eq!(sym("x"), sym("x"));
        assert_ne!(sym("x"), sym("X"));
    }

    #[test]
    fn integer_equality() {
        assert_eq!(int(42), int(42));
        assert_ne!(int(1), int(2));
    }

    #[test]
    fn float_equality_uses_bit_pattern() {
        assert_eq!(flt(1.0), flt(1.0));
        assert_ne!(flt(1.0), flt(2.0));
        // NaN: two NaNs with the same bit pattern compare equal.
        let nan = flt(f64::NAN);
        assert_eq!(nan, nan.clone());
    }

    #[test]
    fn different_variants_are_not_equal() {
        assert_ne!(int(1), flt(1.0));
        assert_ne!(sym("1"), int(1));
    }

    // --- Hash ---------------------------------------------------------------

    #[test]
    fn equal_nodes_have_equal_hashes() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        fn h(node: &IRNode) -> u64 {
            let mut s = DefaultHasher::new();
            node.hash(&mut s);
            s.finish()
        }

        assert_eq!(h(&sym("x")), h(&sym("x")));
        assert_eq!(h(&int(7)), h(&int(7)));
        assert_eq!(h(&flt(2.5)), h(&flt(2.5)));
        assert_eq!(h(&rat(1, 2)), h(&rat(1, 2)));
    }

    // --- Display ------------------------------------------------------------

    #[test]
    fn display_symbol() {
        assert_eq!(sym("x").to_string(), "x");
    }

    #[test]
    fn display_integer() {
        assert_eq!(int(-7).to_string(), "-7");
    }

    #[test]
    fn display_rational() {
        assert_eq!(rat(1, 3).to_string(), "1/3");
    }

    #[test]
    fn display_float() {
        // Rust's {:?} for 1.5 gives "1.5"
        assert_eq!(flt(1.5).to_string(), "1.5");
    }

    #[test]
    fn display_str_node() {
        assert_eq!(str_node("hello").to_string(), "\"hello\"");
    }

    #[test]
    fn display_apply() {
        let e = apply(sym(ADD), vec![sym("x"), int(1)]);
        assert_eq!(e.to_string(), "Add(x, 1)");
    }

    #[test]
    fn display_nested_apply() {
        // Pow(x, 2)
        let e = apply(sym(POW), vec![sym("x"), int(2)]);
        assert_eq!(e.to_string(), "Pow(x, 2)");
    }

    // --- Standard head constants -------------------------------------------

    #[test]
    fn standard_heads_are_the_expected_strings() {
        assert_eq!(ADD, "Add");
        assert_eq!(MUL, "Mul");
        assert_eq!(POW, "Pow");
        assert_eq!(SIN, "Sin");
        assert_eq!(DEFINE, "Define");
    }

    // --- Constructors -------------------------------------------------------

    #[test]
    fn sym_builds_symbol() {
        assert_eq!(sym("Pi"), IRNode::Symbol("Pi".into()));
    }

    #[test]
    fn apply_builds_apply_node() {
        let e = apply(sym(ADD), vec![int(1), int(2)]);
        if let IRNode::Apply(a) = &e {
            assert_eq!(a.head, sym(ADD));
            assert_eq!(a.args.len(), 2);
        } else {
            panic!("expected Apply");
        }
    }

    #[test]
    fn apply_zero_args() {
        let e = apply(sym("True"), vec![]);
        if let IRNode::Apply(a) = &e {
            assert!(a.args.is_empty());
        } else {
            panic!("expected Apply");
        }
    }

    // --- gcd helper ---------------------------------------------------------

    #[test]
    fn gcd_basic() {
        assert_eq!(gcd(12, 8), 4);
        assert_eq!(gcd(7, 3), 1);
        assert_eq!(gcd(0, 5), 5);
        assert_eq!(gcd(5, 0), 5);
    }

    // --- Clone --------------------------------------------------------------

    #[test]
    fn clone_works_for_all_variants() {
        let nodes = vec![
            sym("x"),
            int(42),
            rat(1, 3),
            flt(2.71),
            str_node("test"),
            apply(sym(ADD), vec![int(1), int(2)]),
        ];
        for n in &nodes {
            assert_eq!(n, &n.clone());
        }
    }

    // --- Use in HashMap -----------------------------------------------------

    #[test]
    fn irnode_usable_as_hashmap_key() {
        use std::collections::HashMap;
        let mut m: HashMap<IRNode, i32> = HashMap::new();
        m.insert(sym("x"), 1);
        m.insert(sym("y"), 2);
        assert_eq!(m[&sym("x")], 1);
        assert_eq!(m[&sym("y")], 2);
    }
}
