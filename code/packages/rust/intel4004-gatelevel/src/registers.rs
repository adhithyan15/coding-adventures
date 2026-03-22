//! Register file -- 16 x 4-bit registers built from D flip-flops.
//!
//! # How registers work in hardware
//!
//! A register is a group of D flip-flops that share a clock signal. Each
//! flip-flop stores one bit. A 4-bit register has 4 flip-flops. The Intel
//! 4004 has 16 such registers (R0-R15), for a total of 64 flip-flops just
//! for the register file.
//!
//! In this simulation, each register call goes through:
//!
//! ```text
//! data bits -> D flip-flop x 4 -> output bits
//! ```
//!
//! The flip-flops are edge-triggered: they capture new data on the rising
//! edge of the clock. Between edges, the stored value is stable.
//!
//! # Register pairs
//!
//! The 4004 organizes its 16 registers into 8 pairs:
//!
//! ```text
//! P0 = R0:R1, P1 = R2:R3, ..., P7 = R14:R15
//! ```
//!
//! A register pair holds an 8-bit value (high nibble in even register,
//! low nibble in odd register). Pairs are used for FIM, SRC, FIN, JIN.
//!
//! # Accumulator
//!
//! The accumulator is a separate 4-bit register, not part of the R0-R15
//! file. It has its own dedicated flip-flops and is connected directly to
//! the ALU's output bus.

use logic_gates::sequential::{register, FlipFlopState};

use crate::bits::{bits_to_int, int_to_bits};

/// 16 x 4-bit register file built from D flip-flops.
///
/// Each of the 16 registers is a group of 4 D flip-flops from the
/// logic_gates sequential module. Reading and writing go through
/// actual flip-flop state transitions.
pub struct RegisterFile {
    /// 16 registers, each with 4 flip-flop states (one per bit).
    states: Vec<Vec<FlipFlopState>>,
}

impl RegisterFile {
    /// Initialize 16 registers, each with 4-bit flip-flop state set to 0.
    pub fn new() -> Self {
        let mut states = Vec::with_capacity(16);
        for _ in 0..16 {
            // Initialize state by clocking zeros through
            let mut state: Vec<FlipFlopState> =
                (0..4).map(|_| FlipFlopState::default()).collect();
            register(&[0, 0, 0, 0], 0, &mut state);
            register(&[0, 0, 0, 0], 1, &mut state);
            states.push(state);
        }
        Self { states }
    }

    /// Read a register value. Returns 4-bit integer (0-15).
    ///
    /// In real hardware, this would route through a 16-to-1 multiplexer
    /// built from gates. We simulate the flip-flop read directly.
    pub fn read(&self, index: usize) -> u8 {
        // Read current output from flip-flops (clock=0, no write).
        // We clone the state because register() takes &mut but we don't
        // want to modify state for a read.
        let mut state = self.states[index].clone();
        let output = register(&[0, 0, 0, 0], 0, &mut state);
        bits_to_int(&output) as u8
    }

    /// Write a 4-bit value to a register.
    ///
    /// In real hardware: decoder selects the register, data bus presents
    /// the value, clock edge latches it into the flip-flops.
    pub fn write(&mut self, index: usize, value: u8) {
        let bits = int_to_bits((value & 0xF) as u16, 4);
        // Clock low (setup)
        register(&bits, 0, &mut self.states[index]);
        // Clock high (capture on rising edge)
        register(&bits, 1, &mut self.states[index]);
    }

    /// Read an 8-bit value from a register pair.
    ///
    /// Pair 0 = R0:R1 (R0=high nibble, R1=low nibble).
    pub fn read_pair(&self, pair_index: usize) -> u8 {
        let high = self.read(pair_index * 2);
        let low = self.read(pair_index * 2 + 1);
        (high << 4) | low
    }

    /// Write an 8-bit value to a register pair.
    pub fn write_pair(&mut self, pair_index: usize, value: u8) {
        self.write(pair_index * 2, (value >> 4) & 0xF);
        self.write(pair_index * 2 + 1, value & 0xF);
    }

    /// Reset all registers to 0 by clocking in zeros.
    pub fn reset(&mut self) {
        for i in 0..16 {
            self.write(i, 0);
        }
    }

    /// Gate count for the register file.
    ///
    /// 16 registers x 4 bits x ~6 gates per D flip-flop = 384 gates.
    /// Plus 4-to-16 decoder for write select: ~32 gates.
    /// Plus 16-to-1 mux for read select: ~64 gates.
    /// Total: ~480 gates.
    pub fn gate_count(&self) -> usize {
        480
    }
}

impl Default for RegisterFile {
    fn default() -> Self {
        Self::new()
    }
}

/// 4-bit accumulator register built from D flip-flops.
///
/// The accumulator is the 4004's main working register. Almost every
/// arithmetic and logic operation reads from or writes to it.
pub struct Accumulator {
    state: Vec<FlipFlopState>,
}

impl Accumulator {
    /// Initialize accumulator to 0.
    pub fn new() -> Self {
        let mut state: Vec<FlipFlopState> =
            (0..4).map(|_| FlipFlopState::default()).collect();
        register(&[0, 0, 0, 0], 0, &mut state);
        register(&[0, 0, 0, 0], 1, &mut state);
        Self { state }
    }

    /// Read the accumulator value (0-15).
    pub fn read(&self) -> u8 {
        let mut state = self.state.clone();
        let output = register(&[0, 0, 0, 0], 0, &mut state);
        bits_to_int(&output) as u8
    }

    /// Write a 4-bit value to the accumulator.
    pub fn write(&mut self, value: u8) {
        let bits = int_to_bits((value & 0xF) as u16, 4);
        register(&bits, 0, &mut self.state);
        register(&bits, 1, &mut self.state);
    }

    /// Reset to 0.
    pub fn reset(&mut self) {
        self.write(0);
    }

    /// 4 D flip-flops x ~6 gates = 24 gates.
    pub fn gate_count(&self) -> usize {
        24
    }
}

impl Default for Accumulator {
    fn default() -> Self {
        Self::new()
    }
}

/// 1-bit carry/borrow flag built from a D flip-flop.
///
/// The carry flag is set by arithmetic operations and read by
/// conditional jumps and multi-digit BCD arithmetic.
pub struct CarryFlag {
    state: Vec<FlipFlopState>,
}

impl CarryFlag {
    /// Initialize carry to false (0).
    pub fn new() -> Self {
        let mut state: Vec<FlipFlopState> =
            (0..1).map(|_| FlipFlopState::default()).collect();
        register(&[0], 0, &mut state);
        register(&[0], 1, &mut state);
        Self { state }
    }

    /// Read carry flag as a boolean.
    pub fn read(&self) -> bool {
        let mut state = self.state.clone();
        let output = register(&[0], 0, &mut state);
        output[0] == 1
    }

    /// Write carry flag.
    pub fn write(&mut self, value: bool) {
        let bit = if value { 1u8 } else { 0u8 };
        register(&[bit], 0, &mut self.state);
        register(&[bit], 1, &mut self.state);
    }

    /// Reset to false.
    pub fn reset(&mut self) {
        self.write(false);
    }

    /// 1 D flip-flop x ~6 gates = 6 gates.
    pub fn gate_count(&self) -> usize {
        6
    }
}

impl Default for CarryFlag {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_file_write_read() {
        let mut regs = RegisterFile::new();
        regs.write(3, 0xA);
        assert_eq!(regs.read(3), 0xA);
    }

    #[test]
    fn test_register_file_all_zeros() {
        let regs = RegisterFile::new();
        for i in 0..16 {
            assert_eq!(regs.read(i), 0);
        }
    }

    #[test]
    fn test_register_file_pair() {
        let mut regs = RegisterFile::new();
        regs.write_pair(2, 0xAB); // R4=0xA, R5=0xB
        assert_eq!(regs.read(4), 0xA);
        assert_eq!(regs.read(5), 0xB);
        assert_eq!(regs.read_pair(2), 0xAB);
    }

    #[test]
    fn test_register_file_reset() {
        let mut regs = RegisterFile::new();
        regs.write(0, 5);
        regs.write(15, 10);
        regs.reset();
        assert_eq!(regs.read(0), 0);
        assert_eq!(regs.read(15), 0);
    }

    #[test]
    fn test_accumulator() {
        let mut acc = Accumulator::new();
        assert_eq!(acc.read(), 0);
        acc.write(7);
        assert_eq!(acc.read(), 7);
        acc.write(15);
        assert_eq!(acc.read(), 15);
        acc.reset();
        assert_eq!(acc.read(), 0);
    }

    #[test]
    fn test_carry_flag() {
        let mut carry = CarryFlag::new();
        assert!(!carry.read());
        carry.write(true);
        assert!(carry.read());
        carry.write(false);
        assert!(!carry.read());
        carry.reset();
        assert!(!carry.read());
    }
}
