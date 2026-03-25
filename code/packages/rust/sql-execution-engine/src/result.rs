//! QueryResult — the output of a SQL SELECT execution.
//!
//! A [`QueryResult`] bundles the output column names and the result rows.
//!
//! # Fields
//!
//! - `columns` — the output column names, in SELECT order, after AS aliases.
//! - `rows` — each row is a `HashMap<String, SqlValue>` mapping column
//!   name to value.

use std::collections::HashMap;

use crate::data_source::SqlValue;

/// The output of a successfully executed SELECT query.
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// Output column names in SELECT order.
    pub columns: Vec<String>,
    /// Result rows. Each row maps column name → SQL value.
    pub rows: Vec<HashMap<String, SqlValue>>,
}

impl QueryResult {
    /// Create a new empty QueryResult.
    pub fn empty() -> Self {
        QueryResult {
            columns: vec![],
            rows: vec![],
        }
    }
}
