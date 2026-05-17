//! Cell values and formula errors.
//!
//! Every cell in the engine holds a [`CellValue`].  When a formula is
//! evaluated successfully the cell gets `Number`, `Text`, or `Bool`.  When
//! evaluation fails — division by zero, bad reference, syntax error, etc. —
//! the cell holds `Error(FormulaError)`, and that error propagates to any
//! downstream cell that references it.
//!
//! The error codes mirror the classic VisiCalc display strings (`#DIV/0!`,
//! `#REF!`, …) so that users who grew up with spreadsheets recognise them
//! immediately.

/// A computed cell value.
#[derive(Debug, Clone, PartialEq)]
pub enum CellValue {
    /// The cell has no content.
    Empty,
    /// A text string (from a literal or a formula that evaluates to text).
    Text(String),
    /// A floating-point number.
    Number(f64),
    /// A boolean (produced by `IF`, comparison operators, or the literals
    /// `TRUE`/`FALSE`).
    Bool(bool),
    /// An error value.  Errors propagate: any formula that reads this cell
    /// will itself become the same error.
    Error(FormulaError),
}

/// The set of errors that a formula can produce.
///
/// Each variant corresponds to a VisiCalc-style display string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormulaError {
    /// Division by zero. Displayed as `#DIV/0!`.
    DivZero,
    /// Invalid cell reference (out of bounds, or a range used where a scalar
    /// is expected). Displayed as `#REF!`.
    Ref,
    /// Unknown function name. Displayed as `#NAME?`.
    Name,
    /// Wrong type for an operation (e.g. adding text to a number without
    /// implicit conversion). Displayed as `#VALUE!`.
    Value,
    /// Circular dependency detected. Displayed as `#CIRC`.
    Circ,
    /// Formula syntax error. Displayed as `#PARSE`.
    Parse,
}

impl std::fmt::Display for FormulaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            FormulaError::DivZero => "#DIV/0!",
            FormulaError::Ref => "#REF!",
            FormulaError::Name => "#NAME?",
            FormulaError::Value => "#VALUE!",
            FormulaError::Circ => "#CIRC",
            FormulaError::Parse => "#PARSE",
        };
        write!(f, "{}", s)
    }
}

impl CellValue {
    /// Format the value for display in the cell grid.
    ///
    /// Numbers that are mathematically integers (e.g. `6.0`) are shown
    /// without a decimal point (`"6"`), while fractional numbers retain
    /// their decimal part (`"2.5"`).
    pub fn display_string(&self) -> String {
        match self {
            CellValue::Empty => String::new(),
            CellValue::Text(s) => s.clone(),
            CellValue::Number(n) => format_number(*n),
            CellValue::Bool(b) => if *b { "TRUE".to_string() } else { "FALSE".to_string() },
            CellValue::Error(e) => e.to_string(),
        }
    }

    /// Return the numeric value if this is a `Number`, else `None`.
    pub fn as_number(&self) -> Option<f64> {
        match self {
            CellValue::Number(n) => Some(*n),
            CellValue::Bool(true) => Some(1.0),
            CellValue::Bool(false) => Some(0.0),
            _ => None,
        }
    }

    /// Return true when this value represents logical "true" (non-zero number,
    /// `Bool(true)`, or non-empty text).
    pub fn is_truthy(&self) -> bool {
        match self {
            CellValue::Bool(b) => *b,
            CellValue::Number(n) => *n != 0.0,
            CellValue::Text(s) => !s.is_empty(),
            CellValue::Empty => false,
            CellValue::Error(_) => false,
        }
    }
}

/// Format a floating-point number for spreadsheet display.
///
/// If the value is a finite integer (fractional part == 0), omit the
/// decimal point.  Otherwise use Rust's default `f64` Display, which
/// produces the shortest round-trip representation.
pub fn format_number(n: f64) -> String {
    if n.is_finite() && n.fract() == 0.0 {
        // Cast to i64 for clean integer formatting.
        // Safe because: n is finite, has no fractional part, and i64 can
        // represent all integers up to 2^53 exactly (the range that f64 can
        // represent exactly).
        format!("{}", n as i64)
    } else {
        format!("{}", n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formula_error_display() {
        assert_eq!(FormulaError::DivZero.to_string(), "#DIV/0!");
        assert_eq!(FormulaError::Ref.to_string(), "#REF!");
        assert_eq!(FormulaError::Name.to_string(), "#NAME?");
        assert_eq!(FormulaError::Value.to_string(), "#VALUE!");
        assert_eq!(FormulaError::Circ.to_string(), "#CIRC");
        assert_eq!(FormulaError::Parse.to_string(), "#PARSE");
    }

    #[test]
    fn test_cell_value_display() {
        assert_eq!(CellValue::Empty.display_string(), "");
        assert_eq!(CellValue::Text("hello".to_string()).display_string(), "hello");
        assert_eq!(CellValue::Number(6.0).display_string(), "6");
        assert_eq!(CellValue::Number(2.5).display_string(), "2.5");
        assert_eq!(CellValue::Number(-3.0).display_string(), "-3");
        assert_eq!(CellValue::Bool(true).display_string(), "TRUE");
        assert_eq!(CellValue::Bool(false).display_string(), "FALSE");
        assert_eq!(CellValue::Error(FormulaError::DivZero).display_string(), "#DIV/0!");
    }

    #[test]
    fn test_format_number_integral() {
        assert_eq!(format_number(0.0), "0");
        assert_eq!(format_number(1.0), "1");
        assert_eq!(format_number(-5.0), "-5");
        assert_eq!(format_number(100.0), "100");
    }

    #[test]
    fn test_format_number_fractional() {
        assert_eq!(format_number(2.5), "2.5");
        assert_eq!(format_number(0.1), "0.1");
        assert_eq!(format_number(-1.5), "-1.5");
    }
}
