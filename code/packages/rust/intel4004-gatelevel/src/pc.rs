//! Program counter -- 12-bit register with increment and load.
//!
//! # The 4004's program counter
//!
//! The program counter (PC) holds the address of the next instruction to
//! fetch from ROM. It's 12 bits wide, addressing 4096 bytes of ROM.
//!
//! In real hardware, the PC is:
//! - A 12-bit register (12 D flip-flops)
//! - An incrementer (chain of half-adders for PC+1 or PC+2)
//! - A load input (for jump instructions)
//!
//! The incrementer uses half-adders chained together. To add 1:
//!
//! ```text
//! bit0 -> half_adder(bit0, 1) -> sum0, carry
//! bit1 -> half_adder(bit1, carry) -> sum1, carry
//! ...and so on for all 12 bits.
//! ```
//!
//! This is simpler than a full adder chain because we're always adding
//! a constant (1 or 2), so one input is fixed.

use arithmetic::adders::half_adder;
use logic_gates::sequential::{register, FlipFlopState};

use crate::bits::{bits_to_int, int_to_bits};

/// 12-bit program counter built from flip-flops and half-adders.
///
/// Supports:
/// - `increment()`: PC += 1 (for 1-byte instructions)
/// - `increment2()`: PC += 2 (for 2-byte instructions)
/// - `load(addr)`: PC = addr (for jumps)
/// - `read()`: current PC value
pub struct ProgramCounter {
    state: Vec<FlipFlopState>,
}

impl ProgramCounter {
    /// Initialize PC to 0.
    pub fn new() -> Self {
        let mut state: Vec<FlipFlopState> =
            (0..12).map(|_| FlipFlopState::default()).collect();
        register(&[0; 12], 0, &mut state);
        register(&[0; 12], 1, &mut state);
        Self { state }
    }

    /// Read current PC value (0-4095).
    pub fn read(&self) -> u16 {
        let mut state = self.state.clone();
        let output = register(&[0; 12], 0, &mut state);
        bits_to_int(&output)
    }

    /// Load a new address into the PC (for jumps).
    pub fn load(&mut self, address: u16) {
        let bits = int_to_bits(address & 0xFFF, 12);
        register(&bits, 0, &mut self.state);
        register(&bits, 1, &mut self.state);
    }

    /// Increment PC by 1 using a chain of half-adders.
    ///
    /// This is how a real incrementer works:
    ///
    /// ```text
    /// carry_in = 1 (we're adding 1)
    /// For each bit position:
    ///     (new_bit, carry) = half_adder(old_bit, carry)
    /// ```
    pub fn increment(&mut self) {
        let current_bits = int_to_bits(self.read(), 12);
        let mut carry: u8 = 1; // Adding 1
        let mut new_bits = Vec::with_capacity(12);
        for &bit in &current_bits {
            let (sum_bit, new_carry) = half_adder(bit, carry);
            new_bits.push(sum_bit);
            carry = new_carry;
        }
        self.load(bits_to_int(&new_bits));
    }

    /// Increment PC by 2 (for 2-byte instructions).
    ///
    /// Two cascaded increments through the half-adder chain.
    pub fn increment2(&mut self) {
        self.increment();
        self.increment();
    }

    /// Reset PC to 0.
    pub fn reset(&mut self) {
        self.load(0);
    }

    /// 12-bit register (72 gates) + 12 half-adders (24 gates) = 96.
    pub fn gate_count(&self) -> usize {
        96
    }
}

impl Default for ProgramCounter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pc_initial_value() {
        let pc = ProgramCounter::new();
        assert_eq!(pc.read(), 0);
    }

    #[test]
    fn test_pc_increment() {
        let mut pc = ProgramCounter::new();
        pc.increment();
        assert_eq!(pc.read(), 1);
        pc.increment();
        assert_eq!(pc.read(), 2);
    }

    #[test]
    fn test_pc_increment2() {
        let mut pc = ProgramCounter::new();
        pc.increment2();
        assert_eq!(pc.read(), 2);
    }

    #[test]
    fn test_pc_load() {
        let mut pc = ProgramCounter::new();
        pc.load(0x123);
        assert_eq!(pc.read(), 0x123);
    }

    #[test]
    fn test_pc_wraps_at_4096() {
        let mut pc = ProgramCounter::new();
        pc.load(4095);
        pc.increment();
        assert_eq!(pc.read(), 0);
    }

    #[test]
    fn test_pc_reset() {
        let mut pc = ProgramCounter::new();
        pc.load(500);
        pc.reset();
        assert_eq!(pc.read(), 0);
    }
}
