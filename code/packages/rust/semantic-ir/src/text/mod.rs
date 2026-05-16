//! Textual surface syntax for the SIR.
//!
//! The text format is a human-readable S-expression rendering of a
//! SIR module.  Every node kind has a head keyword that names it.
//! This module exposes:
//!
//! - [`print_module`] — render a `Module` to its canonical text form.
//! - [`print_expr`] — render a single `Expr` (handy for diagnostics).
//!
//! The printer is **deterministic**: the same input produces
//! byte-identical output across runs and platforms.  This is what
//! makes golden tests reliable.
//!
//! A text parser is deliberately out of scope for v0; round-trip
//! checks compare two printer runs of the same module rather than
//! parsing the printed output back.  A parser will be added when a
//! consumer needs it (e.g. an external tool wanting to feed SIR text
//! into a backend).

mod printer;

pub use printer::{print_block, print_expr, print_function, print_module};
