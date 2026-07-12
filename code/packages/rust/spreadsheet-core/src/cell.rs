//! The cell — the atomic unit of a spreadsheet.

use crate::ast::FormulaAst;
use crate::errors::SpreadsheetError;

/// Value carried in a cell or returned from a formula. Matches
/// Excel's cell-value type set plus the spreadsheet error sentinels
/// from [`super::errors::SpreadsheetError`].
#[derive(Debug, Clone, PartialEq)]
pub enum CellValue {
    /// Empty cell — Excel's blank. Coerces to `0` in arithmetic and
    /// `""` in text concatenation.
    Empty,
    /// `TRUE` / `FALSE`.
    Boolean(bool),
    /// Numeric value. Excel stores all numbers as f64.
    Number(f64),
    /// UTF-8 text.
    Text(String),
    /// One of the spreadsheet error sentinels (`#REF!`, `#NAME?`, …).
    Error(SpreadsheetError),
}

impl CellValue {
    /// `true` if this is the empty-cell sentinel.
    pub fn is_empty(&self) -> bool {
        matches!(self, CellValue::Empty)
    }

    /// `true` if this is any error sentinel.
    pub fn is_error(&self) -> bool {
        matches!(self, CellValue::Error(_))
    }

    /// `true` if specifically `#N/A`.
    pub fn is_na(&self) -> bool {
        matches!(self, CellValue::Error(SpreadsheetError::NotAvailable))
    }

    /// Coerce to f64 for arithmetic. Empty → 0; Boolean → 0/1;
    /// numeric → itself; text → tries to parse else `#VALUE!`;
    /// error → propagates.
    pub fn coerce_number(&self) -> Result<f64, SpreadsheetError> {
        match self {
            CellValue::Empty => Ok(0.0),
            CellValue::Boolean(true) => Ok(1.0),
            CellValue::Boolean(false) => Ok(0.0),
            CellValue::Number(n) => Ok(*n),
            CellValue::Text(s) => s.parse::<f64>().map_err(|_| SpreadsheetError::Value),
            CellValue::Error(e) => Err(*e),
        }
    }

    /// Coerce to text for concatenation. Empty → ""; number → its
    /// shortest representation; boolean → "TRUE"/"FALSE"; text →
    /// itself; error → propagates.
    pub fn coerce_text(&self) -> Result<String, SpreadsheetError> {
        match self {
            CellValue::Empty => Ok(String::new()),
            CellValue::Boolean(true) => Ok("TRUE".into()),
            CellValue::Boolean(false) => Ok("FALSE".into()),
            CellValue::Number(n) => Ok(format_number(*n)),
            CellValue::Text(s) => Ok(s.clone()),
            CellValue::Error(e) => Err(*e),
        }
    }

    /// Coerce to bool. Excel rule: 0 → false, anything-else-numeric
    /// → true; "TRUE"/"FALSE" parse; other text → `#VALUE!`.
    pub fn coerce_bool(&self) -> Result<bool, SpreadsheetError> {
        match self {
            CellValue::Empty => Ok(false),
            CellValue::Boolean(b) => Ok(*b),
            CellValue::Number(n) => Ok(*n != 0.0),
            CellValue::Text(s) => match s.to_ascii_uppercase().as_str() {
                "TRUE" => Ok(true),
                "FALSE" => Ok(false),
                _ => Err(SpreadsheetError::Value),
            },
            CellValue::Error(e) => Err(*e),
        }
    }
}

fn format_number(n: f64) -> String {
    if n == n.trunc() && n.abs() < 1e16 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

// ---------------------------------------------------------------------------
// Cell
// ---------------------------------------------------------------------------

/// A cell holds either a literal value, a formula (which evaluates
/// to a value), or nothing at all.
#[derive(Debug, Clone)]
pub enum CellContent {
    /// Nothing in this cell. Treated as `CellValue::Empty` whenever
    /// the cell is referenced.
    Empty,
    /// A literal value entered directly (no formula).
    Value(CellValue),
    /// A parsed formula plus its last evaluated value.
    Formula {
        /// The parsed formula AST.
        ast: FormulaAst,
        /// The original formula text as the user typed it (for
        /// echo-back and save-to-disk).
        text: String,
        /// Cached value from the last successful evaluation. `None`
        /// before the first recalc.
        cached: Option<CellValue>,
    },
}

/// A cell at a known address. Carries content plus a format hint.
#[derive(Debug, Clone)]
pub struct Cell {
    /// What's in this cell.
    pub content: CellContent,
}

impl Cell {
    /// Empty cell.
    pub fn empty() -> Self {
        Self {
            content: CellContent::Empty,
        }
    }

    /// Literal-value cell.
    pub fn value(v: CellValue) -> Self {
        Self {
            content: CellContent::Value(v),
        }
    }

    /// The value the cell evaluates to right now:
    /// - Empty cell → `Empty`
    /// - Literal → the literal
    /// - Formula with cached result → the cached result
    /// - Formula not yet evaluated → `Empty` (recalc not run)
    pub fn current_value(&self) -> CellValue {
        match &self.content {
            CellContent::Empty => CellValue::Empty,
            CellContent::Value(v) => v.clone(),
            CellContent::Formula {
                cached: Some(v), ..
            } => v.clone(),
            CellContent::Formula { cached: None, .. } => CellValue::Empty,
        }
    }

    /// Whether this cell holds a formula (regardless of cache state).
    pub fn is_formula(&self) -> bool {
        matches!(self.content, CellContent::Formula { .. })
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self::empty()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_coerces_to_zero_and_empty_string() {
        assert_eq!(CellValue::Empty.coerce_number().unwrap(), 0.0);
        assert_eq!(CellValue::Empty.coerce_text().unwrap(), "");
        assert!(!CellValue::Empty.coerce_bool().unwrap());
    }

    #[test]
    fn boolean_coerces_to_one_or_zero() {
        assert_eq!(CellValue::Boolean(true).coerce_number().unwrap(), 1.0);
        assert_eq!(CellValue::Boolean(false).coerce_number().unwrap(), 0.0);
        assert_eq!(CellValue::Boolean(true).coerce_text().unwrap(), "TRUE");
    }

    #[test]
    // 3.14 here is arbitrary numeric test data, not an approximation of PI.
    #[allow(clippy::approx_constant)]
    fn number_text_round_trip() {
        let n = CellValue::Number(42.0);
        assert_eq!(n.coerce_text().unwrap(), "42");
        let f = CellValue::Number(3.14);
        assert_eq!(f.coerce_text().unwrap(), "3.14");
    }

    #[test]
    // 3.14 here is arbitrary numeric test data, not an approximation of PI.
    #[allow(clippy::approx_constant)]
    fn text_numeric_parse() {
        let t = CellValue::Text("3.14".into());
        assert!((t.coerce_number().unwrap() - 3.14).abs() < 1e-9);
        // Bad parse -> #VALUE!
        let bad = CellValue::Text("hello".into());
        assert_eq!(bad.coerce_number(), Err(SpreadsheetError::Value));
    }

    #[test]
    fn error_propagates_through_coercion() {
        let e = CellValue::Error(SpreadsheetError::DivZero);
        assert_eq!(e.coerce_number(), Err(SpreadsheetError::DivZero));
        assert_eq!(e.coerce_text(), Err(SpreadsheetError::DivZero));
        assert_eq!(e.coerce_bool(), Err(SpreadsheetError::DivZero));
    }

    #[test]
    fn cell_current_value() {
        let c = Cell::empty();
        assert_eq!(c.current_value(), CellValue::Empty);

        let c = Cell::value(CellValue::Number(7.0));
        assert_eq!(c.current_value(), CellValue::Number(7.0));
    }
}
