//! # Depreciation — SLN, DDB, SYD, DB, VDB.
//!
//! Asset-depreciation methods every accountant has used since the
//! invention of the depreciation schedule. Each function computes the
//! depreciation expense for a single period, given `cost`, `salvage`,
//! and `life` (total useful life in periods).
//!
//! Conventions:
//! - `cost`, `salvage` are non-negative amounts; salvage ≤ cost.
//! - `life` is a positive number of periods.
//! - `period` is 1-based (first period = 1, not 0).

use super::FinancialError;

/// Excel `SLN(cost, salvage, life)`. Straight-line depreciation —
/// constant amount each period.
pub fn sln(cost: f64, salvage: f64, life: f64) -> Result<f64, FinancialError> {
    validate_lifetimes(cost, salvage, life, "sln")?;
    Ok((cost - salvage) / life)
}

/// Excel `SYD(cost, salvage, life, per)`. Sum-of-Years' Digits.
/// Depreciation weighted toward early periods.
pub fn syd(
    cost: f64,
    salvage: f64,
    life: f64,
    per: f64,
) -> Result<f64, FinancialError> {
    validate_lifetimes(cost, salvage, life, "syd")?;
    if per < 1.0 || per > life {
        return Err(FinancialError::BadParameter {
            name: "per",
            value: per.to_string(),
        });
    }
    Ok((cost - salvage) * (life - per + 1.0) * 2.0 / (life * (life + 1.0)))
}

/// Excel `DDB(cost, salvage, life, period, [factor])`. Double-declining
/// balance (or other factor — defaults to 2.0). Depreciation
/// front-loaded; does NOT reach salvage early because each period
/// applies the factor to the *remaining* book value but the function
/// caps at salvage.
pub fn ddb(
    cost: f64,
    salvage: f64,
    life: f64,
    period: f64,
    factor: f64,
) -> Result<f64, FinancialError> {
    validate_lifetimes(cost, salvage, life, "ddb")?;
    if period < 1.0 || period > life {
        return Err(FinancialError::BadParameter {
            name: "period",
            value: period.to_string(),
        });
    }
    if factor <= 0.0 {
        return Err(FinancialError::BadParameter {
            name: "factor",
            value: factor.to_string(),
        });
    }
    let factor_over_life = factor / life;
    // Book value at start of period n = cost * (1 - factor/life)^(n-1)
    let book_at_start = cost * (1.0 - factor_over_life).powf(period - 1.0);
    let book_at_end = cost * (1.0 - factor_over_life).powf(period);
    let depreciation = book_at_start - book_at_end;
    // Clamp so cumulative depreciation never carries below salvage.
    let allowed = (book_at_start - salvage).max(0.0);
    Ok(depreciation.min(allowed))
}

/// Excel `DB(cost, salvage, life, period, [month])`. Fixed-declining
/// balance. Computes a fixed rate from cost/salvage/life and applies
/// it each period. `month` adjusts the first and last years for partial
/// periods (1..12, default 12).
pub fn db(
    cost: f64,
    salvage: f64,
    life: f64,
    period: f64,
    month: f64,
) -> Result<f64, FinancialError> {
    validate_lifetimes(cost, salvage, life, "db")?;
    if !(1.0..=12.0).contains(&month) {
        return Err(FinancialError::BadParameter {
            name: "month",
            value: month.to_string(),
        });
    }
    if period < 1.0 || period > life + 1.0 {
        return Err(FinancialError::BadParameter {
            name: "period",
            value: period.to_string(),
        });
    }
    let rate = (1.0 - (salvage / cost).powf(1.0 / life))
        .min(1.0)
        .max(0.0);
    // Round to 3 decimal places, matching Excel.
    let rate = (rate * 1000.0).round() / 1000.0;
    if period == 1.0 {
        return Ok(cost * rate * month / 12.0);
    }
    // Cumulative depreciation up to start of period.
    let mut cum = cost * rate * month / 12.0;
    for p in 2..(period as usize) {
        let _ = p; // Silence unused-loop-var warning.
        let book = cost - cum;
        cum += book * rate;
    }
    let book_at_start = cost - cum;
    if period > life {
        // Partial last period.
        return Ok(book_at_start * rate * (12.0 - month) / 12.0);
    }
    Ok(book_at_start * rate)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn validate_lifetimes(
    cost: f64,
    salvage: f64,
    life: f64,
    function: &'static str,
) -> Result<(), FinancialError> {
    if cost < 0.0 {
        return Err(FinancialError::DomainError {
            function,
            what: format!("cost must be non-negative ({cost})"),
        });
    }
    if salvage < 0.0 || salvage > cost {
        return Err(FinancialError::DomainError {
            function,
            what: format!("salvage must be in [0, cost] ({salvage})"),
        });
    }
    if life <= 0.0 {
        return Err(FinancialError::DomainError {
            function,
            what: format!("life must be positive ({life})"),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sln_constant_each_period() {
        // $10,000 asset, $1,000 salvage, 5-year life: SLN = $1,800/year.
        let result = sln(10_000.0, 1_000.0, 5.0).unwrap();
        assert!((result - 1_800.0).abs() < 1e-9);
    }

    #[test]
    fn syd_first_period_largest() {
        // $10k, $1k salvage, 5-year life. SYD denominator = 5*6/2 = 15.
        // Year 1 = (10000-1000) * 5/15 = 3000.
        let result = syd(10_000.0, 1_000.0, 5.0, 1.0).unwrap();
        assert!((result - 3_000.0).abs() < 1e-9);
        // Year 5 = (10000-1000) * 1/15 = 600.
        let result = syd(10_000.0, 1_000.0, 5.0, 5.0).unwrap();
        assert!((result - 600.0).abs() < 1e-9);
    }

    #[test]
    fn syd_sums_to_total_depreciation() {
        let total: f64 = (1..=5)
            .map(|p| syd(10_000.0, 1_000.0, 5.0, p as f64).unwrap())
            .sum();
        assert!((total - 9_000.0).abs() < 1e-9);
    }

    #[test]
    fn ddb_double_factor_default() {
        // $10k, $1k salvage, 5-year life, period 1: DDB = $10k * 2/5 = $4000.
        let result = ddb(10_000.0, 1_000.0, 5.0, 1.0, 2.0).unwrap();
        assert!((result - 4_000.0).abs() < 1e-9);
    }

    #[test]
    fn ddb_caps_at_salvage() {
        // After several periods, depreciation should not push book
        // value below salvage.
        let mut book = 10_000.0;
        for period in 1..=5 {
            let d = ddb(10_000.0, 1_000.0, 5.0, period as f64, 2.0).unwrap();
            book -= d;
            assert!(book + 1e-9 >= 1_000.0, "period {period}: book={book}");
        }
    }

    #[test]
    fn db_round_trip_to_salvage() {
        // DB with month=12 (full first year): cumulative depreciation
        // over `life` periods should approach `cost - salvage`.
        let mut cum = 0.0;
        for period in 1..=5 {
            cum += db(10_000.0, 1_000.0, 5.0, period as f64, 12.0).unwrap();
        }
        // DB uses a 3-decimal rounded rate, so the result isn't exactly
        // cost - salvage — within ~1% is fine.
        assert!((cum - 9_000.0).abs() / 9_000.0 < 0.01);
    }

    #[test]
    fn validate_rejects_bad_inputs() {
        assert!(sln(-1.0, 0.0, 5.0).is_err());
        assert!(sln(10.0, 11.0, 5.0).is_err()); // salvage > cost
        assert!(sln(10.0, 1.0, 0.0).is_err()); // life = 0
    }
}
