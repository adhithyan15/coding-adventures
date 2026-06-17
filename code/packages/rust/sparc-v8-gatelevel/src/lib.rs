//! SPARC V8 gate-level simulator.
//!
//! Every data-path operation is built from logic gates (`and_gate`, `or_gate`,
//! `xor_gate`, `not_gate`) and the ripple-carry adder from the `arithmetic`
//! crate.  Nothing uses native `+`, `-`, `*`, `/`, `&`, `|`, `^`, or `!` on
//! multi-bit integers in the ALU or CPU paths; those primitives live only here
//! in the bit-vector conversion helpers.
//!
//! # Architecture overview
//!
//! ```text
//!  ┌─────────────────────────────────────────────────┐
//!  │                   SparcCpu                      │
//!  │  ┌────────────┐  ┌──────────┐  ┌────────────┐  │
//!  │  │ RegisterFile│  │  Alu     │  │  memory[]  │  │
//!  │  │ 56 phys regs│  │ gate ops │  │ 64 KiB     │  │
//!  │  │ PSR, PC, Y  │  └──────────┘  └────────────┘  │
//!  │  └────────────┘                                  │
//!  └─────────────────────────────────────────────────┘
//! ```

pub mod alu;
pub mod bits;
pub mod cpu;
pub mod decoder;
pub mod register_file;

pub use cpu::{SparcCpu, SparcError};
