//! # SQL Execution Engine
//!
//! A SELECT-only SQL execution engine that executes parsed SQL queries
//! against any pluggable [`DataSource`].
//!
//! # Pipeline
//!
//! ```text
//! sql-lexer  →  sql-parser  →  sql-execution-engine
//! ```
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use coding_adventures_sql_execution_engine::{
//!     execute, DataSource, ExecutionError, SqlValue, SqlPrimitive,
//! };
//! use std::collections::HashMap;
//!
//! struct MySource;
//!
//! impl DataSource for MySource {
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
//!                 [("id".to_string(),   Some(SqlPrimitive::Int(1))),
//!                  ("name".to_string(), Some(SqlPrimitive::Text("Alice".to_string())))]
//!                  .into_iter().collect()
//!             ]),
//!             other => Err(ExecutionError::TableNotFound(other.to_string())),
//!         }
//!     }
//! }
//!
//! let result = execute("SELECT name FROM users", &MySource).unwrap();
//! assert_eq!(result.columns, vec!["name".to_string()]);
//! ```

pub mod aggregate;
pub mod data_source;
pub mod engine;
pub mod errors;
pub mod executor;
pub mod expression;
pub mod join;
pub mod result;

pub use data_source::{DataSource, SqlPrimitive, SqlValue};
pub use engine::{execute, execute_all};
pub use errors::ExecutionError;
pub use result::QueryResult;
