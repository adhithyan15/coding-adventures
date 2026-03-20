//! # Register File -- the CPU's fast, small storage.
//!
//! ## What are registers?
//!
//! Registers are the fastest storage in a computer. They sit inside the CPU
//! itself and can be read or written in a single clock cycle. A typical CPU
//! has between 8 and 32 registers, each holding one "word" of data (e.g., 32
//! bits on a 32-bit CPU).
//!
//! Think of registers like the small whiteboard on your desk. You can glance
//! at it instantly (fast), but it only holds a few things. Memory (RAM) is
//! like a filing cabinet across the room -- it holds much more, but you have
//! to walk over to get something (slow).
//!
//! ## Why so few?
//!
//! Registers are expensive to build because they need to be extremely fast.
//! Each register is made of flip-flops (built from logic gates), and the
//! wiring to connect them all to the ALU grows quadratically with the number
//! of registers. So CPUs use a small number of very fast registers combined
//! with large but slower memory.
//!
//! ## Register conventions
//!
//! Different architectures assign special meaning to certain registers:
//!
//! - **RISC-V:** x0 is hardwired to 0, x1 = return address, x2 = stack pointer
//! - **ARM:** R13 = stack pointer, R14 = link register, R15 = program counter
//! - **Intel 4004:** 16 4-bit registers + a 4-bit accumulator
//!
//! Our `RegisterFile` is generic -- the ISA simulator decides which registers
//! have special behavior (like x0 always being 0 in RISC-V).

use std::collections::HashMap;

/// A set of numbered registers, each holding an integer value.
///
/// The register file is like a tiny array of named storage slots:
///
/// ```text
///     +-----+-----+-----+-----+-----+-----+
///     | R0  | R1  | R2  | R3  | ... | R15 |
///     |  0  |  0  |  0  |  0  |     |  0  |
///     +-----+-----+-----+-----+-----+-----+
/// ```
///
/// Read and write by register number:
///
/// ```text
///     registers.read(1)       -> value in R1
///     registers.write(1, 42)  -> R1 = 42
/// ```
///
/// # Example
///
/// ```
/// use cpu_simulator::RegisterFile;
///
/// let mut regs = RegisterFile::new(16, 32);
/// regs.write(1, 42);
/// assert_eq!(regs.read(1), 42);
/// ```
#[derive(Debug, Clone)]
pub struct RegisterFile {
    /// How many registers this file contains.
    pub num_registers: usize,

    /// How many bits wide each register is (e.g., 8, 16, 32).
    pub bit_width: usize,

    /// Internal storage -- one slot per register.
    values: Vec<u32>,

    /// The bitmask used to enforce bit-width limits.
    ///
    /// For an 8-bit register, this is 0xFF (255).
    /// For a 16-bit register, this is 0xFFFF (65535).
    /// For a 32-bit register, this is 0xFFFFFFFF (4294967295).
    ///
    /// In JavaScript, computing this mask is tricky because bit shifts
    /// operate on signed 32-bit integers (`(1 << 32)` wraps to 1). Rust
    /// has no such problem -- `u32` arithmetic is well-defined and unsigned.
    max_value: u32,
}

impl RegisterFile {
    /// Create a new register file with `num_registers` registers, each
    /// `bit_width` bits wide.
    ///
    /// All registers start at 0.
    ///
    /// # Panics
    ///
    /// Panics if `bit_width` is 0 or greater than 32 (we use `u32` storage).
    pub fn new(num_registers: usize, bit_width: usize) -> Self {
        assert!(bit_width > 0 && bit_width <= 32, "bit_width must be 1..=32");

        // For 32-bit registers, (1u32 << 32) would overflow. We handle
        // this by using u32::MAX directly for the 32-bit case.
        //
        // For smaller widths, (1u32 << bit_width) - 1 gives us the correct
        // mask. Example: bit_width=8 -> (1 << 8) - 1 = 255 = 0xFF.
        let max_value = if bit_width >= 32 {
            u32::MAX
        } else {
            (1u32 << bit_width) - 1
        };

        RegisterFile {
            num_registers,
            bit_width,
            values: vec![0u32; num_registers],
            max_value,
        }
    }

    /// Read the value stored in register `index`.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of range.
    ///
    /// # Example
    ///
    /// ```
    /// use cpu_simulator::RegisterFile;
    ///
    /// let mut regs = RegisterFile::new(4, 32);
    /// regs.write(2, 100);
    /// assert_eq!(regs.read(2), 100);
    /// ```
    pub fn read(&self, index: usize) -> u32 {
        assert!(
            index < self.num_registers,
            "Register index {} out of range (0-{})",
            index,
            self.num_registers - 1
        );
        self.values[index]
    }

    /// Write a value to register `index`.
    ///
    /// Values are masked to the register's bit width. For example, on an
    /// 8-bit register file, writing 256 wraps to 0 because `256 & 0xFF == 0`.
    ///
    /// This masking mirrors how real hardware works: if a register is 8 bits
    /// wide, only the lowest 8 bits of the input are stored. The upper bits
    /// are simply discarded -- they "fall off the edge" of the register.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of range.
    ///
    /// # Example
    ///
    /// ```
    /// use cpu_simulator::RegisterFile;
    ///
    /// let mut regs = RegisterFile::new(4, 8);
    /// regs.write(0, 256); // 256 doesn't fit in 8 bits
    /// assert_eq!(regs.read(0), 0); // wrapped: 256 & 0xFF = 0
    /// ```
    pub fn write(&mut self, index: usize, value: u32) {
        assert!(
            index < self.num_registers,
            "Register index {} out of range (0-{})",
            index,
            self.num_registers - 1
        );
        // Mask the value to enforce bit-width limits.
        //
        // In Rust, `&` on u32 is straightforward unsigned arithmetic.
        // No need for JavaScript's `>>> 0` trick -- Rust's u32 is always
        // unsigned, so 0xFFFFFFFF & 0xFFFFFFFF == 0xFFFFFFFF (not -1).
        self.values[index] = value & self.max_value;
    }

    /// Return all register values as a map for inspection.
    ///
    /// Keys are "R0", "R1", etc. This is useful for debugging and for
    /// building pipeline traces that show register state after execution.
    ///
    /// # Example
    ///
    /// ```
    /// use cpu_simulator::RegisterFile;
    ///
    /// let mut regs = RegisterFile::new(4, 32);
    /// regs.write(1, 5);
    /// let dump = regs.dump();
    /// assert_eq!(dump["R1"], 5);
    /// ```
    pub fn dump(&self) -> HashMap<String, u32> {
        let mut result = HashMap::with_capacity(self.num_registers);
        for (i, &v) in self.values.iter().enumerate() {
            result.insert(format!("R{}", i), v);
        }
        result
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_write_basic() {
        let mut regs = RegisterFile::new(4, 16); // 16-bit registers
        regs.write(0, 0xFFFF);
        assert_eq!(regs.read(0), 0xFFFF);
    }

    #[test]
    fn write_overflow_masks_to_bit_width() {
        // A 16-bit register should mask values to 0xFFFF.
        // Writing 0xFF0000 should keep only the lower 16 bits: 0x0000.
        let mut regs = RegisterFile::new(4, 16);
        regs.write(1, 0xFF0000);
        assert_eq!(regs.read(1), 0, "0xFF0000 & 0xFFFF = 0");
    }

    #[test]
    fn eight_bit_overflow() {
        // 8-bit registers: max value is 255.
        let mut regs = RegisterFile::new(4, 8);
        regs.write(0, 256); // 256 = 0x100, overflows 8 bits
        assert_eq!(regs.read(0), 0, "256 & 0xFF = 0");

        regs.write(1, 255);
        assert_eq!(regs.read(1), 255);
    }

    #[test]
    fn thirty_two_bit_full_range() {
        // 32-bit registers should handle the full u32 range.
        let mut regs = RegisterFile::new(4, 32);
        regs.write(0, u32::MAX);
        assert_eq!(regs.read(0), u32::MAX);
    }

    #[test]
    fn dump_returns_all_registers() {
        let mut regs = RegisterFile::new(2, 8);
        regs.write(1, 42);
        let dump = regs.dump();
        assert_eq!(dump["R0"], 0);
        assert_eq!(dump["R1"], 42);
        assert_eq!(dump.len(), 2);
    }

    #[test]
    #[should_panic(expected = "Register index 2 out of range")]
    fn read_out_of_bounds_panics() {
        let regs = RegisterFile::new(2, 8);
        regs.read(2);
    }

    #[test]
    #[should_panic(expected = "Register index 2 out of range")]
    fn write_out_of_bounds_panics() {
        let mut regs = RegisterFile::new(2, 8);
        regs.write(2, 1);
    }

    #[test]
    fn all_registers_start_at_zero() {
        let regs = RegisterFile::new(8, 32);
        for i in 0..8 {
            assert_eq!(regs.read(i), 0, "Register {} should start at 0", i);
        }
    }

    #[test]
    fn write_does_not_affect_other_registers() {
        let mut regs = RegisterFile::new(4, 32);
        regs.write(2, 999);
        assert_eq!(regs.read(0), 0);
        assert_eq!(regs.read(1), 0);
        assert_eq!(regs.read(2), 999);
        assert_eq!(regs.read(3), 0);
    }
}
