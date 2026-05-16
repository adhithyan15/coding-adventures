//! # Interest rate conversions and cumulative payments.
//!
//! EFFECT / NOMINAL for moving between effective and nominal rates,
//! CUMIPMT / CUMPRINC for total interest / principal paid over a range
//! of periods.

use super::{tvm, FinancialError, PaymentTiming};

/// Excel `EFFECT(nominal_rate, npery)`. Convert a nominal rate
/// compounded `npery` times per year to its effective annual rate.
pub fn effect(nominal_rate: f64, npery: f64) -> Result<f64, FinancialError> {
    if nominal_rate <= -1.0 {
        return Err(FinancialError::DomainError {
            function: "effect",
            what: format!("nominal_rate must be > -1 ({nominal_rate})"),
        });
    }
    if npery < 1.0 {
        return Err(FinancialError::BadParameter {
            name: "npery",
            value: npery.to_string(),
        });
    }
    Ok((1.0 + nominal_rate / npery).powf(npery) - 1.0)
}

/// Excel `NOMINAL(effect_rate, npery)`. Inverse of `EFFECT`.
pub fn nominal(effect_rate: f64, npery: f64) -> Result<f64, FinancialError> {
    if effect_rate <= -1.0 {
        return Err(FinancialError::DomainError {
            function: "nominal",
            what: format!("effect_rate must be > -1 ({effect_rate})"),
        });
    }
    if npery < 1.0 {
        return Err(FinancialError::BadParameter {
            name: "npery",
            value: npery.to_string(),
        });
    }
    Ok(((1.0 + effect_rate).powf(1.0 / npery) - 1.0) * npery)
}

/// Excel `CUMIPMT(rate, nper, pv, start_period, end_period, type)`.
/// Total interest paid between `start_period` and `end_period`
/// (inclusive, 1-based).
pub fn cumipmt(
    rate: f64,
    nper: f64,
    pv_value: f64,
    start_period: u32,
    end_period: u32,
    timing: PaymentTiming,
) -> Result<f64, FinancialError> {
    if start_period < 1 || end_period < start_period || end_period as f64 > nper {
        return Err(FinancialError::BadParameter {
            name: "period range",
            value: format!("{start_period}..={end_period}"),
        });
    }
    let mut total = 0.0;
    for p in start_period..=end_period {
        total += tvm::ipmt(rate, p as f64, nper, pv_value, 0.0, timing)?;
    }
    Ok(total)
}

/// Excel `CUMPRINC(rate, nper, pv, start_period, end_period, type)`.
pub fn cumprinc(
    rate: f64,
    nper: f64,
    pv_value: f64,
    start_period: u32,
    end_period: u32,
    timing: PaymentTiming,
) -> Result<f64, FinancialError> {
    if start_period < 1 || end_period < start_period || end_period as f64 > nper {
        return Err(FinancialError::BadParameter {
            name: "period range",
            value: format!("{start_period}..={end_period}"),
        });
    }
    let mut total = 0.0;
    for p in start_period..=end_period {
        total += tvm::ppmt(rate, p as f64, nper, pv_value, 0.0, timing)?;
    }
    Ok(total)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effect_round_trip_with_nominal() {
        // EFFECT(NOMINAL(0.05, 12), 12) ≈ 0.05.
        let nominal_rate = nominal(0.05, 12.0).unwrap();
        let back = effect(nominal_rate, 12.0).unwrap();
        assert!((back - 0.05).abs() < 1e-9);
    }

    #[test]
    fn effect_annual_compounding_is_identity() {
        // Compounded once a year, nominal == effective.
        let result = effect(0.05, 1.0).unwrap();
        assert!((result - 0.05).abs() < 1e-9);
    }

    #[test]
    fn effect_rejects_bad_inputs() {
        assert!(effect(-1.0, 12.0).is_err());
        assert!(effect(0.05, 0.5).is_err());
    }

    #[test]
    fn cumipmt_plus_cumprinc_equals_total_payments() {
        // Mortgage: 5%, 30 years, $200k. Cumulative interest + principal
        // over the first 12 months should equal -12 * PMT.
        let r = 0.05 / 12.0;
        let n = 360.0;
        let principal_amount = 200_000.0;
        let payment = tvm::pmt(r, n, principal_amount, 0.0, PaymentTiming::EndOfPeriod).unwrap();
        let interest =
            cumipmt(r, n, principal_amount, 1, 12, PaymentTiming::EndOfPeriod).unwrap();
        let principal =
            cumprinc(r, n, principal_amount, 1, 12, PaymentTiming::EndOfPeriod).unwrap();
        let total = interest + principal;
        assert!((total - 12.0 * payment).abs() < 1e-4);
    }

    #[test]
    fn cumipmt_period_range_validation() {
        let err = cumipmt(0.05, 12.0, 1000.0, 0, 5, PaymentTiming::EndOfPeriod);
        assert!(matches!(err, Err(FinancialError::BadParameter { .. })));
        let err = cumipmt(0.05, 12.0, 1000.0, 5, 4, PaymentTiming::EndOfPeriod);
        assert!(matches!(err, Err(FinancialError::BadParameter { .. })));
        let err = cumipmt(0.05, 12.0, 1000.0, 1, 13, PaymentTiming::EndOfPeriod);
        assert!(matches!(err, Err(FinancialError::BadParameter { .. })));
    }
}
