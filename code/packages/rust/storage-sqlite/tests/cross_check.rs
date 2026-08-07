//! Cross-check the `SqliteFileBackend` against real bundled SQLite.
//!
//! We build genuine `.sqlite` files with `rusqlite` (the real C library, a
//! **dev-dependency only**), then assert that reading them back through
//! `SqliteFileBackend` — the file-backed `Backend` the query engine will use —
//! yields the same tables, columns, and rows the real library reports over SQL.
//! This is the same oracle discipline the `sqlite-file` crate uses one layer
//! down, lifted to the `Backend` interface.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use coding_adventures_sql_backend::{Backend, SqlValue};
use coding_adventures_storage_sqlite::SqliteFileBackend;

/// Build a real SQLite database from `statements` and return its on-disk bytes.
/// Mirrors the `sqlite-file` cross-check fixture builder: a fresh per-run
/// subdirectory (via `create_dir`, which fails on a pre-existing path) sidesteps
/// the `/tmp` symlink-swap hazard without a `tempfile` dependency.
fn build_sqlite_db(statements: &[&str]) -> Vec<u8> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "storage_sqlite_xcheck_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::create_dir(&dir).expect("create fresh fixture dir");
    let path = dir.join("oracle.db");
    {
        let conn = rusqlite::Connection::open(&path).expect("open sqlite db");
        for stmt in statements {
            conn.execute_batch(stmt).expect("run statement");
        }
    }
    let bytes = std::fs::read(&path).expect("read db file");
    let _ = std::fs::remove_dir_all(&dir);
    bytes
}

/// One oracle row: `(id, name, score, data)` as the real library returns it.
type OracleRow = (i64, Option<String>, Option<f64>, Option<Vec<u8>>);

/// Read `SELECT id, name, score, data FROM t` out of a genuine file with
/// rusqlite — the oracle for the scan test.
fn oracle_rows(db: &[u8]) -> Vec<OracleRow> {
    static COUNTER: AtomicU64 = AtomicU64::new(1_000_000);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "storage_sqlite_rows_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("oracle.db");
    std::fs::write(&path, db).unwrap();
    let conn = rusqlite::Connection::open(&path).unwrap();
    let mut out: Vec<_> = conn
        .prepare("SELECT id, name, score, data FROM t")
        .unwrap()
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, Option<f64>>(2)?,
                r.get::<_, Option<Vec<u8>>>(3)?,
            ))
        })
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    out.sort_by_key(|r| r.0);
    drop(conn);
    let _ = std::fs::remove_dir_all(&dir);
    out
}

#[test]
fn tables_and_columns_match_real_sqlite() {
    let db = build_sqlite_db(&[
        "CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT, score REAL, data BLOB)",
        "CREATE TABLE other (x INTEGER, y INTEGER)",
        "CREATE INDEX idx ON other (x)",
    ]);
    let backend = SqliteFileBackend::open(db).expect("open backend");

    // tables(): user tables, not indexes or sqlite_* internals.
    let mut tables = backend.tables();
    tables.sort();
    assert_eq!(tables, vec!["other".to_string(), "t".to_string()]);

    // columns(): names recovered from the CREATE TABLE text, in order.
    let cols: Vec<String> = backend
        .columns("t")
        .unwrap()
        .into_iter()
        .map(|c| c.name)
        .collect();
    assert_eq!(cols, ["id", "name", "score", "data"]);

    // A missing table is a clean error, not a panic.
    assert!(backend.columns("nope").is_err());
}

#[test]
fn scan_matches_real_sqlite_including_rowid_alias_and_types() {
    // Row 2 exercises NULLs; the INTEGER PRIMARY KEY `id` must come back as the
    // rowid, not the NULL the record stores for it.
    let db = build_sqlite_db(&[
        "CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT, score REAL, data BLOB)",
        "INSERT INTO t VALUES (1, 'Ada', 1.5, x'dead'), (2, NULL, NULL, NULL), (3, 'Grace', 9.0, x'01')",
    ]);
    let theirs = oracle_rows(&db);

    let backend = SqliteFileBackend::open(db).expect("open backend");
    let mut iter = backend.scan("t").expect("scan");

    let mut ours: Vec<BTreeMap<String, SqlValue>> = Vec::new();
    while let Some(row) = iter.next() {
        ours.push(row);
    }
    ours.sort_by_key(|r| match r.get("id") {
        Some(SqlValue::Int(i)) => *i,
        _ => panic!("id column must be a non-null integer (rowid alias)"),
    });

    assert_eq!(ours.len(), theirs.len(), "row count");
    for (row, (id, name, score, data)) in ours.iter().zip(theirs) {
        assert_eq!(row.get("id"), Some(&SqlValue::Int(id)), "rowid-alias id");
        assert_eq!(
            row.get("name"),
            Some(&name.map(SqlValue::Text).unwrap_or(SqlValue::Null)),
            "name"
        );
        assert_eq!(
            row.get("score"),
            Some(&score.map(SqlValue::Float).unwrap_or(SqlValue::Null)),
            "score"
        );
        assert_eq!(
            row.get("data"),
            Some(&data.map(SqlValue::Blob).unwrap_or(SqlValue::Null)),
            "data"
        );
    }
}
