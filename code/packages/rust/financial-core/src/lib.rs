//! # financial-core — Excel/Lotus financial functions.
//!
//! Layer-1 Rust crate that implements the financial-function family
//! every spreadsheet eventually needs: time-value-of-money (NPV, IRR,
//! PMT, FV, PV, RATE, NPER), depreciation (SLN, DDB, SYD, DB, VDB),
//! and a handful of bond / treasury helpers (PRICE, YIELD, ACCRINT,
//! TBILLEQ, TBILLPRICE, TBILLYIELD, DOLLARDE, DOLLARFR, EFFECT,
//! NOMINAL).
//!
//! The crate has no opinion about a *spreadsheet*. Every function is
//! a plain Rust API. Frontend dispatchers translate Excel/Lotus/R names
//! to these signatures elsewhere.
//!
//! ## Conventions
//!
//! - Cash-flow sign convention matches Excel: outflows are negative,
//!   inflows positive. `PV(rate, nper, pmt)` returns a negative value
//!   when `pmt` is positive (you have to fund the future payments).
//! - Periods are integers where the function semantics demand it
//!   (NPER is f64 because it can return a fractional answer; PMT
//!   integer-period count is f64 for the same reason).
//! - Day-count conventions come from [`datetime_core::DayCount`]
//!   when bond functions need them.
//!
//! ## Portability bar
//!
//! Per `backend-crate-catalog.md` §1: `forbid(unsafe_code)`, no
//! `#[cfg(target_os)]`, no I/O, no globals, WASM-friendly via
//! `wall-clock` with `default-features = false`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod tvm;
pub mod depreciation;
pub mod interest;
pub mod conversion;
pub mod bond;
pub mod treasury;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors raised by financial functions.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum FinancialError {
    /// IRR / RATE / similar iterative solver failed to converge within
    /// the iteration budget.
    NoConvergence {
        /// Function that didn't converge.
        function: &'static str,
        /// How many iterations were exhausted.
        iters: u32,
    },
    /// A function received a domain-invalid argument.
    DomainError {
        /// Function name.
        function: &'static str,
        /// Description of the violation.
        what: String,
    },
    /// A parameter name/value pairing is invalid.
    BadParameter {
        /// Parameter name.
        name: &'static str,
        /// Value, stringified.
        value: String,
    },
    /// An aggregate input was empty when the function requires content.
    EmptyInput {
        /// Function name.
        function: &'static str,
    },
}

impl core::fmt::Display for FinancialError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            FinancialError::NoConvergence { function, iters } => {
                write!(f, "{function}: did not converge after {iters} iterations")
            }
            FinancialError::DomainError { function, what } => {
                write!(f, "{function}: {what}")
            }
            FinancialError::BadParameter { name, value } => {
                write!(f, "bad parameter {name}={value}")
            }
            FinancialError::EmptyInput { function } => {
                write!(f, "{function}: empty input")
            }
        }
    }
}

impl std::error::Error for FinancialError {}

// ---------------------------------------------------------------------------
// Common helpers
// ---------------------------------------------------------------------------

/// Whether a payment is due at the start of the period (`true`) or end
/// (`false`). Excel's `type` parameter; we use a bool so callers can't
/// accidentally swap 0 and 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaymentTiming {
    /// Payment at the end of the period (Excel `type = 0`).
    EndOfPeriod,
    /// Payment at the start of the period (Excel `type = 1`).
    StartOfPeriod,
}

impl PaymentTiming {
    /// Construct from Excel's integer `type` parameter.
    pub fn from_excel(type_value: u8) -> Result<Self, FinancialError> {
        match type_value {
            0 => Ok(PaymentTiming::EndOfPeriod),
            1 => Ok(PaymentTiming::StartOfPeriod),
            other => Err(FinancialError::BadParameter {
                name: "type",
                value: other.to_string(),
            }),
        }
    }

    /// True if payments are made at the start of the period.
    pub fn is_start(self) -> bool {
        matches!(self, PaymentTiming::StartOfPeriod)
    }
}

/// Re-export of the most useful types from datetime-core, so financial
/// functions that take dates don't force the caller to also import
/// datetime-core directly.
pub use datetime_core::{Date, DayCount};

// ---------------------------------------------------------------------------
// Tests at the crate root
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payment_timing_from_excel() {
        assert_eq!(
            PaymentTiming::from_excel(0).unwrap(),
            PaymentTiming::EndOfPeriod
        );
        assert_eq!(
            PaymentTiming::from_excel(1).unwrap(),
            PaymentTiming::StartOfPeriod
        );
        assert!(matches!(
            PaymentTiming::from_excel(2),
            Err(FinancialError::BadParameter { .. })
        ));
    }

    #[test]
    fn financial_error_display_round_trip() {
        let err = FinancialError::NoConvergence {
            function: "irr",
            iters: 100,
        };
        assert!(format!("{}", err).contains("100"));
        let err = FinancialError::DomainError {
            function: "rate",
            what: "negative cash flows".into(),
        };
        assert!(format!("{}", err).contains("negative cash flows"));
    }
}
