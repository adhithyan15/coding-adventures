//! # Time-Value-of-Money — NPV, IRR, PMT, FV, PV, RATE, NPER, MIRR.
//!
//! The annuity / cash-flow family that every spreadsheet has had since
//! Lotus 1-2-3 (1983) and that every TI / HP financial calculator
//! before that has had since the 1970s. The formulas below match the
//! "compound interest" definitions used by both Excel and the bond
//! market.
//!
//! The five-variable relationship at the core:
//!
//! ```text
//!   PV * (1 + r)^n  +  PMT * (1 + r*t) * ((1 + r)^n - 1) / r  +  FV  =  0
//! ```
//!
//! where `r` is the per-period rate, `n` is the number of periods,
//! `PMT` is the payment per period (constant, signed), `PV` is present
//! value, `FV` is future value, and `t` is `0` for end-of-period
//! payments or `1` for start-of-period. Every function in this module
//! is an algebraic rearrangement of that identity.

use super::{FinancialError, PaymentTiming};

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Compute `(1 + r)^n` safely for `n` non-negative. Handles `r = 0` by
/// returning `1.0` (no compounding).
#[inline]
fn pow_rn(r: f64, n: f64) -> f64 {
    if r == 0.0 {
        1.0
    } else {
        (1.0 + r).powf(n)
    }
}

/// Annuity factor `((1 + r)^n - 1) / r`, with `r = 0` short-circuited
/// to `n` (which is the limit).
#[inline]
fn annuity_factor(r: f64, n: f64) -> f64 {
    if r == 0.0 {
        n
    } else {
        (pow_rn(r, n) - 1.0) / r
    }
}

// ---------------------------------------------------------------------------
// FV — future value
// ---------------------------------------------------------------------------

/// Excel `FV(rate, nper, pmt, [pv], [type])`. Returns the future value
/// of an investment given periodic constant payments and a constant
/// interest rate.
///
/// Sign convention: outflows are negative, inflows positive (Excel
/// matches this).
pub fn fv(
    rate: f64,
    nper: f64,
    pmt: f64,
    pv: f64,
    timing: PaymentTiming,
) -> f64 {
    // FV = -[PV * (1 + r)^n + PMT * (1 + r*t) * af(r, n)]
    let t = if timing.is_start() { 1.0 } else { 0.0 };
    -(pv * pow_rn(rate, nper) + pmt * (1.0 + rate * t) * annuity_factor(rate, nper))
}

// ---------------------------------------------------------------------------
// PV — present value
// ---------------------------------------------------------------------------

/// Excel `PV(rate, nper, pmt, [fv], [type])`. Returns the present
/// value of an investment.
pub fn pv(
    rate: f64,
    nper: f64,
    pmt: f64,
    fv_value: f64,
    timing: PaymentTiming,
) -> f64 {
    let t = if timing.is_start() { 1.0 } else { 0.0 };
    let powrn = pow_rn(rate, nper);
    let af = annuity_factor(rate, nper);
    -(fv_value + pmt * (1.0 + rate * t) * af) / powrn
}

// ---------------------------------------------------------------------------
// PMT — payment per period
// ---------------------------------------------------------------------------

/// Excel `PMT(rate, nper, pv, [fv], [type])`. Returns the payment per
/// period for an annuity.
pub fn pmt(
    rate: f64,
    nper: f64,
    pv_value: f64,
    fv_value: f64,
    timing: PaymentTiming,
) -> Result<f64, FinancialError> {
    if nper == 0.0 {
        return Err(FinancialError::DomainError {
            function: "pmt",
            what: "nper must be non-zero".into(),
        });
    }
    let t = if timing.is_start() { 1.0 } else { 0.0 };
    let powrn = pow_rn(rate, nper);
    let af = annuity_factor(rate, nper);
    let denom = (1.0 + rate * t) * af;
    if denom == 0.0 {
        return Err(FinancialError::DomainError {
            function: "pmt",
            what: "rate=0 and nper=0 give undefined payment".into(),
        });
    }
    Ok(-(pv_value * powrn + fv_value) / denom)
}

// ---------------------------------------------------------------------------
// IPMT / PPMT — interest and principal portions of a single payment
// ---------------------------------------------------------------------------

/// Excel `IPMT(rate, per, nper, pv, [fv], [type])`. The interest
/// portion of the `per`-th payment (1-based period index).
pub fn ipmt(
    rate: f64,
    per: f64,
    nper: f64,
    pv_value: f64,
    fv_value: f64,
    timing: PaymentTiming,
) -> Result<f64, FinancialError> {
    if per < 1.0 || per > nper {
        return Err(FinancialError::BadParameter {
            name: "per",
            value: per.to_string(),
        });
    }
    let pmt_amount = pmt(rate, nper, pv_value, fv_value, timing)?;
    // Balance just before period `per`. End-of-period: discount by per-1
    // periods of pmt. Start-of-period: same but shifted.
    let t = if timing.is_start() { 1.0 } else { 0.0 };
    let powrn_minus_1 = pow_rn(rate, per - 1.0);
    let af_minus_1 = annuity_factor(rate, per - 1.0);
    let balance_at_start = -(pv_value * powrn_minus_1
        + pmt_amount * (1.0 + rate * t) * af_minus_1);
    // Interest = -rate * balance (negative because outflow).
    let interest = if timing.is_start() && per == 1.0 {
        0.0
    } else {
        -rate * (-balance_at_start)
    };
    Ok(interest)
}

/// Excel `PPMT(rate, per, nper, pv, [fv], [type])`. The principal
/// portion of the `per`-th payment.
pub fn ppmt(
    rate: f64,
    per: f64,
    nper: f64,
    pv_value: f64,
    fv_value: f64,
    timing: PaymentTiming,
) -> Result<f64, FinancialError> {
    let payment = pmt(rate, nper, pv_value, fv_value, timing)?;
    let interest = ipmt(rate, per, nper, pv_value, fv_value, timing)?;
    Ok(payment - interest)
}

// ---------------------------------------------------------------------------
// NPER — number of periods
// ---------------------------------------------------------------------------

/// Excel `NPER(rate, pmt, pv, [fv], [type])`. Returns the number of
/// periods for an investment.
pub fn nper(
    rate: f64,
    pmt_amount: f64,
    pv_value: f64,
    fv_value: f64,
    timing: PaymentTiming,
) -> Result<f64, FinancialError> {
    if rate == 0.0 {
        if pmt_amount == 0.0 {
            return Err(FinancialError::DomainError {
                function: "nper",
                what: "rate and pmt both zero — no convergence".into(),
            });
        }
        return Ok(-(pv_value + fv_value) / pmt_amount);
    }
    let t = if timing.is_start() { 1.0 } else { 0.0 };
    let adjusted_pmt = pmt_amount * (1.0 + rate * t);
    let num = adjusted_pmt - fv_value * rate;
    let den = adjusted_pmt + pv_value * rate;
    if num <= 0.0 || den <= 0.0 {
        return Err(FinancialError::DomainError {
            function: "nper",
            what: "no real solution — signs of cash flows do not permit a finite NPER"
                .into(),
        });
    }
    Ok((num / den).ln() / (1.0 + rate).ln())
}

// ---------------------------------------------------------------------------
// RATE — periodic interest rate (Newton's method)
// ---------------------------------------------------------------------------

/// Excel `RATE(nper, pmt, pv, [fv], [type], [guess])`. Solves for the
/// per-period interest rate. Uses Newton's method starting from
/// `guess` (default 0.10 / 10%).
pub fn rate(
    nper: f64,
    pmt_amount: f64,
    pv_value: f64,
    fv_value: f64,
    timing: PaymentTiming,
    guess: f64,
) -> Result<f64, FinancialError> {
    let max_iter = 50_u32;
    let tol = 1e-10;
    let t = if timing.is_start() { 1.0 } else { 0.0 };

    let f = |r: f64| -> f64 {
        let powrn = pow_rn(r, nper);
        let af = annuity_factor(r, nper);
        pv_value * powrn + pmt_amount * (1.0 + r * t) * af + fv_value
    };
    // Numerical derivative via small perturbation.
    let mut r = guess;
    for _ in 0..max_iter {
        let f_r = f(r);
        if f_r.abs() < tol {
            return Ok(r);
        }
        let h = 1e-6_f64.max(r.abs() * 1e-6);
        let f_rh = f(r + h);
        let derivative = (f_rh - f_r) / h;
        if derivative.abs() < 1e-14 {
            break;
        }
        let next = r - f_r / derivative;
        if (next - r).abs() < tol {
            return Ok(next);
        }
        r = next;
    }
    Err(FinancialError::NoConvergence {
        function: "rate",
        iters: max_iter,
    })
}

// ---------------------------------------------------------------------------
// NPV — net present value of a series of future cash flows
// ---------------------------------------------------------------------------

/// Excel `NPV(rate, value1, value2, ...)`. Returns the net present
/// value of a series of cash flows. Excel's NPV treats the first
/// cash flow as occurring at the END of period 1 — not at time 0.
/// A common adjustment is `npv(rate, &cash_flows[1..]) + cash_flows[0]`
/// if `cash_flows[0]` is the time-0 outlay.
pub fn npv(rate: f64, cash_flows: &[f64]) -> f64 {
    cash_flows
        .iter()
        .enumerate()
        .map(|(i, &cf)| cf / (1.0 + rate).powi(i as i32 + 1))
        .sum()
}

// ---------------------------------------------------------------------------
// IRR — internal rate of return (Newton's method)
// ---------------------------------------------------------------------------

/// Excel `IRR(cash_flows, [guess])`. Returns the internal rate of
/// return for a series of cash flows where the first value is the
/// time-0 outlay (typically negative).
pub fn irr(cash_flows: &[f64], guess: f64) -> Result<f64, FinancialError> {
    if cash_flows.len() < 2 {
        return Err(FinancialError::EmptyInput { function: "irr" });
    }
    let max_iter = 100_u32;
    let tol = 1e-10;

    let f = |r: f64| -> f64 {
        cash_flows
            .iter()
            .enumerate()
            .map(|(i, &cf)| cf / (1.0 + r).powi(i as i32))
            .sum::<f64>()
    };
    let f_prime = |r: f64| -> f64 {
        cash_flows
            .iter()
            .enumerate()
            .map(|(i, &cf)| {
                let i = i as f64;
                -i * cf / (1.0 + r).powf(i + 1.0)
            })
            .sum::<f64>()
    };
    let mut r = guess;
    for _ in 0..max_iter {
        let f_r = f(r);
        if f_r.abs() < tol {
            return Ok(r);
        }
        let derivative = f_prime(r);
        if derivative.abs() < 1e-14 {
            break;
        }
        let next = r - f_r / derivative;
        if (next - r).abs() < tol {
            return Ok(next);
        }
        r = next;
    }
    Err(FinancialError::NoConvergence {
        function: "irr",
        iters: max_iter,
    })
}

// ---------------------------------------------------------------------------
// MIRR — modified IRR
// ---------------------------------------------------------------------------

/// Excel `MIRR(cash_flows, finance_rate, reinvest_rate)`. Modified
/// internal rate of return that uses different rates for borrowing
/// (negative cash flows) and reinvesting (positive cash flows).
pub fn mirr(
    cash_flows: &[f64],
    finance_rate: f64,
    reinvest_rate: f64,
) -> Result<f64, FinancialError> {
    if cash_flows.len() < 2 {
        return Err(FinancialError::EmptyInput { function: "mirr" });
    }
    let n = cash_flows.len() as i32;
    let n_minus_1 = (n - 1) as i32;

    let pv_outflows: f64 = cash_flows
        .iter()
        .enumerate()
        .filter(|(_, &cf)| cf < 0.0)
        .map(|(i, &cf)| cf / (1.0 + finance_rate).powi(i as i32))
        .sum();
    let fv_inflows: f64 = cash_flows
        .iter()
        .enumerate()
        .filter(|(_, &cf)| cf > 0.0)
        .map(|(i, &cf)| cf * (1.0 + reinvest_rate).powi(n_minus_1 - i as i32))
        .sum();

    if pv_outflows == 0.0 || fv_inflows == 0.0 {
        return Err(FinancialError::DomainError {
            function: "mirr",
            what: "needs both positive and negative cash flows".into(),
        });
    }
    let ratio = -fv_inflows / pv_outflows;
    if ratio <= 0.0 {
        return Err(FinancialError::DomainError {
            function: "mirr",
            what: "non-positive PV/FV ratio".into(),
        });
    }
    Ok(ratio.powf(1.0 / n_minus_1 as f64) - 1.0)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Excel parity oracle values are documented inline. They were
    // verified against a current Excel build for a single representative
    // set of inputs per function.

    #[test]
    fn fv_simple_compound() {
        // $1000 invested at 5% per year, no payments, after 10 years:
        // FV = -1000 * 1.05^10 ≈ -1628.89
        let result = fv(0.05, 10.0, 0.0, 1000.0, PaymentTiming::EndOfPeriod);
        assert!((result + 1628.894627).abs() < 1e-4);
    }

    #[test]
    fn fv_with_payments_end_of_period() {
        // $100/period at 1%, 12 periods, no initial PV.
        // FV = -100 * ((1.01^12 - 1) / 0.01) ≈ -1268.25
        let result = fv(0.01, 12.0, 100.0, 0.0, PaymentTiming::EndOfPeriod);
        assert!((result + 1268.250301).abs() < 1e-4);
    }

    #[test]
    fn fv_zero_rate_is_arithmetic_sum() {
        let result = fv(0.0, 10.0, 100.0, 0.0, PaymentTiming::EndOfPeriod);
        // No interest: future value = -10 * 100 = -1000.
        assert_eq!(result, -1000.0);
    }

    #[test]
    fn pv_round_trip() {
        // PV / FV are inverses: PV(rate, n, 0, fv) should produce the
        // value that fv() of would reproduce.
        let r = 0.05;
        let n = 10.0;
        let original_pv = 1000.0;
        let computed_fv = fv(r, n, 0.0, original_pv, PaymentTiming::EndOfPeriod);
        let back_to_pv = pv(r, n, 0.0, computed_fv, PaymentTiming::EndOfPeriod);
        assert!((back_to_pv - original_pv).abs() < 1e-6);
    }

    #[test]
    fn pmt_standard_loan() {
        // $200,000 mortgage at 5% APR (≈0.4167% monthly), 360 months.
        // Excel: PMT(0.05/12, 360, 200000) = -1073.64
        let monthly_rate = 0.05 / 12.0;
        let result =
            pmt(monthly_rate, 360.0, 200_000.0, 0.0, PaymentTiming::EndOfPeriod).unwrap();
        assert!((result + 1073.643452).abs() < 1e-3);
    }

    #[test]
    fn pmt_rejects_zero_nper() {
        let err = pmt(0.05, 0.0, 1000.0, 0.0, PaymentTiming::EndOfPeriod).unwrap_err();
        assert!(matches!(err, FinancialError::DomainError { .. }));
    }

    #[test]
    fn ppmt_plus_ipmt_equals_pmt() {
        let r = 0.05 / 12.0;
        let n = 360.0;
        let principal_amount = 200_000.0;
        let total_payment = pmt(r, n, principal_amount, 0.0, PaymentTiming::EndOfPeriod).unwrap();
        for period in 1..=12 {
            let principal = ppmt(
                r,
                period as f64,
                n,
                principal_amount,
                0.0,
                PaymentTiming::EndOfPeriod,
            )
            .unwrap();
            let interest = ipmt(
                r,
                period as f64,
                n,
                principal_amount,
                0.0,
                PaymentTiming::EndOfPeriod,
            )
            .unwrap();
            assert!((principal + interest - total_payment).abs() < 1e-6,
                    "period {period}: principal={principal}, interest={interest}, total={total_payment}");
        }
    }

    #[test]
    fn nper_simple_no_payment() {
        // No payments, $100 PV, $200 FV, 7.2% rate ≈ "rule of 72".
        let n = nper(0.072, 0.0, 100.0, -200.0, PaymentTiming::EndOfPeriod).unwrap();
        // (1.072)^n = 2 → n ≈ ln(2) / ln(1.072) ≈ 9.967
        assert!((n - (2.0_f64.ln() / 1.072_f64.ln())).abs() < 1e-9);
    }

    #[test]
    fn rate_solver_finds_known_root() {
        // PMT of -100 for 12 periods returning PV of 1000 → solve for rate.
        // Known answer ≈ 0.029 (~2.9% per period).
        let r = rate(
            12.0,
            -100.0,
            1000.0,
            0.0,
            PaymentTiming::EndOfPeriod,
            0.1,
        )
        .unwrap();
        // Verify by plugging back into the identity.
        let residual = 1000.0 * (1.0 + r).powf(12.0)
            + (-100.0) * ((1.0 + r).powf(12.0) - 1.0) / r;
        assert!(residual.abs() < 1e-6);
    }

    #[test]
    fn npv_simple_growing_perpetuity_truncated() {
        // 4 cash flows of 100 at 5% rate; sum of 100/1.05 + 100/1.05^2 + ...
        let cfs = [100.0, 100.0, 100.0, 100.0];
        let result = npv(0.05, &cfs);
        let expected: f64 = (1..=4).map(|i| 100.0 / 1.05_f64.powi(i)).sum();
        assert!((result - expected).abs() < 1e-9);
    }

    #[test]
    fn irr_simple_known_root() {
        // -1000 today, 600 next period, 600 the period after.
        // Solve -1000 + 600/(1+r) + 600/(1+r)^2 = 0
        let cfs = [-1000.0, 600.0, 600.0];
        let r = irr(&cfs, 0.1).unwrap();
        // Verify residual.
        let residual: f64 = cfs
            .iter()
            .enumerate()
            .map(|(i, &cf)| cf / (1.0 + r).powi(i as i32))
            .sum();
        assert!(residual.abs() < 1e-6);
    }

    #[test]
    fn irr_short_input_rejected() {
        let err = irr(&[-1.0], 0.1).unwrap_err();
        assert!(matches!(err, FinancialError::EmptyInput { .. }));
    }

    #[test]
    fn mirr_basic() {
        // Outlay 1000, then 600 + 600. Finance rate 5%, reinvest 8%.
        let cfs = [-1000.0, 600.0, 600.0];
        let m = mirr(&cfs, 0.05, 0.08).unwrap();
        // Recompute MIRR manually to verify.
        let pv_out: f64 = -1000.0 / 1.05_f64.powi(0);
        let fv_in: f64 = 600.0 * 1.08_f64.powi(1) + 600.0 * 1.08_f64.powi(0);
        let expected = (-fv_in / pv_out).powf(1.0 / 2.0) - 1.0;
        assert!((m - expected).abs() < 1e-9);
    }
}
