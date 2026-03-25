//! # SQL CSV Source
//!
//! A thin adapter that implements the [`DataSource`] trait from
//! `coding-adventures-sql-execution-engine` using CSV files on disk.
//!
//! Each `tablename.csv` file in a directory is one queryable table.
//! Column names come from the CSV header row. Values are type-coerced
//! from strings to [`SqlValue`] before being handed to the engine.
//!
//! # How it fits in the stack
//!
//! ```text
//! csv files on disk
//!        │
//!        ▼
//!  CsvDataSource          ← this crate
//!        │
//!        ▼
//! sql-execution-engine    ← runs SELECT queries
//!        │
//!        ▼
//!   QueryResult
//! ```
//!
//! # Quick start
//!
//! ```rust,ignore
//! use coding_adventures_sql_csv_source::CsvDataSource;
//! use coding_adventures_sql_execution_engine::execute;
//!
//! let source = CsvDataSource::new("path/to/csv/dir");
//! let result = execute("SELECT * FROM employees WHERE active = true", &source).unwrap();
//! println!("{}", result.rows.len()); // 3
//! ```
//!
//! # Type coercion
//!
//! CSV is untyped — every field is a `String`. The engine needs [`SqlValue`]
//! (which may be `None` for NULL, or `Some(SqlPrimitive::...)` for typed values).
//!
//! Coercion rules applied in order:
//!
//! | CSV string  | Rust value              |
//! |-------------|-------------------------|
//! | `""`        | `None` (SQL NULL)       |
//! | `"true"`    | `Some(Bool(true))`      |
//! | `"false"`   | `Some(Bool(false))`     |
//! | `"42"`      | `Some(Int(42))`         |
//! | `"3.14"`    | `Some(Float(3.14))`     |
//! | `"hello"`   | `Some(Text("hello"))`   |
//!
//! # Column ordering
//!
//! [`HashMap`] does not preserve insertion order. To return columns in their
//! original header order, [`CsvDataSource::schema`] reads the first line of
//! the file directly and splits on commas — completely bypassing the HashMap.
//! [`CsvDataSource::scan`] uses `parse_csv` which builds HashMaps, but the
//! *keys* in each row map are the same as the header (just unordered for
//! iteration). The engine looks up columns by name, not position, so row
//! order within the map does not matter.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use coding_adventures_csv_parser::parse_csv;
use coding_adventures_sql_execution_engine::{DataSource, ExecutionError, SqlPrimitive, SqlValue};

// ─────────────────────────────────────────────────────────────────────────────
// CsvDataSource
// ─────────────────────────────────────────────────────────────────────────────

/// A [`DataSource`] backed by CSV files in a directory.
///
/// Each `tablename.csv` file in `dir` is one queryable table.
/// Column names come from the CSV header row. Values are type-coerced
/// from strings to [`SqlValue`].
///
/// # Example
///
/// ```rust,ignore
/// let source = CsvDataSource::new("tests/fixtures");
/// let cols = source.schema("employees").unwrap();
/// assert_eq!(cols, vec!["id", "name", "dept_id", "salary", "active"]);
/// ```
pub struct CsvDataSource {
    /// Path to the directory containing `*.csv` files.
    dir: PathBuf,
}

impl CsvDataSource {
    /// Create a new `CsvDataSource` that reads CSV files from `dir`.
    ///
    /// # Arguments
    ///
    /// * `dir` — any type that can be turned into a `PathBuf`: `&str`,
    ///   `String`, `Path`, etc.
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// Build the CSV file path for `table_name`.
    ///
    /// Returns `Err(ExecutionError::TableNotFound)` if the file does not exist.
    fn resolve(&self, table_name: &str) -> Result<PathBuf, ExecutionError> {
        let path = self.dir.join(format!("{table_name}.csv"));
        if path.exists() {
            Ok(path)
        } else {
            Err(ExecutionError::TableNotFound(table_name.to_string()))
        }
    }
}

impl DataSource for CsvDataSource {
    /// Return the column names for `table_name` in header order.
    ///
    /// Reads only the **first line** of the CSV file to extract column names.
    /// This is both fast (no need to parse all rows) and correct: it reads
    /// the header directly, preserving the exact left-to-right order.
    ///
    /// We cannot rely on `parse_csv`'s HashMap for ordering because
    /// `HashMap` randomises iteration order.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionError::TableNotFound`] if `table_name.csv` is missing.
    fn schema(&self, table_name: &str) -> Result<Vec<String>, ExecutionError> {
        let path = self.resolve(table_name)?;

        // Read the full file and grab just the first line.
        // Lines are separated by '\n'; '\r\n' is handled by trimming '\r'.
        let content = fs::read_to_string(&path)
            .map_err(|_| ExecutionError::TableNotFound(table_name.to_string()))?;

        let first_line = content
            .lines()
            .next()
            .unwrap_or("");

        if first_line.is_empty() {
            return Ok(vec![]);
        }

        // Split the header line on commas → column names in file order.
        Ok(first_line
            .split(',')
            .map(|col| col.trim().to_string())
            .collect())
    }

    /// Return all data rows from `table_name` with type-coerced values.
    ///
    /// Uses `parse_csv` for full RFC 4180 support (quoted fields, embedded
    /// commas, escaped double-quotes). Each `String` value is then passed
    /// through [`coerce`] to produce a [`SqlValue`].
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionError::TableNotFound`] if `table_name.csv` is missing.
    /// Returns [`ExecutionError::Other`] if the CSV is malformed (unclosed quote).
    fn scan(&self, table_name: &str) -> Result<Vec<HashMap<String, SqlValue>>, ExecutionError> {
        let path = self.resolve(table_name)?;

        let content = fs::read_to_string(&path)
            .map_err(|_| ExecutionError::TableNotFound(table_name.to_string()))?;

        // parse_csv returns Vec<HashMap<String, String>> — all values are strings.
        let str_rows = parse_csv(&content)
            .map_err(|e| ExecutionError::Other(e.to_string()))?;

        // Coerce each string value to its natural SQL type.
        let typed_rows = str_rows
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|(k, v)| (k, coerce(&v)))
                    .collect::<HashMap<String, SqlValue>>()
            })
            .collect();

        Ok(typed_rows)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Type coercion
// ─────────────────────────────────────────────────────────────────────────────

/// Coerce a CSV string value to the most appropriate [`SqlValue`].
///
/// CSV is untyped — every field comes out of the parser as a `&str`.
/// This function converts it to the richest matching SQL type so the
/// execution engine can evaluate typed comparisons like `WHERE salary > 80000`
/// or `WHERE active = true`.
///
/// # Rules (applied in priority order)
///
/// 1. `""` → `None` — empty field maps to SQL NULL.
/// 2. `"true"` → `Some(Bool(true))` — exact lowercase only (CSV convention).
/// 3. `"false"` → `Some(Bool(false))`.
/// 4. Parseable as `i64` → `Some(Int(n))`.
/// 5. Parseable as `f64` → `Some(Float(f))`.
/// 6. Otherwise → `Some(Text(s))`.
///
/// # Why booleans before numbers?
///
/// In Rust, `"true".parse::<i64>()` returns `Err`, so order wouldn't matter
/// numerically. But checking booleans first makes the intent explicit and
/// mirrors the convention from the Python and Ruby implementations.
///
/// # Examples
///
/// ```rust
/// use coding_adventures_sql_csv_source::coerce;
/// use coding_adventures_sql_execution_engine::SqlPrimitive;
///
/// assert_eq!(coerce(""),      None);
/// assert_eq!(coerce("true"),  Some(SqlPrimitive::Bool(true)));
/// assert_eq!(coerce("42"),    Some(SqlPrimitive::Int(42)));
/// assert_eq!(coerce("3.14"),  Some(SqlPrimitive::Float(3.14)));
/// assert_eq!(coerce("hello"), Some(SqlPrimitive::Text("hello".to_string())));
/// ```
pub fn coerce(s: &str) -> SqlValue {
    // ── NULL ─────────────────────────────────────────────────────────────────
    if s.is_empty() {
        return None;
    }

    // ── Boolean ──────────────────────────────────────────────────────────────
    // Exact lowercase match, following the CSV convention used in the fixtures.
    if s == "true" {
        return Some(SqlPrimitive::Bool(true));
    }
    if s == "false" {
        return Some(SqlPrimitive::Bool(false));
    }

    // ── Integer ──────────────────────────────────────────────────────────────
    // Try parsing as i64 first. "42" succeeds; "3.14" fails (Rust's parse is strict).
    if let Ok(i) = s.parse::<i64>() {
        return Some(SqlPrimitive::Int(i));
    }

    // ── Float ────────────────────────────────────────────────────────────────
    // "3.14" parses as f64; "hello" does not.
    if let Ok(f) = s.parse::<f64>() {
        return Some(SqlPrimitive::Float(f));
    }

    // ── String fallthrough ───────────────────────────────────────────────────
    Some(SqlPrimitive::Text(s.to_string()))
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests (inline)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── coerce() unit tests ──────────────────────────────────────────────────

    #[test]
    fn test_coerce_empty_is_null() {
        assert_eq!(coerce(""), None);
    }

    #[test]
    fn test_coerce_true() {
        assert_eq!(coerce("true"), Some(SqlPrimitive::Bool(true)));
    }

    #[test]
    fn test_coerce_false() {
        assert_eq!(coerce("false"), Some(SqlPrimitive::Bool(false)));
    }

    #[test]
    fn test_coerce_integer() {
        assert_eq!(coerce("42"), Some(SqlPrimitive::Int(42)));
    }

    #[test]
    fn test_coerce_negative_integer() {
        assert_eq!(coerce("-7"), Some(SqlPrimitive::Int(-7)));
    }

    #[test]
    fn test_coerce_zero() {
        assert_eq!(coerce("0"), Some(SqlPrimitive::Int(0)));
    }

    #[test]
    fn test_coerce_float() {
        assert_eq!(coerce("3.14"), Some(SqlPrimitive::Float(3.14)));
    }

    #[test]
    fn test_coerce_string() {
        assert_eq!(
            coerce("hello"),
            Some(SqlPrimitive::Text("hello".to_string()))
        );
    }

    #[test]
    fn test_coerce_string_with_spaces() {
        assert_eq!(
            coerce("Alice Smith"),
            Some(SqlPrimitive::Text("Alice Smith".to_string()))
        );
    }
}
