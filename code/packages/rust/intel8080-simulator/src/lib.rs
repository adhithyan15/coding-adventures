//! # Intel 8080 Simulator (Rust)
//!
//! Behavioral simulator for the Intel 8080 (1974) — Intel's first widely
//! successful 8-bit microprocessor, direct successor to the 8008, and the
//! CPU inside the Altair 8800 that launched the personal-computer era.
//! Rust port of `code/packages/python/intel8080-simulator`; see
//! [`code/specs/07i-intel8080-simulator.md`](../../../specs/07i-intel8080-simulator.md)
//! for the full ISA writeup.
//!
//! Module split mirrors [`mips_r2000_simulator`] (the template for this,
//! the third lane of the 9-architecture expansion):
//!
//! ```text
//! opcodes.rs   -- opcode / register / condition-code constant tables
//! encoding.rs  -- encode_* helpers to construct machine code byte sequences
//! decode.rs    -- variable-length instruction decoder (1, 2, or 3 bytes)
//! execute.rs   -- instruction executor + named-register state
//! simulator.rs -- top-level Intel8080Simulator with fetch-decode-execute
//! ```
//!
//! Unlike MIPS R2000 / RISC-V (fixed 32-bit words, indexed register files),
//! the 8080 is variable-length (1-3 bytes/instruction) with seven
//! individually named 8-bit registers (A, B, C, D, E, H, L) — so
//! [`execute::Registers`] is a plain named-field struct rather than
//! `cpu_simulator::RegisterFile`.  `cpu_simulator::Memory` is still used
//! for the flat, byte-addressable 64Ki memory.
//!
//! ## Quick start
//!
//! ```
//! use intel8080_simulator::Intel8080Simulator;
//! use intel8080_simulator::encoding::{assemble, encode_mvi_a};
//! use intel8080_simulator::opcodes::HLT;
//!
//! let mut sim = Intel8080Simulator::new(65536);
//! sim.run(&assemble(&[encode_mvi_a(42), vec![HLT]]));
//! assert_eq!(sim.regs.a, 42);
//! assert!(sim.halted);
//! ```

pub mod decode;
pub mod encoding;
pub mod execute;
pub mod opcodes;
pub mod simulator;

pub use simulator::{ExecutionResult, Intel8080Simulator};
