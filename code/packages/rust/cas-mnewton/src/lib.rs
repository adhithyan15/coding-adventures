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

mod handlers;
mod newton;

pub use handlers::{
    build_mnewton_handler_table, mnewton_handler, DiffFn, EvalFn, MNewtonHandler,
    MNewtonHandlerTable,
};
pub use newton::{ir_to_float, mnewton_solve, MNewtonError, MNewtonOptions, MNEWTON};
