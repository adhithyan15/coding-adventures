//! Statistics Core
//!
//! Phase 1 implementation for `code/specs/statistics-core.md`: descriptive
//! statistics, counting, and rank/order helpers over `r-vector::Double`.

pub mod counting;
pub mod descriptive;
pub mod rank;

pub use numeric_tower::Number;
pub use r_vector::{is_na_real, na_real, Character, Double, Vector};

#[derive(Debug, Clone, PartialEq)]
pub enum StatsError {
    EmptyInput {
        function: &'static str,
        min_n: usize,
    },
    DomainError {
        function: &'static str,
        what: String,
    },
    ShapeMismatch {
        expected: usize,
        found: usize,
    },
    Singular {
        function: &'static str,
    },
    NoConvergence {
        function: &'static str,
        iters: u32,
    },
    BadParameter {
        name: &'static str,
        value: String,
    },
    Overflow {
        function: &'static str,
    },
}

impl std::fmt::Display for StatsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StatsError::EmptyInput { function, min_n } => {
                write!(f, "{function} requires at least {min_n} input value(s)")
            }
            StatsError::DomainError { function, what } => write!(f, "{function}: {what}"),
            StatsError::ShapeMismatch { expected, found } => {
                write!(f, "shape mismatch: expected {expected}, found {found}")
            }
            StatsError::Singular { function } => write!(f, "{function}: singular matrix"),
            StatsError::NoConvergence { function, iters } => {
                write!(f, "{function}: no convergence after {iters} iterations")
            }
            StatsError::BadParameter { name, value } => write!(f, "bad parameter {name}={value}"),
            StatsError::Overflow { function } => write!(f, "{function}: numerical overflow"),
        }
    }
}

impl std::error::Error for StatsError {}
