//! # Bond pricing — Phase-1 stubs.
//!
//! Excel's full bond family (PRICE, YIELD, DURATION, MDURATION,
//! ACCRINT, ACCRINTM, COUPNCD, COUPDAYBS, COUPNUM, COUPPCD, etc.)
//! requires careful day-count and coupon-schedule handling. Phase 1
//! ships the most common helpers and leaves the full schedule
//! machinery for a follow-up.

use super::{Date, DayCount, FinancialError};

/// Excel `ACCRINTM(issue, settlement, rate, par, [basis])`. Accrued
/// interest for a security that pays interest at maturity.
pub fn accrintm(
    issue: Date,
    settlement: Date,
    rate: f64,
    par: f64,
    basis: DayCount,
) -> Result<f64, FinancialError> {
    if rate <= 0.0 {
        return Err(FinancialError::BadParameter {
            name: "rate",
            value: rate.to_string(),
        });
    }
    if par <= 0.0 {
        return Err(FinancialError::BadParameter {
            name: "par",
            value: par.to_string(),
        });
    }
    let yf = datetime_core::yearfrac(issue, settlement, basis).map_err(|e| {
        FinancialError::DomainError {
            function: "accrintm",
            what: format!("yearfrac failed: {e}"),
        }
    })?;
    Ok(par * rate * yf)
}

/// Excel `DURATION(settlement, maturity, coupon, yld, frequency, [basis])`.
/// Macaulay duration — weighted average time-to-cash-flow expressed in
/// years.
pub fn duration(
    settlement: Date,
    maturity: Date,
    coupon: f64,
    yld: f64,
    frequency: u32,
    basis: DayCount,
) -> Result<f64, FinancialError> {
    if coupon < 0.0 {
        return Err(FinancialError::BadParameter {
            name: "coupon",
            value: coupon.to_string(),
        });
    }
    if yld < 0.0 {
        return Err(FinancialError::BadParameter {
            name: "yld",
            value: yld.to_string(),
        });
    }
    if !matches!(frequency, 1 | 2 | 4) {
        return Err(FinancialError::BadParameter {
            name: "frequency",
            value: frequency.to_string(),
        });
    }

    let years = datetime_core::yearfrac(settlement, maturity, basis).map_err(|e| {
        FinancialError::DomainError {
            function: "duration",
            what: format!("yearfrac failed: {e}"),
        }
    })?;
    let f = frequency as f64;
    let n_payments = (years * f).ceil();
    let r = yld / f;
    let c = coupon / f;

    let mut weighted_sum = 0.0;
    let mut price = 0.0;
    for k in 1..=(n_payments as u32) {
        let t = k as f64 / f;
        let cf = if k as f64 == n_payments { c + 1.0 } else { c };
        let pv = cf / (1.0 + r).powf(k as f64);
        price += pv;
        weighted_sum += t * pv;
    }
    if price <= 0.0 {
        return Err(FinancialError::DomainError {
            function: "duration",
            what: "price degenerated to zero or negative".into(),
        });
    }
    Ok(weighted_sum / price)
}

/// Excel `MDURATION(...)`. Modified duration = `DURATION / (1 + yld/frequency)`.
pub fn mduration(
    settlement: Date,
    maturity: Date,
    coupon: f64,
    yld: f64,
    frequency: u32,
    basis: DayCount,
) -> Result<f64, FinancialError> {
    let dur = duration(settlement, maturity, coupon, yld, frequency, basis)?;
    Ok(dur / (1.0 + yld / frequency as f64))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accrintm_simple_actual_365() {
        let issue = Date::from_ymd(2024, 1, 1).unwrap();
        let settlement = Date::from_ymd(2024, 7, 1).unwrap();
        // 6 months out of 366 (leap year) at 5% on $1000 par.
        let interest = accrintm(issue, settlement, 0.05, 1000.0, DayCount::Actual365)
            .unwrap();
        // Roughly 182 days / 365 * 5% * 1000 ≈ 24.93
        assert!((interest - 24.93).abs() < 0.05);
    }

    #[test]
    fn accrintm_rejects_bad_inputs() {
        let d = Date::from_ymd(2024, 1, 1).unwrap();
        assert!(accrintm(d, d, -0.01, 1000.0, DayCount::Actual365).is_err());
        assert!(accrintm(d, d, 0.05, -1000.0, DayCount::Actual365).is_err());
    }

    #[test]
    fn duration_below_term() {
        // 5-year bond should have duration ≤ 5.
        let settlement = Date::from_ymd(2024, 1, 1).unwrap();
        let maturity = Date::from_ymd(2029, 1, 1).unwrap();
        let dur = duration(settlement, maturity, 0.05, 0.06, 2, DayCount::Us30360)
            .unwrap();
        assert!(dur > 0.0 && dur < 5.0, "duration={dur}");
    }

    #[test]
    fn mduration_less_than_duration() {
        let settlement = Date::from_ymd(2024, 1, 1).unwrap();
        let maturity = Date::from_ymd(2029, 1, 1).unwrap();
        let dur = duration(settlement, maturity, 0.05, 0.06, 2, DayCount::Us30360)
            .unwrap();
        let mdur = mduration(settlement, maturity, 0.05, 0.06, 2, DayCount::Us30360)
            .unwrap();
        assert!(mdur < dur);
        assert!((mdur - dur / (1.0 + 0.06 / 2.0)).abs() < 1e-9);
    }

    #[test]
    fn duration_rejects_bad_frequency() {
        let d = Date::from_ymd(2024, 1, 1).unwrap();
        let e = Date::from_ymd(2029, 1, 1).unwrap();
        assert!(duration(d, e, 0.05, 0.06, 3, DayCount::Us30360).is_err());
    }
}
