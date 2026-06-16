//! Register file for the Intel 8080 gate-level simulator.
//!
//! # Architecture
//!
//! The 8080 has seven 8-bit working registers: B, C, D, E, H, L, A.
//! It also has a 16-bit program counter (PC) and 16-bit stack pointer (SP).
//!
//! Compared to the 8008:
//! - Same 7 working registers (same 3-bit encoding)
//! - PC expanded to 16 bits (was 14-bit in 8008)
//! - SP is now explicit 16-bit (8008 had a push-down hardware stack)
//! - 64 KB address space (8008: 16 KB)
//!
//! # Register encoding (3-bit field)
//!
//! | Code | Register |
//! |------|----------|
//! | 000  | B (idx 0)|
//! | 001  | C (idx 1)|
//! | 010  | D (idx 2)|
//! | 011  | E (idx 3)|
//! | 100  | H (idx 4)|
//! | 101  | L (idx 5)|
//! | 110  | M (pseudo — raises panic if accessed directly)|
//! | 111  | A (idx 6)|
//!
//! Note: We store A at internal index 6 (to match 7 registers total),
//! but the opcode field 111 maps to the accumulator.
//!
//! # Register pair codes (2-bit)
//!
//! | Code | High | Low |
//! |------|------|-----|
//! | 00   | B    | C   |
//! | 01   | D    | E   |
//! | 10   | H    | L   |
//! | 11   | SP (16-bit register) |
//!
//! # Gate cost
//!
//! Each 8-bit register: 8 D flip-flops ≈ 48 gates (8 × 6 per flip-flop).
//! 7 × 8-bit + flags ≈ 336 + 40 gates.
//! 2 × 16-bit (PC + SP) ≈ 192 gates.

use crate::bits::{add_16bit, bits_to_u16, int_to_bits16};

// Register index constants matching the 3-bit opcode encoding
pub const REG_B: u8 = 0;
pub const REG_C: u8 = 1;
pub const REG_D: u8 = 2;
pub const REG_E: u8 = 3;
pub const REG_H: u8 = 4;
pub const REG_L: u8 = 5;
pub const REG_M: u8 = 6; // pseudo-register — use hl_addr() for memory access
pub const REG_A: u8 = 7;

// Register pair codes
pub const PAIR_BC: u8 = 0;
pub const PAIR_DE: u8 = 1;
pub const PAIR_HL: u8 = 2;
pub const PAIR_SP: u8 = 3;

/// Seven 8-bit working registers, modelled as D flip-flop arrays.
///
/// Internal layout: indices 0–5 = B,C,D,E,H,L; index 6 = A.
/// The M pseudo-register (opcode code 6) is NOT stored here.
pub struct RegisterFile {
    // bits[reg][bit]: bit[0] = LSB, bit[7] = MSB (LSB-first flip-flop ordering)
    bits: [[u8; 8]; 7],
}

impl Default for RegisterFile {
    fn default() -> Self {
        Self::new()
    }
}

impl RegisterFile {
    /// Create a register file with all bits cleared.
    pub fn new() -> Self {
        RegisterFile { bits: [[0u8; 8]; 7] }
    }

    /// Read a register by its 3-bit opcode code.
    ///
    /// # Panics
    /// Panics if `reg == REG_M` (6) — caller must handle M as a memory access.
    pub fn read(&self, reg: u8) -> u8 {
        let idx = self.idx(reg);
        let bits = &self.bits[idx];
        bits.iter().enumerate().fold(0u8, |acc, (i, &b)| acc | (b << i))
    }

    /// Write a register by its 3-bit opcode code.
    ///
    /// # Panics
    /// Panics if `reg == REG_M` (6).
    pub fn write(&mut self, reg: u8, value: u8) {
        let idx = self.idx(reg);
        for i in 0..8 {
            self.bits[idx][i] = (value >> i) & 1;
        }
    }

    /// Read a 16-bit register pair, returning (high, low) bytes.
    ///
    /// Pair codes: 0=BC, 1=DE, 2=HL. Pair 3 (SP) is handled by `Register16`.
    pub fn read_pair(&self, pair: u8) -> u16 {
        let (hi, lo) = self.pair_regs(pair);
        ((self.read(hi) as u16) << 8) | (self.read(lo) as u16)
    }

    /// Write a 16-bit value into a register pair.
    pub fn write_pair(&mut self, pair: u8, value: u16) {
        let (hi, lo) = self.pair_regs(pair);
        self.write(hi, (value >> 8) as u8);
        self.write(lo, (value & 0xFF) as u8);
    }

    /// HL address: the 16-bit value of the H:L pair (memory pointer).
    pub fn hl_addr(&self) -> u16 {
        self.read_pair(PAIR_HL)
    }

    fn pair_regs(&self, pair: u8) -> (u8, u8) {
        match pair & 3 {
            0 => (REG_B, REG_C),
            1 => (REG_D, REG_E),
            2 => (REG_H, REG_L),
            _ => panic!("pair 3 (SP) is a 16-bit register, handled separately"),
        }
    }

    fn idx(&self, reg: u8) -> usize {
        match reg {
            0..=5 => reg as usize,       // B=0, C=1, D=2, E=3, H=4, L=5
            7 => 6,                       // A maps to internal index 6
            6 => panic!("REG_M (6) is a pseudo-register; handle as memory access"),
            _ => panic!("invalid register code {reg}"),
        }
    }
}

/// 16-bit register built from a 16-element D flip-flop array.
///
/// Used for both the program counter (PC) and stack pointer (SP).
///
/// Every increment or decrement routes through the `add_16bit` gate chain —
/// 16 full-adder stages, the same ripple-carry adder used for DAD.
pub struct Register16 {
    bits: [u8; 16],
}

impl Default for Register16 {
    fn default() -> Self {
        Self::new()
    }
}

impl Register16 {
    /// Create a 16-bit register cleared to 0.
    pub fn new() -> Self {
        Register16 { bits: [0u8; 16] }
    }

    /// Read the current value.
    pub fn read(&self) -> u16 {
        bits_to_u16(&self.bits)
    }

    /// Write a new value (clock all 16 flip-flops).
    pub fn write(&mut self, value: u16) {
        let bits = int_to_bits16(value);
        self.bits.copy_from_slice(&bits);
    }

    /// Increment by `n` through the 16-bit ripple-carry adder chain.
    pub fn inc(&mut self, n: u16) {
        let (new_val, _carry) = add_16bit(self.read(), n, 0);
        self.write(new_val);
    }

    /// Decrement by `n` through the adder chain (as a + NOT(b) + 1).
    pub fn dec(&mut self, n: u16) {
        // a - n = a + NOT(n) + 1
        let not_n = !n;
        let (new_val, _carry) = add_16bit(self.read(), not_n, 1);
        self.write(new_val);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_write_all_regs() {
        let mut rf = RegisterFile::new();
        for reg in [REG_B, REG_C, REG_D, REG_E, REG_H, REG_L, REG_A] {
            rf.write(reg, 0xAB);
            assert_eq!(rf.read(reg), 0xAB, "failed for reg {reg}");
        }
    }

    #[test]
    fn register_pairs() {
        let mut rf = RegisterFile::new();
        rf.write_pair(PAIR_BC, 0x1234);
        assert_eq!(rf.read(REG_B), 0x12);
        assert_eq!(rf.read(REG_C), 0x34);
        assert_eq!(rf.read_pair(PAIR_BC), 0x1234);
    }

    #[test]
    fn hl_addr() {
        let mut rf = RegisterFile::new();
        rf.write_pair(PAIR_HL, 0x2050);
        assert_eq!(rf.hl_addr(), 0x2050);
    }

    #[test]
    fn pc_increment() {
        let mut pc = Register16::new();
        pc.inc(1);
        assert_eq!(pc.read(), 1);
        pc.inc(0xFFFE);
        assert_eq!(pc.read(), 0xFFFF);
        pc.inc(1);
        assert_eq!(pc.read(), 0); // wrap on 16-bit overflow
    }

    #[test]
    fn sp_decrement() {
        let mut sp = Register16::new();
        sp.write(0x2400);
        sp.dec(2);
        assert_eq!(sp.read(), 0x23FE);
    }

    #[test]
    #[should_panic(expected = "pseudo-register")]
    fn reg_m_panics() {
        let rf = RegisterFile::new();
        rf.read(REG_M);
    }
}
