//! Look-Up Table (LUT) — the atom of programmable logic.
//!
//! # What is a LUT?
//!
//! A Look-Up Table is the fundamental building block of every FPGA. The key
//! insight behind programmable logic is deceptively simple:
//!
//! > *A truth table IS a program.*
//!
//! Any boolean function of K inputs can be described by a truth table with
//! 2^K entries. A K-input LUT stores that truth table in SRAM and uses a
//! MUX tree to select the correct output for any combination of inputs.
//!
//! This means a single LUT can implement ANY boolean function of K variables:
//! AND, OR, XOR, majority vote, parity — anything. To "reprogram" the LUT,
//! you just load a different truth table into the SRAM.
//!
//! # How it works
//!
//! A 4-input LUT (K=4) has:
//! - 16 SRAM cells (2^4 = 16 truth table entries)
//! - A 16-to-1 MUX tree (built from 2:1 MUXes)
//! - 4 input signals that act as MUX select lines
//!
//! # MUX Tree Structure (4-input LUT)
//!
//! ```text
//! SRAM[0]  -+
//!            +- MUX(sel=I0) -+
//! SRAM[1]  -+                |
//!                            +- MUX(sel=I1) -+
//! SRAM[2]  -+                |               |
//!            +- MUX(sel=I0) -+               |
//! SRAM[3]  -+                                +- MUX(sel=I2) -+
//!                                            |               |
//! SRAM[4..7] --- (same) --- MUX(sel=I1) ----+               |
//!                                                            +- Out
//! SRAM[8..15] --- (same structure) --- MUX(sel=I2) ---------+
//!                                            (sel=I3)
//! ```
//!
//! This is exactly what `mux_n` from logic-gates does: it recursively builds
//! a 2^K-to-1 MUX tree from 2:1 MUXes, using the select bits to choose
//! one of the 2^K inputs.

use block_ram::sram::SRAMCell;
use logic_gates::combinational::mux_n;

/// K-input Look-Up Table — the atom of programmable logic.
///
/// A LUT stores a truth table in SRAM cells and uses a MUX tree to
/// select the output based on input signals. It can implement ANY
/// boolean function of K variables.
///
/// # Example — 2-input AND gate in a 4-input LUT
///
/// ```
/// use fpga::lut::LUT;
/// // Truth table: output 1 only when I0=1 AND I1=1 (index 3)
/// let mut and_table = vec![0u8; 16];
/// and_table[3] = 1; // I0=1, I1=1 -> index = 1 + 2 = 3
/// let lut = LUT::new(4, Some(&and_table));
/// assert_eq!(lut.evaluate(&[0, 0, 0, 0]), 0);
/// assert_eq!(lut.evaluate(&[1, 1, 0, 0]), 1); // I0=1, I1=1
/// ```
///
/// # Example — 2-input XOR gate
///
/// ```
/// use fpga::lut::LUT;
/// let mut xor_table = vec![0u8; 16];
/// xor_table[1] = 1; // I0=1, I1=0
/// xor_table[2] = 1; // I0=0, I1=1
/// let lut = LUT::new(4, Some(&xor_table));
/// assert_eq!(lut.evaluate(&[1, 0, 0, 0]), 1);
/// assert_eq!(lut.evaluate(&[1, 1, 0, 0]), 0);
/// ```
#[derive(Debug, Clone)]
pub struct LUT {
    /// Number of inputs (2 to 6).
    k: usize,
    /// Size of the truth table (2^k).
    size: usize,
    /// SRAM cells storing the truth table.
    sram: Vec<SRAMCell>,
}

impl LUT {
    /// Create a new K-input LUT.
    ///
    /// # Parameters
    ///
    /// - `k`: Number of inputs (2 to 6)
    /// - `truth_table`: Optional initial truth table (2^k entries, each 0 or 1).
    ///   If `None`, all entries default to 0.
    ///
    /// # Panics
    ///
    /// Panics if `k` is not in the range 2..=6.
    pub fn new(k: usize, truth_table: Option<&[u8]>) -> Self {
        assert!((2..=6).contains(&k), "k must be between 2 and 6, got {k}");

        let size = 1 << k;
        let sram = (0..size).map(|_| SRAMCell::new()).collect();

        let mut lut = Self { k, size, sram };

        if let Some(tt) = truth_table {
            lut.configure(tt);
        }

        lut
    }

    /// Load a new truth table (reprogram the LUT).
    ///
    /// # Panics
    ///
    /// Panics if the truth table length does not equal 2^k or entries are not 0/1.
    pub fn configure(&mut self, truth_table: &[u8]) {
        assert!(
            truth_table.len() == self.size,
            "truth_table length {} does not match 2^k = {}",
            truth_table.len(),
            self.size
        );

        for (i, &bit) in truth_table.iter().enumerate() {
            debug_assert!(
                bit == 0 || bit == 1,
                "truth_table[{i}] must be 0 or 1, got {bit}"
            );
            self.sram[i].write(1, bit);
        }
    }

    /// Compute the LUT output for the given inputs.
    ///
    /// Uses a MUX tree (via `mux_n`) to select the correct truth table
    /// entry based on the input signals.
    ///
    /// # Panics
    ///
    /// Panics if inputs length does not equal k.
    pub fn evaluate(&self, inputs: &[u8]) -> u8 {
        assert!(
            inputs.len() == self.k,
            "inputs length {} does not match k = {}",
            inputs.len(),
            self.k
        );

        // Read all SRAM cells to form the MUX data inputs
        let data: Vec<u8> = self.sram.iter().map(|c| c.read(1).unwrap()).collect();

        // Use MUX tree to select the output
        mux_n(&data, inputs)
    }

    /// Number of inputs.
    pub fn k(&self) -> usize {
        self.k
    }

    /// Current truth table (copy).
    pub fn truth_table(&self) -> Vec<u8> {
        self.sram.iter().map(|c| c.read(1).unwrap()).collect()
    }
}

// ===========================================================================
// Inline unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lut_default_all_zeros() {
        let lut = LUT::new(4, None);
        for i in 0..16 {
            let inputs: Vec<u8> = (0..4).map(|b| ((i >> b) & 1) as u8).collect();
            assert_eq!(lut.evaluate(&inputs), 0);
        }
    }

    #[test]
    fn test_lut_and_gate() {
        let mut tt = vec![0u8; 16];
        tt[3] = 1; // I0=1, I1=1
        let lut = LUT::new(4, Some(&tt));
        assert_eq!(lut.evaluate(&[0, 0, 0, 0]), 0);
        assert_eq!(lut.evaluate(&[1, 0, 0, 0]), 0);
        assert_eq!(lut.evaluate(&[0, 1, 0, 0]), 0);
        assert_eq!(lut.evaluate(&[1, 1, 0, 0]), 1);
    }

    #[test]
    fn test_lut_reconfigure() {
        let mut lut = LUT::new(4, None);
        assert_eq!(lut.evaluate(&[1, 1, 0, 0]), 0);

        let mut tt = vec![0u8; 16];
        tt[3] = 1;
        lut.configure(&tt);
        assert_eq!(lut.evaluate(&[1, 1, 0, 0]), 1);
    }

    #[test]
    fn test_lut_truth_table_getter() {
        let tt = vec![0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let lut = LUT::new(4, Some(&tt));
        assert_eq!(lut.truth_table(), tt);
    }
}
