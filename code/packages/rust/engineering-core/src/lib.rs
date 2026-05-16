//! # engineering-core — Excel/Lotus engineering functions.
//!
//! Layer-1 Rust crate covering the "engineering" function family from
//! Excel and Lotus 1-2-3: base conversions (BIN2DEC, DEC2HEX, etc.),
//! bitwise operations on 48-bit unsigned integers (BITAND/OR/XOR/LSHIFT/
//! RSHIFT), complex-number arithmetic (the IM* family — IMABS, IMSUM,
//! IMPRODUCT, IMEXP, IMLN, etc.), the Kronecker delta (DELTA / GESTEP),
//! and the error function (ERF / ERFC).
//!
//! Bessel functions (BESSELJ, BESSELY, BESSELI, BESSELK) and the full
//! CONVERT unit table are deferred to Phase 2 — both need substantially
//! more implementation than fits cleanly in a single PR.
//!
//! ## Portability bar
//!
//! Per `backend-crate-catalog.md` §1: `forbid(unsafe_code)`, no
//! `#[cfg(target_os)]`, no I/O, no globals, WASM-friendly.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod base;
pub mod bitwise;
pub mod complex;
pub mod delta;
pub mod erf;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors raised by engineering functions.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum EngineeringError {
    /// Input is outside the function's domain (e.g., complex parse
    /// fail, ERF given NaN).
    DomainError {
        /// Function name.
        function: &'static str,
        /// Description of the violation.
        what: String,
    },
    /// Input is in range but exceeds Excel's documented limits
    /// (e.g., BIN2DEC of >10 chars).
    OutOfRange {
        /// Function name.
        function: &'static str,
        /// What exceeded what.
        what: String,
    },
    /// A string could not be parsed (e.g., complex with garbage).
    ParseError {
        /// Function attempting the parse.
        function: &'static str,
        /// The input that failed.
        input: String,
    },
    /// A parameter name/value pairing is invalid.
    BadParameter {
        /// Parameter name.
        name: &'static str,
        /// Value, stringified.
        value: String,
    },
}

impl core::fmt::Display for EngineeringError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            EngineeringError::DomainError { function, what } => {
                write!(f, "{function}: {what}")
            }
            EngineeringError::OutOfRange { function, what } => {
                write!(f, "{function}: out of range — {what}")
            }
            EngineeringError::ParseError { function, input } => {
                write!(f, "{function}: cannot parse '{input}'")
            }
            EngineeringError::BadParameter { name, value } => {
                write!(f, "bad parameter {name}={value}")
            }
        }
    }
}

impl std::error::Error for EngineeringError {}

// ---------------------------------------------------------------------------
// Tests at crate root
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let e = EngineeringError::OutOfRange {
            function: "bin2dec",
            what: "input too long".into(),
        };
        assert!(format!("{}", e).contains("input too long"));
    }
}
