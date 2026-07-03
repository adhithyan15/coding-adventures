//! Gate-level register file for the Intel 8051.
//!
//! The 8051 has 256 bytes of internal RAM (IRAM) that serves triple duty:
//! general-purpose registers, bit-addressable space, and Special Function
//! Registers (SFRs).  The 16-bit Program Counter (PC) lives in dedicated
//! flip-flops outside IRAM.
//!
//! # IRAM layout
//!
//! ```text
//! 0x00-0x1F  →  Register banks 0-3 (R0-R7 each, 8 bytes per bank)
//! 0x20-0x2F  →  Bit-addressable RAM (128 bits = 16 bytes)
//! 0x30-0x7F  →  General scratchpad
//! 0x80-0xFF  →  Special Function Registers (SFRs)
//! ```
//!
//! # Bit addressing
//!
//! The 8051 can address individual bits via a 256-entry bit address space:
//!
//! ```text
//! Bit addr 0x00-0x7F  →  byte = 0x20 + (bit_addr >> 3),  bit = bit_addr & 7
//! Bit addr 0x80-0xFF  →  byte = bit_addr & 0xF8,          bit = bit_addr & 7
//! ```
//!
//! This means, for example, bit address 0xD7 (PSW.CY) maps to byte 0xD0
//! (PSW SFR) at bit position 7.
//!
//! # PC increment
//!
//! PC is incremented using the gate-level `add_16bit_full` adder from
//! `bits.rs`, keeping the PC update in the gate-level data path.

use crate::bits::add_16bit_full;

/// The 8051 register file: 256-byte IRAM (including SFRs) + 16-bit PC.
///
/// IRAM is stored as a flat `[u8; 256]` array.  All arithmetic/logic
/// operations on register contents go through the ALU gate primitives —
/// this struct only handles storage and addressing.
pub struct RegisterFile8051 {
    /// Internal RAM: 0x00-0x7F lower RAM, 0x80-0xFF SFRs.
    pub iram: [u8; 256],
    /// Program counter (16-bit, not memory-mapped on real 8051).
    pub pc: u16,
}

impl RegisterFile8051 {
    /// Create a zeroed register file.
    ///
    /// Real 8051 power-on state is undefined for most IRAM; we zero for
    /// determinism.  The caller (CPU reset) is responsible for setting
    /// SFR initial values (SP=0x07, P0-P3=0xFF, etc.).
    pub fn new() -> Self {
        Self { iram: [0u8; 256], pc: 0 }
    }

    // ── IRAM byte access ─────────────────────────────────────────────────────

    /// Read one byte from IRAM (or SFR space) at `addr`.
    #[inline]
    pub fn read_iram8(&self, addr: u8) -> u8 {
        self.iram[addr as usize]
    }

    /// Write one byte to IRAM (or SFR space) at `addr`.
    #[inline]
    pub fn write_iram8(&mut self, addr: u8, val: u8) {
        self.iram[addr as usize] = val;
    }

    // ── PC access ────────────────────────────────────────────────────────────

    /// Read the 16-bit program counter.
    #[inline]
    pub fn read_pc(&self) -> u16 {
        self.pc
    }

    /// Write the 16-bit program counter.
    #[inline]
    pub fn write_pc(&mut self, val: u16) {
        self.pc = val;
    }

    /// Increment the PC by `by` using the gate-level 16-bit adder.
    ///
    /// Wraps at 0xFFFF → 0x0000.
    pub fn increment_pc(&mut self, by: u16) {
        let (new_pc, _carry) = add_16bit_full(self.pc, by, 0);
        self.pc = new_pc;
    }

    // ── Bit addressing ───────────────────────────────────────────────────────

    /// Resolve a bit address to `(byte_addr, bit_position)`.
    ///
    /// The 8051 bit address space is split into two ranges:
    ///
    /// | Range | Byte mapping | Bit mapping |
    /// |-------|-------------|-------------|
    /// | 0x00-0x7F | 0x20 + (bit_addr >> 3) | bit_addr & 7 |
    /// | 0x80-0xFF | bit_addr & 0xF8         | bit_addr & 7 |
    ///
    /// Bit position 0 = LSB (value 1), position 7 = MSB (value 128).
    pub fn resolve_bit_addr(&self, bit_addr: u8) -> (u8, u8) {
        if bit_addr < 0x80 {
            let byte_addr = 0x20u8.wrapping_add(bit_addr >> 3);
            let bit_pos = bit_addr & 0x07;
            (byte_addr, bit_pos)
        } else {
            let byte_addr = bit_addr & 0xF8;
            let bit_pos = bit_addr & 0x07;
            (byte_addr, bit_pos)
        }
    }

    /// Read one bit from the bit-addressable space.  Returns 0 or 1.
    pub fn read_bit(&self, bit_addr: u8) -> u8 {
        let (byte_addr, bit_pos) = self.resolve_bit_addr(bit_addr);
        (self.iram[byte_addr as usize] >> bit_pos) & 1
    }

    /// Write one bit to the bit-addressable space (`val` must be 0 or 1).
    pub fn write_bit(&mut self, bit_addr: u8, val: u8) {
        let (byte_addr, bit_pos) = self.resolve_bit_addr(bit_addr);
        if val & 1 != 0 {
            self.iram[byte_addr as usize] |= 1 << bit_pos;
        } else {
            self.iram[byte_addr as usize] &= !(1u8 << bit_pos);
        }
    }
}

impl Default for RegisterFile8051 {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iram_round_trip() {
        let mut rf = RegisterFile8051::new();
        rf.write_iram8(0xE0, 0xAB); // ACC
        assert_eq!(rf.read_iram8(0xE0), 0xAB);
        rf.write_iram8(0x00, 0x55); // R0 bank0
        assert_eq!(rf.read_iram8(0x00), 0x55);
    }

    #[test]
    fn pc_increment() {
        let mut rf = RegisterFile8051::new();
        rf.write_pc(0xFFFE);
        rf.increment_pc(1);
        assert_eq!(rf.read_pc(), 0xFFFF);
        rf.increment_pc(1); // wraps
        assert_eq!(rf.read_pc(), 0x0000);
    }

    #[test]
    fn bit_addressing_lower() {
        let mut rf = RegisterFile8051::new();
        // Bit addr 0x00-0x07 map to byte 0x20, bits 0-7
        rf.write_bit(0x00, 1); // byte 0x20, bit 0
        assert_eq!(rf.read_bit(0x00), 1);
        assert_eq!(rf.iram[0x20], 0x01);
        rf.write_bit(0x07, 1); // byte 0x20, bit 7
        assert_eq!(rf.read_bit(0x07), 1);
        assert_eq!(rf.iram[0x20], 0x81);
        rf.write_bit(0x00, 0); // clear bit 0
        assert_eq!(rf.read_bit(0x00), 0);
        assert_eq!(rf.iram[0x20], 0x80);
    }

    #[test]
    fn bit_addressing_sfr() {
        let mut rf = RegisterFile8051::new();
        // PSW bit 7 (CY) = bit addr 0xD7
        rf.write_bit(0xD7, 1);
        assert_eq!(rf.read_bit(0xD7), 1);
        assert_eq!(rf.iram[0xD0], 0x80); // PSW byte, bit 7 set
        // PSW bit 0 (P) = bit addr 0xD0
        rf.write_bit(0xD0, 1);
        assert_eq!(rf.read_bit(0xD0), 1);
        assert_eq!(rf.iram[0xD0], 0x81); // bits 7 and 0
    }

    #[test]
    fn resolve_bit_addr_boundaries() {
        let rf = RegisterFile8051::new();
        // 0x00 → (0x20, 0)
        assert_eq!(rf.resolve_bit_addr(0x00), (0x20, 0));
        // 0x7F → (0x2F, 7) = 0x20 + (0x7F>>3=15) = 0x2F, bit = 7
        assert_eq!(rf.resolve_bit_addr(0x7F), (0x2F, 7));
        // 0x80 → (0x80, 0)
        assert_eq!(rf.resolve_bit_addr(0x80), (0x80, 0));
        // 0xD7 → (0xD0, 7)
        assert_eq!(rf.resolve_bit_addr(0xD7), (0xD0, 7));
        // 0xFF → (0xF8, 7)
        assert_eq!(rf.resolve_bit_addr(0xFF), (0xF8, 7));
    }
}
