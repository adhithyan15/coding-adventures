//! Intel 4004 Gate-Level Simulator -- every operation routes through real logic gates.
//!
//! All computation flows through: NOT/AND/OR/XOR -> half_adder -> full_adder ->
//! ripple_carry_adder -> ALU, and state is stored in D flip-flop registers.
//!
//! # Architecture
//!
//! The simulator is organized into modules matching the real 4004's hardware blocks:
//!
//! - **`bits`** -- conversion between integers and bit vectors (LSB first)
//! - **`gate_alu`** -- 4-bit ALU built from logic gates and adders
//! - **`registers`** -- 16x4-bit register file, accumulator, and carry flag (all flip-flops)
//! - **`decoder`** -- combinational instruction decoder using AND/OR/NOT gates
//! - **`pc`** -- 12-bit program counter with half-adder incrementer
//! - **`stack`** -- 3-level hardware call stack (36 flip-flops)
//! - **`ram`** -- 4 banks x 4 registers x 20 nibbles (1,280 flip-flops)
//! - **`cpu`** -- top-level CPU tying all components together
//!
//! # Example
//!
//! ```
//! use intel4004_gatelevel::Intel4004GateLevel;
//!
//! let mut cpu = Intel4004GateLevel::new();
//! // LDM 5 (load 5 into accumulator), HLT (halt)
//! let traces = cpu.run(&[0xD5, 0x01], 100);
//! assert_eq!(cpu.accumulator(), 5);
//! assert!(cpu.halted());
//! ```

pub mod bits;
pub mod cpu;
pub mod decoder;
pub mod gate_alu;
pub mod pc;
pub mod ram;
pub mod registers;
pub mod stack;

pub use cpu::{GateTrace, Intel4004GateLevel};
