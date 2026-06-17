//! # Intel 8086 Gate-Level Simulator
//!
//! Every arithmetic and logical operation routes through real gate functions:
//! `AND → OR → XOR → NOT → full_adder → ripple_carry_adder → ALU`.
//! Registers are modelled as 16-bit D flip-flop arrays. The instruction decoder
//! routes opcode bits through combinational AND/OR gate trees.
//!
//! ## Why gate-level?
//!
//! The real Intel 8086 had ~29,000 transistors (NMOS, 3-micron). By simulating
//! at gate level, we can count exactly how many gates each operation uses and
//! trace a bit through the 16-bit ripple-carry adder (16 full-adder stages ≈ 80
//! gate outputs). This makes the simulator an ideal teaching tool for digital
//! logic.
//!
//! ## Architecture
//!
//! ```text
//! bits.rs      — integer ↔ bit-vector conversion; 8-/16-/20-bit adder wrappers
//!                nibble_borrow() for the AF flag in subtraction
//! alu.rs       — AluResult8086: all ALU operations through gate primitives
//!                add/sub/and/or/xor/inc/dec/neg/not, shifts, rotates, BCD, MUL/DIV
//! registers.rs — RegisterFile8086: all 14 registers + FLAGS + physical_address()
//! cpu.rs       — Cpu8086: full fetch-decode-execute loop, ~120 opcodes
//! ```
//!
//! ## Design constraints
//!
//! | Area              | Constraint |
//! |-------------------|-----------|
//! | Data path         | Every +/–/AND/OR/XOR goes through `full_adder` / gate functions |
//! | MUL/DIV           | Host arithmetic — gate-level ×16 multiplier is out of scope |
//! | Segment × 16      | Bit rewiring — not computed |
//! | Address bus       | 20-bit via `add_20bit()` ripple-carry chain |
//! | Memory            | 1 MB flat `Box<[u8; 1_048_576]>` |
//!
//! ## Example
//!
//! ```rust
//! use coding_adventures_intel8086_gatelevel::cpu::Cpu8086;
//!
//! let mut cpu = Cpu8086::new();
//! // MOV AX, 10; MOV BX, 5; ADD AX, BX; HLT
//! let steps = cpu.execute(&[
//!     0xB8, 10, 0,   // MOV AX, 10
//!     0xBB, 5, 0,    // MOV BX, 5
//!     0x03, 0xC3,    // ADD AX, BX  (03 /r)
//!     0xF4,          // HLT
//! ], 1000);
//! assert_eq!(cpu.rf.ax, 15);
//! assert_eq!(cpu.rf.flag_cf, 0);
//! assert!(cpu.halted);
//! ```

pub mod alu;
pub mod bits;
pub mod cpu;
pub mod registers;

pub use cpu::{Cpu8086, CpuState};
