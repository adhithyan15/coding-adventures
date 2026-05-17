//! Cell address parsing and formatting.
//!
//! A spreadsheet cell address looks like "A1", "B12", or "Z99".  The first
//! character is a single uppercase letter (the *column*, A = 0, Z = 25) and
//! the remaining characters are a decimal integer (the *row*, 1–99).
//!
//! Think of it like coordinates on a chess board: the letter picks the column,
//! and the number picks the row.
//!
//! # Examples
//!
//! ```rust
//! use mosaic_formula_engine::{CellAddr, FormulaError};
//!
//! let a1 = CellAddr::parse("A1").unwrap();
//! assert_eq!(a1.col(), 0);
//! assert_eq!(a1.row(), 1);
//! assert_eq!(a1.to_string(), "A1");
//!
//! assert!(CellAddr::parse("AA1").is_err());   // two letters — invalid
//! assert!(CellAddr::parse("A0").is_err());    // row 0 — invalid
//! assert!(CellAddr::parse("A100").is_err());  // row > 99 — invalid
//! ```

use crate::FormulaError;

/// A validated spreadsheet cell address.
///
/// `col` is 0-based (0 = 'A', 25 = 'Z').
/// `row` is 1-based (1..=99).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CellAddr {
    pub(crate) col: u8, // 0..=25
    pub(crate) row: u8, // 1..=99
}

impl CellAddr {
    /// Parse a cell address string such as "A1" or "Z99".
    ///
    /// Rules:
    /// - Exactly one uppercase or lowercase letter for the column.
    /// - One or two decimal digits (1–99) for the row.
    /// - Row must be in 1..=99.
    ///
    /// Returns `Err(FormulaError::Parse)` for anything that doesn't fit.
    pub fn parse(s: &str) -> Result<Self, FormulaError> {
        if s.is_empty() {
            return Err(FormulaError::Parse);
        }

        let bytes = s.as_bytes();

        // The first character must be a single letter (A-Z or a-z).
        let col_ch = bytes[0];
        let col = if col_ch.is_ascii_alphabetic() {
            col_ch.to_ascii_uppercase() - b'A'
        } else {
            return Err(FormulaError::Parse);
        };

        // col must be 0..=25 (guaranteed by the A-Z check above, but let's be explicit).
        if col > 25 {
            return Err(FormulaError::Parse);
        }

        // The rest must be a non-empty sequence of digits, representing a row number.
        let row_str = &s[1..];
        if row_str.is_empty() {
            return Err(FormulaError::Parse);
        }

        // Ensure every remaining character is a digit (no letters like "AA1").
        if !row_str.bytes().all(|b| b.is_ascii_digit()) {
            return Err(FormulaError::Parse);
        }

        // Parse the row number. We cap at 3 digits to avoid overflow before the
        // range check; any four-digit number exceeds 99 anyway.
        if row_str.len() > 2 {
            return Err(FormulaError::Parse);
        }

        let row: u8 = row_str
            .parse::<u8>()
            .map_err(|_| FormulaError::Parse)?;

        // Row must be in the valid range 1..=99.
        if !(1..=99).contains(&row) {
            return Err(FormulaError::Parse);
        }

        Ok(CellAddr { col, row })
    }

    /// Return the column index (0 = 'A', 25 = 'Z').
    pub fn col(&self) -> u8 {
        self.col
    }

    /// Return the row number (1..=99).
    pub fn row(&self) -> u8 {
        self.row
    }

    /// Format as "A1", "B12", etc.
    pub fn to_addr_string(&self) -> String {
        let col_ch = (b'A' + self.col) as char;
        format!("{}{}", col_ch, self.row)
    }
}

impl std::fmt::Display for CellAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_addr_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_addr_parse_valid() {
        let a1 = CellAddr::parse("A1").unwrap();
        assert_eq!(a1.col(), 0);
        assert_eq!(a1.row(), 1);
        assert_eq!(a1.to_addr_string(), "A1");

        let z99 = CellAddr::parse("Z99").unwrap();
        assert_eq!(z99.col(), 25);
        assert_eq!(z99.row(), 99);
        assert_eq!(z99.to_addr_string(), "Z99");

        let b12 = CellAddr::parse("B12").unwrap();
        assert_eq!(b12.col(), 1);
        assert_eq!(b12.row(), 12);
        assert_eq!(b12.to_addr_string(), "B12");

        // Lowercase letter should also parse.
        let a5 = CellAddr::parse("a5").unwrap();
        assert_eq!(a5.col(), 0);
        assert_eq!(a5.row(), 5);
    }

    #[test]
    fn test_cell_addr_parse_invalid() {
        // Empty string.
        assert!(CellAddr::parse("").is_err());
        // No letter prefix.
        assert!(CellAddr::parse("11").is_err());
        // Two letters.
        assert!(CellAddr::parse("AA1").is_err());
        // Row 0.
        assert!(CellAddr::parse("A0").is_err());
        // Row 100 (three digits that exceed 99).
        assert!(CellAddr::parse("A100").is_err());
        // No row digits.
        assert!(CellAddr::parse("A").is_err());
        // Letter after digit.
        assert!(CellAddr::parse("A1B").is_err());
    }
}
