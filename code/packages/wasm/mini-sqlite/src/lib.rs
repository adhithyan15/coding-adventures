//! WebAssembly bindings for the Rust mini-sqlite Level 0 facade.
//!
//! This crate wraps `coding-adventures-mini-sqlite` with `wasm-bindgen` so
//! JavaScript (and any other Wasm host) can open an in-memory SQL database,
//! execute statements, and read results through a cursor-style API.
//!
//! # Architecture
//!
//! ```text
//!   JavaScript
//!       │ JSON strings for params / results
//!       ▼
//!   mini-sqlite-wasm  (this crate, wasm-bindgen glue)
//!       │ &[SqlValue]  /  Vec<Vec<SqlValue>>
//!       ▼
//!   coding-adventures-mini-sqlite  (Rust Level 0 facade)
//!       │ SQL text
//!       ▼
//!   coding-adventures-sql-execution-engine  (SELECT evaluation)
//!   + hand-rolled DDL/DML parser inside mini-sqlite
//! ```
//!
//! JavaScript cannot hold a Rust `Cursor` object across calls, so this wrapper
//! materialises the result set into a `buf_rows` / `buf_columns` buffer inside
//! the `Connection` struct after every `query()` / `execute_for_fetch()` call.
//! `fetchone()`, `fetchmany()`, and `fetchall()` drain that buffer in order,
//! matching Python DB-API 2.0 semantics.
//!
//! # Quick start
//!
//! ```js
//! import init, { Connection } from "mini-sqlite-wasm";
//! await init();
//!
//! const db = new Connection();
//! db.execute("CREATE TABLE users (id, name, age)");
//! db.execute("INSERT INTO users VALUES (?, ?, ?)", JSON.stringify([1, "Alice", 30]));
//! db.execute("INSERT INTO users VALUES (?, ?, ?)", JSON.stringify([2, "Bob", 25]));
//!
//! const result = JSON.parse(db.query("SELECT * FROM users ORDER BY id"));
//! // { columns: ["id", "name", "age"], rows: [[1, "Alice", 30], [2, "Bob", 25]] }
//!
//! db.execute("BEGIN");
//! db.execute("INSERT INTO users VALUES (?, ?, ?)", JSON.stringify([3, "Charlie", 35]));
//! db.rollback();
//! // Row 3 is gone — rollback restored the snapshot.
//! ```

use coding_adventures_mini_sqlite::{
    connect, Connection as InnerConnection, MiniSqliteError, SqlPrimitive, SqlValue,
};
use wasm_bindgen::prelude::*;

// ── Error helpers ──────────────────────────────────────────────────────────────

/// Map a `MiniSqliteError` to a JS-throwable string with a type-name prefix.
///
/// Conformance test runners identify the error kind by checking whether the
/// thrown string starts with the expected prefix:
///
/// | MiniSqliteError variant  | Thrown string prefix    |
/// |--------------------------|-------------------------|
/// | ProgrammingError(m)      | `"ProgrammingError: m"` |
/// | OperationalError(m)      | `"OperationalError: m"` |
/// | IntegrityError(m)        | `"IntegrityError: m"`   |
/// | NotSupportedError(m)     | `"NotSupportedError: m"`|
///
/// On non-wasm32 targets `JsValue::from_str` calls a Wasm host import and
/// aborts.  The wasm32 variant produces the real error string; the native
/// variant returns `JsValue::NULL` so that success-path unit tests can run on
/// the host without a browser.  Error-path behaviour is verified in
/// `wasm-bindgen-test` (wasm32 only).
#[cfg(target_arch = "wasm32")]
fn sqlite_err_to_js(e: MiniSqliteError) -> JsValue {
    let s = match &e {
        MiniSqliteError::ProgrammingError(m) => format!("ProgrammingError: {m}"),
        MiniSqliteError::OperationalError(m) => format!("OperationalError: {m}"),
        MiniSqliteError::IntegrityError(m) => format!("IntegrityError: {m}"),
        MiniSqliteError::NotSupportedError(m) => format!("NotSupportedError: {m}"),
    };
    JsValue::from_str(&s)
}

#[cfg(not(target_arch = "wasm32"))]
fn sqlite_err_to_js(_e: MiniSqliteError) -> JsValue {
    JsValue::NULL
}

#[cfg(target_arch = "wasm32")]
fn programming_err(msg: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&format!("ProgrammingError: {msg}"))
}

#[cfg(not(target_arch = "wasm32"))]
fn programming_err(_msg: impl std::fmt::Display) -> JsValue {
    JsValue::NULL
}

// ── JSON ↔ SqlValue ───────────────────────────────────────────────────────────

/// Convert one element of a JSON params array to a `SqlValue`.
///
/// | JSON type  | Resulting SqlValue              |
/// |------------|---------------------------------|
/// | `null`     | `None`                          |
/// | integer    | `Some(SqlPrimitive::Int(i64))`  |
/// | float      | `Some(SqlPrimitive::Float(f64))`|
/// | string     | `Some(SqlPrimitive::Text(…))`   |
/// | boolean    | `Some(SqlPrimitive::Bool(…))`   |
/// | other      | `Err` (ProgrammingError)        |
fn json_to_sql_value(v: &serde_json::Value) -> Result<SqlValue, JsValue> {
    match v {
        serde_json::Value::Null => Ok(None),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Some(SqlPrimitive::Int(i)))
            } else if let Some(f) = n.as_f64() {
                Ok(Some(SqlPrimitive::Float(f)))
            } else {
                Err(programming_err("numeric param value out of range"))
            }
        }
        serde_json::Value::String(s) => Ok(Some(SqlPrimitive::Text(s.clone()))),
        serde_json::Value::Bool(b) => Ok(Some(SqlPrimitive::Bool(*b))),
        other => Err(programming_err(format!("unsupported param type: {other}"))),
    }
}

/// Convert a `SqlValue` to a `serde_json::Value` for serialising result rows.
///
/// `None` (SQL NULL) maps to JSON `null`, matching the type conventions in the
/// conformance fixture README.
fn sql_value_to_json(v: &SqlValue) -> serde_json::Value {
    match v {
        None => serde_json::Value::Null,
        Some(SqlPrimitive::Int(i)) => serde_json::json!(i),
        Some(SqlPrimitive::Float(f)) => serde_json::json!(f),
        Some(SqlPrimitive::Text(s)) => serde_json::json!(s),
        Some(SqlPrimitive::Bool(b)) => serde_json::json!(b),
    }
}

/// Parse an optional JSON params string into a `Vec<SqlValue>`.
///
/// Accepts:
/// - `None`              → empty params (no `?` binding needed)
/// - JSON `"null"`       → empty params
/// - JSON `"[]"`         → empty params
/// - JSON `"[v1, v2…]"` → one SqlValue per element
fn parse_params(json: Option<String>) -> Result<Vec<SqlValue>, JsValue> {
    match json {
        None => Ok(vec![]),
        Some(s) if s == "null" || s == "[]" => Ok(vec![]),
        Some(s) => {
            let arr: Vec<serde_json::Value> = serde_json::from_str(&s)
                .map_err(|e| programming_err(format!("params JSON parse error: {e}")))?;
            arr.iter().map(json_to_sql_value).collect()
        }
    }
}

/// Serialise a complete result set to the canonical JSON shape.
///
/// ```json
/// { "columns": ["id", "name"], "rows": [[1, "Alice"], [2, "Bob"]] }
/// ```
fn rows_to_json(columns: &[String], rows: &[Vec<SqlValue>]) -> String {
    let json_rows: Vec<Vec<serde_json::Value>> = rows
        .iter()
        .map(|row| row.iter().map(sql_value_to_json).collect())
        .collect();
    serde_json::json!({ "columns": columns, "rows": json_rows }).to_string()
}

// ── Connection ─────────────────────────────────────────────────────────────────

/// A handle to an in-memory SQLite-compatible database, exposed to JavaScript.
///
/// Only `":memory:"` is accepted at Level 0.  Requesting a file-backed
/// connection throws a `NotSupportedError`.
///
/// ## Cursor buffering
///
/// After `query()` or `execute_for_fetch()`, the full result set is
/// materialised into `buf_rows` / `buf_columns` inside this struct.  Subsequent
/// calls to `fetchone()`, `fetchmany(n)`, and `fetchall()` drain the buffer in
/// order — exactly like Python's cursor fetch methods.
#[wasm_bindgen]
pub struct Connection {
    inner: InnerConnection,
    buf_columns: Vec<String>,
    buf_rows: Vec<Vec<SqlValue>>,
    buf_offset: usize,
}

#[wasm_bindgen]
impl Connection {
    /// Open a new `:memory:` database.
    ///
    /// ```js
    /// const db = new Connection();
    /// ```
    ///
    /// Throws `NotSupportedError` if the Level 0 engine rejects the database
    /// string (only `:memory:` is allowed).
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<Connection, JsValue> {
        connect(":memory:")
            .map(|inner| Connection {
                inner,
                buf_columns: Vec::new(),
                buf_rows: Vec::new(),
                buf_offset: 0,
            })
            .map_err(sqlite_err_to_js)
    }

    /// Execute a DDL or DML statement (CREATE, DROP, INSERT, UPDATE, DELETE).
    ///
    /// `params` is an optional JSON array string for `?` binding:
    /// ```js
    /// db.execute("INSERT INTO t VALUES (?, ?)", JSON.stringify([1, "hello"]));
    /// ```
    ///
    /// Clears the cursor buffer.  Throws on SQL errors or wrong parameter count.
    pub fn execute(&mut self, sql: &str, params: Option<String>) -> Result<(), JsValue> {
        let p = parse_params(params)?;
        self.inner.execute(sql, &p).map_err(sqlite_err_to_js)?;
        self.clear_buffer();
        Ok(())
    }

    /// Execute the same DML statement for each row of parameters.
    ///
    /// `param_seq` must be a JSON array of arrays:
    /// ```js
    /// db.executemany(
    ///   "INSERT INTO nums VALUES (?)",
    ///   JSON.stringify([[1], [2], [3]])
    /// );
    /// ```
    ///
    /// Clears the cursor buffer.  Throws if any row fails.
    pub fn executemany(&mut self, sql: &str, param_seq: &str) -> Result<(), JsValue> {
        let outer: Vec<Vec<serde_json::Value>> = serde_json::from_str(param_seq)
            .map_err(|e| programming_err(format!("param_seq JSON parse error: {e}")))?;
        let sql_seq: Vec<Vec<SqlValue>> = outer
            .iter()
            .map(|row| row.iter().map(json_to_sql_value).collect::<Result<Vec<_>, _>>())
            .collect::<Result<Vec<_>, _>>()?;
        self.inner
            .executemany(sql, &sql_seq)
            .map_err(sqlite_err_to_js)?;
        self.clear_buffer();
        Ok(())
    }

    /// Execute a SELECT and return all results as a JSON string.
    ///
    /// The returned string has the shape:
    /// ```json
    /// { "columns": ["id", "name"], "rows": [[1, "Alice"], [2, "Bob"]] }
    /// ```
    ///
    /// The cursor buffer is also populated so `fetchone()` / `fetchmany()` /
    /// `fetchall()` can be called afterwards for incremental access.
    pub fn query(&mut self, sql: &str, params: Option<String>) -> Result<String, JsValue> {
        let p = parse_params(params)?;
        let mut cursor = self.inner.execute(sql, &p).map_err(sqlite_err_to_js)?;
        self.buf_columns = cursor.description.iter().map(|d| d.name.clone()).collect();
        self.buf_rows = cursor.fetchall();
        self.buf_offset = 0;
        Ok(rows_to_json(&self.buf_columns, &self.buf_rows))
    }

    /// Execute a SELECT and buffer the result set WITHOUT returning it.
    ///
    /// Use when you want cursor-style incremental access via `fetchone()` /
    /// `fetchmany()` / `fetchall()` without receiving the full JSON upfront.
    ///
    /// ```js
    /// db.execute_for_fetch("SELECT n FROM nums ORDER BY n");
    /// const first = JSON.parse(db.fetchone());  // [1]
    /// const next  = JSON.parse(db.fetchone());  // [2]
    /// ```
    pub fn execute_for_fetch(&mut self, sql: &str, params: Option<String>) -> Result<(), JsValue> {
        let p = parse_params(params)?;
        let mut cursor = self.inner.execute(sql, &p).map_err(sqlite_err_to_js)?;
        self.buf_columns = cursor.description.iter().map(|d| d.name.clone()).collect();
        self.buf_rows = cursor.fetchall();
        self.buf_offset = 0;
        Ok(())
    }

    /// Return the next row from the cursor buffer as a JSON array string, or `null`.
    ///
    /// Each call advances the internal offset by one.  Returns JavaScript
    /// `null` (via `Option::None`) when the buffer is exhausted.
    ///
    /// ```js
    /// db.execute_for_fetch("SELECT n FROM nums ORDER BY n");
    /// JSON.parse(db.fetchone()); // [1]
    /// JSON.parse(db.fetchone()); // [2]
    /// db.fetchone();             // null — buffer empty
    /// ```
    pub fn fetchone(&mut self) -> Option<String> {
        if self.buf_offset >= self.buf_rows.len() {
            return None;
        }
        let row = &self.buf_rows[self.buf_offset];
        self.buf_offset += 1;
        let json: Vec<serde_json::Value> = row.iter().map(sql_value_to_json).collect();
        Some(serde_json::to_string(&json).unwrap_or_default())
    }

    /// Return the next `size` rows as a JSON array-of-arrays string.
    ///
    /// If fewer than `size` rows remain, returns only the remaining rows.
    /// Returns `"[]"` when the buffer is exhausted.
    ///
    /// ```js
    /// db.execute_for_fetch("SELECT n FROM nums ORDER BY n");
    /// JSON.parse(db.fetchmany(3)); // [[1], [2], [3]]
    /// JSON.parse(db.fetchmany(3)); // [[4], [5]]  — only 2 rows left
    /// JSON.parse(db.fetchmany(3)); // []
    /// ```
    pub fn fetchmany(&mut self, size: usize) -> String {
        let start = self.buf_offset;
        let end = (start + size).min(self.buf_rows.len());
        self.buf_offset = end;
        let batch = &self.buf_rows[start..end];
        let json_rows: Vec<Vec<serde_json::Value>> = batch
            .iter()
            .map(|row| row.iter().map(sql_value_to_json).collect())
            .collect();
        serde_json::to_string(&json_rows).unwrap_or_else(|_| "[]".to_string())
    }

    /// Return all remaining rows in the buffer as a JSON array-of-arrays string.
    ///
    /// Returns `"[]"` if the buffer is already exhausted.
    ///
    /// ```js
    /// db.execute_for_fetch("SELECT n FROM nums ORDER BY n");
    /// JSON.parse(db.fetchall()); // [[1], [2], [3], [4], [5]]
    /// JSON.parse(db.fetchall()); // []  — already drained
    /// ```
    pub fn fetchall(&mut self) -> String {
        let remaining = &self.buf_rows[self.buf_offset..];
        self.buf_offset = self.buf_rows.len();
        let json_rows: Vec<Vec<serde_json::Value>> = remaining
            .iter()
            .map(|row| row.iter().map(sql_value_to_json).collect())
            .collect();
        serde_json::to_string(&json_rows).unwrap_or_else(|_| "[]".to_string())
    }

    /// Commit the current transaction.
    ///
    /// At Level 0 the database is fully in-memory.  `commit()` discards the
    /// rollback snapshot so that `rollback()` can no longer undo changes.
    /// Calling it outside a transaction is a no-op.
    pub fn commit(&mut self) -> Result<(), JsValue> {
        self.inner.commit().map_err(sqlite_err_to_js)
    }

    /// Roll back the current transaction to the last committed state.
    ///
    /// Restores the database to the snapshot taken when the last `BEGIN` (or
    /// first modifying statement in autocommit-off mode) ran.  If no snapshot
    /// exists, this is a no-op.
    pub fn rollback(&mut self) -> Result<(), JsValue> {
        self.inner.rollback().map_err(sqlite_err_to_js)
    }

    fn clear_buffer(&mut self) {
        self.buf_columns.clear();
        self.buf_rows.clear();
        self.buf_offset = 0;
    }
}

/// Open a database by path string.  Only `":memory:"` is accepted at Level 0.
///
/// This free function mirrors `connect(":memory:")` in the Rust API and is
/// provided for hosts where calling a constructor is awkward.
///
/// ```js
/// import { open } from "mini-sqlite-wasm";
/// const db = open(":memory:");
/// ```
#[wasm_bindgen]
pub fn open(database: &str) -> Result<Connection, JsValue> {
    connect(database)
        .map(|inner| Connection {
            inner,
            buf_columns: Vec::new(),
            buf_rows: Vec::new(),
            buf_offset: 0,
        })
        .map_err(sqlite_err_to_js)
}

// ── Tests (native only — wasm-bindgen JsValue is not available on native) ────

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    fn new_conn() -> Connection {
        Connection::new().expect("connect to :memory: must succeed")
    }

    // ── DDL / DML ────────────────────────────────────────────────────────────

    #[test]
    fn create_and_query_round_trips() {
        let mut db = new_conn();
        db.execute("CREATE TABLE t (id, name)", None).unwrap();
        db.execute(
            "INSERT INTO t VALUES (?, ?)",
            Some(r#"[1, "Alice"]"#.to_string()),
        )
        .unwrap();
        let json = db.query("SELECT id, name FROM t", None).unwrap();
        assert!(json.contains("\"Alice\""));
        assert!(json.contains("\"id\""));
    }

    #[test]
    fn executemany_inserts_all_rows() {
        let mut db = new_conn();
        db.execute("CREATE TABLE nums (n)", None).unwrap();
        db.executemany("INSERT INTO nums VALUES (?)", "[[1],[2],[3]]")
            .unwrap();
        let json = db.query("SELECT n FROM nums ORDER BY n", None).unwrap();
        assert!(json.contains("[[1],[2],[3]]") || json.contains("[[1], [2], [3]]"));
    }

    #[test]
    fn query_returns_columns_and_rows() {
        let mut db = new_conn();
        db.execute("CREATE TABLE t (x, y)", None).unwrap();
        db.execute("INSERT INTO t VALUES (?, ?)", Some("[10, 20]".to_string()))
            .unwrap();
        let v: serde_json::Value =
            serde_json::from_str(&db.query("SELECT x, y FROM t", None).unwrap()).unwrap();
        assert_eq!(v["columns"], serde_json::json!(["x", "y"]));
        assert_eq!(v["rows"], serde_json::json!([[10, 20]]));
    }

    // ── Cursor fetch methods ─────────────────────────────────────────────────

    #[test]
    fn fetchone_returns_rows_in_order() {
        let mut db = new_conn();
        db.execute("CREATE TABLE n (v)", None).unwrap();
        db.executemany("INSERT INTO n VALUES (?)", "[[1],[2],[3]]")
            .unwrap();
        db.execute_for_fetch("SELECT v FROM n ORDER BY v", None)
            .unwrap();
        let r1 = db.fetchone().unwrap();
        let r2 = db.fetchone().unwrap();
        let r3 = db.fetchone().unwrap();
        let r4 = db.fetchone(); // exhausted
        assert_eq!(r1, "[1]");
        assert_eq!(r2, "[2]");
        assert_eq!(r3, "[3]");
        assert!(r4.is_none());
    }

    #[test]
    fn fetchmany_batches_correctly() {
        let mut db = new_conn();
        db.execute("CREATE TABLE n (v)", None).unwrap();
        db.executemany("INSERT INTO n VALUES (?)", "[[1],[2],[3],[4],[5]]")
            .unwrap();
        db.execute_for_fetch("SELECT v FROM n ORDER BY v", None)
            .unwrap();
        let b1: serde_json::Value = serde_json::from_str(&db.fetchmany(3)).unwrap();
        let b2: serde_json::Value = serde_json::from_str(&db.fetchmany(3)).unwrap();
        assert_eq!(b1, serde_json::json!([[1], [2], [3]]));
        assert_eq!(b2, serde_json::json!([[4], [5]]));
    }

    #[test]
    fn fetchall_returns_remaining() {
        let mut db = new_conn();
        db.execute("CREATE TABLE n (v)", None).unwrap();
        db.executemany("INSERT INTO n VALUES (?)", "[[1],[2],[3]]")
            .unwrap();
        db.execute_for_fetch("SELECT v FROM n ORDER BY v", None)
            .unwrap();
        let all: serde_json::Value = serde_json::from_str(&db.fetchall()).unwrap();
        assert_eq!(all, serde_json::json!([[1], [2], [3]]));
        // Second call — buffer drained
        let empty: serde_json::Value = serde_json::from_str(&db.fetchall()).unwrap();
        assert_eq!(empty, serde_json::json!([]));
    }

    // ── Transactions ─────────────────────────────────────────────────────────

    #[test]
    fn commit_makes_changes_visible() {
        let mut db = new_conn();
        db.execute("CREATE TABLE t (v)", None).unwrap();
        db.execute("BEGIN", None).unwrap();
        db.execute("INSERT INTO t VALUES (42)", None).unwrap();
        db.commit().unwrap();
        let v: serde_json::Value =
            serde_json::from_str(&db.query("SELECT v FROM t", None).unwrap()).unwrap();
        assert_eq!(v["rows"], serde_json::json!([[42]]));
    }

    #[test]
    fn rollback_restores_pre_transaction_state() {
        let mut db = new_conn();
        // Setup: DDL and base row must be committed so the snapshot starts fresh.
        // mini-sqlite takes a snapshot on the FIRST mutation; commit() clears it,
        // so subsequent mutations start a new snapshot from the committed state.
        db.execute("CREATE TABLE t (v)", None).unwrap();
        db.commit().unwrap();
        db.execute("INSERT INTO t VALUES (1)", None).unwrap();
        db.commit().unwrap();
        // Transaction under test
        db.execute("BEGIN", None).unwrap(); // snapshot = {t: [{v:1}]}
        db.execute("INSERT INTO t VALUES (2)", None).unwrap();
        assert!(db.rollback().is_ok()); // restore → {t: [{v:1}]}
        // Only row 1 must survive
        let json = db.query("SELECT v FROM t", None).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["rows"].as_array().unwrap().len(), 1);
    }

    // ── NULL handling ─────────────────────────────────────────────────────────

    #[test]
    fn null_params_and_results_round_trip() {
        let mut db = new_conn();
        db.execute("CREATE TABLE maybe (id, value)", None).unwrap();
        db.execute(
            "INSERT INTO maybe VALUES (?, ?)",
            Some("[1, null]".to_string()),
        )
        .unwrap();
        let v: serde_json::Value =
            serde_json::from_str(&db.query("SELECT id, value FROM maybe", None).unwrap()).unwrap();
        assert_eq!(v["rows"], serde_json::json!([[1, null]]));
    }

    // ── Error types ──────────────────────────────────────────────────────────
    //
    // Error-path tests (wrong param count → ProgrammingError, unknown table →
    // OperationalError, file path → NotSupportedError) require `JsValue::from_str`
    // which panics on native/non-wasm32 targets.  These are covered in the
    // wasm-bindgen-test suite (wasm32 only).  Here we just smoke-test that the
    // error path returns Err (using `JsValue::NULL` on native).

    #[test]
    fn wrong_param_count_returns_err() {
        let mut db = new_conn();
        db.execute("CREATE TABLE t (a, b)", None).unwrap();
        assert!(db
            .execute("INSERT INTO t VALUES (?, ?)", Some("[1]".to_string()))
            .is_err());
    }

    #[test]
    fn unknown_table_returns_err() {
        let mut db = new_conn();
        assert!(db.query("SELECT * FROM nonexistent", None).is_err());
    }

    #[test]
    fn file_backed_open_returns_err() {
        assert!(Connection::new().is_ok());
        assert!(open("app.db").is_err());
    }

    // ── DROP TABLE / IF NOT EXISTS ────────────────────────────────────────────

    #[test]
    fn drop_table_removes_table() {
        let mut db = new_conn();
        db.execute("CREATE TABLE t (v)", None).unwrap();
        db.execute("DROP TABLE t", None).unwrap();
        // After DROP, SELECT must fail (OperationalError on wasm32; Err(NULL) on native)
        assert!(db.query("SELECT * FROM t", None).is_err());
    }

    #[test]
    fn update_and_delete_with_where() {
        let mut db = new_conn();
        db.execute("CREATE TABLE t (id, val)", None).unwrap();
        db.executemany(
            "INSERT INTO t VALUES (?, ?)",
            "[[1, 10], [2, 20], [3, 30]]",
        )
        .unwrap();
        db.execute(
            "UPDATE t SET val = ? WHERE id = ?",
            Some("[99, 2]".to_string()),
        )
        .unwrap();
        db.execute("DELETE FROM t WHERE id = ?", Some("[3]".to_string()))
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(
            &db.query("SELECT id, val FROM t ORDER BY id", None).unwrap(),
        )
        .unwrap();
        assert_eq!(v["rows"], serde_json::json!([[1, 10], [2, 99]]));
    }
}
