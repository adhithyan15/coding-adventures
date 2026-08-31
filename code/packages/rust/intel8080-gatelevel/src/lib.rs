//! # Intel 8080 Gate-Level Simulator
//!
//! Every arithmetic and logic operation routes through real gate functions:
//! `AND → OR → XOR → NOT → half_adder → full_adder → ripple_carry_adder → ALU`.
//! Registers are modelled as D flip-flop arrays. The instruction decoder uses
//! combinational AND/OR/NOT gate trees to extract control signals from opcode bits.
//!
//! ## Why gate-level?
//!
//! The real Intel 8080A had ~6,000 transistors (NMOS). By simulating at gate level,
//! we can count exactly how many gates each operation uses and trace a bit through
//! the full 8-bit ripple-carry adder (8 full-adder stages = 40 gates).
//!
//! ## Architecture
//!
//! ```text
//! bits.rs      — integer ↔ bit-vector conversion; 8-bit and 16-bit adder wrappers
//! alu.rs       — GateAlu8080: all ALU operations through gate primitives
//! decoder.rs   — combinational instruction decoder (AND/NOT/OR gate tree)
//! registers.rs — 7×8-bit RegisterFile + Register16 (PC and SP)
//! cpu.rs       — GateLevelCpu: fetch-decode-execute loop + public API
//! ```
//!
//! ## Example
//!
//! ```rust
//! use coding_adventures_intel8080_gatelevel::GateLevelCpu;
//!
//! let mut cpu = GateLevelCpu::new();
//! // MVI A,10; MVI B,5; ADD B; HLT
//! let result = cpu
//!     .run(&[0x3E, 0x0A, 0x06, 0x05, 0x80, 0x76], 100)
//!     .unwrap();
//! assert_eq!(result.final_state.regs.a, 15);
//! assert!(!result.final_state.flags.cy);
//! ```

pub mod alu;
pub mod bits;
pub mod cpu;
pub mod decoder;
pub mod registers;
mod state;

pub use cpu::{GateLevelCpu, FLIP_FLOP_COUNT};
pub use intel8080_simulator::{ExecutionResult, Intel8080Error, Intel8080State, StepTrace};
