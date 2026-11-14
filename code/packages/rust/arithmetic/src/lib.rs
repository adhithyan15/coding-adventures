//! # Arithmetic — adder circuits and ALU built from logic gates.
//!
//! This crate implements the arithmetic building blocks of a CPU:
//!
//! - **`adders`** — half adder, full adder, and ripple-carry adder for N-bit addition
//! - **`alu`** — Arithmetic Logic Unit with ADD, SUB, AND, OR, XOR, NOT operations
//!   and status flags (zero, carry, negative, overflow)
//!
//! Everything is built on top of the [`logic_gates`] crate, using only
//! fundamental gates (AND, OR, XOR, NOT). This mirrors how real hardware
//! implements arithmetic — there is no "add instruction" in silicon, just
//! carefully wired logic gates.

pub mod adders;
pub mod alu;

// The Warrant evidence harness (VER04h) resolves an `export=` by name at the crate
// root, so the public operations and the three result types are re-exported there.
// They are the same items as `adders::half_adder` and `alu::alu`, so no existing call
// site changes.
pub use adders::{
    full_adder, half_adder, ripple_carry_adder, ripple_carry_adder_with_carry, RippleCarryResult,
};
pub use alu::{alu, AluOp, AluResult};
