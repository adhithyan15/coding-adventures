//! Math Core
//!
//! Phase 1 implementation for the Layer-1 `math-core` crate from
//! `code/specs/backend-crate-catalog.md`. This crate exposes Excel/Lotus/R
//! math functions: arithmetic, power/log, trig, modular, combinatorics,
//! angle conversion, aggregate, and named constants.
//!
//! ## Portability
//!
//! Pure Rust, no `unsafe`, no platform-specific code, no file I/O, no clock
//! reads, no globals — WASM-compatible per the catalog spec.
//!
//! ## Numeric model
//!
//! Scalars are `f64` (matching Excel and the R `double` rung in
//! `numeric-tower`). Vector inputs use `r_vector::Double`. NA propagates
//! element-wise: any NA input position produces an NA output position,
//! detected and produced via `r_vector::is_na_real` / `r_vector::na_real`.
//!
//! Fallible scalar functions return `Result<Number, MathError>`. Vector
//! functions return `Double` and encode any per-element failure as a NaN or
//! NA depending on the function's semantics (documented per-function).
//!
//! ## Excel vs R parity
//!
//! Where Excel and R diverge we match Excel and document the choice in the
//! function's doc comment. The most consequential divergences are:
//!
//! * `LOG(x)` in Excel is base-10; in R `log(x)` is natural. We provide both
//!   as separate functions (`log10`, `ln`) and a 2-arg `log_base(x, base)`.
//! * `MOD(-3, 2)` is `1` in both Excel and R (sign follows the divisor).
//! * `POWER(0, 0)` is `#NUM!` in Excel — we return `MathError::DomainError`.
//! * `FACT(171)` overflows `f64` — we return `MathError::Overflow`.

pub mod aggregate;
pub mod arithmetic;
pub mod combinatorics;
pub mod constants;
pub mod conversion;
pub mod modular;
pub mod power_log;
pub mod trig;

pub use numeric_tower::Number;
pub use r_vector::{is_na_real, na_real, Double, Vector};

/// Errors raised by `math-core` scalar functions.
///
/// Modeled on `statistics-core::StatsError`. Domain errors carry both the
/// function name and a human-readable description of which precondition was
/// violated; this lets a frontend translate to the right spreadsheet error
/// (e.g. Excel `#NUM!`, `#DIV/0!`, `#VALUE!`).
#[derive(Debug, Clone, PartialEq)]
pub enum MathError {
    /// The argument is outside the function's mathematical domain
    /// (e.g. `SQRT(-1)`, `LN(0)`, `ACOS(2)`, `FACT(-1)`).
    DomainError {
        function: &'static str,
        what: String,
    },
    /// The result is not representable in `f64` (e.g. `FACT(171)`).
    Overflow { function: &'static str },
    /// Division by zero on a function whose Excel mapping is `#DIV/0!`.
    DivisionByZero { function: &'static str },
    /// A named parameter received an out-of-range or nonsensical value.
    BadParameter {
        name: &'static str,
        value: String,
    },
}

impl std::fmt::Display for MathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MathError::DomainError { function, what } => write!(f, "{function}: {what}"),
            MathError::Overflow { function } => write!(f, "{function}: numerical overflow"),
            MathError::DivisionByZero { function } => write!(f, "{function}: division by zero"),
            MathError::BadParameter { name, value } => {
                write!(f, "bad parameter {name}={value}")
            }
        }
    }
}

impl std::error::Error for MathError {}

/// Convenience alias used throughout the crate.
pub type MathResult<T> = Result<T, MathError>;
