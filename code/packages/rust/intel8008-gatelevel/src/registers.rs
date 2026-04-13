//! 7×8-bit register file for the Intel 8008.
//!
//! # Hardware model
//!
//! Each register is an 8-bit D flip-flop array. Reading a register samples
//! the stored bit values; writing a register clocks new values in. This
//! module uses `logic_gates::sequential::register` to simulate the flip-flop
//! behavior.
//!
//! The `register(data, clock, state)` function operates on an entire N-bit
//! word at once:
//!   - clock=0: master latches absorb `data`
//!   - clock=1: slaves propagate from masters → output becomes readable
//!
//! # Register map
//!
//! The 8008 has 7 working registers (A, B, C, D, E, H, L) plus the M
//! pseudo-register which aliases memory. Indices match the 3-bit opcode
//! encoding:
//!
//! ```text
//! 0=B  1=C  2=D  3=E  4=H  5=L  6=(M, handled externally)  7=A
//! ```
//!
//! Register 6 (M) is NOT stored here — M accesses go through the CPU's
//! memory module using the address formed by H and L. This module stores
//! registers 0-5 and 7 (index 6 is left unused/zeroed).

use logic_gates::sequential::{register, FlipFlopState};

use crate::bits::{bits_to_int, int_to_bits};

/// 7×8-bit register file for the Intel 8008.
///
/// Each of the 8 register slots is stored as a `Vec<FlipFlopState>` of
/// length 8 (LSB-first). Write operations simulate the two-phase clock;
/// read operations sample the slave-latch Q outputs.
pub struct RegisterFile {
    /// 8 register slots: indices 0-5 = B,C,D,E,H,L; index 6 = unused;
    /// index 7 = A (accumulator).
    regs: Vec<Vec<FlipFlopState>>,
}

impl RegisterFile {
    /// Create a new register file with all registers zeroed.
    pub fn new() -> Self {
        let regs: Vec<Vec<FlipFlopState>> = (0..8)
            .map(|_| (0..8).map(|_| FlipFlopState::default()).collect())
            .collect();
        RegisterFile { regs }
    }

    /// Read register `idx` (0-7, index 6 returns 0).
    ///
    /// Samples the slave_q output of each flip-flop. Returns an 8-bit value
    /// with bit 0 = LSB.
    pub fn read(&self, idx: usize) -> u8 {
        let bits: Vec<u8> = self.regs[idx].iter().map(|s| s.slave_q).collect();
        bits_to_int(&bits)
    }

    /// Write a value into register `idx`. Clocks bits into flip-flops.
    ///
    /// Two-phase clock: clock=0 loads master latches, clock=1 propagates
    /// to slave latches so the value is immediately readable.
    pub fn write(&mut self, idx: usize, value: u8) {
        let bits = int_to_bits(value, 8);
        // Phase 1: falling edge — master latches absorb new data
        register(&bits, 0, &mut self.regs[idx]);
        // Phase 2: rising edge — slave latches receive from masters
        register(&bits, 1, &mut self.regs[idx]);
    }
}

impl Default for RegisterFile {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_file_read_write() {
        let mut rf = RegisterFile::new();
        // All registers start at 0
        for i in 0..8 {
            assert_eq!(rf.read(i), 0);
        }
        // Write and read back
        rf.write(7, 0xAB); // A = 0xAB
        assert_eq!(rf.read(7), 0xAB);
        rf.write(0, 0x55); // B = 0x55
        assert_eq!(rf.read(0), 0x55);
        // Other registers unaffected
        assert_eq!(rf.read(1), 0);
    }

    #[test]
    fn test_all_values_round_trip() {
        let mut rf = RegisterFile::new();
        for v in 0u8..=255 {
            rf.write(7, v);
            assert_eq!(rf.read(7), v);
        }
    }
}
