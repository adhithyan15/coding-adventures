//! # intel8051-gatelevel
//!
//! Gate-level simulation of the Intel 8051 microcontroller (1980).
//!
//! Every arithmetic and logical operation routes through AND, OR, XOR, NOT
//! gates and a ripple-carry adder — no native Rust integer arithmetic in the
//! data path.
//!
//! ## Architecture overview
//!
//! The 8051 is an 8-bit Harvard-architecture microcontroller.  Three
//! independent memory spaces:
//!
//! | Space | Width  | Size  | Purpose |
//! |-------|--------|-------|---------|
//! | Code  | 8-bit  | 64 KB | Program instructions (ROM in real chips) |
//! | IRAM  | 8-bit  | 256 B | Internal RAM + SFRs |
//! | XDATA | 8-bit  | 64 KB | External data RAM |
//!
//! The internal RAM (IRAM) is divided into four regions:
//!
//! ```text
//! 0x00-0x1F  →  4 register banks (R0-R7 each), selected via PSW.RS1:RS0
//! 0x20-0x2F  →  bit-addressable area (128 individual bits, addresses 0x00-0x7F)
//! 0x30-0x7F  →  general-purpose scratchpad
//! 0x80-0xFF  →  Special Function Registers (SFRs)
//! ```
//!
//! SFRs sit at fixed IRAM addresses:
//!
//! | SFR  | Addr  | Purpose |
//! |------|-------|---------|
//! | P0   | 0x80  | Port 0 latch |
//! | SP   | 0x81  | Stack Pointer (init 0x07) |
//! | DPL  | 0x82  | Data Pointer low byte |
//! | DPH  | 0x83  | Data Pointer high byte |
//! | P1   | 0x90  | Port 1 latch |
//! | P2   | 0xA0  | Port 2 latch |
//! | P3   | 0xB0  | Port 3 latch |
//! | PSW  | 0xD0  | Program Status Word |
//! | ACC  | 0xE0  | Accumulator |
//! | B    | 0xF0  | B register (MUL/DIV helper) |
//!
//! ## PSW (Program Status Word) bit layout
//!
//! ```text
//! Bit 7  CY  — Carry flag
//! Bit 6  AC  — Auxiliary carry (half-carry from nibble)
//! Bit 5  F0  — User-defined flag
//! Bit 4  RS1 — Register bank select bit 1
//! Bit 3  RS0 — Register bank select bit 0
//! Bit 2  OV  — Overflow flag
//! Bit 1  —   — (reserved)
//! Bit 0  P   — Parity of ACC (1 = odd number of 1-bits)
//! ```
//!
//! ## Gate-level guarantee
//!
//! All data-path operations are implemented by routing bit vectors through
//! `full_adder` stages and individual `and_gate`, `or_gate`, `xor_gate`,
//! `not_gate` calls from the `arithmetic` and `logic-gates` crates.

pub mod alu;
pub mod bits;
pub mod cpu;
pub mod registers;
