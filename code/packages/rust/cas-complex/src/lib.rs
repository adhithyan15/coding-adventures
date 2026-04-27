//! # cas-complex
//!
//! Complex number operations over the symbolic IR.
//!
//! The imaginary unit `I` (√−1) is the symbol `"I"`.  A complex number
//! `a + b·i` is represented in the IR as `Add(a, Mul(b, I))`.
//!
//! ## Modules
//!
//! | Module | Functions |
//! |--------|-----------|
//! | [`constants`] | Symbol name constants (`IMAGINARY_UNIT`, `RE`, `IM`, …) |
//! | [`normalize`] | [`complex_normalize`] — rewrite to canonical `a + b·I` form |
//! | [`parts`] | [`real_part`], [`imag_part`], [`conjugate`] |
//! | [`polar`] | [`modulus`], [`argument`] |
//! | [`power`] | [`complex_pow`] — integer powers via De Moivre's theorem |
//!
//! ## Quick start
//!
//! ```rust
//! use cas_complex::{complex_normalize, real_part, imag_part, conjugate, modulus, complex_pow};
//! use symbolic_ir::{apply, int, sym, ADD, MUL};
//!
//! // 3 + 4*I
//! let z = apply(sym(ADD), vec![
//!     int(3),
//!     apply(sym(MUL), vec![int(4), sym("I")]),
//! ]);
//!
//! assert_eq!(real_part(&z), int(3));
//! assert_eq!(imag_part(&z), int(4));
//!
//! // (1 + I)^2 = 2*I
//! let w = apply(sym(ADD), vec![int(1), sym("I")]);
//! let result = complex_pow(&w, &int(2));
//! assert_eq!(imag_part(&result), int(2));
//!
//! // I^2 = -1
//! use symbolic_ir::POW;
//! let i_sq = apply(sym(POW), vec![sym("I"), int(2)]);
//! assert_eq!(complex_normalize(&i_sq), int(-1));
//! ```
//!
//! ## Stack position
//!
//! ```text
//! symbolic-ir  ←  cas-complex
//! ```

pub mod constants;
pub mod normalize;
pub mod parts;
pub mod polar;
pub mod power;

// Re-export the public API.

pub use constants::{ABS, ARG, CONJUGATE, IMAGINARY_UNIT, IM, RE};
pub use normalize::complex_normalize;
pub use parts::{conjugate, imag_part, real_part};
pub use polar::{argument, modulus};
pub use power::complex_pow;
