//! Cell addresses, ranges, and sheets.
//!
//! - 1-based at the public surface (A1 notation; matches Excel,
//!   Lotus, R-vector indexing convention).
//! - 0-based internally — converted at the parsing / formatting
//!   boundary so the rest of the engine never has to think about it.

use crate::errors::SpreadsheetError;

/// Identifier for a sheet within a workbook. Dense `u32`; assigned
/// at sheet-creation time and never reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SheetId(pub u32);

/// A cell address — `(sheet?, row, col)`. `sheet` is `None` when the
/// address is sheet-local (the parser drops it inside a single-sheet
/// formula); the recalc engine fills it in at evaluation time.
///
/// The `absolute_row` / `absolute_col` flags survive copy-paste
/// arithmetic — see [`CellAddress::shift`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CellAddress {
    /// 1-based row.
    pub row: u32,
    /// 1-based column. `1 = A`, `26 = Z`, `27 = AA`.
    pub col: u32,
    /// `$row` notation — survives relative shifts.
    pub absolute_row: bool,
    /// `$col` notation — survives relative shifts.
    pub absolute_col: bool,
}

impl CellAddress {
    /// Construct a relative address.
    pub const fn new(row: u32, col: u32) -> Self {
        Self {
            row,
            col,
            absolute_row: false,
            absolute_col: false,
        }
    }

    /// Construct an absolute address (`$A$1`).
    pub const fn absolute(row: u32, col: u32) -> Self {
        Self {
            row,
            col,
            absolute_row: true,
            absolute_col: true,
        }
    }

    /// This address with its `$` absolute markers stripped (position only).
    ///
    /// A cell is *stored* and *tracked* by position alone — `A1`, `$A1`, `A$1`,
    /// and `$A$1` in formulas must all resolve to the one cell at row 1, col 1.
    /// The absolute markers only steer how a reference *shifts* on copy/fill (see
    /// [`shift`](Self::shift)); they are not part of a cell's identity. So every
    /// place that uses a reference's address as a key into the cell store or the
    /// dependency graph normalises it through this first — otherwise a `$A$1`
    /// lookup (flags set) would miss the relatively-keyed `A1` cell and read as
    /// empty.
    pub fn without_absolute(&self) -> CellAddress {
        CellAddress::new(self.row, self.col)
    }

    /// Shift by `(d_row, d_col)`, respecting absolute flags.
    /// Returns `Err(Ref)` if the shift would push the address to row
    /// or column 0 or below (i.e. would invalidate the reference).
    pub fn shift(&self, d_row: i32, d_col: i32) -> Result<Self, SpreadsheetError> {
        let new_row = if self.absolute_row {
            self.row as i32
        } else {
            self.row as i32 + d_row
        };
        let new_col = if self.absolute_col {
            self.col as i32
        } else {
            self.col as i32 + d_col
        };
        if new_row < 1 || new_col < 1 {
            return Err(SpreadsheetError::Ref);
        }
        Ok(CellAddress {
            row: new_row as u32,
            col: new_col as u32,
            absolute_row: self.absolute_row,
            absolute_col: self.absolute_col,
        })
    }

    /// Parse an A1-style string into a `CellAddress`. Accepts the
    /// `$` absolute markers.
    pub fn parse(text: &str) -> Result<Self, SpreadsheetError> {
        let mut chars = text.chars().peekable();
        let absolute_col = if let Some(&'$') = chars.peek() {
            chars.next();
            true
        } else {
            false
        };
        let mut col_chars = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_ascii_alphabetic() {
                col_chars.push(c.to_ascii_uppercase());
                chars.next();
            } else {
                break;
            }
        }
        if col_chars.is_empty() {
            return Err(SpreadsheetError::Ref);
        }
        let absolute_row = if let Some(&'$') = chars.peek() {
            chars.next();
            true
        } else {
            false
        };
        let row_str: String = chars.collect();
        if row_str.is_empty() {
            return Err(SpreadsheetError::Ref);
        }
        let row: u32 = row_str.parse().map_err(|_| SpreadsheetError::Ref)?;
        if row == 0 {
            return Err(SpreadsheetError::Ref);
        }
        Ok(CellAddress {
            row,
            col: column_letters_to_index(&col_chars)?,
            absolute_row,
            absolute_col,
        })
    }

    /// Format back to an A1 string, preserving absolute markers.
    pub fn to_a1(&self) -> String {
        let mut s = String::new();
        if self.absolute_col {
            s.push('$');
        }
        s.push_str(&column_index_to_letters(self.col));
        if self.absolute_row {
            s.push('$');
        }
        s.push_str(&self.row.to_string());
        s
    }
}

/// Convert a column-letter string ("A", "Z", "AA", "AZ") to a
/// 1-based column index.
pub fn column_letters_to_index(letters: &str) -> Result<u32, SpreadsheetError> {
    if letters.is_empty() {
        return Err(SpreadsheetError::Ref);
    }
    let mut index: u32 = 0;
    for c in letters.chars() {
        if !c.is_ascii_alphabetic() {
            return Err(SpreadsheetError::Ref);
        }
        let v = c.to_ascii_uppercase() as u32 - 'A' as u32 + 1;
        index = index
            .checked_mul(26)
            .and_then(|i| i.checked_add(v))
            .ok_or(SpreadsheetError::Ref)?;
    }
    Ok(index)
}

/// Convert a 1-based column index to letters.
pub fn column_index_to_letters(mut index: u32) -> String {
    let mut s = String::new();
    while index > 0 {
        let rem = (index - 1) % 26;
        s.insert(0, (b'A' + rem as u8) as char);
        index = (index - 1) / 26;
    }
    s
}

// ---------------------------------------------------------------------------
// CellRange — a rectangle of addresses.
// ---------------------------------------------------------------------------

/// Maximum number of cells a single range may name before it is
/// rejected as oversized (one full Excel column = 2²⁰ = 1,048,576).
/// This is the security cap that stops a single adversarial formula
/// such as `=SUM(A1:XFD1048576)` (~17 billion cells) from exhausting
/// memory when it is expanded into the dependency graph or the
/// evaluator's argument vector. See [`CellRange::is_oversized`].
pub const MAX_RANGE_CELLS: u64 = 1 << 20;

/// A rectangular cell range, inclusive at both ends. Always
/// canonicalised so `start.row <= end.row` and `start.col <= end.col`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellRange {
    /// Top-left corner.
    pub start: CellAddress,
    /// Bottom-right corner.
    pub end: CellAddress,
}

impl CellRange {
    /// Construct from two addresses; auto-canonicalises so the
    /// stored `start` is top-left.
    pub fn new(a: CellAddress, b: CellAddress) -> Self {
        let (sr, er) = if a.row <= b.row {
            (a.row, b.row)
        } else {
            (b.row, a.row)
        };
        let (sc, ec) = if a.col <= b.col {
            (a.col, b.col)
        } else {
            (b.col, a.col)
        };
        Self {
            start: CellAddress {
                row: sr,
                col: sc,
                absolute_row: a.absolute_row,
                absolute_col: a.absolute_col,
            },
            end: CellAddress {
                row: er,
                col: ec,
                absolute_row: b.absolute_row,
                absolute_col: b.absolute_col,
            },
        }
    }

    /// Iterate over all addresses in row-major order.
    pub fn iter(&self) -> impl Iterator<Item = CellAddress> + '_ {
        let start = self.start;
        let end = self.end;
        (start.row..=end.row).flat_map(move |r| {
            (start.col..=end.col).map(move |c| CellAddress {
                row: r,
                col: c,
                absolute_row: false,
                absolute_col: false,
            })
        })
    }

    /// Number of cells in the range. Computed via [`cell_count`] in
    /// `u64` and narrowed, so it cannot overflow even on 32-bit
    /// targets (wasm32, where `usize` is 32 bits); it saturates at
    /// `usize::MAX` rather than wrapping.
    ///
    /// [`cell_count`]: CellRange::cell_count
    pub fn count(&self) -> usize {
        usize::try_from(self.cell_count()).unwrap_or(usize::MAX)
    }

    /// Number of cells in the range as a `u64`. The grid is at most
    /// `u32::MAX × u32::MAX`, which fits comfortably in `u64`, so this
    /// never overflows — unlike a `usize` multiply on a 32-bit target.
    pub fn cell_count(&self) -> u64 {
        let rows = (self.end.row - self.start.row) as u64 + 1;
        let cols = (self.end.col - self.start.col) as u64 + 1;
        rows * cols
    }

    /// Whether this range exceeds [`MAX_RANGE_CELLS`]. Callers reject
    /// oversized ranges (surfacing `#REF!`) before expanding them, so
    /// one adversarial formula cannot exhaust memory.
    pub fn is_oversized(&self) -> bool {
        self.cell_count() > MAX_RANGE_CELLS
    }

    /// Whether `addr` is inside the range (sheet-agnostic).
    pub fn contains(&self, addr: CellAddress) -> bool {
        addr.row >= self.start.row
            && addr.row <= self.end.row
            && addr.col >= self.start.col
            && addr.col <= self.end.col
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_letters_round_trip() {
        for col in [1, 26, 27, 52, 53, 702, 703, 16384] {
            let letters = column_index_to_letters(col);
            let back = column_letters_to_index(&letters).unwrap();
            assert_eq!(back, col, "column {col} -> {letters} -> back");
        }
        assert_eq!(column_index_to_letters(1), "A");
        assert_eq!(column_index_to_letters(26), "Z");
        assert_eq!(column_index_to_letters(27), "AA");
        assert_eq!(column_index_to_letters(52), "AZ");
        assert_eq!(column_index_to_letters(703), "AAA");
    }

    #[test]
    fn parse_a1_relative() {
        let a = CellAddress::parse("A1").unwrap();
        assert_eq!(a, CellAddress::new(1, 1));
        let aa1 = CellAddress::parse("AA1").unwrap();
        assert_eq!(aa1.row, 1);
        assert_eq!(aa1.col, 27);
    }

    #[test]
    fn parse_a1_absolute() {
        let a = CellAddress::parse("$A$1").unwrap();
        assert_eq!(a, CellAddress::absolute(1, 1));
        let mixed = CellAddress::parse("$B5").unwrap();
        assert_eq!(mixed.row, 5);
        assert_eq!(mixed.col, 2);
        assert!(mixed.absolute_col);
        assert!(!mixed.absolute_row);
    }

    #[test]
    fn parse_rejects_malformed() {
        assert!(CellAddress::parse("").is_err());
        assert!(CellAddress::parse("1A").is_err());
        assert!(CellAddress::parse("A").is_err());
        assert!(CellAddress::parse("A0").is_err());
    }

    #[test]
    fn to_a1_round_trips() {
        for s in ["A1", "Z99", "AA1", "$A$1", "$B5"] {
            let parsed = CellAddress::parse(s).unwrap();
            assert_eq!(parsed.to_a1(), s);
        }
    }

    #[test]
    fn shift_respects_absolute_flags() {
        let a = CellAddress::parse("$A$1").unwrap();
        let shifted = a.shift(5, 5).unwrap();
        assert_eq!(shifted, a); // absolute on both -> unchanged

        let b = CellAddress::new(2, 2);
        let shifted = b.shift(1, 1).unwrap();
        assert_eq!(shifted, CellAddress::new(3, 3));

        // Shifting to row 0 is an error.
        let c = CellAddress::new(1, 1);
        assert!(c.shift(-1, 0).is_err());
    }

    #[test]
    fn range_canonicalises_and_iterates() {
        let r = CellRange::new(CellAddress::new(2, 2), CellAddress::new(1, 1));
        assert_eq!(r.start, CellAddress::new(1, 1));
        assert_eq!(r.end, CellAddress::new(2, 2));
        assert_eq!(r.count(), 4);
        let cells: Vec<_> = r.iter().collect();
        assert_eq!(cells.len(), 4);
    }

    #[test]
    fn range_contains() {
        let r = CellRange::new(CellAddress::new(1, 1), CellAddress::new(5, 5));
        assert!(r.contains(CellAddress::new(3, 3)));
        assert!(r.contains(CellAddress::new(1, 1)));
        assert!(r.contains(CellAddress::new(5, 5)));
        assert!(!r.contains(CellAddress::new(6, 1)));
    }

    #[test]
    fn cell_count_does_not_overflow_on_a_huge_range() {
        // A1:XFD1048576 ≈ Excel's full grid — far more cells than a
        // 32-bit usize can hold. cell_count() must compute in u64
        // without wrapping (this would be wrong on wasm32 with a usize
        // multiply), and the range must be flagged oversized.
        let huge = CellRange::new(CellAddress::new(1, 1), CellAddress::new(1_048_576, 16_384));
        assert_eq!(huge.cell_count(), 1_048_576u64 * 16_384u64);
        assert!(huge.is_oversized());
    }

    #[test]
    fn small_range_is_not_oversized() {
        let small = CellRange::new(CellAddress::new(1, 1), CellAddress::new(100, 100));
        assert!(!small.is_oversized());
        assert_eq!(small.count(), 10_000);
    }

    #[test]
    fn oversized_threshold_is_exact() {
        // Exactly MAX_RANGE_CELLS (one full column) is allowed; one
        // more is not.
        let one_col = CellRange::new(CellAddress::new(1, 1), CellAddress::new(1 << 20, 1));
        assert_eq!(one_col.cell_count(), MAX_RANGE_CELLS);
        assert!(!one_col.is_oversized());
        let over = CellRange::new(CellAddress::new(1, 1), CellAddress::new((1 << 20) + 1, 1));
        assert!(over.is_oversized());
    }
}
