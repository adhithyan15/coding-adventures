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

/// The file backend's `list_indexes` matches what real SQLite reports through
/// `PRAGMA index_list` / `PRAGMA index_info` — tools can introspect a real
/// database's indexes even though the planner still scans.
#[test]
fn list_indexes_matches_real_sqlite() {
    use coding_adventures_sql_backend::Backend;
    use coding_adventures_storage_sqlite::SqliteFileBackend;

    let path = build_sqlite_file(&[
        "CREATE TABLE cards (id INTEGER PRIMARY KEY, due INTEGER, ord INTEGER, note TEXT)",
        "CREATE INDEX ix_due ON cards(due)",
        "CREATE UNIQUE INDEX ix_ord ON cards(ord, due)",
        "CREATE TABLE other (x INTEGER)",
        "CREATE INDEX ix_x ON other(x)",
    ]);

    // Normalized (name, unique, columns) from real SQLite for one table.
    let real = |table: &str| -> Vec<(String, bool, Vec<String>)> {
        let conn = rusqlite::Connection::open(&path).unwrap();
        let mut list = conn.prepare(&format!("PRAGMA index_list('{table}')")).unwrap();
        let idxs: Vec<(String, bool)> = list
            .query_map([], |r| Ok((r.get::<_, String>(1)?, r.get::<_, i64>(2)? != 0)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        let mut out = Vec::new();
        for (name, uniq) in idxs {
            let mut info = conn.prepare(&format!("PRAGMA index_info('{name}')")).unwrap();
            let cols: Vec<String> = info
                .query_map([], |r| r.get::<_, Option<String>>(2))
                .unwrap()
                .map(|c| c.unwrap().unwrap_or_default())
                .collect();
            out.push((name, uniq, cols));
        }
        out.sort();
        out
    };

    let backend = SqliteFileBackend::open(std::fs::read(&path).unwrap()).unwrap();
    let mine = |table: &str| -> Vec<(String, bool, Vec<String>)> {
        let mut v: Vec<(String, bool, Vec<String>)> = backend
            .list_indexes(Some(table))
            .into_iter()
            .map(|i| (i.name, i.unique, i.columns))
            .collect();
        v.sort();
        v
    };

    // cards has ix_due (non-unique, [due]) and ix_ord (unique, [ord, due]).
    assert_eq!(mine("cards"), real("cards"), "cards indexes");
    // Filtering to one table excludes the other table's index.
    assert_eq!(mine("other"), real("other"), "other indexes");
    assert_eq!(mine("cards").len(), 2);

    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

/// `WITHOUT ROWID` tables — stored in an index b-tree, not a table b-tree — read
/// end-to-end through the full pipeline and match real SQLite. Covers a scalar
/// INTEGER primary key, a TEXT primary key, a composite primary key, and a large
/// table whose index b-tree spans multiple levels (interior pages), so the
/// interior-key emission in `walk_index` is exercised through the whole engine.
#[test]
fn without_rowid_tables_match_real_sqlite() {
    // A big WITHOUT ROWID table (800 rows → a two-level index b-tree) plus two
    // small ones exercising TEXT and composite primary keys.
    let mut setup = vec![
        "CREATE TABLE kv (k INTEGER PRIMARY KEY, v TEXT) WITHOUT ROWID".to_string(),
        "CREATE TABLE names (name TEXT PRIMARY KEY, age INTEGER) WITHOUT ROWID".to_string(),
        "CREATE TABLE pairs (a INTEGER, b INTEGER, note TEXT, PRIMARY KEY (a, b)) WITHOUT ROWID"
            .to_string(),
        "INSERT INTO names VALUES ('Ada', 36), ('Grace', 45), ('Alan', 41)".to_string(),
        "INSERT INTO pairs VALUES (1, 2, 'x'), (1, 3, 'y'), (2, 1, 'z')".to_string(),
    ];
    let mut big = String::from("INSERT INTO kv VALUES ");
    for i in 0..800 {
        if i > 0 {
            big.push(',');
        }
        big.push_str(&format!("({i}, 'val{i}')"));
    }
    setup.push(big);
    let setup_refs: Vec<&str> = setup.iter().map(String::as_str).collect();
    let path = build_sqlite_file(&setup_refs);

    let conn = connect(path.to_str().unwrap()).expect("open file-backed connection");

    // Each check: (query, column count). Diff mini-sqlite against real SQLite.
    let checks: &[(&str, usize)] = &[
        // Interior-page keys must appear: a full scan of the 800-row table.
        ("SELECT COUNT(*) FROM kv", 1),
        // Spot rows from across the tree (small, mid, and the maximum key).
        ("SELECT k, v FROM kv WHERE k IN (0, 5, 400, 799) ORDER BY k", 2),
        ("SELECT k FROM kv WHERE k >= 797 ORDER BY k", 1),
        ("SELECT SUM(k) FROM kv", 1),
        // TEXT primary key.
        ("SELECT name, age FROM names ORDER BY name", 2),
        ("SELECT name FROM names WHERE age > 40 ORDER BY age", 1),
        // Composite primary key — every column is stored in the record.
        ("SELECT a, b, note FROM pairs ORDER BY a, b", 3),
    ];
    for (q, ncols) in checks {
        let mut cur = conn
            .execute(q, &[])
            .unwrap_or_else(|e| panic!("mini failed on {q}: {e:?}"));
        let mine: Vec<Vec<String>> = cur
            .fetchall()
            .iter()
            .map(|row| row.iter().map(norm_mini).collect())
            .collect();
        let theirs = real_rows(&path, q, *ncols);
        assert_eq!(mine, theirs, "WITHOUT ROWID divergence on: {q}");
    }

    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}
