//! # Mini-SQLite — Level 1 Rust Facade
//!
//! A DB-API 2.0-inspired SQLite façade backed by the full Mini-SQLite
//! pipeline: `sql-parser → sql-planner → sql-optimizer → sql-codegen → sql-vm`.
//!
//! ## Pipeline overview
//!
//! ```text
//! SQL text  (e.g. "SELECT name FROM users WHERE age > 18")
//!     │
//!     ▼ sql_parser::parse_sql        → GrammarASTNode (AST)
//!     │
//!     ▼ sql_planner::plan_sql        → LogicalPlan
//!       (uses SchemaProvider to look up column lists)
//!     │
//!     ▼ sql_optimizer::optimize      → OptimizedPlan
//!       (constant folding, predicate pushdown, etc.)
//!     │
//!     ▼ sql_codegen::compile         → Program (bytecode)
//!     │
//!     ▼ sql_vm::execute              → QueryResult
//!       (runs bytecode against InMemoryBackend)
//!     │
//!     ▼ mini-sqlite facade           → rows / rowcount / lastrowid
//! ```
//!
//! ## API level
//!
//! This crate ships at Level 1: it routes all SQL — including DDL, DML, and
//! SELECT — through the full pipeline above. The connection and cursor types
//! mirror the DB-API 2.0 spec (PEP 249) to make the API familiar to anyone
//! who has used Python's `sqlite3` module.
//!
//! ## Parameters
//!
//! Only `?` (qmark) positional parameters are supported, matching SQLite's
//! most common binding style. Each `?` is substituted as a SQL literal before
//! the query is parsed, keeping the parameter substitution simple and
//! independent of the SQL grammar.
//!
//! ## Transactions
//!
//! The `InMemoryBackend` maintains an internal snapshot on `begin_transaction`.
//! `commit()` discards the snapshot; `rollback()` restores it.
//! `execute()` on DML statements automatically begins a transaction if one
//! is not already active (snapshot-on-first-write semantics).

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use coding_adventures_sql_backend::{
    backend_as_schema_provider, Backend, InMemoryBackend, TransactionHandle,
};

// Re-export SqlValue so callers can use coding_adventures_mini_sqlite::SqlValue
// (mirrors the Level 0 API which re-exported it from sql-execution-engine).
pub use coding_adventures_sql_backend::SqlValue;
use coding_adventures_sql_codegen::compile;
use coding_adventures_sql_optimizer::optimize;
use coding_adventures_sql_planner::{plan_sql, PlanError};
use coding_adventures_sql_vm::execute as vm_execute;
// The file-backed storage engine: exposes a real `.sqlite` file as a `Backend`.
use coding_adventures_storage_sqlite::SqliteFileBackend;

// ===========================================================================
// Public constants (DB-API 2.0 spec attributes)
// ===========================================================================

/// API level this crate conforms to.
///
/// The string `"2.0"` follows DB-API 2.0 (PEP 249).
pub const API_LEVEL: &str = "2.0";

/// Thread safety level (1 = threads may share the module but NOT connections).
///
/// Since `Connection` uses `Rc<RefCell<…>>` it is explicitly NOT `Send`, so
/// this is conservatively set to 1 (share module only).
pub const THREAD_SAFETY: u8 = 1;

/// Parameter style: `?` positional parameters (SQLite convention).
pub const PARAM_STYLE: &str = "qmark";

// ===========================================================================
// Re-export SqlValue helpers for callers
// ===========================================================================


/// Create a NULL `SqlValue`.
pub fn null() -> SqlValue {
    SqlValue::Null
}

/// Create an integer `SqlValue`.
pub fn int(value: i64) -> SqlValue {
    SqlValue::Int(value)
}

/// Create a float `SqlValue`.
pub fn real(value: f64) -> SqlValue {
    SqlValue::Float(value)
}

/// Create a text `SqlValue`.
pub fn text(value: impl Into<String>) -> SqlValue {
    SqlValue::Text(value.into())
}

/// Create a boolean `SqlValue`.
pub fn boolean(value: bool) -> SqlValue {
    SqlValue::Bool(value)
}

// ===========================================================================
// Error type
// ===========================================================================

/// Errors from mini-sqlite operations.
///
/// The four variants mirror DB-API 2.0's exception hierarchy and SQLite's
/// own error classification:
///
/// | Variant              | When raised                                          |
/// |----------------------|------------------------------------------------------|
/// | `ProgrammingError`   | Wrong param count, bad SQL, misuse of the API        |
/// | `OperationalError`   | Table not found, division by zero, runtime SQL error |
/// | `IntegrityError`     | Constraint violation (NOT NULL, UNIQUE, FOREIGN KEY) |
/// | `NotSupportedError`  | Feature not available at this level (e.g. file DBs)  |
#[derive(Debug, Clone, PartialEq)]
pub enum MiniSqliteError {
    ProgrammingError(String),
    OperationalError(String),
    IntegrityError(String),
    NotSupportedError(String),
}

impl fmt::Display for MiniSqliteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MiniSqliteError::ProgrammingError(m) => write!(f, "Programming error: {m}"),
            MiniSqliteError::OperationalError(m) => write!(f, "Operational error: {m}"),
            MiniSqliteError::IntegrityError(m) => write!(f, "Integrity error: {m}"),
            MiniSqliteError::NotSupportedError(m) => write!(f, "Not supported: {m}"),
        }
    }
}

impl std::error::Error for MiniSqliteError {}

// ===========================================================================
// Result alias
// ===========================================================================

pub type Result<T> = std::result::Result<T, MiniSqliteError>;

// ===========================================================================
// ConnectOptions
// ===========================================================================

/// Options for opening a connection.
///
/// Currently only `autocommit` is meaningful.  When `autocommit = true` each
/// statement is committed immediately; when `false` (the default) changes are
/// buffered in a snapshot until `commit()` or `rollback()`.
#[derive(Clone, Debug, Copy, Default)]
pub struct ConnectOptions {
    pub autocommit: bool,
}

// ===========================================================================
// Connection
// ===========================================================================

/// A database connection.
///
/// The connection holds its storage backend (an in-memory store, or a real
/// `.sqlite` file) wrapped in a shared `Rc<RefCell<…>>` so that `Cursor` objects
/// can borrow it after creation.
///
/// Cloning a `Connection` shares the underlying backend state (both clones
/// see the same tables).
#[derive(Clone, Debug)]
pub struct Connection {
    state: Rc<RefCell<ConnectionState>>,
}

struct ConnectionState {
    /// The storage engine the pipeline reads/writes through. `InMemoryBackend`
    /// for `:memory:`; a read-only `SqliteFileBackend` for a real `.sqlite` file.
    backend: Box<dyn Backend>,
    /// The active transaction handle, if one is open. Tracked here (rather than
    /// asked of the backend) because `current_transaction` is not part of the
    /// `Backend` trait — the connection owns its transaction lifecycle.
    active_tx: Option<TransactionHandle>,
    autocommit: bool,
    closed: bool,
}

// `dyn Backend` is not `Debug`, so derive won't do — print the plain fields.
impl fmt::Debug for ConnectionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConnectionState")
            .field("autocommit", &self.autocommit)
            .field("closed", &self.closed)
            .finish_non_exhaustive()
    }
}

// ===========================================================================
// Cursor
// ===========================================================================

/// One-way result cursor, produced by `Connection::execute` or `cursor.execute`.
///
/// After execution the cursor holds a buffered result set that callers can
/// iterate with `fetchone`, `fetchmany`, or `fetchall`.
#[derive(Debug)]
pub struct Cursor {
    conn: Rc<RefCell<ConnectionState>>,
    /// Column descriptions, one per SELECT column.
    pub description: Vec<ColumnDescription>,
    rowcount: isize,
    lastrowid: Option<i64>,
    arraysize: usize,
    rows: Vec<Vec<SqlValue>>,
    offset: usize,
    closed: bool,
}

/// A single column description in a result set.
///
/// Mirrors DB-API 2.0's cursor `.description` attribute (we only expose
/// `name` since the schema is dynamically typed at Level 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnDescription {
    pub name: String,
}

// ===========================================================================
// connect() — public entry point
// ===========================================================================

/// Open a new in-memory database connection.
///
/// Pass `":memory:"` for a fresh in-memory database, or a filesystem path to
/// open a real `.sqlite` file. A file is opened **read-only** for now (queries
/// work; `INSERT`/`UPDATE`/`CREATE` against it error) — the byte-compatible
/// writer is a later milestone.
///
/// ```rust
/// use coding_adventures_mini_sqlite::connect;
/// let conn = connect(":memory:").unwrap();
/// ```
pub fn connect(database: &str) -> Result<Connection> {
    connect_with_options(database, ConnectOptions::default())
}

/// Open a connection with explicit options.
///
/// `":memory:"` builds an in-memory backend; any other string is treated as a
/// path to a SQLite database file, read into memory and exposed through the
/// read-only [`SqliteFileBackend`]. A missing file or non-SQLite bytes surface
/// as `OperationalError` (SQLite's class for such runtime failures).
pub fn connect_with_options(database: &str, options: ConnectOptions) -> Result<Connection> {
    let backend: Box<dyn Backend> = if database == ":memory:" {
        Box::new(InMemoryBackend::new())
    } else {
        let bytes = std::fs::read(database).map_err(|e| {
            MiniSqliteError::OperationalError(format!("cannot open database file {database:?}: {e}"))
        })?;
        let file_backend = SqliteFileBackend::open(bytes).map_err(|e| {
            MiniSqliteError::OperationalError(format!("not a valid SQLite database {database:?}: {e}"))
        })?;
        Box::new(file_backend)
    };
    Ok(Connection {
        state: Rc::new(RefCell::new(ConnectionState {
            backend,
            active_tx: None,
            autocommit: options.autocommit,
            closed: false,
        })),
    })
}

// ===========================================================================
// Connection impl
// ===========================================================================

impl Connection {
    /// Create a new cursor bound to this connection.
    pub fn cursor(&self) -> Result<Cursor> {
        self.assert_open()?;
        Ok(Cursor::new(Rc::clone(&self.state)))
    }

    /// Execute `sql` with parameters and return a new cursor.
    ///
    /// This is a convenience shortcut for `cursor().execute(sql, params)`.
    pub fn execute(&self, sql: &str, params: &[SqlValue]) -> Result<Cursor> {
        let mut cursor = self.cursor()?;
        cursor.execute(sql, params)?;
        Ok(cursor)
    }

    /// Execute `sql` with each parameter slice in `params_seq`.
    ///
    /// All executions share the same cursor; `rowcount` reflects the total
    /// rows affected across all executions.
    pub fn executemany(&self, sql: &str, params_seq: &[Vec<SqlValue>]) -> Result<Cursor> {
        let mut cursor = self.cursor()?;
        cursor.executemany(sql, params_seq)?;
        Ok(cursor)
    }

    /// Commit the current transaction.
    ///
    /// In autocommit mode this is a no-op (each statement already committed).
    pub fn commit(&self) -> Result<()> {
        let mut state = self.state.borrow_mut();
        state.assert_open()?;
        // Delegate to the backend's commit if there's an active transaction.
        if let Some(h) = state.active_tx.take() {
            state
                .backend
                .commit(h)
                .map_err(|e| MiniSqliteError::OperationalError(e.to_string()))?;
        }
        Ok(())
    }

    /// Roll back the current transaction.
    ///
    /// Restores the backend to the state it had at `BEGIN TRANSACTION`.
    pub fn rollback(&self) -> Result<()> {
        let mut state = self.state.borrow_mut();
        state.assert_open()?;
        if let Some(h) = state.active_tx.take() {
            state
                .backend
                .rollback(h)
                .map_err(|e| MiniSqliteError::OperationalError(e.to_string()))?;
        }
        Ok(())
    }

    /// Close the connection. Uncommitted changes are rolled back.
    pub fn close(&self) -> Result<()> {
        let mut state = self.state.borrow_mut();
        if state.closed {
            return Ok(());
        }
        // Roll back any pending transaction before closing.
        if let Some(h) = state.active_tx.take() {
            let _ = state.backend.rollback(h);
        }
        state.closed = true;
        Ok(())
    }

    fn assert_open(&self) -> Result<()> {
        self.state.borrow().assert_open()
    }
}

// ===========================================================================
// ConnectionState impl
// ===========================================================================

impl ConnectionState {
    fn assert_open(&self) -> Result<()> {
        if self.closed {
            return Err(MiniSqliteError::ProgrammingError(
                "connection is closed".to_string(),
            ));
        }
        Ok(())
    }

    /// Run `sql` (with parameters already substituted) through the full
    /// pipeline and return a `StatementOutcome`.
    fn run_sql(&mut self, sql_with_params: &str) -> Result<StatementOutcome> {
        self.assert_open()?;

        // Determine the first keyword so we can handle transaction control
        // statements that the codegen does not emit instructions for.
        let first_kw = first_keyword(sql_with_params);
        match first_kw.as_str() {
            "BEGIN" => {
                // Manual `BEGIN` — start a transaction on the backend and record
                // its handle on the connection.
                let handle = self
                    .backend
                    .begin_transaction()
                    .map_err(|e| MiniSqliteError::OperationalError(e.to_string()))?;
                self.active_tx = Some(handle);
                return Ok(StatementOutcome {
                    columns: Vec::new(),
                    rows: Vec::new(),
                    rows_affected: 0,
                });
            }
            "COMMIT" => {
                if let Some(h) = self.active_tx.take() {
                    self.backend
                        .commit(h)
                        .map_err(|e| MiniSqliteError::OperationalError(e.to_string()))?;
                }
                return Ok(StatementOutcome {
                    columns: Vec::new(),
                    rows: Vec::new(),
                    rows_affected: 0,
                });
            }
            "ROLLBACK" => {
                if let Some(h) = self.active_tx.take() {
                    self.backend
                        .rollback(h)
                        .map_err(|e| MiniSqliteError::OperationalError(e.to_string()))?;
                }
                return Ok(StatementOutcome {
                    columns: Vec::new(),
                    rows: Vec::new(),
                    rows_affected: 0,
                });
            }
            _ => {}
        }

        // For DML and DDL, ensure a transaction is open so changes can be
        // rolled back if the caller calls rollback() later.
        let needs_transaction = matches!(
            first_kw.as_str(),
            "INSERT" | "UPDATE" | "DELETE" | "CREATE" | "DROP"
        );
        if needs_transaction && !self.autocommit && self.active_tx.is_none() {
            let handle = self
                .backend
                .begin_transaction()
                .map_err(|e| MiniSqliteError::OperationalError(e.to_string()))?;
            self.active_tx = Some(handle);
        }

        // ── Pipeline ────────────────────────────────────────────────────────

        // 1. Plan (parse + plan in one step via plan_sql)
        let schema_provider = backend_as_schema_provider(&*self.backend);
        let logical_plan = plan_sql(sql_with_params, &schema_provider)
            .map_err(|e| match &e {
                // UnknownTable is a runtime condition (table was dropped or
                // never created) — mirrors SQLite's OperationalError class.
                PlanError::UnknownTable(_) => {
                    MiniSqliteError::OperationalError(format!("{e}"))
                }
                // All other plan errors (parse failures, unsupported syntax,
                // unknown columns) are programming errors — the SQL is wrong.
                _ => MiniSqliteError::ProgrammingError(format!("{e:?}")),
            })?;

        // 2. Optimize
        let optimized_plan = optimize(logical_plan);

        // 3. Codegen
        let program = compile(&optimized_plan);

        // 4. Execute
        let result = vm_execute(&program, &mut *self.backend)
            .map_err(|e| MiniSqliteError::OperationalError(e.to_string()))?;

        Ok(StatementOutcome {
            columns: result.columns,
            rows: result.rows,
            rows_affected: result.rows_affected,
        })
    }
}

// Internal outcome of one SQL statement execution.
struct StatementOutcome {
    columns: Vec<String>,
    rows: Vec<Vec<SqlValue>>,
    rows_affected: i64,
}

// ===========================================================================
// Cursor impl
// ===========================================================================

impl Cursor {
    fn new(conn: Rc<RefCell<ConnectionState>>) -> Self {
        Self {
            conn,
            description: Vec::new(),
            rowcount: -1,
            lastrowid: None,
            arraysize: 1,
            rows: Vec::new(),
            offset: 0,
            closed: false,
        }
    }

    /// The number of rows produced (SELECT) or affected (DML) by the last
    /// `execute` call.  `-1` means unknown or not applicable.
    pub fn rowcount(&self) -> isize {
        self.rowcount
    }

    /// The row ID of the last inserted row, or `None` for non-INSERT statements.
    pub fn lastrowid(&self) -> Option<i64> {
        self.lastrowid
    }

    /// The default number of rows fetched by `fetchmany` when `size` is 0.
    pub fn arraysize(&self) -> usize {
        self.arraysize
    }

    /// Set `arraysize`.
    pub fn set_arraysize(&mut self, arraysize: usize) {
        if arraysize > 0 {
            self.arraysize = arraysize;
        }
    }

    /// Execute `sql` with `params` substituted for `?` placeholders.
    ///
    /// The cursor's `description`, `rowcount`, and row buffer are reset before
    /// each call.  Returns `&mut Self` for method chaining.
    pub fn execute(&mut self, sql: &str, params: &[SqlValue]) -> Result<&mut Self> {
        self.assert_open()?;

        // Substitute ? parameters before parsing.
        let bound = bind_parameters(sql, params)?;

        let outcome = self.conn.borrow_mut().run_sql(&bound)?;

        self.description = outcome
            .columns
            .iter()
            .map(|name| ColumnDescription { name: name.clone() })
            .collect();
        self.rows = outcome.rows;
        self.offset = 0;
        // rowcount: for SELECT -1 is conventional; for DML use the affected count.
        self.rowcount = if outcome.rows_affected >= 0 && outcome.columns.is_empty() {
            outcome.rows_affected as isize
        } else {
            -1
        };
        self.lastrowid = None;

        Ok(self)
    }

    /// Execute `sql` for each parameter slice in `params_seq`.
    pub fn executemany(&mut self, sql: &str, params_seq: &[Vec<SqlValue>]) -> Result<&mut Self> {
        self.assert_open()?;
        let mut total: isize = 0;
        for params in params_seq {
            self.execute(sql, params)?;
            if self.rowcount >= 0 {
                total += self.rowcount;
            }
        }
        if !params_seq.is_empty() {
            self.rowcount = total;
        }
        Ok(self)
    }

    /// Fetch the next row, or `None` if exhausted.
    pub fn fetchone(&mut self) -> Option<Vec<SqlValue>> {
        if self.closed || self.offset >= self.rows.len() {
            return None;
        }
        let row = self.rows[self.offset].clone();
        self.offset += 1;
        Some(row)
    }

    /// Fetch up to `size` rows (uses `arraysize` when `size == 0`).
    pub fn fetchmany(&mut self, size: usize) -> Vec<Vec<SqlValue>> {
        if self.closed {
            return Vec::new();
        }
        let count = if size == 0 { self.arraysize } else { size };
        let end = (self.offset + count).min(self.rows.len());
        let rows = self.rows[self.offset..end].to_vec();
        self.offset = end;
        rows
    }

    /// Fetch all remaining rows.
    pub fn fetchall(&mut self) -> Vec<Vec<SqlValue>> {
        if self.closed {
            return Vec::new();
        }
        let rows = self.rows[self.offset..].to_vec();
        self.offset = self.rows.len();
        rows
    }

    /// Close the cursor. Subsequent operations will return `ProgrammingError`.
    pub fn close(&mut self) {
        self.closed = true;
        self.rows.clear();
        self.description.clear();
    }

    fn assert_open(&self) -> Result<()> {
        if self.closed {
            return Err(MiniSqliteError::ProgrammingError(
                "cursor is closed".to_string(),
            ));
        }
        self.conn.borrow().assert_open()
    }
}

// ===========================================================================
// Parameter binding — substitute ? placeholders with SQL literals
// ===========================================================================

/// Substitute `?` placeholders in `sql` with SQL literal representations of
/// the corresponding `params` values.
///
/// The substitution is done at the text level before parsing so that the
/// full SQL pipeline sees a concrete query with no placeholders. String
/// literals and comments are skipped to avoid false `?` matches inside them.
///
/// ## Example
///
/// ```text
/// bind_parameters("SELECT * FROM t WHERE id = ? AND name = ?", [1, "Alice"])
///   → "SELECT * FROM t WHERE id = 1 AND name = 'Alice'"
/// ```
fn bind_parameters(sql: &str, params: &[SqlValue]) -> Result<String> {
    let bytes = sql.as_bytes();
    let mut out = String::with_capacity(sql.len() + params.len() * 8);
    let mut index = 0;
    let mut i = 0;

    while i < bytes.len() {
        let ch = bytes[i] as char;

        // Skip quoted strings (single- or double-quoted identifiers/values).
        if ch == '\'' || ch == '"' {
            let next = read_quoted(sql, i, ch);
            out.push_str(&sql[i..next]);
            i = next;
            continue;
        }

        // Skip line comments `--`.
        if ch == '-' && i + 1 < bytes.len() && bytes[i + 1] == b'-' {
            let next = read_line_comment(sql, i);
            out.push_str(&sql[i..next]);
            i = next;
            continue;
        }

        // Skip block comments `/* … */`.
        if ch == '/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            let next = read_block_comment(sql, i);
            out.push_str(&sql[i..next]);
            i = next;
            continue;
        }

        // Replace `?` with the next parameter.
        if ch == '?' {
            if index >= params.len() {
                return Err(MiniSqliteError::ProgrammingError(
                    "not enough parameters for SQL statement".to_string(),
                ));
            }
            out.push_str(&sql_literal(&params[index])?);
            index += 1;
            i += 1;
            continue;
        }

        out.push(ch);
        i += 1;
    }

    if index != params.len() {
        return Err(MiniSqliteError::ProgrammingError(
            "too many parameters for SQL statement".to_string(),
        ));
    }
    Ok(out)
}

fn read_quoted(sql: &str, start: usize, quote: char) -> usize {
    let bytes = sql.as_bytes();
    let q = quote as u8;
    let mut i = start + 1;
    while i < bytes.len() {
        let ch = bytes[i] as char;
        if ch == '\\' {
            // Backslash escape: skip the next byte.
            i += 2;
            continue;
        }
        if bytes[i] == q {
            // Check for doubled-quote escape sequence (e.g. '' inside a
            // single-quoted SQL literal, or "" inside a double-quoted identifier).
            // If the *next* byte is also the quote character, both bytes belong
            // to the literal; skip them and continue scanning.
            if i + 1 < bytes.len() && bytes[i + 1] == q {
                i += 2;
                continue;
            }
            // Single closing quote: end of the literal.
            return i + 1;
        }
        i += 1;
    }
    bytes.len()
}

fn read_line_comment(sql: &str, start: usize) -> usize {
    let bytes = sql.as_bytes();
    let mut i = start + 2;
    while i < bytes.len() && bytes[i] != b'\n' {
        i += 1;
    }
    i
}

fn read_block_comment(sql: &str, start: usize) -> usize {
    let bytes = sql.as_bytes();
    let mut i = start + 2;
    while i + 1 < bytes.len() {
        if bytes[i] == b'*' && bytes[i + 1] == b'/' {
            return i + 2;
        }
        i += 1;
    }
    bytes.len()
}

/// Convert a `SqlValue` to a SQL literal string.
///
/// | Value type | Literal form        | Example        |
/// |------------|---------------------|----------------|
/// | NULL       | `NULL`              | `NULL`         |
/// | Bool(true) | `TRUE`              | `TRUE`         |
/// | Bool(false)| `FALSE`             | `FALSE`        |
/// | Int(n)     | decimal digits      | `42`           |
/// | Float(f)   | Rust `f64` Display  | `3.14`         |
/// | Text(s)    | single-quoted       | `'hello'`      |
/// | Blob(_)    | unsupported         | —              |
fn sql_literal(value: &SqlValue) -> Result<String> {
    match value {
        SqlValue::Null => Ok("NULL".to_string()),
        SqlValue::Bool(true) => Ok("TRUE".to_string()),
        SqlValue::Bool(false) => Ok("FALSE".to_string()),
        SqlValue::Int(n) => Ok(n.to_string()),
        SqlValue::Float(f) => {
            if !f.is_finite() {
                return Err(MiniSqliteError::ProgrammingError(
                    "non-finite float parameter is not supported".to_string(),
                ));
            }
            // Ensure the literal has a decimal point so the SQL parser
            // recognises it as a REAL constant, not an INTEGER.
            let s = format!("{f}");
            if s.contains('.') || s.contains('e') || s.contains('E') {
                Ok(s)
            } else {
                Ok(format!("{s}.0"))
            }
        }
        SqlValue::Text(s) => Ok(quote_sql_string(s)),
        SqlValue::Blob(_) => Err(MiniSqliteError::NotSupportedError(
            "BLOB parameters are not supported at Level 1".to_string(),
        )),
    }
}

/// Escape a string value for use as a SQL single-quoted literal.
///
/// The escaping rules mirror SQLite: backslash is escaped, single-quotes
/// are doubled (`''`), and common control characters are escaped.
///
/// NUL bytes (`\0`) are stripped.  Some SQL engines treat an embedded NUL as
/// a string terminator; removing them prevents a malicious value from
/// truncating the literal and injecting raw SQL after it.
fn quote_sql_string(value: &str) -> String {
    // Strip NUL bytes before escaping — they have no meaningful SQL representation
    // and can cause string-termination ambiguity in some lexer implementations.
    let sanitized: String = value.chars().filter(|&c| c != '\0').collect();
    let escaped = sanitized
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\t', "\\t");
    format!("'{escaped}'")
}

// ===========================================================================
// Utility: extract the first keyword from a SQL statement
// ===========================================================================

/// Return the first alphabetic word in `sql` (uppercased), or an empty
/// string if there are no alphabetic characters.
///
/// Used to dispatch transaction control statements (`BEGIN`, `COMMIT`,
/// `ROLLBACK`) before feeding the query to the pipeline.
fn first_keyword(sql: &str) -> String {
    sql.trim_start()
        .chars()
        .take_while(|ch| ch.is_ascii_alphabetic())
        .collect::<String>()
        .to_uppercase()
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ── Test helpers ──────────────────────────────────────────────────────────

    fn conn() -> Connection {
        connect(":memory:").expect("connect failed")
    }

    fn exec(c: &Connection, sql: &str) {
        c.execute(sql, &[]).expect(sql);
    }

    fn exec_p(c: &Connection, sql: &str, params: &[SqlValue]) {
        c.execute(sql, params).expect(sql);
    }

    fn query(c: &Connection, sql: &str) -> Vec<Vec<SqlValue>> {
        c.execute(sql, &[])
            .expect(sql)
            .fetchall()
    }

    fn query_p(c: &Connection, sql: &str, params: &[SqlValue]) -> Vec<Vec<SqlValue>> {
        c.execute(sql, params).expect(sql).fetchall()
    }

    // ── Module constants ──────────────────────────────────────────────────────

    #[test]
    fn exposes_module_constants() {
        assert_eq!(API_LEVEL, "2.0");
        assert_eq!(THREAD_SAFETY, 1);
        assert_eq!(PARAM_STYLE, "qmark");
    }

    // ── File-backed connections ───────────────────────────────────────────────

    #[test]
    fn missing_or_invalid_database_file_is_an_operational_error() {
        // A path to a file that does not exist is a runtime failure, not an
        // unsupported feature: file-backed databases are now supported.
        let err = connect("this/path/does/not/exist/nowhere.sqlite").unwrap_err();
        assert!(
            matches!(err, MiniSqliteError::OperationalError(_)),
            "missing file should be OperationalError, got {err:?}"
        );
    }

    // ── CREATE TABLE / INSERT / SELECT * ─────────────────────────────────────

    #[test]
    fn create_table_insert_select() {
        let c = conn();
        exec(&c, "CREATE TABLE users (id INTEGER, name TEXT, age INTEGER)");
        exec(&c, "INSERT INTO users VALUES (1, 'Alice', 30)");
        exec(&c, "INSERT INTO users VALUES (2, 'Bob', 25)");
        exec(&c, "INSERT INTO users VALUES (3, 'Charlie', 35)");

        let rows = query(&c, "SELECT id, name, age FROM users ORDER BY id");
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0][0], SqlValue::Int(1));
        assert_eq!(rows[0][1], SqlValue::Text("Alice".to_string()));
        assert_eq!(rows[2][2], SqlValue::Int(35));
    }

    // ── SELECT WHERE ─────────────────────────────────────────────────────────

    #[test]
    fn select_where() {
        let c = conn();
        exec(&c, "CREATE TABLE t (id INTEGER, val INTEGER)");
        exec(&c, "INSERT INTO t VALUES (1, 10)");
        exec(&c, "INSERT INTO t VALUES (2, 20)");
        exec(&c, "INSERT INTO t VALUES (3, 30)");

        let rows = query(&c, "SELECT id FROM t WHERE val > 15 ORDER BY id");
        assert_eq!(rows, vec![vec![SqlValue::Int(2)], vec![SqlValue::Int(3)]]);
    }

    // ── SELECT ORDER BY ──────────────────────────────────────────────────────

    #[test]
    fn select_order_by() {
        let c = conn();
        exec(&c, "CREATE TABLE t (n INTEGER)");
        exec(&c, "INSERT INTO t VALUES (3)");
        exec(&c, "INSERT INTO t VALUES (1)");
        exec(&c, "INSERT INTO t VALUES (2)");

        let rows = query(&c, "SELECT n FROM t ORDER BY n ASC");
        assert_eq!(
            rows,
            vec![
                vec![SqlValue::Int(1)],
                vec![SqlValue::Int(2)],
                vec![SqlValue::Int(3)],
            ]
        );
    }

    // ── SELECT LIMIT ─────────────────────────────────────────────────────────

    #[test]
    fn select_limit() {
        let c = conn();
        exec(&c, "CREATE TABLE t (n INTEGER)");
        for i in 1..=10 {
            exec_p(&c, "INSERT INTO t VALUES (?)", &[SqlValue::Int(i)]);
        }

        let rows = query(&c, "SELECT n FROM t ORDER BY n LIMIT 3");
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0][0], SqlValue::Int(1));
        assert_eq!(rows[2][0], SqlValue::Int(3));
    }

    // ── SELECT aggregates ────────────────────────────────────────────────────

    #[test]
    fn select_aggregate_count_sum_avg_min_max() {
        let c = conn();
        exec(&c, "CREATE TABLE scores (v INTEGER)");
        exec(&c, "INSERT INTO scores VALUES (10)");
        exec(&c, "INSERT INTO scores VALUES (20)");
        exec(&c, "INSERT INTO scores VALUES (30)");

        let rows = query(&c, "SELECT COUNT(*) AS n, SUM(v) AS s, MIN(v) AS lo, MAX(v) AS hi FROM scores");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][0], SqlValue::Int(3));
        assert_eq!(rows[0][1], SqlValue::Int(60));
        assert_eq!(rows[0][2], SqlValue::Int(10));
        assert_eq!(rows[0][3], SqlValue::Int(30));
    }

    // ── SELECT DISTINCT ───────────────────────────────────────────────────────

    #[test]
    fn select_distinct() {
        let c = conn();
        exec(&c, "CREATE TABLE t (v TEXT)");
        exec(&c, "INSERT INTO t VALUES ('a')");
        exec(&c, "INSERT INTO t VALUES ('b')");
        exec(&c, "INSERT INTO t VALUES ('a')");

        let rows = query(&c, "SELECT DISTINCT v FROM t ORDER BY v");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0][0], SqlValue::Text("a".to_string()));
        assert_eq!(rows[1][0], SqlValue::Text("b".to_string()));
    }

    // ── UPDATE ────────────────────────────────────────────────────────────────

    #[test]
    fn update_with_where() {
        let c = conn();
        exec(&c, "CREATE TABLE counters (name TEXT, value INTEGER)");
        exec(&c, "INSERT INTO counters VALUES ('hits', 10)");
        exec(&c, "INSERT INTO counters VALUES ('misses', 3)");

        exec(&c, "UPDATE counters SET value = 15 WHERE name = 'hits'");

        let rows = query(&c, "SELECT value FROM counters WHERE name = 'hits'");
        assert_eq!(rows, vec![vec![SqlValue::Int(15)]]);

        // Unaffected row stays the same.
        let rows = query(&c, "SELECT value FROM counters WHERE name = 'misses'");
        assert_eq!(rows, vec![vec![SqlValue::Int(3)]]);
    }

    // ── DELETE ────────────────────────────────────────────────────────────────

    #[test]
    fn delete_with_where() {
        let c = conn();
        exec(&c, "CREATE TABLE t (id INTEGER)");
        exec(&c, "INSERT INTO t VALUES (1)");
        exec(&c, "INSERT INTO t VALUES (2)");
        exec(&c, "INSERT INTO t VALUES (3)");

        exec(&c, "DELETE FROM t WHERE id = 2");

        let rows = query(&c, "SELECT id FROM t ORDER BY id");
        assert_eq!(
            rows,
            vec![vec![SqlValue::Int(1)], vec![SqlValue::Int(3)]]
        );
    }

    // ── DROP TABLE ────────────────────────────────────────────────────────────

    #[test]
    fn drop_table() {
        let c = conn();
        exec(&c, "CREATE TABLE t (id INTEGER)");
        exec(&c, "INSERT INTO t VALUES (42)");
        exec(&c, "DROP TABLE t");

        // Table no longer exists; querying it should fail.
        let err = c.execute("SELECT id FROM t", &[]).unwrap_err();
        assert!(matches!(
            err,
            MiniSqliteError::ProgrammingError(_) | MiniSqliteError::OperationalError(_)
        ));
    }

    // ── NULL semantics ────────────────────────────────────────────────────────

    #[test]
    fn null_values_and_is_null() {
        let c = conn();
        exec(&c, "CREATE TABLE maybe (id INTEGER, value TEXT)");
        exec(&c, "INSERT INTO maybe VALUES (1, 'present')");
        exec(&c, "INSERT INTO maybe VALUES (2, NULL)");
        exec(&c, "INSERT INTO maybe VALUES (3, 'also present')");

        let rows = query(&c, "SELECT id FROM maybe WHERE value IS NULL ORDER BY id");
        assert_eq!(rows, vec![vec![SqlValue::Int(2)]]);

        let rows = query(&c, "SELECT id FROM maybe WHERE value IS NOT NULL ORDER BY id");
        assert_eq!(
            rows,
            vec![vec![SqlValue::Int(1)], vec![SqlValue::Int(3)]]
        );
    }

    // ── qmark parameter binding ───────────────────────────────────────────────

    #[test]
    fn qmark_parameter_binding() {
        let c = conn();
        exec(&c, "CREATE TABLE products (id INTEGER, name TEXT, price REAL)");
        exec_p(
            &c,
            "INSERT INTO products VALUES (?, ?, ?)",
            &[SqlValue::Int(1), SqlValue::Text("Widget".to_string()), SqlValue::Float(9.99)],
        );
        exec_p(
            &c,
            "INSERT INTO products VALUES (?, ?, ?)",
            &[SqlValue::Int(2), SqlValue::Text("Gadget".to_string()), SqlValue::Float(24.99)],
        );

        let rows = query_p(
            &c,
            "SELECT id, name FROM products WHERE price < ? ORDER BY id",
            &[SqlValue::Float(15.0)],
        );
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][0], SqlValue::Int(1));
        assert_eq!(rows[0][1], SqlValue::Text("Widget".to_string()));
    }

    // ── Wrong parameter count ─────────────────────────────────────────────────

    #[test]
    fn wrong_param_count_raises_programming_error() {
        let c = conn();
        exec(&c, "CREATE TABLE t (a INTEGER, b INTEGER)");

        let err = c.execute("INSERT INTO t VALUES (?, ?)", &[SqlValue::Int(1)]).unwrap_err();
        assert!(matches!(err, MiniSqliteError::ProgrammingError(_)));

        let err = c
            .execute(
                "INSERT INTO t VALUES (?, ?)",
                &[SqlValue::Int(1), SqlValue::Int(2), SqlValue::Int(3)],
            )
            .unwrap_err();
        assert!(matches!(err, MiniSqliteError::ProgrammingError(_)));
    }

    // ── fetchone / fetchmany ──────────────────────────────────────────────────

    #[test]
    fn fetchone_fetchmany_fetchall() {
        let c = conn();
        exec(&c, "CREATE TABLE nums (n INTEGER)");
        for i in 1..=5 {
            exec_p(&c, "INSERT INTO nums VALUES (?)", &[SqlValue::Int(i)]);
        }

        let mut cursor = c.execute("SELECT n FROM nums ORDER BY n", &[]).unwrap();
        assert_eq!(cursor.fetchone(), Some(vec![SqlValue::Int(1)]));
        assert_eq!(cursor.fetchone(), Some(vec![SqlValue::Int(2)]));
        let rest = cursor.fetchall();
        assert_eq!(rest.len(), 3);
        assert_eq!(rest[0][0], SqlValue::Int(3));
    }

    // ── commit / rollback ─────────────────────────────────────────────────────

    #[test]
    fn commit_persists_changes() {
        let c = conn();
        exec(&c, "CREATE TABLE accounts (owner TEXT, balance INTEGER)");
        exec(&c, "INSERT INTO accounts VALUES ('alice', 1000)");
        c.commit().unwrap();

        exec(&c, "UPDATE accounts SET balance = 900 WHERE owner = 'alice'");
        c.commit().unwrap();

        let rows = query(&c, "SELECT balance FROM accounts WHERE owner = 'alice'");
        assert_eq!(rows, vec![vec![SqlValue::Int(900)]]);
    }

    #[test]
    fn rollback_reverts_changes() {
        let c = conn();
        exec(&c, "CREATE TABLE t (id INTEGER)");
        c.commit().unwrap();

        exec(&c, "INSERT INTO t VALUES (99)");
        c.rollback().unwrap();

        let rows = query(&c, "SELECT COUNT(*) AS n FROM t");
        assert_eq!(rows, vec![vec![SqlValue::Int(0)]]);
    }

    // ── executemany ───────────────────────────────────────────────────────────

    #[test]
    fn executemany_inserts_all_rows() {
        let c = conn();
        exec(&c, "CREATE TABLE t (v INTEGER)");
        c.executemany(
            "INSERT INTO t VALUES (?)",
            &[
                vec![SqlValue::Int(10)],
                vec![SqlValue::Int(20)],
                vec![SqlValue::Int(30)],
            ],
        )
        .unwrap();

        let rows = query(&c, "SELECT COUNT(*) AS n FROM t");
        assert_eq!(rows, vec![vec![SqlValue::Int(3)]]);
    }

    // ── cursor description ────────────────────────────────────────────────────

    #[test]
    fn cursor_description_reflects_columns() {
        let c = conn();
        exec(&c, "CREATE TABLE t (id INTEGER, name TEXT)");
        exec(&c, "INSERT INTO t VALUES (1, 'x')");

        let cursor = c.execute("SELECT id, name FROM t", &[]).unwrap();
        assert_eq!(cursor.description.len(), 2);
        assert_eq!(cursor.description[0].name.to_lowercase(), "id");
        assert_eq!(cursor.description[1].name.to_lowercase(), "name");
    }

    // ── projection aliases ────────────────────────────────────────────────────

    #[test]
    fn projection_aliases() {
        let c = conn();
        exec(&c, "CREATE TABLE t (id INTEGER, name TEXT)");
        exec(&c, "INSERT INTO t VALUES (1, 'Alice')");

        let mut cursor = c.execute("SELECT id AS user_id, name AS user_name FROM t", &[]).unwrap();
        let cols: Vec<String> = cursor.description.iter().map(|d| d.name.to_lowercase()).collect();
        assert!(cols.contains(&"user_id".to_string()));
        assert!(cols.contains(&"user_name".to_string()));
        let rows = cursor.fetchall();
        assert_eq!(rows[0][0], SqlValue::Int(1));
        assert_eq!(rows[0][1], SqlValue::Text("Alice".to_string()));
    }

    // ── error: unknown table ──────────────────────────────────────────────────

    #[test]
    fn error_unknown_table() {
        let c = conn();
        let err = c.execute("SELECT * FROM no_such_table", &[]).unwrap_err();
        assert!(matches!(
            err,
            MiniSqliteError::ProgrammingError(_) | MiniSqliteError::OperationalError(_)
        ));
    }

    // ══════════════════════════════════════════════════════════════════════════
    // Level 1 conformance: conformance fixture runner
    //
    // We load every fixture from
    // `code/specs/mini-sqlite-conformance/fixtures/` and run it against a
    // fresh connection, verifying columns and rows for `query` steps, checking
    // error types for `expect_error` and `connect_expect_error` steps, etc.
    // ══════════════════════════════════════════════════════════════════════════

    mod conformance {
        use super::*;
        use std::path::PathBuf;

        // ── JSON value → SqlValue conversion ─────────────────────────────────

        fn json_to_sql(v: &serde_json::Value) -> SqlValue {
            match v {
                serde_json::Value::Null => SqlValue::Null,
                serde_json::Value::Bool(b) => SqlValue::Bool(*b),
                serde_json::Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        SqlValue::Int(i)
                    } else if let Some(f) = n.as_f64() {
                        SqlValue::Float(f)
                    } else {
                        SqlValue::Null
                    }
                }
                serde_json::Value::String(s) => SqlValue::Text(s.clone()),
                other => SqlValue::Text(other.to_string()),
            }
        }

        fn json_array_to_params(arr: Option<&serde_json::Value>) -> Vec<SqlValue> {
            match arr {
                None => Vec::new(),
                Some(serde_json::Value::Array(items)) => {
                    items.iter().map(json_to_sql).collect()
                }
                _ => Vec::new(),
            }
        }

        // ── Fixture path resolution ───────────────────────────────────────────

        fn fixture_dir() -> PathBuf {
            // The worktree root is several directories up from the crate.
            // We walk up from CARGO_MANIFEST_DIR (…/mini-sqlite) to find
            // code/specs/mini-sqlite-conformance/fixtures/.
            let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
                .expect("CARGO_MANIFEST_DIR not set");
            let mut p = PathBuf::from(manifest_dir);
            // …/mini-sqlite → …/rust → …/packages → …/code → repo root
            for _ in 0..4 {
                p.pop();
            }
            p.push("code");
            p.push("specs");
            p.push("mini-sqlite-conformance");
            p.push("fixtures");
            p
        }

        // ── Run a single fixture file ─────────────────────────────────────────

        fn run_fixture(path: &std::path::Path) {
            let content = std::fs::read_to_string(path)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
            let fixture: serde_json::Value = serde_json::from_str(&content)
                .unwrap_or_else(|e| panic!("invalid JSON in {}: {e}", path.display()));

            let fixture_id = fixture["id"].as_str().unwrap_or("?");

            // Handle `connect_steps` (Level 0 file-path error tests).
            if let Some(steps) = fixture.get("connect_steps") {
                for step in steps.as_array().unwrap() {
                    let op = step["op"].as_str().unwrap_or("");
                    if op == "connect_expect_error" {
                        let db = step["database"].as_str().unwrap_or(":memory:");
                        // A file-path connect must be rejected at Level 1; an Err
                        // is the expected outcome, so only success is a failure.
                        if connect(db).is_ok() {
                            panic!(
                                "{fixture_id}: connect({db:?}) should have returned Err but succeeded"
                            );
                        }
                    }
                }
                return; // connect_steps fixture handled
            }

            // Regular `steps` fixture.
            let steps = match fixture.get("steps") {
                Some(s) => s.as_array().unwrap(),
                None => return,
            };

            let c = conn();

            for (step_idx, step) in steps.iter().enumerate() {
                let op = step["op"].as_str().unwrap_or("");
                let sql = step.get("sql").and_then(|v| v.as_str()).unwrap_or("");
                let params = json_array_to_params(step.get("params"));

                match op {
                    "execute" => {
                        c.execute(sql, &params).unwrap_or_else(|e| {
                            panic!("{fixture_id} step {step_idx}: execute({sql:?}) failed: {e}");
                        });
                    }

                    "executemany" => {
                        // Fixtures use "param_seq" for the batch parameter list.
                        let param_seq: Vec<Vec<SqlValue>> = step
                            .get("param_seq")
                            .and_then(|v| v.as_array())
                            .unwrap_or(&vec![])
                            .iter()
                            .map(|row| {
                                row.as_array()
                                    .unwrap_or(&vec![])
                                    .iter()
                                    .map(json_to_sql)
                                    .collect()
                            })
                            .collect();
                        c.executemany(sql, &param_seq).unwrap_or_else(|e| {
                            panic!("{fixture_id} step {step_idx}: executemany({sql:?}) failed: {e}");
                        });
                    }

                    "query" => {
                        let mut cursor = c.execute(sql, &params).unwrap_or_else(|e| {
                            panic!("{fixture_id} step {step_idx}: query({sql:?}) failed: {e}");
                        });

                        // Column name check (case-insensitive).
                        if let Some(exp_cols) = step.get("expected_columns").and_then(|v| v.as_array()) {
                            let got_cols: Vec<String> = cursor
                                .description
                                .iter()
                                .map(|d| d.name.to_lowercase())
                                .collect();
                            let exp_lower: Vec<String> = exp_cols
                                .iter()
                                .map(|v| v.as_str().unwrap_or("").to_lowercase())
                                .collect();
                            assert_eq!(
                                got_cols, exp_lower,
                                "{fixture_id} step {step_idx}: columns mismatch for {sql:?}"
                            );
                        }

                        // Row check.
                        if let Some(exp_rows) = step.get("expected_rows").and_then(|v| v.as_array()) {
                            let got_rows = cursor.fetchall();
                            assert_eq!(
                                got_rows.len(),
                                exp_rows.len(),
                                "{fixture_id} step {step_idx}: row count mismatch for {sql:?}: got {} want {}",
                                got_rows.len(),
                                exp_rows.len()
                            );
                            for (ri, (got_row, exp_row)) in got_rows.iter().zip(exp_rows.iter()).enumerate() {
                                let exp_vals: Vec<SqlValue> = exp_row
                                    .as_array()
                                    .unwrap_or(&vec![])
                                    .iter()
                                    .map(json_to_sql)
                                    .collect();
                                assert_eq!(
                                    got_row.len(),
                                    exp_vals.len(),
                                    "{fixture_id} step {step_idx} row {ri}: column count mismatch"
                                );
                                for (ci, (g, e)) in got_row.iter().zip(exp_vals.iter()).enumerate() {
                                    // Float comparison with tolerance.
                                    match (g, e) {
                                        (SqlValue::Float(gf), SqlValue::Float(ef)) => {
                                            assert!(
                                                (gf - ef).abs() < 1e-9,
                                                "{fixture_id} step {step_idx} row {ri} col {ci}: float mismatch: got {gf} want {ef}"
                                            );
                                        }
                                        (SqlValue::Float(gf), SqlValue::Int(ei)) => {
                                            let ef = *ei as f64;
                                            assert!(
                                                (gf - ef).abs() < 1e-9,
                                                "{fixture_id} step {step_idx} row {ri} col {ci}: float/int mismatch: got {gf} want {ei}"
                                            );
                                        }
                                        _ => {
                                            assert_eq!(
                                                g, e,
                                                "{fixture_id} step {step_idx} row {ri} col {ci}: value mismatch"
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }

                    "fetchone_test" => {
                        // Fixtures use "expected_first" / "expected_second" for the
                        // first and second fetchone() calls.  Fall back to an
                        // "expected_rows" array for backwards compatibility.
                        let mut cursor = c.execute(sql, &params).unwrap_or_else(|e| {
                            panic!("{fixture_id} step {step_idx}: {sql:?} failed: {e}");
                        });
                        if let (Some(first), Some(second)) = (
                            step.get("expected_first").and_then(|v| v.as_array()),
                            step.get("expected_second").and_then(|v| v.as_array()),
                        ) {
                            let got1 = cursor.fetchone();
                            let exp1: Vec<SqlValue> = first.iter().map(json_to_sql).collect();
                            assert_eq!(got1, Some(exp1), "{fixture_id} step {step_idx} fetchone 0");
                            let got2 = cursor.fetchone();
                            let exp2: Vec<SqlValue> = second.iter().map(json_to_sql).collect();
                            assert_eq!(got2, Some(exp2), "{fixture_id} step {step_idx} fetchone 1");
                        } else if let Some(exp) = step.get("expected_rows").and_then(|v| v.as_array()) {
                            for (i, exp_row) in exp.iter().enumerate() {
                                let got = cursor.fetchone();
                                let exp_vals: Vec<SqlValue> = exp_row.as_array().unwrap().iter().map(json_to_sql).collect();
                                assert_eq!(got, Some(exp_vals), "{fixture_id} step {step_idx} fetchone {i}");
                            }
                        }
                    }

                    "fetchmany_test" => {
                        let size = step["size"].as_u64().unwrap_or(1) as usize;
                        let mut cursor = c.execute(sql, &params).unwrap_or_else(|e| {
                            panic!("{fixture_id} step {step_idx}: {sql:?} failed: {e}");
                        });
                        // Fixtures use "expected_first_batch" + "expected_second_batch".
                        // Fall back to "expected_batches" array for backwards compat.
                        if let (Some(b1), Some(b2)) = (
                            step.get("expected_first_batch").and_then(|v| v.as_array()),
                            step.get("expected_second_batch").and_then(|v| v.as_array()),
                        ) {
                            let got1 = cursor.fetchmany(size);
                            let exp1: Vec<Vec<SqlValue>> = b1.iter().map(|row| row.as_array().unwrap().iter().map(json_to_sql).collect()).collect();
                            assert_eq!(got1, exp1, "{fixture_id} step {step_idx} fetchmany batch 0");
                            let got2 = cursor.fetchmany(size);
                            let exp2: Vec<Vec<SqlValue>> = b2.iter().map(|row| row.as_array().unwrap().iter().map(json_to_sql).collect()).collect();
                            assert_eq!(got2, exp2, "{fixture_id} step {step_idx} fetchmany batch 1");
                        } else if let Some(exp_batches) = step.get("expected_batches").and_then(|v| v.as_array()) {
                            for (bi, exp_batch) in exp_batches.iter().enumerate() {
                                let got = cursor.fetchmany(size);
                                let exp: Vec<Vec<SqlValue>> = exp_batch
                                    .as_array()
                                    .unwrap()
                                    .iter()
                                    .map(|row| {
                                        row.as_array().unwrap().iter().map(json_to_sql).collect()
                                    })
                                    .collect();
                                assert_eq!(got, exp, "{fixture_id} step {step_idx} fetchmany batch {bi}");
                            }
                        }
                    }

                    "fetchall_test" | "fetchall_empty_test" => {
                        let mut cursor = c.execute(sql, &params).unwrap_or_else(|e| {
                            panic!("{fixture_id} step {step_idx}: {sql:?} failed: {e}");
                        });
                        let got = cursor.fetchall();
                        if op == "fetchall_empty_test" {
                            assert!(got.is_empty(), "{fixture_id} step {step_idx}: expected empty result");
                        } else if let Some(exp_rows) = step.get("expected_rows") {
                            let exp: Vec<Vec<SqlValue>> = exp_rows
                                .as_array()
                                .unwrap()
                                .iter()
                                .map(|row| row.as_array().unwrap().iter().map(json_to_sql).collect())
                                .collect();
                            assert_eq!(got, exp, "{fixture_id} step {step_idx}: fetchall mismatch");
                        }
                    }

                    "commit" => {
                        c.commit().unwrap_or_else(|e| {
                            panic!("{fixture_id} step {step_idx}: commit failed: {e}");
                        });
                    }

                    "rollback" => {
                        c.rollback().unwrap_or_else(|e| {
                            panic!("{fixture_id} step {step_idx}: rollback failed: {e}");
                        });
                    }

                    "expect_error" => {
                        let result = c.execute(sql, &params);
                        assert!(
                            result.is_err(),
                            "{fixture_id} step {step_idx}: expected error from {sql:?} but got Ok"
                        );
                        // Optionally check the error variant name.
                        if let Some(expected_type) = step.get("error_type").and_then(|v| v.as_str()) {
                            let actual_err = result.unwrap_err();
                            let actual_type = match &actual_err {
                                MiniSqliteError::ProgrammingError(_) => "ProgrammingError",
                                MiniSqliteError::OperationalError(_) => "OperationalError",
                                MiniSqliteError::IntegrityError(_) => "IntegrityError",
                                MiniSqliteError::NotSupportedError(_) => "NotSupportedError",
                            };
                            assert_eq!(
                                actual_type, expected_type,
                                "{fixture_id} step {step_idx}: wrong error type for {sql:?}"
                            );
                        }
                    }

                    "connect_expect_error" => {
                        let db = step["database"].as_str().unwrap_or(":memory:");
                        assert!(
                            connect(db).is_err(),
                            "{fixture_id} step {step_idx}: expected connect({db:?}) to fail"
                        );
                    }

                    other => {
                        // Unknown ops are silently skipped (forward-compat).
                        eprintln!("mini-sqlite conformance: unknown op {other:?} in {fixture_id}");
                    }
                }
            }
        }

        // ── One test per fixture file ─────────────────────────────────────────

        #[test]
        fn fixture_01_create_select() {
            run_fixture(&fixture_dir().join("01-create-select.json"));
        }
        #[test]
        fn fixture_02_qmark_binding_insert() {
            run_fixture(&fixture_dir().join("02-qmark-binding-insert.json"));
        }
        #[test]
        fn fixture_03_projection_aliases() {
            run_fixture(&fixture_dir().join("03-projection-aliases.json"));
        }
        #[test]
        fn fixture_04_where_filtering() {
            run_fixture(&fixture_dir().join("04-where-filtering.json"));
        }
        #[test]
        fn fixture_05_order_by_limit_offset() {
            run_fixture(&fixture_dir().join("05-order-by-limit-offset.json"));
        }
        #[test]
        fn fixture_06_aggregates() {
            run_fixture(&fixture_dir().join("06-aggregates.json"));
        }
        #[test]
        fn fixture_07_update_delete() {
            run_fixture(&fixture_dir().join("07-update-delete.json"));
        }
        #[test]
        fn fixture_08_transaction_commit() {
            run_fixture(&fixture_dir().join("08-transaction-commit.json"));
        }
        #[test]
        fn fixture_09_transaction_rollback() {
            run_fixture(&fixture_dir().join("09-transaction-rollback.json"));
        }
        #[test]
        fn fixture_10_error_wrong_param_count() {
            run_fixture(&fixture_dir().join("10-error-wrong-param-count.json"));
        }
        #[test]
        fn fixture_11_error_unknown_table() {
            run_fixture(&fixture_dir().join("11-error-unknown-table.json"));
        }
        #[test]
        fn fixture_12_error_file_path_level0() {
            run_fixture(&fixture_dir().join("12-error-file-path-level0.json"));
        }
        #[test]
        fn fixture_13_drop_table() {
            run_fixture(&fixture_dir().join("13-drop-table.json"));
        }
        #[test]
        fn fixture_14_executemany() {
            run_fixture(&fixture_dir().join("14-executemany.json"));
        }
        #[test]
        fn fixture_15_fetchone_fetchmany() {
            run_fixture(&fixture_dir().join("15-fetchone-fetchmany.json"));
        }
        #[test]
        fn fixture_16_null_handling() {
            run_fixture(&fixture_dir().join("16-null-handling.json"));
        }
        #[test]
        fn fixture_17_null_aggregate_semantics() {
            run_fixture(&fixture_dir().join("17-null-aggregate-semantics.json"));
        }
        #[test]
        fn fixture_18_string_functions() {
            run_fixture(&fixture_dir().join("18-string-functions.json"));
        }
        #[test]
        fn fixture_19_math_functions() {
            run_fixture(&fixture_dir().join("19-math-functions.json"));
        }
        #[test]
        fn fixture_20_limit_edge_cases() {
            run_fixture(&fixture_dir().join("20-limit-edge-cases.json"));
        }
        #[test]
        fn fixture_21_distinct_aggregate() {
            run_fixture(&fixture_dir().join("21-distinct-aggregate.json"));
        }
        #[test]
        fn fixture_22_string_concat_null() {
            run_fixture(&fixture_dir().join("22-string-concat-null.json"));
        }
        #[test]
        fn fixture_23_null_in_order_by() {
            run_fixture(&fixture_dir().join("23-null-in-order-by.json"));
        }
        #[test]
        fn fixture_24_having_aggregate() {
            run_fixture(&fixture_dir().join("24-having-aggregate.json"));
        }
    }
}
