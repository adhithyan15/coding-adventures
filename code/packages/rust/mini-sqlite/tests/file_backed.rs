//! End-to-end proof that the mini-sqlite query engine can `SELECT` from a **real
//! `.sqlite` file** — the whole pipeline (parser → planner → optimizer → codegen
//! → VM) running unmodified over a `SqliteFileBackend`.
//!
//! We build the fixture with real bundled SQLite (`rusqlite`, a **dev-dependency
//! only**), write it to a temp path, then open it through mini-sqlite's own
//! `connect(path)` and run queries — comparing the engine's answers to what the
//! real library returns over the same SQL.

use std::sync::atomic::{AtomicU64, Ordering};

use coding_adventures_mini_sqlite::{connect, SqlValue};

/// Build a real SQLite file from `statements` at a fresh temp path and return
/// that path (kept alive for the test). Uses a per-run `create_dir`'d subdir to
/// avoid the `/tmp` symlink-swap hazard without a `tempfile` dependency.
fn build_sqlite_file(statements: &[&str]) -> std::path::PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut dir = std::env::temp_dir();
    dir.push(format!("mini_sqlite_filebacked_{}_{}", std::process::id(), unique));
    std::fs::create_dir(&dir).expect("create fixture dir");
    let path = dir.join("data.sqlite");
    {
        let conn = rusqlite::Connection::open(&path).expect("open sqlite");
        for stmt in statements {
            conn.execute_batch(stmt).expect("run statement");
        }
    } // flushed + closed on drop
    path
}

#[test]
fn selects_from_a_real_sqlite_file_through_the_full_pipeline() {
    let path = build_sqlite_file(&[
        "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, score REAL)",
        "INSERT INTO users VALUES (1, 'Ada', 1.5), (2, 'Grace', 9.0), (3, 'Alan', 4.0)",
    ]);

    // Open the real file through mini-sqlite and run SQL through the whole engine.
    let conn = connect(path.to_str().unwrap()).expect("open file-backed connection");

    // Projection + WHERE + ORDER BY, all executed over the file's b-tree pages.
    let mut cur = conn
        .execute(
            "SELECT name FROM users WHERE score >= 4.0 ORDER BY name",
            &[],
        )
        .expect("query the file");
    let rows = cur.fetchall();
    assert_eq!(
        rows,
        vec![
            vec![SqlValue::Text("Alan".into())],
            vec![SqlValue::Text("Grace".into())],
        ],
        "WHERE + ORDER BY over a real file"
    );

    // An aggregate over the file.
    let mut cur = conn.execute("SELECT COUNT(*) FROM users", &[]).unwrap();
    assert_eq!(cur.fetchall(), vec![vec![SqlValue::Int(3)]]);

    // The INTEGER PRIMARY KEY rowid alias comes back as its real value, not NULL.
    let mut cur = conn
        .execute("SELECT id FROM users ORDER BY id", &[])
        .unwrap();
    assert_eq!(
        cur.fetchall(),
        vec![
            vec![SqlValue::Int(1)],
            vec![SqlValue::Int(2)],
            vec![SqlValue::Int(3)],
        ],
    );

    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn writes_to_a_file_backed_connection_are_rejected_for_now() {
    let path = build_sqlite_file(&["CREATE TABLE t (x INTEGER)", "INSERT INTO t VALUES (1)"]);
    let conn = connect(path.to_str().unwrap()).unwrap();

    // The file backend is read-only this milestone; DML must surface an error,
    // not silently no-op.
    let result = conn.execute("INSERT INTO t VALUES (2)", &[]);
    assert!(result.is_err(), "INSERT into a file-backed db should error");

    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

/// Normalize a mini-sqlite `SqlValue` to a comparable string so results from the
/// two engines can be diffed regardless of representation nuance.
fn norm_mini(v: &SqlValue) -> String {
    match v {
        SqlValue::Null => "NULL".to_string(),
        SqlValue::Bool(b) => (*b as i64).to_string(),
        SqlValue::Int(i) => i.to_string(),
        SqlValue::Float(f) => format!("{f:?}"),
        SqlValue::Text(s) => format!("T:{s}"),
        SqlValue::Blob(b) => format!("B:{b:?}"),
    }
}

/// The same normalization for a real-SQLite (`rusqlite`) value.
fn norm_real(v: &rusqlite::types::Value) -> String {
    use rusqlite::types::Value;
    match v {
        Value::Null => "NULL".to_string(),
        Value::Integer(i) => i.to_string(),
        Value::Real(f) => format!("{f:?}"),
        Value::Text(s) => format!("T:{s}"),
        Value::Blob(b) => format!("B:{b:?}"),
    }
}

/// Run `query` through real SQLite over the same file and return normalized rows.
fn real_rows(path: &std::path::Path, query: &str, ncols: usize) -> Vec<Vec<String>> {
    let conn = rusqlite::Connection::open(path).expect("open real sqlite");
    let mut stmt = conn.prepare(query).expect("prepare");
    let rows = stmt
        .query_map([], |row| {
            let mut out = Vec::with_capacity(ncols);
            for i in 0..ncols {
                out.push(norm_real(&row.get::<_, rusqlite::types::Value>(i).unwrap()));
            }
            Ok(out)
        })
        .expect("query_map")
        .map(|r| r.unwrap())
        .collect();
    rows
}

/// The schema catalog (`sqlite_master` / `sqlite_schema`) is queryable and its
/// contents match real SQLite — applications introspect the database this way.
#[test]
fn sqlite_master_is_queryable_and_matches_real_sqlite() {
    let path = build_sqlite_file(&[
        "CREATE TABLE cards (id INTEGER PRIMARY KEY, due INTEGER, note TEXT)",
        "CREATE TABLE notes (id INTEGER PRIMARY KEY, flds TEXT)",
        "CREATE INDEX ix_due ON cards(due)",
        "INSERT INTO cards VALUES (1, 100, 'a'), (2, 50, 'b')",
    ]);
    let conn = connect(path.to_str().unwrap()).expect("open file-backed connection");

    // A representative spread: filtered projection, the `sqlite_schema` alias,
    // an aggregate, and the full five-column row shape including `rootpage`/`sql`.
    let checks: &[(&str, usize)] = &[
        ("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name", 1),
        ("SELECT name FROM sqlite_schema WHERE type = 'index' ORDER BY name", 1),
        ("SELECT COUNT(*) FROM sqlite_master", 1),
        ("SELECT type, name, tbl_name FROM sqlite_master ORDER BY name, type", 3),
        ("SELECT type, name, tbl_name, rootpage, sql FROM sqlite_master ORDER BY name, type", 5),
    ];
    for (q, ncols) in checks {
        let mut cur = conn.execute(q, &[]).unwrap_or_else(|e| panic!("mini failed on {q}: {e:?}"));
        let mine: Vec<Vec<String>> = cur
            .fetchall()
            .iter()
            .map(|row| row.iter().map(norm_mini).collect())
            .collect();
        let theirs = real_rows(&path, q, *ncols);
        assert_eq!(mine, theirs, "sqlite_master divergence on: {q}");
    }

    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}
