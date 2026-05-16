//! # Dollar fractional/decimal conversion.
//!
//! Bond-market quotes traditionally use fractional notation
//! (`52/16` = $52 and 8/16ths = $52.50). Excel's DOLLARDE and DOLLARFR
//! convert between these representations.

use super::FinancialError;

/// Excel `DOLLARDE(fractional_dollar, fraction)`. Converts a number
/// expressed as fractional dollars (e.g., "1.02" meaning 1 + 2/16)
/// into decimal form (1.125 for that example with fraction = 16).
pub fn dollarde(fractional_dollar: f64, fraction: f64) -> Result<f64, FinancialError> {
    if fraction < 1.0 {
        return Err(FinancialError::BadParameter {
            name: "fraction",
            value: fraction.to_string(),
        });
    }
    let fraction = fraction.trunc();
    // Number of digits the fraction needs in the decimal part.
    let digits = fraction.log10().ceil() as u32;
    let scale = 10_f64.powi(digits as i32);

    let int_part = fractional_dollar.trunc();
    let frac_raw = (fractional_dollar - int_part) * scale;
    Ok(int_part + frac_raw / fraction)
}

/// Excel `DOLLARFR(decimal_dollar, fraction)`. Inverse of `DOLLARDE`.
pub fn dollarfr(decimal_dollar: f64, fraction: f64) -> Result<f64, FinancialError> {
    if fraction < 1.0 {
        return Err(FinancialError::BadParameter {
            name: "fraction",
            value: fraction.to_string(),
        });
    }
    let fraction = fraction.trunc();
    let digits = fraction.log10().ceil() as u32;
    let scale = 10_f64.powi(digits as i32);

    let int_part = decimal_dollar.trunc();
    let frac_raw = (decimal_dollar - int_part) * fraction;
    Ok(int_part + frac_raw / scale)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dollarde_round_trips_with_dollarfr() {
        let decimal = 1.125;
        let fraction = 16.0;
        let fractional = dollarfr(decimal, fraction).unwrap();
        let back = dollarde(fractional, fraction).unwrap();
        assert!((back - decimal).abs() < 1e-9);
    }

    #[test]
    fn dollarde_known_value() {
        // 1.02 in 1/16 ths = 1 + 2/16 = 1.125.
        let result = dollarde(1.02, 16.0).unwrap();
        assert!((result - 1.125).abs() < 1e-9);
    }

    #[test]
    fn rejects_bad_fraction() {
        assert!(dollarde(1.0, 0.5).is_err());
        assert!(dollarfr(1.0, -1.0).is_err());
    }
}
