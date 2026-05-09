//! Newton-Raphson numeric root finding over symbolic IR.
//!
//! The crate mirrors Python `cas-mnewton` while staying independent from a
//! concrete VM. Callers provide two callbacks:
//!
//! - `eval_fn`: collapses a substituted IR expression to a numeric literal.
//! - `diff_fn`: computes the symbolic derivative once before iteration.
//!
//! This shape avoids an import cycle with `symbolic-vm` and keeps the core
//! suitable for standalone Rust and WASM use.

mod newton;

pub use newton::{ir_to_float, mnewton_solve, MNewtonError, MNewtonOptions, MNEWTON};
