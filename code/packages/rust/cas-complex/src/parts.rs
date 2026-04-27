//! Real and imaginary part extraction.
//!
//! Given a complex expression `z` in canonical `a + b·I` form (or anything
//! expressible in that form), these functions return its real and imaginary
//! components as plain (non-complex) IR expressions.
//!
//! Both functions normalise the input first via [`complex_normalize`] before
//! extracting the part.
//!
//! # Examples
//!
//! ```text
//! real_part(3 + 4*I) = 3
//! real_part(5)       = 5      — a real number
//! real_part(2*I)     = 0
//!
//! imag_part(3 + 4*I) = 4
//! imag_part(5)       = 0
//! imag_part(I)       = 1
//! ```

use symbolic_ir::IRNode;

use crate::normalize::split_complex;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Return the real part of `expr`.
///
/// Normalises `expr` into `a + b·I` form and returns `a`.
///
/// # Examples
///
/// ```rust
/// use cas_complex::real_part;
/// use symbolic_ir::{apply, int, sym, ADD, MUL};
///
/// // Re(3 + 4*I) = 3
/// let z = apply(sym(ADD), vec![int(3), apply(sym(MUL), vec![int(4), sym("I")])]);
/// assert_eq!(real_part(&z), int(3));
///
/// // Re(5) = 5
/// assert_eq!(real_part(&int(5)), int(5));
///
/// // Re(2*I) = 0
/// let pure_imag = apply(sym(MUL), vec![int(2), sym("I")]);
/// assert_eq!(real_part(&pure_imag), int(0));
/// ```
pub fn real_part(expr: &IRNode) -> IRNode {
    let (re, _im) = split_complex(expr);
    re
}

/// Return the imaginary part of `expr`.
///
/// Normalises `expr` into `a + b·I` form and returns `b` (the coefficient
/// of `I`).
///
/// # Examples
///
/// ```rust
/// use cas_complex::imag_part;
/// use symbolic_ir::{apply, int, sym, ADD, MUL};
///
/// // Im(3 + 4*I) = 4
/// let z = apply(sym(ADD), vec![int(3), apply(sym(MUL), vec![int(4), sym("I")])]);
/// assert_eq!(imag_part(&z), int(4));
///
/// // Im(5) = 0
/// assert_eq!(imag_part(&int(5)), int(0));
///
/// // Im(I) = 1
/// assert_eq!(imag_part(&sym("I")), int(1));
/// ```
pub fn imag_part(expr: &IRNode) -> IRNode {
    let (_re, im) = split_complex(expr);
    im
}

/// Return the complex conjugate of `expr`.
///
/// If `expr = a + b·I`, returns `a − b·I` (negate the imaginary part).
///
/// # Examples
///
/// ```rust
/// use cas_complex::{conjugate, imag_part};
/// use symbolic_ir::{apply, int, sym, ADD, MUL};
///
/// // conj(3 + 4*I) = 3 - 4*I (represented as Add(3, Mul(-4, I)))
/// let z = apply(sym(ADD), vec![int(3), apply(sym(MUL), vec![int(4), sym("I")])]);
/// let c = conjugate(&z);
/// // imaginary part of conjugate is -4
/// assert_eq!(imag_part(&c), int(-4));
/// ```
pub fn conjugate(expr: &IRNode) -> IRNode {
    use crate::normalize::{assemble, split_complex};
    use symbolic_ir::{apply, sym, IRNode};

    let (re, im) = split_complex(expr);
    // Negate imaginary part.
    let neg_im = match im {
        IRNode::Integer(n) => symbolic_ir::int(-n),
        IRNode::Float(f) => symbolic_ir::flt(-f),
        other => apply(sym(symbolic_ir::NEG), vec![other]),
    };
    assemble(re, neg_im)
}
