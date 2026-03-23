//! Configurable Block RAM — FPGA-style memory with reconfigurable aspect ratio.
//!
//! # What is Block RAM?
//!
//! In an FPGA, Block RAM (BRAM) tiles are dedicated memory blocks separate
//! from the configurable logic. Each tile has a fixed total storage (typically
//! 18 Kbit or 36 Kbit) but can be configured with different width/depth ratios:
//!
//! ```text
//! 18 Kbit BRAM configurations:
//! +-----------------+-------+-------+------------+
//! | Configuration   | Depth | Width | Total bits |
//! +-----------------+-------+-------+------------+
//! | 16K x 1         | 16384 |     1 |      16384 |
//! |  8K x 2         |  8192 |     2 |      16384 |
//! |  4K x 4         |  4096 |     4 |      16384 |
//! |  2K x 8         |  2048 |     8 |      16384 |
//! |  1K x 16        |  1024 |    16 |      16384 |
//! | 512 x 32        |   512 |    32 |      16384 |
//! +-----------------+-------+-------+------------+
//! ```
//!
//! The total storage is fixed; you trade depth for width by changing how the
//! address decoder and column MUX are configured. The underlying SRAM cells
//! do not change — only the access pattern changes.
//!
//! This module wraps [`DualPortRAM`](crate::ram::DualPortRAM) with
//! reconfiguration support.

use crate::ram::{DualPortRAM, ReadMode};
use crate::sram::validate_bit;

/// Block RAM with configurable aspect ratio.
///
/// Total storage is fixed at initialization. Width and depth can be
/// reconfigured as long as `width * depth == total_bits`.
///
/// Supports dual-port access via `tick_a` and `tick_b`.
///
/// # Example
///
/// ```
/// use block_ram::bram::ConfigurableBRAM;
/// let mut bram = ConfigurableBRAM::new(1024, 8);
/// assert_eq!(bram.depth(), 128); // 1024 / 8 = 128
///
/// bram.reconfigure(16);
/// assert_eq!(bram.depth(), 64); // 1024 / 16 = 64
/// ```
#[derive(Debug, Clone)]
pub struct ConfigurableBRAM {
    total_bits: usize,
    width: usize,
    depth: usize,
    ram: DualPortRAM,
    prev_clock: u8,
}

impl ConfigurableBRAM {
    /// Create a new configurable Block RAM.
    ///
    /// # Parameters
    ///
    /// - `total_bits`: Total storage in bits (must be >= 1)
    /// - `width`: Initial bits per word (must be >= 1, must divide total_bits evenly)
    ///
    /// # Panics
    ///
    /// Panics if `total_bits < 1`, `width < 1`, or `width` does not divide `total_bits`.
    pub fn new(total_bits: usize, width: usize) -> Self {
        assert!(total_bits >= 1, "total_bits must be >= 1, got {total_bits}");
        assert!(width >= 1, "width must be >= 1, got {width}");
        assert!(
            total_bits % width == 0,
            "width {width} does not evenly divide total_bits {total_bits}"
        );

        let depth = total_bits / width;
        Self {
            total_bits,
            width,
            depth,
            ram: DualPortRAM::new(depth, width, ReadMode::ReadFirst, ReadMode::ReadFirst),
            prev_clock: 0,
        }
    }

    /// Change the aspect ratio. Clears all stored data.
    ///
    /// # Panics
    ///
    /// Panics if `width < 1` or does not divide `total_bits`.
    pub fn reconfigure(&mut self, width: usize) {
        assert!(width >= 1, "width must be >= 1, got {width}");
        assert!(
            self.total_bits % width == 0,
            "width {width} does not evenly divide total_bits {}",
            self.total_bits
        );

        self.width = width;
        self.depth = self.total_bits / width;
        self.ram = DualPortRAM::new(
            self.depth,
            self.width,
            ReadMode::ReadFirst,
            ReadMode::ReadFirst,
        );
        self.prev_clock = 0;
    }

    /// Port A operation.
    ///
    /// Uses the dual-port RAM with port B idle (read address 0).
    ///
    /// # Panics
    ///
    /// Panics if address is out of range or data_in has the wrong length.
    pub fn tick_a(
        &mut self,
        clock: u8,
        address: usize,
        data_in: &[u8],
        write_enable: u8,
    ) -> Vec<u8> {
        validate_bit(clock, "clock");

        let zeros = vec![0u8; self.width];
        let (out_a, _) = self
            .ram
            .tick(clock, address, data_in, write_enable, 0, &zeros, 0)
            .expect("tick_a should not produce a write collision");
        out_a
    }

    /// Port B operation.
    ///
    /// Uses the dual-port RAM with port A idle (read address 0).
    ///
    /// # Panics
    ///
    /// Panics if address is out of range or data_in has the wrong length.
    pub fn tick_b(
        &mut self,
        clock: u8,
        address: usize,
        data_in: &[u8],
        write_enable: u8,
    ) -> Vec<u8> {
        validate_bit(clock, "clock");

        let zeros = vec![0u8; self.width];
        let (_, out_b) = self
            .ram
            .tick(clock, 0, &zeros, 0, address, data_in, write_enable)
            .expect("tick_b should not produce a write collision");
        out_b
    }

    /// Number of addressable words at current configuration.
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Bits per word at current configuration.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Total storage capacity in bits (fixed).
    pub fn total_bits(&self) -> usize {
        self.total_bits
    }
}

// ===========================================================================
// Inline unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bram_initial_config() {
        let bram = ConfigurableBRAM::new(1024, 8);
        assert_eq!(bram.depth(), 128);
        assert_eq!(bram.width(), 8);
        assert_eq!(bram.total_bits(), 1024);
    }

    #[test]
    fn test_bram_reconfigure() {
        let mut bram = ConfigurableBRAM::new(1024, 8);
        bram.reconfigure(16);
        assert_eq!(bram.depth(), 64);
        assert_eq!(bram.width(), 16);
    }

    #[test]
    fn test_bram_write_and_read() {
        let mut bram = ConfigurableBRAM::new(64, 4);
        // depth = 16

        // Write via port A
        bram.tick_a(0, 0, &[1, 0, 1, 0], 1);
        bram.tick_a(1, 0, &[1, 0, 1, 0], 1);

        // Read via port A
        bram.tick_a(0, 0, &[0; 4], 0);
        let out = bram.tick_a(1, 0, &[0; 4], 0);
        assert_eq!(out, vec![1, 0, 1, 0]);
    }

    #[test]
    #[should_panic(expected = "does not evenly divide")]
    fn test_bram_invalid_width() {
        ConfigurableBRAM::new(1024, 3);
    }
}
