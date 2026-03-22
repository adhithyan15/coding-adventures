//! SRAM — Static Random-Access Memory at the gate level.
//!
//! # What is SRAM?
//!
//! SRAM (Static Random-Access Memory) is the fastest type of memory in a
//! computer. It is used for CPU caches (L1/L2/L3), register files, and FPGA
//! Block RAM. "Static" means the memory holds its value as long as power is
//! supplied — unlike DRAM, which must be periodically refreshed.
//!
//! # The SRAM Cell — 6 Transistors Holding 1 Bit
//!
//! In real hardware, each SRAM cell uses 6 transistors:
//! - 2 cross-coupled inverters forming a bistable latch (stores the bit)
//! - 2 access transistors controlled by the word line (gates read/write)
//!
//! We model this at the gate level:
//! - Cross-coupled inverters = two NOT gates in a feedback loop
//!   (identical to the logic behind an SR latch from logic_gates::sequential)
//! - Access transistors = AND gates that pass data only when word_line=1
//!
//! The cell has three operations:
//! - **Hold** (word_line=0): Access transistors block external access.
//!   The inverter loop maintains the stored value indefinitely.
//! - **Read** (word_line=1): Access transistors open. The stored value
//!   appears on the bit lines without disturbing it.
//! - **Write** (word_line=1 + drive bit lines): The external driver
//!   overpowers the internal inverters, forcing a new value.
//!
//! # From Cell to Array
//!
//! A RAM chip is a 2D grid of SRAM cells. To access a specific cell:
//! 1. A **row decoder** converts address bits into a one-hot word line signal
//! 2. A **column MUX** selects which columns to read/write
//!
//! This module provides:
//! - [`SRAMCell`]: single-bit storage at the gate level
//! - [`SRAMArray`]: 2D grid with row/column addressing

/// Panics (in debug mode) if the value is not 0 or 1.
#[inline]
pub(crate) fn validate_bit(value: u8, name: &str) {
    debug_assert!(
        value == 0 || value == 1,
        "{name} must be 0 or 1, got {value}"
    );
}

// ===========================================================================
// SRAMCell — Single-bit storage element
// ===========================================================================

/// Single-bit storage element modeled at the gate level.
///
/// Internally, this is a pair of cross-coupled inverters (forming a
/// bistable latch) gated by access transistors controlled by the word line.
///
/// In our simulation, we model the steady-state behavior directly rather
/// than simulating individual gate delays:
/// - word_line=0: cell is isolated, value is retained
/// - word_line=1, reading: value is output
/// - word_line=1, writing: new value overwrites stored value
///
/// This matches the real behavior of a 6T SRAM cell while keeping the
/// simulation fast enough to model arrays of thousands of cells.
///
/// # Example
///
/// ```
/// use block_ram::sram::SRAMCell;
/// let mut cell = SRAMCell::new();
/// assert_eq!(cell.value(), 0);
///
/// cell.write(1, 1); // word_line=1, bit_line=1
/// assert_eq!(cell.value(), 1);
/// assert_eq!(cell.read(1), Some(1));
/// assert_eq!(cell.read(0), None); // not selected
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SRAMCell {
    /// The stored bit value (0 or 1).
    value: u8,
}

impl SRAMCell {
    /// Create an SRAM cell initialized to 0.
    ///
    /// The initial state of 0 represents the cell after power-on reset.
    /// In real hardware, SRAM cells power up in an indeterminate state,
    /// but we initialize to 0 for predictability in simulation.
    pub fn new() -> Self {
        Self { value: 0 }
    }

    /// Read the stored bit if the cell is selected.
    ///
    /// Returns `Some(value)` when word_line=1 (cell selected),
    /// `None` when word_line=0 (cell not selected, no output).
    ///
    /// # Example
    ///
    /// ```
    /// use block_ram::sram::SRAMCell;
    /// let mut cell = SRAMCell::new();
    /// cell.write(1, 1);
    /// assert_eq!(cell.read(1), Some(1)); // selected
    /// assert_eq!(cell.read(0), None);    // not selected
    /// ```
    pub fn read(&self, word_line: u8) -> Option<u8> {
        validate_bit(word_line, "word_line");

        if word_line == 0 {
            None
        } else {
            Some(self.value)
        }
    }

    /// Write a bit to the cell if selected.
    ///
    /// When word_line=1, the access transistors open and the external
    /// bit_line driver overpowers the internal inverter loop, forcing
    /// the cell to store the new value.
    ///
    /// When word_line=0, the access transistors are closed and the
    /// write has no effect — the cell retains its previous value.
    ///
    /// # Example
    ///
    /// ```
    /// use block_ram::sram::SRAMCell;
    /// let mut cell = SRAMCell::new();
    /// cell.write(1, 1);  // selected: stores 1
    /// assert_eq!(cell.value(), 1);
    /// cell.write(0, 0);  // not selected: no change
    /// assert_eq!(cell.value(), 1);
    /// ```
    pub fn write(&mut self, word_line: u8, bit_line: u8) {
        validate_bit(word_line, "word_line");
        validate_bit(bit_line, "bit_line");

        if word_line == 1 {
            self.value = bit_line;
        }
    }

    /// Current stored value (for inspection/debugging).
    pub fn value(&self) -> u8 {
        self.value
    }
}

impl Default for SRAMCell {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// SRAMArray — 2D grid of SRAM cells with row/column addressing
// ===========================================================================

/// 2D grid of SRAM cells with row/column addressing.
///
/// An SRAM array organizes cells into rows and columns:
/// - Each row shares a word line (activated by the row decoder)
/// - Each column shares a bit line (carries data in/out)
///
/// To read: activate a row's word line, and all cells in that row
/// output their values onto their respective bit lines.
///
/// To write: activate a row's word line and drive the bit lines
/// with the desired data — all cells in that row store the new values.
///
/// ```text
/// Memory map (4x4 array):
///
/// Row 0 (WL0): [Cell00] [Cell01] [Cell02] [Cell03]
/// Row 1 (WL1): [Cell10] [Cell11] [Cell12] [Cell13]
/// Row 2 (WL2): [Cell20] [Cell21] [Cell22] [Cell23]
/// Row 3 (WL3): [Cell30] [Cell31] [Cell32] [Cell33]
/// ```
///
/// # Example
///
/// ```
/// use block_ram::sram::SRAMArray;
/// let mut arr = SRAMArray::new(4, 8); // 4 rows x 8 columns
/// arr.write(0, &[1,0,1,0, 0,1,0,1]);
/// assert_eq!(arr.read(0), vec![1,0,1,0, 0,1,0,1]);
/// assert_eq!(arr.read(1), vec![0,0,0,0, 0,0,0,0]); // never written
/// ```
#[derive(Debug, Clone)]
pub struct SRAMArray {
    rows: usize,
    cols: usize,
    cells: Vec<Vec<SRAMCell>>,
}

impl SRAMArray {
    /// Create an SRAM array initialized to all zeros.
    ///
    /// # Panics
    ///
    /// Panics if rows or cols < 1.
    pub fn new(rows: usize, cols: usize) -> Self {
        assert!(rows >= 1, "rows must be >= 1, got {rows}");
        assert!(cols >= 1, "cols must be >= 1, got {cols}");

        let cells = (0..rows)
            .map(|_| (0..cols).map(|_| SRAMCell::new()).collect())
            .collect();

        Self { rows, cols, cells }
    }

    /// Read all columns of a row.
    ///
    /// Activates the word line for the given row, causing all cells
    /// in that row to output their stored values.
    ///
    /// # Panics
    ///
    /// Panics if row is out of range.
    pub fn read(&self, row: usize) -> Vec<u8> {
        assert!(row < self.rows, "row {row} out of range [0, {}]", self.rows - 1);

        self.cells[row]
            .iter()
            .map(|cell| cell.read(1).unwrap())
            .collect()
    }

    /// Write data to a row.
    ///
    /// Activates the word line for the given row and drives the bit
    /// lines with the given data, storing values in all cells of the row.
    ///
    /// # Panics
    ///
    /// Panics if row is out of range or data length does not match cols.
    pub fn write(&mut self, row: usize, data: &[u8]) {
        assert!(row < self.rows, "row {row} out of range [0, {}]", self.rows - 1);
        assert!(
            data.len() == self.cols,
            "data length {} does not match cols {}",
            data.len(),
            self.cols
        );

        for (i, &bit) in data.iter().enumerate() {
            validate_bit(bit, &format!("data[{i}]"));
        }

        for (col, &bit) in data.iter().enumerate() {
            self.cells[row][col].write(1, bit);
        }
    }

    /// Array dimensions as (rows, cols).
    pub fn shape(&self) -> (usize, usize) {
        (self.rows, self.cols)
    }
}

// ===========================================================================
// Inline unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sram_cell_initial_value() {
        let cell = SRAMCell::new();
        assert_eq!(cell.value(), 0);
    }

    #[test]
    fn test_sram_cell_write_and_read() {
        let mut cell = SRAMCell::new();
        cell.write(1, 1);
        assert_eq!(cell.read(1), Some(1));
        assert_eq!(cell.value(), 1);
    }

    #[test]
    fn test_sram_cell_not_selected() {
        let mut cell = SRAMCell::new();
        cell.write(1, 1);
        assert_eq!(cell.read(0), None);
        // Write with word_line=0 has no effect
        cell.write(0, 0);
        assert_eq!(cell.value(), 1);
    }

    #[test]
    fn test_sram_array_read_write() {
        let mut arr = SRAMArray::new(4, 8);
        arr.write(0, &[1, 0, 1, 0, 0, 1, 0, 1]);
        assert_eq!(arr.read(0), vec![1, 0, 1, 0, 0, 1, 0, 1]);
        assert_eq!(arr.read(1), vec![0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_sram_array_shape() {
        let arr = SRAMArray::new(2, 4);
        assert_eq!(arr.shape(), (2, 4));
    }
}
