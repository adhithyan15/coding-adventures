//! Gate-level MIPS R2000 (1985) simulator.
//!
//! Every arithmetic and logical data-path operation routes through
//! `and_gate`, `or_gate`, `xor_gate`, `not_gate` from the `logic-gates`
//! crate and `ripple_carry_adder` from the `arithmetic` crate.
//! No native Rust integer arithmetic (`+`, `-`, `*`, `/`, `&`, `|`, `^`)
//! appears in any data-path computation.

pub mod alu;
pub mod bits;
pub mod cpu;
pub mod decoder;
pub mod register_file;
