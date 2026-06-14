//! CHOOSE — pick the n-th argument from a variadic list.
//!
//! In Excel: `CHOOSE(index, value1, value2, ..., value254)` returns the
//! `index`-th value (1-based).  We model the variadic list as a slice.

use crate::{one_based_to_zero, LookupResult, LookupValue};

/// `CHOOSE(index, values...)`.  Returns `#VALUE!` (mapped to
/// `LookupError::OutOfRange`) when `index` is outside `1..=values.len()`.
pub fn choose(index: i64, values: &[LookupValue]) -> LookupResult<LookupValue> {
    let zero = one_based_to_zero("CHOOSE", index, values.len())?;
    Ok(values[zero].clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LookupError;

    fn t(s: &str) -> LookupValue {
        LookupValue::Text(s.into())
    }

    #[test]
    fn choose_first_value() {
        let r = choose(1, &[t("a"), t("b"), t("c")]).unwrap();
        assert_eq!(r, t("a"));
    }

    #[test]
    fn choose_middle_value() {
        let r = choose(2, &[t("a"), t("b"), t("c")]).unwrap();
        assert_eq!(r, t("b"));
    }

    #[test]
    fn choose_last_value() {
        let r = choose(3, &[t("a"), t("b"), t("c")]).unwrap();
        assert_eq!(r, t("c"));
    }

    #[test]
    fn choose_zero_is_out_of_range() {
        let err = choose(0, &[t("a")]).unwrap_err();
        assert!(matches!(err, LookupError::OutOfRange { .. }));
    }

    #[test]
    fn choose_too_large_is_out_of_range() {
        let err = choose(10, &[t("a")]).unwrap_err();
        assert!(matches!(err, LookupError::OutOfRange { .. }));
    }

    #[test]
    fn choose_empty_values_is_out_of_range() {
        let err = choose(1, &[]).unwrap_err();
        assert!(matches!(err, LookupError::OutOfRange { .. }));
    }
}
