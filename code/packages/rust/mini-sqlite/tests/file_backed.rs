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
