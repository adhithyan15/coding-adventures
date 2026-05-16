//! The spreadsheet-side error sentinels — `#REF!`, `#NAME?`,
//! `#DIV/0!`, etc.
//!
//! These match the seven classic Excel/Lotus error values plus the
//! three modern dynamic-array errors (`#SPILL!`, `#CALC!`,
//! `#GETTING_DATA`). They are first-class cell values in
//! [`super::cell::CellValue`] and propagate through arithmetic per
//! Excel's left-wins rule.

/// The classic + modern spreadsheet error sentinels. Match Microsoft
/// Excel's published list exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SpreadsheetError {
    /// `#REF!` — a reference no longer points to a valid cell.
    Ref,
    /// `#NAME?` — an unrecognised function or identifier in the
    /// formula.
    Name,
    /// `#DIV/0!` — division by zero (or `MOD(_, 0)`, etc.).
    DivZero,
    /// `#VALUE!` — type mismatch (e.g. arithmetic on a string).
    Value,
    /// `#N/A` — explicit NA, or a lookup that found no match.
    NotAvailable,
    /// `#NUM!` — numerical error (domain, overflow, no convergence).
    Num,
    /// `#NULL!` — the intersection of two ranges is empty.
    Null,
    /// `#CALC!` — dynamic-array calculation issue (Excel 365).
    Calc,
    /// `#SPILL!` — dynamic-array would overwrite a non-empty cell.
    Spill,
    /// `#GETTING_DATA` — async / external data is pending.
    GettingData,
}

impl SpreadsheetError {
    /// The error code Excel writes to disk in XLSX
    /// (`ERROR.TYPE(...)` returns this for each).
    pub fn excel_code(self) -> u32 {
        // The numbers below come from the Excel function
        // `ERROR.TYPE`: see Microsoft Learn's reference.
        match self {
            SpreadsheetError::Null => 1,
            SpreadsheetError::DivZero => 2,
            SpreadsheetError::Value => 3,
            SpreadsheetError::Ref => 4,
            SpreadsheetError::Name => 5,
            SpreadsheetError::Num => 6,
            SpreadsheetError::NotAvailable => 7,
            SpreadsheetError::GettingData => 8,
            SpreadsheetError::Calc => 14,
            SpreadsheetError::Spill => 9,
        }
    }

    /// Human-readable display string (e.g. `"#REF!"`).
    pub fn display(self) -> &'static str {
        match self {
            SpreadsheetError::Ref => "#REF!",
            SpreadsheetError::Name => "#NAME?",
            SpreadsheetError::DivZero => "#DIV/0!",
            SpreadsheetError::Value => "#VALUE!",
            SpreadsheetError::NotAvailable => "#N/A",
            SpreadsheetError::Num => "#NUM!",
            SpreadsheetError::Null => "#NULL!",
            SpreadsheetError::Calc => "#CALC!",
            SpreadsheetError::Spill => "#SPILL!",
            SpreadsheetError::GettingData => "#GETTING_DATA",
        }
    }
}

impl core::fmt::Display for SpreadsheetError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.display())
    }
}

impl std::error::Error for SpreadsheetError {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_matches_excel() {
        assert_eq!(SpreadsheetError::Ref.display(), "#REF!");
        assert_eq!(SpreadsheetError::Name.display(), "#NAME?");
        assert_eq!(SpreadsheetError::DivZero.display(), "#DIV/0!");
        assert_eq!(SpreadsheetError::NotAvailable.display(), "#N/A");
        assert_eq!(SpreadsheetError::Num.display(), "#NUM!");
    }

    #[test]
    fn excel_code_for_each_classic_error() {
        // From Microsoft Learn, ERROR.TYPE reference.
        assert_eq!(SpreadsheetError::Null.excel_code(), 1);
        assert_eq!(SpreadsheetError::DivZero.excel_code(), 2);
        assert_eq!(SpreadsheetError::Value.excel_code(), 3);
        assert_eq!(SpreadsheetError::Ref.excel_code(), 4);
        assert_eq!(SpreadsheetError::Name.excel_code(), 5);
        assert_eq!(SpreadsheetError::Num.excel_code(), 6);
        assert_eq!(SpreadsheetError::NotAvailable.excel_code(), 7);
    }
}
