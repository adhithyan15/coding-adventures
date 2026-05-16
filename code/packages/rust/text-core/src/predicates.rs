//! T / N — type predicates.
//!
//! These are tiny Excel adapters. In a spreadsheet:
//!
//! - `T(value)` returns the value unchanged if it's text, else `""`.
//! - `N(value)` returns the value unchanged if it's a number, else `0`.
//!
//! In Rust, the cell's *type* is encoded by which function the bridge layer
//! calls. We implement two thin helpers:
//!
//! - `t_text(Some("hi")) == "hi"`; `t_text(None) == ""`.
//! - `n_number(Some(3.14)) == 3.14`; `n_number(None) == 0.0`.
//!
//! These look trivial in isolation but the bridge needs canonical names so
//! that `=T(A1)` and `=N(A1)` always reach the same implementation.

/// `T(value)`. Returns the text if present, empty string if not.
pub fn t_text(value: Option<&str>) -> String {
    value.unwrap_or("").to_string()
}

/// `N(value)`. Returns the number if present, `0.0` if not.
pub fn n_number(value: Option<f64>) -> f64 {
    value.unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_basic() {
        assert_eq!(t_text(Some("hello")), "hello");
        assert_eq!(t_text(None), "");
        assert_eq!(t_text(Some("")), "");
    }

    #[test]
    fn n_basic() {
        assert_eq!(n_number(Some(3.14)), 3.14);
        assert_eq!(n_number(None), 0.0);
        assert_eq!(n_number(Some(0.0)), 0.0);
        assert_eq!(n_number(Some(-7.0)), -7.0);
    }
}
