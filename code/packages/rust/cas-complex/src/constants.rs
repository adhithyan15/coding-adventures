//! Complex number IR constants.
//!
//! The imaginary unit `I` is represented as a plain `IRNode::Symbol("I")` in
//! the symbolic IR.  It plays the same role as the physical constant `Pi`:
//! a named atom that carries special meaning to the CAS but is just a symbol
//! at the IR level.
//!
//! # IR representation of complex numbers
//!
//! A complex number `a + b·i` is expressed in the IR as:
//!
//! ```text
//! Add(a, Mul(b, I))
//! ```
//!
//! where `a` and `b` are themselves `IRNode` sub-expressions.  Pure imaginary
//! `b·i` is just `Mul(b, I)`.  The pure real `a` stays as `a`.
//!
//! Special cases:
//!
//! | Math | IR |
//! |------|----|
//! | `i`  | `Symbol("I")` |
//! | `2i` | `Mul(2, I)` |
//! | `3 + 4i` | `Add(3, Mul(4, I))` |
//! | `-i` | `Neg(I)` or `Mul(-1, I)` |
//!
//! ## Constants
//!
//! | Rust constant | Value | Meaning |
//! |---|---|---|
//! | [`IMAGINARY_UNIT`] | `"I"` | The imaginary unit symbol name |
//! | [`RE`] | `"Re"` | Real-part head |
//! | [`IM`] | `"Im"` | Imaginary-part head |
//! | [`CONJUGATE`] | `"Conjugate"` | Complex conjugate head |
//! | [`ABS`] | `"Abs"` | Complex modulus head |
//! | [`ARG`] | `"Arg"` | Complex argument (angle) head |

/// Symbol name for the imaginary unit `i` (√−1).
pub const IMAGINARY_UNIT: &str = "I";

/// Head for the real-part function: `Re(z)`.
pub const RE: &str = "Re";

/// Head for the imaginary-part function: `Im(z)`.
pub const IM: &str = "Im";

/// Head for the complex conjugate: `Conjugate(z)`.
pub const CONJUGATE: &str = "Conjugate";

/// Head for the complex modulus (absolute value): `Abs(z)`.
pub const ABS: &str = "Abs";

/// Head for the complex argument (phase angle): `Arg(z)`.
pub const ARG: &str = "Arg";
