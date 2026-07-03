//! Viewport primitives for the virtualized *infinite* sheet.
//!
//! The sheet is already unbounded at the storage layer: `CellAddress` is a pair
//! of `u32`s (≈4.3 billion rows × 4.3 billion columns) and cells live in a
//! sparse `HashMap`, so only the cells you actually touch exist. What a
//! *virtualized* host needs on top of that is a way to read just the rectangle
//! the user can currently see — not the whole sheet — plus enough metadata to
//! size scrollbars and to refresh efficiently after an edit.
//!
//! This module defines the value types those reads return. The methods that
//! produce them live on [`Workbook`](crate::Workbook):
//!
//! - [`Workbook::get_window`] — the dense rectangle the host renders.
//! - [`Workbook::used_range`] — the data's bounding box, for scrollbar sizing.
//! - [`Workbook::changed_since`] — which cells changed since a prior revision,
//!   so a host re-fetches only the dirtied *visible* cells after an edit.
//!
//! Coordinates are **1-based and inclusive** everywhere here, matching the A1
//! surface (`A1` is row 1, column 1).

use crate::address::CellAddress;
use crate::cell::CellValue;

/// Maximum number of cells a single [`get_window`](crate::Workbook::get_window)
/// may return. This is a *screen-scale* safety cap, not the data scale: a 4K
/// display shows a few thousand cells, and 65 536 leaves generous headroom for
/// overscan. A host clamps its request to the visible window long before this
/// bites; the cap only stops a buggy or hostile caller from asking for a
/// billion-cell rectangle and exhausting memory — the same role
/// [`MAX_RANGE_CELLS`](crate::address::MAX_RANGE_CELLS) plays for formula ranges.
pub const MAX_WINDOW_CELLS: u64 = 1 << 16; // 65,536

/// How many change-log entries the workbook retains for
/// [`changed_since`](crate::Workbook::changed_since) diffing. Past this, the
/// oldest entries are dropped and a query reaching back before the retained
/// window returns [`ChangeSet::Stale`] (the safe "re-read everything" signal)
/// rather than silently missing a change.
pub const CHANGELOG_RETAIN: usize = 4096;

/// A dense rectangle of computed values, as returned by
/// [`Workbook::get_window`]. Empty cells are included as [`CellValue::Empty`]
/// (not omitted) so the host can index the result directly by position and
/// render a solid grid. Values are stored **row-major**.
#[derive(Debug, Clone, PartialEq)]
pub struct Window {
    /// 1-based row of the top-left corner (echoes the request).
    pub row0: u32,
    /// 1-based column of the top-left corner (echoes the request).
    pub col0: u32,
    /// Number of rows in the window.
    pub rows: u32,
    /// Number of columns in the window.
    pub cols: u32,
    /// `rows * cols` values in row-major order: index `(r * cols + c)` is the
    /// cell at absolute address `(row0 + r, col0 + c)`.
    pub values: Vec<CellValue>,
}

/// A dense rectangle of **display strings**, as returned by
/// [`Workbook::get_display_window`] — like [`Window`], but each cell is already
/// rendered through its format code (what to paint), so a host draws the strings
/// directly without converting typed values itself. Empty cells are `""`, stored
/// **row-major**.
///
/// [`Workbook::get_display_window`]: crate::Workbook::get_display_window
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayWindow {
    /// 1-based row of the top-left corner (echoes the request).
    pub row0: u32,
    /// 1-based column of the top-left corner (echoes the request).
    pub col0: u32,
    /// Number of rows in the window.
    pub rows: u32,
    /// Number of columns in the window.
    pub cols: u32,
    /// `rows * cols` display strings in row-major order: index `(r * cols + c)`
    /// is the cell at absolute address `(row0 + r, col0 + c)`.
    pub cells: Vec<String>,
}

impl Window {
    /// The value at an absolute 1-based `(row, col)`, if it falls inside this
    /// window. Convenience for tests and hosts that hold an address rather than
    /// an offset.
    pub fn get(&self, row: u32, col: u32) -> Option<&CellValue> {
        if row < self.row0 || col < self.col0 {
            return None;
        }
        let dr = row - self.row0;
        let dc = col - self.col0;
        if dr >= self.rows || dc >= self.cols {
            return None;
        }
        self.values.get((dr * self.cols + dc) as usize)
    }
}

/// The bounding box of all materialised, non-empty cells on a sheet, as
/// returned by [`Workbook::used_range`]. 1-based and inclusive. A host sizes its
/// scrollable area to this (plus a comfortable blank margin) so the scrollbar
/// reflects the data while still letting the user scroll into empty space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsedRange {
    /// Topmost row containing a non-empty cell.
    pub min_row: u32,
    /// Leftmost column containing a non-empty cell.
    pub min_col: u32,
    /// Bottommost row containing a non-empty cell.
    pub max_row: u32,
    /// Rightmost column containing a non-empty cell.
    pub max_col: u32,
}

/// The result of [`Workbook::changed_since`].
///
/// `Delta` lists exactly the cells whose value changed strictly after the
/// queried revision. `Stale` means the query reached back before the retained
/// change-log window, so completeness can't be guaranteed and the host should
/// re-read its whole visible window — never silently miss a change.
#[derive(Debug, Clone, PartialEq)]
pub enum ChangeSet {
    /// A complete list of changed addresses since the queried revision.
    Delta {
        /// The workbook's current revision (the host stores this for next time).
        current_revision: u64,
        /// Addresses whose value changed after the queried revision, deduped.
        changed: Vec<CellAddress>,
    },
    /// The queried revision is older than the retained log; re-read the window.
    Stale {
        /// The workbook's current revision.
        current_revision: u64,
    },
}
