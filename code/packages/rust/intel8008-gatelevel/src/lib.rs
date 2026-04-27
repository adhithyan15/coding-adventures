//! # Intel 8008 Gate-Level Simulator
//!
//! Every arithmetic operation routes through real logic gate functions:
//! AND → OR → XOR → NOT → half_adder → full_adder → ripple_carry_adder → ALU.
//! Registers are implemented as D flip-flop arrays. The instruction decoder
//! uses combinational AND/OR/NOT gate trees to decompose opcode bits into
//! control signals.
//!
//! ## Why gate-level?
//!
//! The real Intel 8008 had ~3,500 transistors. By simulating at gate level,
//! we can count exactly how many gates each operation uses, trace a bit through
//! the full 8-bit ripple-carry adder (8 full-adder stages = 40 gates), and
//! understand how the hardware decoder produces control signals from opcode bits.
//!
//! ## Architecture
//!
//! ```text
//! bits.rs      — integer ↔ bit-vector conversion (LSB-first)
//! alu.rs       — 8-bit ALU wrapping the arithmetic crate's ripple-carry adder
//! registers.rs — 7×8-bit register file built from D flip-flops
//! decoder.rs   — combinational instruction decoder using AND/OR/NOT gate trees
//! pc.rs        — 14-bit program counter with half-adder incrementer
//! stack.rs     — 8-level push-down stack (8×14-bit flip-flop arrays)
//! cpu.rs       — top-level wiring + public API
//! ```
//!
//! ## Example
//!
//! ```rust
//! use coding_adventures_intel8008_gatelevel::GateLevelCpu;
//!
//! let mut cpu = GateLevelCpu::new();
//! // MVI B,1; MVI A,2; ADD B; HLT
//! let program = &[0x06u8, 0x01, 0x3E, 0x02, 0x80, 0x76];
//! let traces = cpu.run(program, 100);
//! assert_eq!(cpu.a(), 3);
//! ```

pub mod alu;
pub mod bits;
pub mod cpu;
pub mod decoder;
pub mod pc;
pub mod registers;
pub mod stack;

pub use cpu::GateLevelCpu;
