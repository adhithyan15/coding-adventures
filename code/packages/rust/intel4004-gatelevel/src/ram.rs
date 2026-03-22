//! RAM -- 4 banks x 4 registers x 20 nibbles, built from flip-flops.
//!
//! # The 4004's RAM architecture
//!
//! The Intel 4004 used separate RAM chips (Intel 4002), each containing:
//! - 4 registers
//! - Each register has 16 main characters + 4 status characters
//! - Each character is a 4-bit nibble
//! - Total per chip: 4 x 20 x 4 = 320 bits
//!
//! The full system supports up to 4 RAM banks (4 chips), selected by the
//! DCL instruction. Within a bank, the SRC instruction sets which register
//! and character to access.
//!
//! In real hardware, each nibble is stored in 4 D flip-flops. The full
//! RAM system uses 4 x 4 x 20 x 4 = 1,280 flip-flops.
//!
//! # Addressing
//!
//! RAM is addressed in two steps:
//! 1. DCL sets the bank (0-3, from accumulator bits 0-2)
//! 2. SRC sends an 8-bit address from a register pair:
//!    - High nibble -> register index (0-3)
//!    - Low nibble -> character index (0-15)

use logic_gates::sequential::{register, FlipFlopState};

use crate::bits::{bits_to_int, int_to_bits};

/// 4004 RAM: 4 banks x 4 registers x (16 main + 4 status) nibbles.
///
/// Every nibble is stored in 4 D flip-flops from the sequential logic
/// crate. Reading and writing physically route through flip-flop
/// state transitions.
pub struct RAM {
    /// main[bank][reg][char] = flip-flop state for one nibble.
    main: Vec<Vec<Vec<Vec<FlipFlopState>>>>,
    /// status[bank][reg][index] = flip-flop state for one status nibble.
    status: Vec<Vec<Vec<Vec<FlipFlopState>>>>,
    /// Output ports (one per bank, written by WMP).
    output: [u8; 4],
}

impl RAM {
    /// Initialize all RAM to 0.
    pub fn new() -> Self {
        let mut main = Vec::with_capacity(4);
        let mut status = Vec::with_capacity(4);

        for _bank in 0..4 {
            let mut bank_main = Vec::with_capacity(4);
            let mut bank_status = Vec::with_capacity(4);

            for _reg in 0..4 {
                let mut reg_main = Vec::with_capacity(16);
                for _char in 0..16 {
                    let mut state: Vec<FlipFlopState> =
                        (0..4).map(|_| FlipFlopState::default()).collect();
                    register(&[0, 0, 0, 0], 0, &mut state);
                    register(&[0, 0, 0, 0], 1, &mut state);
                    reg_main.push(state);
                }
                bank_main.push(reg_main);

                let mut reg_status = Vec::with_capacity(4);
                for _stat in 0..4 {
                    let mut state: Vec<FlipFlopState> =
                        (0..4).map(|_| FlipFlopState::default()).collect();
                    register(&[0, 0, 0, 0], 0, &mut state);
                    register(&[0, 0, 0, 0], 1, &mut state);
                    reg_status.push(state);
                }
                bank_status.push(reg_status);
            }

            main.push(bank_main);
            status.push(bank_status);
        }

        Self {
            main,
            status,
            output: [0; 4],
        }
    }

    /// Read a main character (4-bit nibble) from RAM.
    pub fn read_main(&self, bank: usize, reg: usize, char_idx: usize) -> u8 {
        let mut state = self.main[bank & 3][reg & 3][char_idx & 0xF].clone();
        let output = register(&[0, 0, 0, 0], 0, &mut state);
        bits_to_int(&output) as u8
    }

    /// Write a 4-bit value to a main character.
    pub fn write_main(&mut self, bank: usize, reg: usize, char_idx: usize, value: u8) {
        let bits = int_to_bits((value & 0xF) as u16, 4);
        let state = &mut self.main[bank & 3][reg & 3][char_idx & 0xF];
        register(&bits, 0, state);
        register(&bits, 1, state);
    }

    /// Read a status character (0-3) from RAM.
    pub fn read_status(&self, bank: usize, reg: usize, index: usize) -> u8 {
        let mut state = self.status[bank & 3][reg & 3][index & 3].clone();
        let output = register(&[0, 0, 0, 0], 0, &mut state);
        bits_to_int(&output) as u8
    }

    /// Write a 4-bit value to a status character.
    pub fn write_status(&mut self, bank: usize, reg: usize, index: usize, value: u8) {
        let bits = int_to_bits((value & 0xF) as u16, 4);
        let state = &mut self.status[bank & 3][reg & 3][index & 3];
        register(&bits, 0, state);
        register(&bits, 1, state);
    }

    /// Read a RAM output port value.
    pub fn read_output(&self, bank: usize) -> u8 {
        self.output[bank & 3]
    }

    /// Write to a RAM output port (WMP instruction).
    pub fn write_output(&mut self, bank: usize, value: u8) {
        self.output[bank & 3] = value & 0xF;
    }

    /// Reset all RAM to 0.
    pub fn reset(&mut self) {
        for bank in 0..4 {
            for reg in 0..4 {
                for ch in 0..16 {
                    self.write_main(bank, reg, ch, 0);
                }
                for stat in 0..4 {
                    self.write_status(bank, reg, stat, 0);
                }
            }
            self.output[bank] = 0;
        }
    }

    /// 4 banks x 4 regs x 20 nibbles x 4 bits x 6 gates/ff = 7680.
    /// Plus addressing/decoding: ~200 gates.
    pub fn gate_count(&self) -> usize {
        7880
    }
}

impl Default for RAM {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ram_initial_zero() {
        let ram = RAM::new();
        assert_eq!(ram.read_main(0, 0, 0), 0);
        assert_eq!(ram.read_status(0, 0, 0), 0);
    }

    #[test]
    fn test_ram_write_read_main() {
        let mut ram = RAM::new();
        ram.write_main(1, 2, 3, 0xA);
        assert_eq!(ram.read_main(1, 2, 3), 0xA);
        // Other locations untouched
        assert_eq!(ram.read_main(0, 0, 0), 0);
    }

    #[test]
    fn test_ram_write_read_status() {
        let mut ram = RAM::new();
        ram.write_status(2, 1, 0, 5);
        assert_eq!(ram.read_status(2, 1, 0), 5);
    }

    #[test]
    fn test_ram_output_port() {
        let mut ram = RAM::new();
        ram.write_output(0, 0xF);
        assert_eq!(ram.read_output(0), 0xF);
        assert_eq!(ram.read_output(1), 0);
    }

    #[test]
    fn test_ram_reset() {
        let mut ram = RAM::new();
        ram.write_main(0, 0, 0, 7);
        ram.write_status(1, 1, 1, 3);
        ram.write_output(2, 5);
        ram.reset();
        assert_eq!(ram.read_main(0, 0, 0), 0);
        assert_eq!(ram.read_status(1, 1, 1), 0);
        assert_eq!(ram.read_output(2), 0);
    }
}
