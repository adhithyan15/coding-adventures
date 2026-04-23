//! 14-bit Program Counter for the Intel 8008.
//!
//! # Hardware model
//!
//! The program counter is a 14-bit register (matching the 8008's 16 KiB
//! address space). Incrementing uses a half-adder chain:
//!
//! ```text
//! PC + 1:
//!   bit[0]: sum  = XOR(pc[0], 1)  carry = AND(pc[0], 1) = pc[0]
//!   bit[1]: sum  = XOR(pc[1], c0) carry = AND(pc[1], c0)
//!   bit[2]: sum  = XOR(pc[2], c1) carry = AND(pc[2], c1)
//!   ...
//!   bit[13]: sum = XOR(pc[13], c12)
//! ```
//!
//! This is a 14-stage half-adder chain — 14 × 2 gates = 28 gates.
//! Compare: the 4004 used a 12-bit PC (12 × 2 = 24 gates).
//! The 8008's extra 2 bits add 4 gates to address the larger memory.
//!
//! # State storage
//!
//! Each of the 14 bits is held by a D flip-flop. The `register()` function
//! from `logic_gates::sequential` simulates N flip-flops simultaneously:
//! clock=0 (falling edge) loads data into the master latch; clock=1 (rising
//! edge) propagates master to slave so the output becomes stable.

use logic_gates::gates::{and_gate, xor_gate};
use logic_gates::sequential::{register, FlipFlopState};

/// 14-bit program counter.
///
/// The bits are stored in a 14-element flip-flop state slice (LSB-first).
/// Using `register()` with clock=0 then clock=1 simulates a rising-edge
/// clocked write.
pub struct ProgramCounter {
    /// Flip-flop state for each of the 14 address bits (LSB = index 0).
    state: Vec<FlipFlopState>,
}

impl ProgramCounter {
    /// Create a new PC initialized to 0x0000.
    pub fn new() -> Self {
        let state: Vec<FlipFlopState> = (0..14).map(|_| FlipFlopState::default()).collect();
        ProgramCounter { state }
    }

    /// Read the current 14-bit program counter value.
    ///
    /// Reads by sampling the slave-latch Q output in each flip-flop.
    pub fn read(&self) -> u16 {
        self.state.iter().enumerate().fold(0u16, |acc, (i, s)| {
            acc | ((s.slave_q as u16) << i)
        })
    }

    /// Load a new 14-bit value into the PC (used by JMP, CALL, RETURN).
    ///
    /// Simulates two clock phases: clock=0 loads masters, clock=1 propagates
    /// to slaves so the value is immediately readable.
    pub fn load(&mut self, value: u16) {
        let masked = value & 0x3FFF;
        // Build 14-bit LSB-first bit vector
        let bits14: Vec<u8> = (0..14).map(|i| ((masked >> i) & 1) as u8).collect();

        // Phase 1: clock=0 (falling edge) — master latches absorb new data
        register(&bits14, 0, &mut self.state);
        // Phase 2: clock=1 (rising edge) — slave latches receive master values
        register(&bits14, 1, &mut self.state);
    }

    /// Increment the PC by 1 using a half-adder chain.
    ///
    /// Each stage:
    ///   sum[i]   = XOR(bit[i], carry_in)
    ///   carry[i] = AND(bit[i], carry_in)
    ///
    /// Starting carry_in = 1 (we are adding 1).
    pub fn increment(&mut self) {
        let current = self.read();
        let mut carry = 1u8; // Adding 1 means initial carry_in = 1
        let mut new_bits14: Vec<u8> = vec![0u8; 14];
        for i in 0..14 {
            let old_bit = ((current >> i) & 1) as u8;
            let sum = xor_gate(old_bit, carry);
            carry = and_gate(old_bit, carry);
            new_bits14[i] = sum;
        }
        // Clock the new bits into the flip-flops
        register(&new_bits14, 0, &mut self.state);
        register(&new_bits14, 1, &mut self.state);
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
    fn test_pc_increment() {
        let mut pc = ProgramCounter::new();
        assert_eq!(pc.read(), 0);
        pc.increment();
        assert_eq!(pc.read(), 1);
        pc.increment();
        assert_eq!(pc.read(), 2);
    }

    #[test]
    fn test_pc_load() {
        let mut pc = ProgramCounter::new();
        pc.load(0x1234);
        assert_eq!(pc.read(), 0x1234 & 0x3FFF);
        pc.load(0x3FFF);
        assert_eq!(pc.read(), 0x3FFF);
    }

    #[test]
    fn test_pc_wrap() {
        let mut pc = ProgramCounter::new();
        pc.load(0x3FFF);
        pc.increment(); // Should wrap to 0 (14-bit)
        assert_eq!(pc.read(), 0);
    }

    #[test]
    fn test_pc_load_then_increment() {
        let mut pc = ProgramCounter::new();
        pc.load(0x100);
        pc.increment();
        assert_eq!(pc.read(), 0x101);
    }
}
