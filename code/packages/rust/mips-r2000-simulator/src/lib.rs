//! # MIPS R2000 (1985) behavioral simulator
//!
//! Rust port of `code/packages/python/mips-r2000-simulator` (Layer 07q) —
//! see [`code/specs/07q-mips-r2000-simulator.md`](../../../specs/07q-mips-r2000-simulator.md)
//! for the full ISA writeup (this crate documents the port, not the ISA
//! semantics again).
//!
//! Module split mirrors [`riscv_simulator`]:
//!
//! ```text
//! opcodes.rs   -- opcode / funct-field constant tables (R/I/J formats)
//! encoding.rs  -- encode_* helpers to construct machine code words
//! decode.rs    -- instruction decoder for all three formats
//! execute.rs   -- instruction executor + big-endian memory accessors
//! simulator.rs -- top-level MipsR2000Simulator with fetch-decode-execute
//! ```
//!
//! ## What makes MIPS R2000 different from RV32I here
//!
//! - **Big-endian memory.**  `cpu_simulator::Memory::read_word`/
//!   `write_word` are little-endian (shared with the RISC-V/ARM/x86
//!   simulators), so `execute.rs` builds its own big-endian word/halfword
//!   accessors on top of `Memory`'s endian-agnostic `read_byte`/
//!   `write_byte`.
//! - **No branch-delay slots.**  Matches the Python original's explicit
//!   simplification — branches and jumps take effect immediately.
//! - **HI/LO registers.**  `MULT`/`MULTU`/`DIV`/`DIVU` write a 64-bit
//!   result across `hi`/`lo` fields on [`simulator::MipsR2000Simulator`]
//!   rather than a GPR, read back via `MFHI`/`MFLO`.
//! - **Fail-closed halting instead of exceptions.**  The Python simulator
//!   raises `ValueError` on `ADD`/`ADDI`/`SUB` signed overflow and on
//!   `DIV`/`DIVU` by zero.  This Rust port has no exception channel
//!   through `step() -> String`, so it halts instead (destination
//!   register/HI/LO left unwritten) — see `execute.rs` module docs.
//!
//! ## Usage
//!
//! ```rust
//! use mips_r2000_simulator::MipsR2000Simulator;
//! use mips_r2000_simulator::encoding::*;
//!
//! let mut sim = MipsR2000Simulator::new(65536);
//! sim.run_instructions(&[
//!     encode_addiu(8, 0, 1),   // $t0 = 1
//!     encode_addiu(9, 0, 2),   // $t1 = 2
//!     encode_add(10, 8, 9),    // $t2 = 3
//!     encode_syscall(),         // halt
//! ]);
//! assert_eq!(sim.regs.read(10), 3);
//! ```

pub mod decode;
pub mod encoding;
pub mod execute;
pub mod opcodes;
pub mod simulator;

pub use simulator::{ExecutionResult, MipsR2000Simulator};
