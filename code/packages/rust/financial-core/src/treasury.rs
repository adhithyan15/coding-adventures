//! # US Treasury bill helpers — TBILLEQ / TBILLPRICE / TBILLYIELD.
//!
//! T-bills are discount instruments — quoted at a discount from par.
//! These three functions move between the equivalent representations
//! (discount, yield, price) using the bond-market conventions Excel
//! follows.

use super::{Date, FinancialError};

/// Excel `TBILLEQ(settlement, maturity, discount)`. Returns the
/// bond-equivalent yield for a Treasury bill given its discount rate.
pub fn tbilleq(
    settlement: Date,
    maturity: Date,
    discount: f64,
) -> Result<f64, FinancialError> {
    if discount <= 0.0 || discount >= 1.0 {
        return Err(FinancialError::BadParameter {
            name: "discount",
            value: discount.to_string(),
        });
    }
    let dsm = settlement.days_until(maturity);
    if dsm <= 0 {
        return Err(FinancialError::DomainError {
            function: "tbilleq",
            what: "maturity must be after settlement".into(),
        });
    }
    if dsm > 365 {
        return Err(FinancialError::DomainError {
            function: "tbilleq",
            what: "maturity more than 1 year after settlement".into(),
        });
    }
    Ok((365.0 * discount) / (360.0 - discount * dsm as f64))
}

/// Excel `TBILLPRICE(settlement, maturity, discount)`. Returns the
/// price per $100 face value.
pub fn tbillprice(
    settlement: Date,
    maturity: Date,
    discount: f64,
) -> Result<f64, FinancialError> {
    if discount <= 0.0 || discount >= 1.0 {
        return Err(FinancialError::BadParameter {
            name: "discount",
            value: discount.to_string(),
        });
    }
    let dsm = settlement.days_until(maturity);
    if dsm <= 0 {
        return Err(FinancialError::DomainError {
            function: "tbillprice",
            what: "maturity must be after settlement".into(),
        });
    }
    Ok(100.0 * (1.0 - discount * dsm as f64 / 360.0))
}

/// Excel `TBILLYIELD(settlement, maturity, pr)`. Returns the yield
/// of a Treasury bill given its price.
pub fn tbillyield(
    settlement: Date,
    maturity: Date,
    pr: f64,
) -> Result<f64, FinancialError> {
    if pr <= 0.0 {
        return Err(FinancialError::BadParameter {
            name: "pr",
            value: pr.to_string(),
        });
    }
    let dsm = settlement.days_until(maturity);
    if dsm <= 0 {
        return Err(FinancialError::DomainError {
            function: "tbillyield",
            what: "maturity must be after settlement".into(),
        });
    }
    Ok(((100.0 - pr) / pr) * (360.0 / dsm as f64))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tbillprice_then_tbillyield_round_trips() {
        let settlement = Date::from_ymd(2024, 1, 1).unwrap();
        let maturity = Date::from_ymd(2024, 7, 1).unwrap();
        let price = tbillprice(settlement, maturity, 0.05).unwrap();
        let yld = tbillyield(settlement, maturity, price).unwrap();
        // Recompute discount from yield: discount = yld / (1 + yld * dsm/360)
        // — derived from inverting both formulas; let's just check the
        // resulting price re-roundtrips.
        let dsm = settlement.days_until(maturity) as f64;
        let implied_discount = yld / (1.0 + yld * dsm / 360.0);
        let recomputed = tbillprice(settlement, maturity, implied_discount).unwrap();
        assert!((recomputed - price).abs() < 1e-6);
    }

    #[test]
    fn tbilleq_known_value() {
        let settlement = Date::from_ymd(2024, 1, 1).unwrap();
        let maturity = Date::from_ymd(2024, 7, 1).unwrap();
        let eq = tbilleq(settlement, maturity, 0.05).unwrap();
        // BEY = 365*0.05 / (360 - 0.05*dsm); for ~182 days it's ~0.0521.
        assert!(eq > 0.05 && eq < 0.06);
    }

    #[test]
    fn rejects_bad_inputs() {
        let s = Date::from_ymd(2024, 1, 1).unwrap();
        let m = Date::from_ymd(2024, 7, 1).unwrap();
        // Negative discount.
        assert!(tbillprice(s, m, -0.05).is_err());
        // Maturity before settlement.
        assert!(tbillprice(m, s, 0.05).is_err());
        // Out-of-range tbilleq.
        let far_maturity = Date::from_ymd(2026, 1, 1).unwrap();
        assert!(tbilleq(s, far_maturity, 0.05).is_err());
    }
}
