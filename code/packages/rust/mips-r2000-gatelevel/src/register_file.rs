//! register_file.rs — Gate-level register file for the MIPS R2000.
//!
//! # Structure
//!
//! The MIPS R2000 has 32 general-purpose registers (GPRs) plus HI, LO, and PC.
//! In real hardware, each register is implemented as 32 D flip-flops — one
//! per bit.  We model this as a `[[u8; 32]; 32]` 2D array: `gprs[n]` holds
//! the 32-bit LSB-first bit array for GPR n.
//!
//! # R0 ($zero)
//!
//! Reads always return 0.  Writes are silently discarded.
//! On the real chip, R0 is tied to ground (constant 0 source).
//!
//! # PC increment
//!
//! `increment_pc` uses `add_32bit` (gate-level ripple-carry adder) to add 4.

use crate::bits::{add_32bit, bits_to_u32, int_to_bits32};

/// MIPS R2000 register file.
///
/// Stores all 32 GPRs, HI, LO, and PC as LSB-first bit arrays.
pub struct RegisterFile32 {
    /// 32 GPRs × 32 bits each — the "flip-flop" storage.
    gprs: [[u8; 32]; 32],
    /// HI register (upper 32 bits of MULT/DIV result).
    hi: [u8; 32],
    /// LO register (lower 32 bits of MULT/DIV result).
    lo: [u8; 32],
    /// Program Counter.
    pc: [u8; 32],
}

impl RegisterFile32 {
    /// Create a new zeroed register file.
    pub fn new() -> Self {
        Self {
            gprs: [[0u8; 32]; 32],
            hi: [0u8; 32],
            lo: [0u8; 32],
            pc: [0u8; 32],
        }
    }

    // ── GPRs ─────────────────────────────────────────────────────────────────

    /// Read GPR `n` as a `u32`.  R0 always returns 0.
    ///
    /// ```
    /// # use coding_adventures_mips_r2000_gatelevel::register_file::RegisterFile32;
    /// let mut rf = RegisterFile32::new();
    /// rf.write_reg(1, 42);
    /// assert_eq!(rf.read_reg(1), 42);
    /// assert_eq!(rf.read_reg(0), 0); // R0 = $zero always 0
    /// ```
    pub fn read_reg(&self, n: usize) -> u32 {
        if n == 0 {
            return 0;
        }
        bits_to_u32(self.gprs[n])
    }

    /// Write `value` to GPR `n`.  Writes to R0 are silently discarded.
    pub fn write_reg(&mut self, n: usize, value: u32) {
        if n == 0 {
            return; // R0 hardwired to 0
        }
        self.gprs[n] = int_to_bits32(value);
    }

    // ── HI ───────────────────────────────────────────────────────────────────

    /// Read HI as `u32`.
    pub fn read_hi(&self) -> u32 {
        bits_to_u32(self.hi)
    }

    /// Write `value` to HI.
    pub fn write_hi(&mut self, value: u32) {
        self.hi = int_to_bits32(value);
    }

    // ── LO ───────────────────────────────────────────────────────────────────

    /// Read LO as `u32`.
    pub fn read_lo(&self) -> u32 {
        bits_to_u32(self.lo)
    }

    /// Write `value` to LO.
    pub fn write_lo(&mut self, value: u32) {
        self.lo = int_to_bits32(value);
    }

    // ── PC ───────────────────────────────────────────────────────────────────

    /// Read PC as `u32`.
    pub fn read_pc(&self) -> u32 {
        bits_to_u32(self.pc)
    }

    /// Write `value` to PC.
    pub fn write_pc(&mut self, value: u32) {
        self.pc = int_to_bits32(value);
    }

    /// Increment PC by `by` bytes using a gate-level ripple-carry adder.
    ///
    /// On the real MIPS R2000, a dedicated adder hardwired to add 4 advances
    /// the PC after each instruction fetch.  We model this with `add_32bit`.
    ///
    /// ```
    /// # use coding_adventures_mips_r2000_gatelevel::register_file::RegisterFile32;
    /// let mut rf = RegisterFile32::new();
    /// rf.increment_pc(4);
    /// assert_eq!(rf.read_pc(), 4);
    /// rf.increment_pc(4);
    /// assert_eq!(rf.read_pc(), 8);
    /// ```
    pub fn increment_pc(&mut self, by: u32) {
        let current = bits_to_u32(self.pc);
        let (new_pc, _, _) = add_32bit(current, by, 0);
        self.pc = int_to_bits32(new_pc);
    }
}

impl Default for RegisterFile32 {
    fn default() -> Self {
        Self::new()
    }
}
