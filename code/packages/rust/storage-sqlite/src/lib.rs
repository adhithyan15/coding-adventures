//! # storage-sqlite — a real `.sqlite` file as a query-engine backend
//!
//! The mini-sqlite pipeline (`sql-lexer → parser → planner → optimizer →
//! codegen → sql-vm`) reads and writes tables through one seam: the
//! [`Backend`](coding_adventures_sql_backend::Backend) trait. Today the only
//! implementation is an in-memory table store. This crate adds a **file-backed**
//! one: [`SqliteFileBackend`] exposes a genuine SQLite database file as a
//! `Backend`, so the *unmodified* query engine can run `SELECT` against a real
//! `.sqlite` on disk.
//!
//! It is the Rust sibling of `python/storage-sqlite`, which already proved this
//! architecture end to end. It is built entirely on the from-scratch,
//! zero-dependency [`sqlite_file`] reader (the Phase E work), so reading a real
//! database pulls in **no** third-party SQLite — the real library appears only as
//! a dev-dependency oracle in the tests.
//!
//! ## Scope (this increment)
//!
//! **Read-only.** The three read methods of `Backend` are implemented for real —
//! [`SqliteFileBackend::tables`], [`SqliteFileBackend::columns`], and
//! [`SqliteFileBackend::scan`] — which is everything a `SELECT` needs (the VM
//! reads a table by opening a scan over it and resolving columns by name). Every
//! mutating method (`insert`/`update`/`delete`, DDL, indexes) returns
//! `BackendError::Unsupported`; writing a byte-compatible file is a later
//! increment (the storage engine's Phase-F writer). Wiring mini-sqlite's
//! `connect()` to open a file through this backend is the next step after this.
//!
//! ## The two things this layer reconciles
//!
//! 1. **Column names.** The on-disk format stores a table's rows but not, per
//!    row, its column names — those live once in the `sqlite_schema` catalog as
//!    the original `CREATE TABLE` text. [`parse_create_columns`] recovers the
//!    column list from that text.
//! 2. **The rowid alias.** A column declared `INTEGER PRIMARY KEY` is an alias
//!    for the row's 64-bit `rowid`; SQLite stores `NULL` for it in the record and
//!    keeps the real value as the rowid. `scan` substitutes the rowid back in, so
//!    `SELECT *` returns what the real library returns.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use coding_adventures_sql_backend::{
    Backend, BackendError, ColumnDef, Cursor, IndexDef, ListRowIterator, Row, RowIterator,
    SqlValue, TransactionHandle,
};
use sqlite_file::{
    read_schema, read_table, read_without_rowid_table, SchemaEntry, SqlValue as FileValue,
    SqliteError,
};

/// A read-only [`Backend`] over the bytes of a SQLite database file.
///
/// Construct with [`SqliteFileBackend::open`], then hand it to the query engine
/// like any other backend. The database is held in memory (the importer already
/// hands us the deserialized bytes); the schema catalog is parsed once up front
/// so table/column lookups don't re-walk page 1 every time.
pub struct SqliteFileBackend {
    data: Vec<u8>,
    schema: Vec<SchemaEntry>,
}

impl SqliteFileBackend {
    /// Open a database from its raw file bytes. Parses (and thereby validates)
    /// the `sqlite_schema` catalog up front; a non-SQLite or corrupt buffer
    /// fails here rather than on first query.
    pub fn open(data: Vec<u8>) -> Result<Self, SqliteError> {
        let schema = read_schema(&data)?;
        Ok(Self { data, schema })
    }

    /// The `sqlite_schema` row for a user table named `table` (case-insensitive),
    /// or `None` if there is no such table.
    fn table_entry(&self, table: &str) -> Option<&SchemaEntry> {
        self.schema
            .iter()
            .find(|e| e.object_type == "table" && e.name.eq_ignore_ascii_case(table))
    }

    /// Is `name` the schema catalog table? SQLite exposes the catalog under two
    /// interchangeable names, `sqlite_master` (historical) and `sqlite_schema`
    /// (modern), both case-insensitive. It is not returned by [`Backend::tables`]
    /// (nor by SQLite's `.tables`), but `SELECT … FROM sqlite_master` must work —
    /// applications, Anki included, introspect the database this way.
    fn is_schema_table(name: &str) -> bool {
        name.eq_ignore_ascii_case("sqlite_master") || name.eq_ignore_ascii_case("sqlite_schema")
    }

    /// The fixed five-column shape of `sqlite_master`, matching real SQLite:
    /// `CREATE TABLE sqlite_master(type text, name text, tbl_name text, rootpage
    /// integer, sql text)`.
    fn schema_column_defs() -> Vec<ColumnDef> {
        vec![
            ColumnDef::new("type", "text"),
            ColumnDef::new("name", "text"),
            ColumnDef::new("tbl_name", "text"),
            ColumnDef::new("rootpage", "integer"),
            ColumnDef::new("sql", "text"),
        ]
    }

    /// Project the parsed schema catalog into `sqlite_master` rows — one per
    /// catalog object (tables, indexes, views, triggers), in the b-tree/rowid
    /// order SQLite itself stores them. `rootpage` is `0` for objects with no
    /// b-tree (views and triggers), exactly as SQLite reports it; `sql` is NULL
    /// only when SQLite stored no text (e.g. auto-created indexes).
    fn schema_rows(&self) -> Vec<Row> {
        self.schema
            .iter()
            .map(|e| {
                let mut row: Row = BTreeMap::new();
                row.insert("type".to_string(), SqlValue::Text(e.object_type.clone()));
                row.insert("name".to_string(), SqlValue::Text(e.name.clone()));
                row.insert("tbl_name".to_string(), SqlValue::Text(e.table_name.clone()));
                row.insert(
                    "rootpage".to_string(),
                    SqlValue::Int(e.root_page.unwrap_or(0) as i64),
                );
                row.insert(
                    "sql".to_string(),
                    e.sql.clone().map_or(SqlValue::Null, SqlValue::Text),
                );
                row
            })
            .collect()
    }
}

/// Map a value decoded by the on-disk reader onto the query engine's value type.
/// The two enums line up one-to-one except that the file layer has no boolean
/// (SQLite stores booleans as `0`/`1` integers), so nothing produces `Bool`.
fn file_value_to_backend(value: FileValue) -> SqlValue {
    match value {
        FileValue::Null => SqlValue::Null,
        FileValue::Int(i) => SqlValue::Int(i),
        FileValue::Real(f) => SqlValue::Float(f),
        FileValue::Text(s) => SqlValue::Text(s),
        FileValue::Blob(b) => SqlValue::Blob(b),
    }
}

/// Assemble one query-engine [`Row`] from a record's raw column values.
///
/// `rowid` is `Some` for an ordinary table (so an `INTEGER PRIMARY KEY` column,
/// stored as `NULL` in the record, can be materialized from it) and `None` for a
/// `WITHOUT ROWID` table (which stores every column directly and has no rowid).
/// REAL affinity is applied either way: an integer stored in a REAL column is
/// presented back as a float, matching SQLite's integer-storage optimization.
fn build_row(values: &[FileValue], columns: &[ParsedColumn], rowid: Option<i64>) -> Row {
    let mut row: Row = BTreeMap::new();
    for (i, col) in columns.iter().enumerate() {
        let decoded = values
            .get(i)
            .cloned()
            .map(file_value_to_backend)
            .unwrap_or(SqlValue::Null);
        // Materialize a rowid-alias column from the rowid, but only for a rowid
        // table (`rowid.is_some()`); a WITHOUT ROWID table stores the value.
        let value = match (col.is_rowid_alias, &decoded, rowid) {
            (true, SqlValue::Null, Some(r)) => SqlValue::Int(r),
            _ => decoded,
        };
        let value = match value {
            SqlValue::Int(n) if col.real_affinity => SqlValue::Float(n as f64),
            other => other,
        };
        row.insert(col.name.clone(), value);
    }
    row
}

/// Does this `CREATE TABLE` declare a `WITHOUT ROWID` table? Such a table stores
/// its rows in an index b-tree keyed by the primary key — read via
/// [`read_without_rowid_table`], not [`read_table`].
///
/// The `WITHOUT ROWID` clause follows the closing paren of the column list, so we
/// test only the text *after* the outermost parentheses (case-insensitively, with
/// whitespace normalized). Checking the whole SQL would risk a false positive
/// from a column name or string literal that happened to contain the words.
fn is_without_rowid(create_sql: &str) -> bool {
    // The column list's closing paren is the last `)` in a well-formed CREATE
    // TABLE (any inner `PRIMARY KEY(...)` closes before it).
    match create_sql.rfind(')') {
        Some(idx) => {
            let tail: String = create_sql[idx + 1..]
                .to_ascii_uppercase()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            tail.contains("WITHOUT ROWID")
        }
        None => false,
    }
}

/// One column recovered from a `CREATE TABLE` statement.
struct ParsedColumn {
    name: String,
    type_name: String,
    /// Whether this column is declared `INTEGER PRIMARY KEY` — i.e. a rowid
    /// alias, which SQLite stores as `NULL` in the record and materializes from
    /// the rowid on read.
    is_rowid_alias: bool,
    /// Whether this column has **REAL affinity**. It matters on read: SQLite
    /// stores a float with no fractional part (e.g. `9.0`) as an integer to save
    /// space, then presents it back as a float *because the column is REAL*. A
    /// raw record read sees the integer; we must coerce it back.
    real_affinity: bool,
}

/// Does a declared type name give a column **REAL affinity**, per SQLite's column
/// affinity rules? REAL affinity applies when the type contains none of the
/// higher-precedence markers (`INT` → INTEGER, `CHAR`/`CLOB`/`TEXT` → TEXT,
/// `BLOB` → none) and does contain `REAL`, `FLOA`, or `DOUB`.
fn affinity_is_real(type_name: &str) -> bool {
    let t = type_name.to_ascii_uppercase();
    if t.contains("INT") || t.contains("CHAR") || t.contains("CLOB") || t.contains("TEXT") {
        return false;
    }
    if t.contains("BLOB") {
        return false;
    }
    t.contains("REAL") || t.contains("FLOA") || t.contains("DOUB")
}

/// Recover the column list from a table's `CREATE TABLE …` text.
///
/// The interesting content is the comma-separated list inside the outermost
/// parentheses. We split it at *top-level* commas (commas nested inside a
/// column's own parentheses — e.g. `DECIMAL(10, 2)` or a `CHECK(...)` — don't
/// separate columns), skip any *table-level* constraint clause (one that starts
/// with `CONSTRAINT`/`PRIMARY`/`UNIQUE`/`CHECK`/`FOREIGN`), and read each real
/// column's name and declared type.
///
/// This covers the ordinary schemas mini-sqlite and the Anki tables use. Exotic
/// corners (deeply nested constraint expressions, unusual quoting) are refined in
/// later increments; the cross-check tests measure it against real SQLite.
/// Recover the indexed column names from a `CREATE INDEX … ON t (a, b, …)`
/// statement — the parenthesised list, split at top level and reduced to each
/// leading identifier (so `col DESC` or `col COLLATE NOCASE` yields `col`).
/// Expression indexes (`ON t (a + b)`) degrade to their first identifier, which
/// is the best a name-only view can offer.
fn parse_index_columns(create_index_sql: &str) -> Vec<String> {
    let Some(inner) = outermost_parens(create_index_sql) else {
        return Vec::new();
    };
    split_top_level_commas(inner)
        .into_iter()
        .filter_map(|piece| {
            let (name, _) = read_identifier(piece);
            (!name.is_empty()).then_some(name)
        })
        .collect()
}

/// Whether a `CREATE INDEX` statement declares a UNIQUE index. We scan only the
/// tokens *before* the column-list `(`, and stop at `INDEX`, so an index or
/// table whose name merely contains "unique" is not misread as unique.
fn index_is_unique(create_index_sql: &str) -> bool {
    let head = create_index_sql.split('(').next().unwrap_or("");
    for tok in head.split_whitespace() {
        match tok.to_ascii_uppercase().as_str() {
            "UNIQUE" => return true,
            "INDEX" => return false,
            _ => {}
        }
    }
    false
}

fn parse_create_columns(create_sql: &str) -> Vec<ParsedColumn> {
    let Some(inner) = outermost_parens(create_sql) else {
        return Vec::new();
    };

    let mut columns = Vec::new();
    for piece in split_top_level_commas(inner) {
        let piece = piece.trim();
        if piece.is_empty() || is_table_constraint(piece) {
            continue;
        }
        let (name, rest) = read_identifier(piece);
        if name.is_empty() {
            continue;
        }
        let rest = rest.trim();
        let type_name = read_type_name(rest);
        let is_rowid_alias = column_is_rowid_alias(&type_name, rest);
        let real_affinity = affinity_is_real(&type_name);
        columns.push(ParsedColumn {
            name,
            type_name,
            is_rowid_alias,
            real_affinity,
        });
    }
    columns
}

/// The text between the first `(` and its matching `)`, or `None` if unbalanced.
fn outermost_parens(sql: &str) -> Option<&str> {
    let start = sql.find('(')?;
    let bytes = sql.as_bytes();
    let mut depth = 0usize;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        match b {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&sql[start + 1..i]);
                }
            }
            _ => {}
        }
    }
    None
}

/// Split `list` at commas that sit at parenthesis depth zero.
fn split_top_level_commas(list: &str) -> Vec<&str> {
    let mut pieces = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (i, ch) in list.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                pieces.push(&list[start..i]);
                start = i + ch.len_utf8();
            }
            _ => {}
        }
    }
    pieces.push(&list[start..]);
    pieces
}

/// Whether a comma-separated piece is a *table-level* constraint rather than a
/// column definition.
fn is_table_constraint(piece: &str) -> bool {
    let upper = piece.trim_start().to_ascii_uppercase();
    ["CONSTRAINT", "PRIMARY", "UNIQUE", "CHECK", "FOREIGN"]
        .iter()
        .any(|kw| starts_with_keyword(&upper, kw))
}

/// Whether `upper` begins with SQL keyword `kw` followed by a word boundary.
fn starts_with_keyword(upper: &str, kw: &str) -> bool {
    upper.strip_prefix(kw).is_some_and(|rest| {
        rest.is_empty()
            || rest
                .chars()
                .next()
                .is_some_and(|c| !c.is_alphanumeric() && c != '_')
    })
}

/// Read a leading column identifier, honoring the four quoting styles SQLite
/// accepts (`"a"`, `` `a` ``, `[a]`, or a bare word). Returns the unquoted name
/// and the remaining text after it.
fn read_identifier(piece: &str) -> (String, &str) {
    let piece = piece.trim_start();
    let mut chars = piece.char_indices();
    match chars.next() {
        Some((_, q @ ('"' | '`'))) => {
            // Quoted; a doubled quote is an escaped literal quote.
            let mut name = String::new();
            let bytes = piece.char_indices().skip(1).collect::<Vec<_>>();
            let mut i = 0;
            while i < bytes.len() {
                let (idx, c) = bytes[i];
                if c == q {
                    if bytes.get(i + 1).map(|(_, n)| *n) == Some(q) {
                        name.push(q);
                        i += 2;
                        continue;
                    }
                    let after = idx + c.len_utf8();
                    return (name, &piece[after..]);
                }
                name.push(c);
                i += 1;
            }
            (name, "")
        }
        Some((_, '[')) => {
            if let Some(end) = piece.find(']') {
                (piece[1..end].to_string(), &piece[end + 1..])
            } else {
                (String::new(), piece)
            }
        }
        Some(_) => {
            let end = piece
                .find(|c: char| c.is_whitespace() || c == '(')
                .unwrap_or(piece.len());
            (piece[..end].to_string(), &piece[end..])
        }
        None => (String::new(), piece),
    }
}

/// Read the declared type name that follows a column's identifier: everything up
/// to the first column constraint keyword (or the end). May be empty (SQLite
/// allows typeless columns).
fn read_type_name(rest: &str) -> String {
    const CONSTRAINT_KEYWORDS: &[&str] = &[
        "PRIMARY",
        "NOT",
        "NULL",
        "UNIQUE",
        "CHECK",
        "DEFAULT",
        "COLLATE",
        "REFERENCES",
        "GENERATED",
        "AS",
    ];
    let mut type_tokens: Vec<&str> = Vec::new();
    for token in rest.split_whitespace() {
        let bare = token.split('(').next().unwrap_or(token);
        if CONSTRAINT_KEYWORDS
            .iter()
            .any(|kw| bare.eq_ignore_ascii_case(kw))
        {
            break;
        }
        type_tokens.push(token);
    }
    type_tokens.join(" ")
}

/// A column is a rowid alias exactly when its declared type is `INTEGER` (not
/// `INT`, per SQLite's rule) and its definition contains `PRIMARY KEY`.
fn column_is_rowid_alias(type_name: &str, rest: &str) -> bool {
    if !type_name.eq_ignore_ascii_case("integer") {
        return false;
    }
    let upper = rest.to_ascii_uppercase();
    if let Some(pos) = upper.find("PRIMARY") {
        return upper[pos + "PRIMARY".len()..]
            .trim_start()
            .starts_with("KEY");
    }
    false
}

/// `Unsupported` error for a write attempt against this read-only backend.
fn unsupported(operation: &str) -> BackendError {
    BackendError::Unsupported {
        operation: operation.to_string(),
    }
}

impl Backend for SqliteFileBackend {
    fn tables(&self) -> Vec<String> {
        // User tables only — SQLite's own bookkeeping tables (`sqlite_*`) are not
        // part of the queryable schema the engine offers.
        self.schema
            .iter()
            .filter(|e| e.object_type == "table" && !e.name.starts_with("sqlite_"))
            .map(|e| e.name.clone())
            .collect()
    }

    fn columns(&self, table: &str) -> Result<Vec<ColumnDef>, BackendError> {
        if Self::is_schema_table(table) {
            return Ok(Self::schema_column_defs());
        }
        let entry = self
            .table_entry(table)
            .ok_or_else(|| BackendError::TableNotFound {
                table: table.to_string(),
            })?;
        let create_sql = entry.sql.as_deref().unwrap_or("");
        let parsed = parse_create_columns(create_sql);
        Ok(parsed
            .into_iter()
            .map(|c| ColumnDef::new(c.name, c.type_name))
            .collect())
    }

    fn scan(&self, table: &str) -> Result<Box<dyn RowIterator>, BackendError> {
        if Self::is_schema_table(table) {
            return Ok(Box::new(ListRowIterator::new(self.schema_rows())));
        }
        let entry = self
            .table_entry(table)
            .ok_or_else(|| BackendError::TableNotFound {
                table: table.to_string(),
            })?;
        let create_sql = entry.sql.as_deref().unwrap_or("");
        let columns = parse_create_columns(create_sql);

        let rows: Vec<Row> = if is_without_rowid(create_sql) {
            // A `WITHOUT ROWID` table lives in an index b-tree keyed by its
            // primary key. It has no rowid, so we read it through `walk_index`
            // (via `read_without_rowid_table`) and build each row with `None` for
            // the rowid — the record already stores every column, including the
            // primary key, directly.
            let raw = read_without_rowid_table(&self.data, &entry.name).map_err(|e| {
                BackendError::Internal {
                    message: e.to_string(),
                }
            })?;
            raw.into_iter()
                .map(|values| build_row(&values, &columns, None))
                .collect()
        } else {
            // Ordinary rowid table: walk the table b-tree, and materialize an
            // INTEGER PRIMARY KEY column from the rowid (the record stores NULL).
            let raw = read_table(&self.data, &entry.name).map_err(|e| BackendError::Internal {
                message: e.to_string(),
            })?;
            raw.into_iter()
                .map(|(rowid, values)| build_row(&values, &columns, Some(rowid)))
                .collect()
        };
        Ok(Box::new(ListRowIterator::new(rows)))
    }

    // ── Write path: unsupported in this read-only backend ────────────────────

    fn insert(&mut self, _table: &str, _row: Row) -> Result<(), BackendError> {
        Err(unsupported("insert into a read-only .sqlite file"))
    }

    fn update(
        &mut self,
        _table: &str,
        _cursor: &dyn Cursor,
        _assignments: Row,
    ) -> Result<(), BackendError> {
        Err(unsupported("update a read-only .sqlite file"))
    }

    fn delete(&mut self, _table: &str, _cursor: &mut dyn Cursor) -> Result<(), BackendError> {
        Err(unsupported("delete from a read-only .sqlite file"))
    }

    fn create_table(
        &mut self,
        _table: &str,
        _columns: Vec<ColumnDef>,
        _if_not_exists: bool,
    ) -> Result<(), BackendError> {
        Err(unsupported("create a table in a read-only .sqlite file"))
    }

    fn drop_table(&mut self, _table: &str, _if_exists: bool) -> Result<(), BackendError> {
        Err(unsupported("drop a table in a read-only .sqlite file"))
    }

    fn add_column(&mut self, _table: &str, _column: ColumnDef) -> Result<(), BackendError> {
        Err(unsupported("alter a read-only .sqlite file"))
    }

    fn create_index(&mut self, _index: IndexDef) -> Result<(), BackendError> {
        Err(unsupported("create an index in a read-only .sqlite file"))
    }

    fn drop_index(&mut self, _name: &str, _if_exists: bool) -> Result<(), BackendError> {
        Err(unsupported("drop an index in a read-only .sqlite file"))
    }

    fn list_indexes(&self, table: Option<&str>) -> Vec<IndexDef> {
        // Report the catalog's index objects (optionally filtered to one table),
        // recovered from the same parsed `sqlite_schema` we serve everything from.
        // We still don't offer `scan_index`, so the planner stays on the full
        // scan — but tools that introspect indexes (PRAGMA-style) can now see
        // them. Explicit `CREATE INDEX` objects carry their SQL, from which we
        // recover the unique flag and column list. Auto-indexes (the ones SQLite
        // creates to back `UNIQUE`/`PRIMARY KEY` constraints) store no SQL text
        // in the catalog; they are always unique, and their columns aren't
        // recoverable from the catalog SQL, so we report them with an empty list.
        self.schema
            .iter()
            .filter(|e| e.object_type == "index")
            .filter(|e| table.is_none_or(|t| e.table_name.eq_ignore_ascii_case(t)))
            .map(|e| {
                let auto = e.sql.is_none();
                let (unique, columns) = match &e.sql {
                    Some(sql) => (index_is_unique(sql), parse_index_columns(sql)),
                    None => (true, Vec::new()),
                };
                IndexDef {
                    name: e.name.clone(),
                    table: e.table_name.clone(),
                    columns,
                    unique,
                    auto,
                }
            })
            .collect()
    }

    fn scan_index(
        &self,
        _index_name: &str,
        _lo: Option<&[SqlValue]>,
        _hi: Option<&[SqlValue]>,
        _lo_inclusive: bool,
        _hi_inclusive: bool,
    ) -> Result<Vec<usize>, BackendError> {
        Err(unsupported("index scan over a read-only .sqlite file"))
    }

    fn scan_by_rowids(
        &self,
        _table: &str,
        _rowids: &[usize],
    ) -> Result<Box<dyn RowIterator>, BackendError> {
        Err(unsupported("rowid lookup over a read-only .sqlite file"))
    }

    // ── Transactions: benign no-ops (a read-only file needs none) ─────────────

    fn begin_transaction(&mut self) -> Result<TransactionHandle, BackendError> {
        Ok(0)
    }

    fn commit(&mut self, _handle: TransactionHandle) -> Result<(), BackendError> {
        Ok(())
    }

    fn rollback(&mut self, _handle: TransactionHandle) -> Result<(), BackendError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_simple_create_table() {
        let cols =
            parse_create_columns("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT, score REAL)");
        let names: Vec<&str> = cols.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["id", "name", "score"]);
        assert!(cols[0].is_rowid_alias, "id is INTEGER PRIMARY KEY");
        assert!(!cols[1].is_rowid_alias);
        assert_eq!(cols[1].type_name, "TEXT");
    }

    #[test]
    fn skips_table_level_constraints_and_nested_commas() {
        let cols = parse_create_columns(
            "CREATE TABLE t (a INTEGER, b DECIMAL(10, 2), PRIMARY KEY (a, b))",
        );
        let names: Vec<&str> = cols.iter().map(|c| c.name.as_str()).collect();
        // The DECIMAL(10, 2) comma must not split a column, and the table-level
        // PRIMARY KEY clause must be skipped.
        assert_eq!(names, ["a", "b"]);
        assert_eq!(cols[1].type_name, "DECIMAL(10, 2)");
    }

    #[test]
    fn handles_quoted_identifiers() {
        let cols = parse_create_columns(r#"CREATE TABLE t ("odd name" TEXT, [b] INT, `c` REAL)"#);
        let names: Vec<&str> = cols.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["odd name", "b", "c"]);
    }

    #[test]
    fn int_primary_key_is_not_a_rowid_alias() {
        // Only INTEGER PRIMARY KEY aliases the rowid; INT PRIMARY KEY does not.
        let cols = parse_create_columns("CREATE TABLE t (id INT PRIMARY KEY, v TEXT)");
        assert!(!cols[0].is_rowid_alias);
    }

    #[test]
    fn real_affinity_follows_sqlite_rules() {
        assert!(affinity_is_real("REAL"));
        assert!(affinity_is_real("DOUBLE PRECISION"));
        assert!(affinity_is_real("FLOAT"));
        // Higher-precedence markers win.
        assert!(!affinity_is_real("INTEGER"));
        assert!(!affinity_is_real("TEXT"));
        assert!(!affinity_is_real("BLOB"));
        assert!(!affinity_is_real("")); // BLOB/none affinity
        assert!(!affinity_is_real("NUMERIC")); // NUMERIC affinity, not REAL
    }

    #[test]
    fn value_mapping_covers_every_variant() {
        assert_eq!(file_value_to_backend(FileValue::Null), SqlValue::Null);
        assert_eq!(file_value_to_backend(FileValue::Int(7)), SqlValue::Int(7));
        assert_eq!(
            file_value_to_backend(FileValue::Real(1.5)),
            SqlValue::Float(1.5)
        );
        assert_eq!(
            file_value_to_backend(FileValue::Text("hi".into())),
            SqlValue::Text("hi".into())
        );
        assert_eq!(
            file_value_to_backend(FileValue::Blob(vec![1, 2])),
            SqlValue::Blob(vec![1, 2])
        );
    }

    #[test]
    fn recognizes_both_schema_table_names_case_insensitively() {
        assert!(SqliteFileBackend::is_schema_table("sqlite_master"));
        assert!(SqliteFileBackend::is_schema_table("SQLite_Master"));
        assert!(SqliteFileBackend::is_schema_table("sqlite_schema"));
        assert!(SqliteFileBackend::is_schema_table("SQLITE_SCHEMA"));
        assert!(!SqliteFileBackend::is_schema_table("cards"));
        assert!(!SqliteFileBackend::is_schema_table("sqlite_sequence"));
    }

    #[test]
    fn schema_catalog_has_the_five_sqlite_master_columns() {
        let cols = SqliteFileBackend::schema_column_defs();
        let names: Vec<&str> = cols.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["type", "name", "tbl_name", "rootpage", "sql"]);
    }

    #[test]
    fn parses_indexed_columns_from_create_index() {
        assert_eq!(parse_index_columns("CREATE INDEX ix ON t (a)"), ["a"]);
        assert_eq!(
            parse_index_columns("CREATE UNIQUE INDEX ix ON t (ord, due)"),
            ["ord", "due"]
        );
        // Sort order / collation suffixes reduce to the bare column name.
        assert_eq!(
            parse_index_columns("CREATE INDEX ix ON t (a DESC, b COLLATE NOCASE)"),
            ["a", "b"]
        );
        assert_eq!(
            parse_index_columns(r#"CREATE INDEX ix ON t ("odd name")"#),
            ["odd name"]
        );
    }

    #[test]
    fn detects_unique_index_without_false_positives() {
        assert!(index_is_unique("CREATE UNIQUE INDEX ix ON t (a)"));
        assert!(!index_is_unique("CREATE INDEX ix ON t (a)"));
        // "unique" appearing only in a name (after INDEX) is not a UNIQUE index.
        assert!(!index_is_unique("CREATE INDEX my_unique_idx ON t (a)"));
    }
}
