//! # Differential conformance oracle: mini-sqlite vs. real SQLite
//!
//! mini-sqlite is a from-scratch reimplementation of SQLite. The only honest way
//! to know whether it *is* SQLite is to ask SQLite. This harness runs the same
//! SQL through both engines and asserts they agree — the same technique the
//! `sqlite-file` crate uses to prove its byte-level reader against the real
//! on-disk format, applied here to the query engine.
//!
//! `rusqlite` (the real bundled C SQLite) is a **dev-dependency only**. It is the
//! measuring instrument, never part of the shipped crate — see `Cargo.toml`.
//!
//! ## How a case is judged
//!
//! Each [`Case`] is a `setup` (DDL/DML run for side effects) followed by one
//! `query`. We execute the whole case against a fresh in-memory database in each
//! engine and compare the query's outcome:
//!
//! - **Both succeed:** column names must match (case-insensitively — SQLite is
//!   case-insensitive about identifiers) and the result rows must match. Row
//!   *order* is only compared when the query contains `ORDER BY`; otherwise both
//!   result sets are canonically sorted first, because SQL leaves unordered row
//!   order unspecified.
//! - **Both fail:** treated as agreement (we compare *that* an error occurred,
//!   not the exact message — error text legitimately differs between engines).
//! - **One succeeds, the other fails:** a divergence.
//!
//! ## The known-divergence ledger
//!
//! Where mini-sqlite does not yet match SQLite, the case is listed in
//! [`LEDGER`] with a reason instead of being deleted or silently skipped. Ledger
//! cases are executed (so they can't panic) but exempted from the equality gate;
//! **shrinking the ledger is the conformance metric.** If a ledger case starts
//! matching, the harness prints a note nudging its removal — so a fix can't
//! quietly leave stale ledger entries behind.
//!
//! On introduction this harness measured 12 of 22 seed cases already matching
//! real SQLite, and reproduced ten genuine gaps (see [`LEDGER`]). Three have
//! since been retired: `INNER JOIN` qualified-column resolution, and correct
//! `LEFT`/`RIGHT OUTER JOIN` (NULL-padded via a per-outer-row match flag). The
//! remaining ledger entries — `FULL JOIN`, misnamed aggregate columns, and a
//! wrong `UPPER()` — are each a tracked increment; the harness is what makes
//! fixing them verifiable.

use coding_adventures_mini_sqlite::{connect, SqlValue};

/// A normalized cell value both engines are projected onto so results are
/// comparable. SQLite has no boolean type (it stores 0/1 integers), so
/// mini-sqlite's `Bool` collapses to `Int` here.
#[derive(Debug, Clone)]
enum Cell {
    Null,
    Int(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

impl Cell {
    /// Value equality with SQLite's numeric flexibility: an integer and a real
    /// that denote the same number compare equal (e.g. `SUM` may come back typed
    /// differently than a literal), and reals compare within a tiny epsilon.
    fn same(&self, other: &Cell) -> bool {
        match (self, other) {
            (Cell::Null, Cell::Null) => true,
            (Cell::Int(a), Cell::Int(b)) => a == b,
            (Cell::Real(a), Cell::Real(b)) => (a - b).abs() < 1e-9,
            (Cell::Int(a), Cell::Real(b)) | (Cell::Real(b), Cell::Int(a)) => {
                (*a as f64 - b).abs() < 1e-9
            }
            (Cell::Text(a), Cell::Text(b)) => a == b,
            (Cell::Blob(a), Cell::Blob(b)) => a == b,
            _ => false,
        }
    }

    /// A canonical string used only to sort unordered result sets before
    /// comparison (never for equality — equality goes through [`Cell::same`]).
    fn sort_key(&self) -> String {
        match self {
            Cell::Null => "0".to_string(),
            Cell::Int(i) => format!("1:{i}"),
            Cell::Real(f) => format!("1:{f}"),
            Cell::Text(s) => format!("2:{s}"),
            Cell::Blob(b) => format!("3:{b:?}"),
        }
    }
}

impl From<SqlValue> for Cell {
    fn from(v: SqlValue) -> Cell {
        match v {
            SqlValue::Null => Cell::Null,
            SqlValue::Bool(b) => Cell::Int(b as i64),
            SqlValue::Int(i) => Cell::Int(i),
            SqlValue::Float(f) => Cell::Real(f),
            SqlValue::Text(s) => Cell::Text(s),
            SqlValue::Blob(b) => Cell::Blob(b),
        }
    }
}

impl From<rusqlite::types::Value> for Cell {
    fn from(v: rusqlite::types::Value) -> Cell {
        use rusqlite::types::Value;
        match v {
            Value::Null => Cell::Null,
            Value::Integer(i) => Cell::Int(i),
            Value::Real(f) => Cell::Real(f),
            Value::Text(s) => Cell::Text(s),
            Value::Blob(b) => Cell::Blob(b),
        }
    }
}

/// The outcome of running a case's query in one engine: either its columns and
/// rows, or an error (message text is intentionally discarded — see the module
/// doc; we only compare error-vs-success).
type Outcome = Result<(Vec<String>, Vec<Vec<Cell>>), String>;

/// Run `setup` then `query` through **mini-sqlite** on a fresh in-memory db.
fn run_mini(setup: &[&str], query: &str) -> Outcome {
    let conn = connect(":memory:").map_err(|e| e.to_string())?;
    for stmt in setup {
        conn.execute(stmt, &[]).map_err(|e| e.to_string())?;
    }
    let mut cursor = conn.execute(query, &[]).map_err(|e| e.to_string())?;
    let columns = cursor.description.iter().map(|d| d.name.clone()).collect();
    let rows = cursor
        .fetchall()
        .into_iter()
        .map(|row| row.into_iter().map(Cell::from).collect())
        .collect();
    Ok((columns, rows))
}

/// Run `setup` then `query` through **real bundled SQLite** on a fresh in-memory
/// db — the oracle.
fn run_sqlite(setup: &[&str], query: &str) -> Outcome {
    let conn = rusqlite::Connection::open_in_memory().map_err(|e| e.to_string())?;
    for stmt in setup {
        conn.execute_batch(stmt).map_err(|e| e.to_string())?;
    }
    let mut stmt = conn.prepare(query).map_err(|e| e.to_string())?;
    let columns: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
    let ncols = stmt.column_count();
    let rows = stmt
        .query_map([], |row| {
            let mut cells = Vec::with_capacity(ncols);
            for i in 0..ncols {
                cells.push(Cell::from(row.get::<usize, rusqlite::types::Value>(i)?));
            }
            Ok(cells)
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok((columns, rows))
}

/// Column-name lists match ignoring case (SQLite treats identifiers case-
/// insensitively).
fn columns_match(a: &[String], b: &[String]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.eq_ignore_ascii_case(y))
}

fn rows_match(a: &[Vec<Cell>], b: &[Vec<Cell>], ordered: bool) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let row_eq = |x: &Vec<Cell>, y: &Vec<Cell>| {
        x.len() == y.len() && x.iter().zip(y).all(|(p, q)| p.same(q))
    };
    if ordered {
        a.iter().zip(b).all(|(x, y)| row_eq(x, y))
    } else {
        // Unordered: sort both sides by a canonical key, then compare positionally.
        let key = |row: &Vec<Cell>| row.iter().map(Cell::sort_key).collect::<Vec<_>>().join("|");
        let mut a2: Vec<&Vec<Cell>> = a.iter().collect();
        let mut b2: Vec<&Vec<Cell>> = b.iter().collect();
        a2.sort_by_key(|r| key(r));
        b2.sort_by_key(|r| key(r));
        a2.iter().zip(&b2).all(|(x, y)| row_eq(x, y))
    }
}

fn outcomes_match(mine: &Outcome, theirs: &Outcome, ordered: bool) -> bool {
    match (mine, theirs) {
        (Ok((ca, ra)), Ok((cb, rb))) => columns_match(ca, cb) && rows_match(ra, rb, ordered),
        (Err(_), Err(_)) => true,
        _ => false,
    }
}

/// One conformance case: run `setup` for side effects, then compare `query`.
struct Case {
    id: &'static str,
    setup: &'static [&'static str],
    query: &'static str,
}

/// Cases mini-sqlite is expected to match real SQLite on today.
const CASES: &[Case] = &[
    Case {
        id: "select_all",
        setup: &[
            "CREATE TABLE users (id INTEGER, name TEXT)",
            "INSERT INTO users VALUES (1, 'Ada'), (2, 'Grace'), (3, 'Alan')",
        ],
        query: "SELECT id, name FROM users ORDER BY id",
    },
    Case {
        id: "projection_alias",
        setup: &[
            "CREATE TABLE users (id INTEGER, name TEXT)",
            "INSERT INTO users VALUES (1, 'Ada'), (2, 'Grace')",
        ],
        query: "SELECT name AS who FROM users ORDER BY id",
    },
    Case {
        id: "where_equals",
        setup: &[
            "CREATE TABLE users (id INTEGER, name TEXT)",
            "INSERT INTO users VALUES (1, 'Ada'), (2, 'Grace')",
        ],
        query: "SELECT name FROM users WHERE id = 2",
    },
    Case {
        id: "where_less_than",
        setup: &[
            "CREATE TABLE users (id INTEGER, name TEXT)",
            "INSERT INTO users VALUES (1, 'Ada'), (2, 'Grace'), (3, 'Alan')",
        ],
        query: "SELECT id FROM users WHERE id < 3 ORDER BY id",
    },
    Case {
        id: "order_by_desc",
        setup: &[
            "CREATE TABLE users (id INTEGER)",
            "INSERT INTO users VALUES (1), (2), (3)",
        ],
        query: "SELECT id FROM users ORDER BY id DESC",
    },
    Case {
        id: "limit_offset",
        setup: &[
            "CREATE TABLE users (id INTEGER)",
            "INSERT INTO users VALUES (1), (2), (3), (4)",
        ],
        query: "SELECT id FROM users ORDER BY id LIMIT 2 OFFSET 1",
    },
    Case {
        id: "distinct",
        setup: &[
            "CREATE TABLE t (x INTEGER)",
            "INSERT INTO t VALUES (1), (1), (2), (3), (3)",
        ],
        query: "SELECT DISTINCT x FROM t ORDER BY x",
    },
    Case {
        id: "count_star",
        setup: &[
            "CREATE TABLE users (id INTEGER)",
            "INSERT INTO users VALUES (1), (2), (3)",
        ],
        query: "SELECT COUNT(*) FROM users",
    },
    Case {
        id: "sum_min_max",
        setup: &[
            "CREATE TABLE nums (n INTEGER)",
            "INSERT INTO nums VALUES (5), (2), (8), (1)",
        ],
        query: "SELECT SUM(n), MIN(n), MAX(n) FROM nums",
    },
    Case {
        id: "avg",
        setup: &[
            "CREATE TABLE nums (n INTEGER)",
            "INSERT INTO nums VALUES (2), (4), (6)",
        ],
        query: "SELECT AVG(n) FROM nums",
    },
    Case {
        id: "group_by",
        setup: &[
            "CREATE TABLE sales (dept TEXT, amt INTEGER)",
            "INSERT INTO sales VALUES ('a', 10), ('b', 5), ('a', 7), ('b', 3)",
        ],
        query: "SELECT dept, SUM(amt) FROM sales GROUP BY dept ORDER BY dept",
    },
    Case {
        id: "having",
        setup: &[
            "CREATE TABLE sales (dept TEXT, amt INTEGER)",
            "INSERT INTO sales VALUES ('a', 10), ('b', 5), ('a', 7), ('b', 3)",
        ],
        query: "SELECT dept, SUM(amt) FROM sales GROUP BY dept HAVING SUM(amt) > 10 ORDER BY dept",
    },
    Case {
        id: "inner_join",
        setup: &[
            "CREATE TABLE a (id INTEGER, name TEXT)",
            "CREATE TABLE b (aid INTEGER, tag TEXT)",
            "INSERT INTO a VALUES (1, 'x'), (2, 'y')",
            "INSERT INTO b VALUES (1, 'p'), (2, 'q'), (1, 'r')",
        ],
        query: "SELECT a.name, b.tag FROM a INNER JOIN b ON a.id = b.aid ORDER BY a.name, b.tag",
    },
    Case {
        id: "is_null",
        setup: &[
            "CREATE TABLE t (id INTEGER, v TEXT)",
            "INSERT INTO t VALUES (1, 'x'), (2, NULL), (3, 'z')",
        ],
        query: "SELECT id FROM t WHERE v IS NULL ORDER BY id",
    },
    Case {
        id: "string_functions",
        setup: &[
            "CREATE TABLE users (id INTEGER, name TEXT)",
            "INSERT INTO users VALUES (1, 'Ada'), (2, 'Grace')",
        ],
        query: "SELECT UPPER(name), LENGTH(name) FROM users ORDER BY id",
    },
    Case {
        id: "between",
        setup: &[
            "CREATE TABLE users (id INTEGER)",
            "INSERT INTO users VALUES (1), (2), (3), (4)",
        ],
        query: "SELECT id FROM users WHERE id BETWEEN 2 AND 3 ORDER BY id",
    },
    Case {
        id: "in_list",
        setup: &[
            "CREATE TABLE users (id INTEGER)",
            "INSERT INTO users VALUES (1), (2), (3), (4)",
        ],
        query: "SELECT id FROM users WHERE id IN (1, 3) ORDER BY id",
    },
    Case {
        id: "like",
        setup: &[
            "CREATE TABLE users (id INTEGER, name TEXT)",
            "INSERT INTO users VALUES (1, 'Ada'), (2, 'Alan'), (3, 'Grace')",
        ],
        query: "SELECT name FROM users WHERE name LIKE 'A%' ORDER BY name",
    },
    Case {
        id: "unknown_table_errors",
        setup: &["CREATE TABLE users (id INTEGER)"],
        query: "SELECT * FROM does_not_exist",
    },
    // --- Cases below currently diverge; see LEDGER. ---
    Case {
        id: "left_join",
        setup: &[
            "CREATE TABLE a (id INTEGER, name TEXT)",
            "CREATE TABLE b (aid INTEGER, tag TEXT)",
            "INSERT INTO a VALUES (1, 'x'), (2, 'y'), (3, 'z')",
            "INSERT INTO b VALUES (1, 'p'), (1, 'q')",
        ],
        query: "SELECT a.name, b.tag FROM a LEFT JOIN b ON a.id = b.aid ORDER BY a.name, b.tag",
    },
    Case {
        id: "right_join",
        setup: &[
            "CREATE TABLE a (id INTEGER, name TEXT)",
            "CREATE TABLE b (aid INTEGER, tag TEXT)",
            "INSERT INTO a VALUES (1, 'x'), (2, 'y')",
            "INSERT INTO b VALUES (1, 'p'), (3, 'r')",
        ],
        query: "SELECT a.name, b.tag FROM a RIGHT JOIN b ON a.id = b.aid ORDER BY a.name, b.tag",
    },
    Case {
        id: "full_join",
        setup: &[
            "CREATE TABLE a (id INTEGER, name TEXT)",
            "CREATE TABLE b (aid INTEGER, tag TEXT)",
            "INSERT INTO a VALUES (1, 'x'), (2, 'y')",
            "INSERT INTO b VALUES (1, 'p'), (3, 'r')",
        ],
        query: "SELECT a.name, b.tag FROM a FULL JOIN b ON a.id = b.aid ORDER BY a.name, b.tag",
    },
];

/// Documented divergences: `(case id, reason)`. Ledger cases are executed but
/// exempt from the equality gate. **Shrinking this list is the conformance
/// metric.** This is the honest baseline the harness measured on introduction —
/// every entry is a real, reproduced gap between mini-sqlite and SQLite, each
/// slated for a later increment.
///
/// The gaps fall into a few families:
///
/// - **Join column resolution** (`inner_join`): a qualified reference like
///   `a.name` across a join resolves to `NULL`, because a `FROM a` with no
///   explicit alias opens its cursor keyed under `None` while `LoadColumn` looks
///   it up under `Some("a")` (`sql-vm` `LoadColumn`). This breaks *even* inner
///   joins and is the highest-priority fix — it underlies the outer joins too.
/// - **`FULL JOIN`**: needs the unmatched right rows too, which a single forward
///   pass can't produce; still degrades to a cross product. (`LEFT`/`RIGHT` are
///   now implemented via a per-outer-row match flag and no longer diverge.)
/// - **Computed-column naming** (`count_star`/`sum_min_max`/`avg`/`group_by`/
///   `having`): the result *rows* match SQLite exactly, but mini-sqlite names an
///   aggregate output column `agg_N` where SQLite uses the expression text
///   (`SUM(n)`). A naming-only divergence.
/// - **Scalar functions** (`string_functions`): function-call output columns are
///   unnamed (`?`), and `UPPER(name)` returns the wrong value (an integer, not
///   the uppercased text) — a real codegen/builtin bug.
const LEDGER: &[(&str, &str)] = &[
    (
        "full_join",
        "FULL JOIN needs the unmatched right rows too, which a single forward pass can't produce; still degrades to a cross product. (LEFT/RIGHT are now implemented and no longer ledgered.)",
    ),
    (
        "count_star",
        "rows match; computed-column naming differs — mini names it agg_0, SQLite names it COUNT(*).",
    ),
    (
        "sum_min_max",
        "rows match; computed-column naming differs — agg_N vs SUM(n)/MIN(n)/MAX(n).",
    ),
    (
        "avg",
        "rows match; computed-column naming differs — agg_0 vs AVG(n).",
    ),
    (
        "group_by",
        "rows match; computed-column naming differs — agg_0 vs SUM(amt).",
    ),
    (
        "having",
        "rows match; computed-column naming differs — agg_0 vs SUM(amt).",
    ),
    (
        "string_functions",
        "function-call output columns unnamed (?), and UPPER(name) returns the wrong value (int, not uppercased text). Real codegen/builtin bug.",
    ),
];

#[test]
fn mini_sqlite_matches_real_sqlite() {
    let mut divergences: Vec<String> = Vec::new();
    let mut stale_ledger: Vec<&str> = Vec::new();

    for case in CASES {
        let mine = run_mini(case.setup, case.query);
        let theirs = run_sqlite(case.setup, case.query);
        let ordered = case.query.to_lowercase().contains("order by");
        let matched = outcomes_match(&mine, &theirs, ordered);

        if let Some((_, reason)) = LEDGER.iter().find(|(id, _)| *id == case.id) {
            // A documented divergence. Don't gate on it — but if it now matches,
            // flag the ledger entry as stale so the fix can retire it.
            if matched {
                stale_ledger.push(case.id);
            }
            let _ = reason;
            continue;
        }

        if !matched {
            divergences.push(format!(
                "case '{}':\n    query : {}\n    mini  : {:?}\n    sqlite: {:?}",
                case.id, case.query, mine, theirs
            ));
        }
    }

    for id in &stale_ledger {
        eprintln!("NOTE: ledger case '{id}' now MATCHES real SQLite — remove it from LEDGER.");
    }

    assert!(
        divergences.is_empty(),
        "mini-sqlite diverged from real SQLite on {} non-ledger case(s):\n{}",
        divergences.len(),
        divergences.join("\n")
    );
}
