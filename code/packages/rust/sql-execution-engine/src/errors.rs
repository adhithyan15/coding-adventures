//! Error types for the SQL execution engine.
//!
//! All errors are variants of the [`ExecutionError`] enum.
//!
//! # Error variants
//!
//! | Variant          | When raised                                |
//! |------------------|--------------------------------------------|
//! | `TableNotFound`  | DataSource doesn't recognize the table name|
//! | `ColumnNotFound` | Column reference cannot be resolved        |
//! | `ParseError`     | SQL text has syntax errors                 |
//! | `Other`          | Any other execution-time error             |

use std::fmt;

/// All errors raised by the SQL execution engine.
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionError {
    /// A table referenced in FROM or JOIN was not found in the DataSource.
    TableNotFound(String),
    /// A column name used in SELECT, WHERE, etc. could not be resolved.
    ColumnNotFound(String),
    /// The SQL text could not be parsed.
    ParseError(String),
    /// Any other execution-time error.
    Other(String),
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecutionError::TableNotFound(name) => write!(f, "Table not found: {name:?}"),
            ExecutionError::ColumnNotFound(name) => write!(f, "Column not found: {name:?}"),
            ExecutionError::ParseError(msg) => write!(f, "Parse error: {msg}"),
            ExecutionError::Other(msg) => write!(f, "Execution error: {msg}"),
        }
    }
}

impl std::error::Error for ExecutionError {}
