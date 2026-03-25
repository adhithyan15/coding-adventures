//! DataSource trait — the pluggable data interface.
//!
//! The execution engine is decoupled from any particular storage via the
//! [`DataSource`] trait.
//!
//! # SQL Value Types
//!
//! SQL values map to Rust as follows:
//!
//! | SQL type    | Rust type          |
//! |-------------|-------------------|
//! | NULL        | `None`            |
//! | INTEGER     | `Some(Int(i64))`  |
//! | REAL/FLOAT  | `Some(Float(f64))`|
//! | TEXT/VARCHAR| `Some(Text(...))`  |
//! | BOOLEAN     | `Some(Bool(...))`  |
//!
//! # Example implementation
//!
//! ```rust,ignore
//! use coding_adventures_sql_execution_engine::{DataSource, SqlValue, SqlPrimitive, ExecutionError};
//! use std::collections::HashMap;
//!
//! struct MemorySource;
//!
//! impl DataSource for MemorySource {
//!     fn schema(&self, table_name: &str) -> Result<Vec<String>, ExecutionError> {
//!         match table_name {
//!             "users" => Ok(vec!["id".to_string(), "name".to_string()]),
//!             other   => Err(ExecutionError::TableNotFound(other.to_string())),
//!         }
//!     }
//!
//!     fn scan(&self, table_name: &str) -> Result<Vec<HashMap<String, SqlValue>>, ExecutionError> {
//!         match table_name {
//!             "users" => Ok(vec![
//!                 [("id".to_string(), Some(SqlPrimitive::Int(1))),
//!                  ("name".to_string(), Some(SqlPrimitive::Text("Alice".to_string())))]
//!                  .into_iter().collect()
//!             ]),
//!             other => Err(ExecutionError::TableNotFound(other.to_string())),
//!         }
//!     }
//! }
//! ```

use std::collections::HashMap;

use crate::errors::ExecutionError;

/// A SQL primitive value (non-NULL).
///
/// Represents the set of concrete values a SQL column can hold.
#[derive(Debug, Clone, PartialEq)]
pub enum SqlPrimitive {
    /// A 64-bit integer (SQL INTEGER, INT, BIGINT).
    Int(i64),
    /// A 64-bit float (SQL REAL, FLOAT, DOUBLE).
    Float(f64),
    /// A UTF-8 string (SQL TEXT, VARCHAR, CHAR).
    Text(String),
    /// A boolean (SQL BOOLEAN, BOOL).
    Bool(bool),
}

/// A nullable SQL value.
///
/// `None` represents SQL NULL; `Some(primitive)` is a non-NULL value.
pub type SqlValue = Option<SqlPrimitive>;

/// Trait for pluggable data providers.
///
/// Implement this trait to connect the execution engine to any data store.
pub trait DataSource {
    /// Return the column names for the given table.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionError::TableNotFound`] if the table is unknown.
    fn schema(&self, table_name: &str) -> Result<Vec<String>, ExecutionError>;

    /// Return all rows of a table as a list of column→value maps.
    ///
    /// Each row is a `HashMap<String, SqlValue>` where keys are column names.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionError::TableNotFound`] if the table is unknown.
    fn scan(&self, table_name: &str) -> Result<Vec<HashMap<String, SqlValue>>, ExecutionError>;
}
