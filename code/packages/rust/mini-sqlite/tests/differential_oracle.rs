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
//! real SQLite, and reproduced ten genuine gaps (see [`LEDGER`]). **All ten have
//! since been retired** — the ledger is now empty and every seed case matches
//! real SQLite. The gaps closed, in order: `INNER JOIN` qualified-column
//! resolution; `LEFT`/`RIGHT OUTER JOIN` NULL-padding (a per-outer-row match
//! flag); `FULL OUTER JOIN` (a LEFT pass unioned with a RIGHT anti-join pass);
//! scalar functions (`UPPER()` returned the right value once same-named output
//! columns stopped colliding, and columns got SQLite-style names); and finally
//! aggregate computed-column names (`agg_N` → `COUNT(*)`/`SUM(n)`/`AVG(n)`). The
//! harness is what made fixing each one verifiable — and what will catch the
//! next divergence a new case surfaces.

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
    // A trickier FULL JOIN: duplicate join keys on both sides (many-to-many)
    // plus rows unmatched on each side. Guards the two-pass implementation
    // against emitting a matched pair twice or missing an anti-join row.
    Case {
        id: "full_join_multi",
        setup: &[
            "CREATE TABLE a (id INTEGER, name TEXT)",
            "CREATE TABLE b (aid INTEGER, tag TEXT)",
            "INSERT INTO a VALUES (1, 'x'), (1, 'x2'), (2, 'y'), (4, 'w')",
            "INSERT INTO b VALUES (1, 'p'), (1, 'q'), (3, 'r')",
        ],
        query: "SELECT a.name, b.tag FROM a FULL JOIN b ON a.id = b.aid ORDER BY a.name, b.tag",
    },
    // --- Scalar functions: IFNULL / NULLIF / TYPEOF / INSTR / HEX. Aliased so
    //     the case exercises the function *values* (column naming is covered
    //     elsewhere); each is diffed against real SQLite. ---
    Case {
        id: "ifnull",
        setup: &[
            "CREATE TABLE t (id INTEGER, v INTEGER)",
            "INSERT INTO t VALUES (1, 10), (2, NULL), (3, 30)",
        ],
        query: "SELECT IFNULL(v, -1) AS r FROM t ORDER BY id",
    },
    Case {
        id: "nullif",
        setup: &[
            "CREATE TABLE t (id INTEGER)",
            "INSERT INTO t VALUES (1), (2), (3)",
        ],
        query: "SELECT NULLIF(id, 2) AS r FROM t ORDER BY id",
    },
    Case {
        id: "typeof",
        setup: &[
            "CREATE TABLE t (id INTEGER, v INTEGER, name TEXT)",
            "INSERT INTO t VALUES (1, 10, 'a'), (2, NULL, NULL)",
        ],
        query: "SELECT TYPEOF(v) AS tv, TYPEOF(name) AS tn, TYPEOF(id) AS ti FROM t ORDER BY id",
    },
    Case {
        id: "instr",
        setup: &[
            "CREATE TABLE t (id INTEGER, name TEXT)",
            "INSERT INTO t VALUES (1, 'abc'), (2, 'bbc'), (3, NULL)",
        ],
        query: "SELECT INSTR(name, 'b') AS r FROM t ORDER BY id",
    },
    Case {
        id: "hex",
        setup: &[
            "CREATE TABLE t (id INTEGER, name TEXT)",
            "INSERT INTO t VALUES (255, 'abc'), (16, 'z')",
        ],
        query: "SELECT HEX(id) AS hi, HEX(name) AS hn FROM t ORDER BY id",
    },
    // --- More scalar functions: SIGN / UNICODE / CHAR / ZEROBLOB / QUOTE. ---
    Case {
        id: "sign",
        setup: &[
            "CREATE TABLE t (id INTEGER, n INTEGER)",
            "INSERT INTO t VALUES (1, -7), (2, 0), (3, 42)",
        ],
        query: "SELECT SIGN(n) AS r FROM t ORDER BY id",
    },
    Case {
        id: "unicode",
        setup: &[
            "CREATE TABLE t (id INTEGER, s TEXT)",
            "INSERT INTO t VALUES (1, 'abc'), (2, 'Z')",
        ],
        query: "SELECT UNICODE(s) AS r FROM t ORDER BY id",
    },
    Case {
        id: "char_fn",
        setup: &[
            "CREATE TABLE t (id INTEGER)",
            "INSERT INTO t VALUES (1)",
        ],
        query: "SELECT CHAR(72,105,33) AS r FROM t ORDER BY id",
    },
    Case {
        id: "zeroblob",
        setup: &[
            "CREATE TABLE t (id INTEGER, n INTEGER)",
            "INSERT INTO t VALUES (1, 3), (2, 0)",
        ],
        query: "SELECT ZEROBLOB(n) AS r FROM t ORDER BY id",
    },
    Case {
        id: "quote",
        setup: &[
            "CREATE TABLE t (id INTEGER, s TEXT, n INTEGER)",
            "INSERT INTO t VALUES (1, 'abc', -7), (2, NULL, 5)",
        ],
        query: "SELECT QUOTE(s) AS qs, QUOTE(n) AS qn FROM t ORDER BY id",
    },
    // IIF(x, y, z) — the function form of CASE WHEN x THEN y ELSE z END.
    Case {
        id: "iif",
        setup: &[
            "CREATE TABLE t (id INTEGER, n INTEGER)",
            "INSERT INTO t VALUES (1, 8), (2, 2), (3, NULL)",
        ],
        query: "SELECT IIF(n > 5, 'big', 'small') AS r FROM t ORDER BY id",
    },
    // Multi-argument MAX/MIN are the SCALAR forms (return the largest/smallest
    // argument, NULL if any is NULL) — distinct from the 1-arg aggregate.
    Case {
        id: "scalar_max_min",
        setup: &[
            "CREATE TABLE t (id INTEGER, n INTEGER)",
            "INSERT INTO t VALUES (1, 7), (2, 20)",
        ],
        query: "SELECT MAX(n, 10, 3) AS mx, MIN(n, 10, 3) AS mn FROM t ORDER BY id",
    },
    Case {
        id: "scalar_max_null",
        setup: &[
            "CREATE TABLE t (id INTEGER, n INTEGER)",
            "INSERT INTO t VALUES (1, 5), (2, NULL)",
        ],
        query: "SELECT MAX(n, 3) AS r FROM t ORDER BY id",
    },
    // The single-argument aggregate MAX/MIN still work (regression guard).
    Case {
        id: "agg_max_min_still_work",
        setup: &[
            "CREATE TABLE t (dept TEXT, sal INTEGER)",
            "INSERT INTO t VALUES ('a', 10), ('a', 30), ('b', 20)",
        ],
        query: "SELECT dept, MAX(sal), MIN(sal) FROM t GROUP BY dept ORDER BY dept",
    },
    // TRIM/LTRIM/RTRIM with a second argument strip a *set of characters*
    // rather than whitespace: trim('xxhixx','x') -> 'hi'.
    Case {
        id: "trim_charset",
        setup: &[
            "CREATE TABLE t (id INTEGER, s TEXT)",
            "INSERT INTO t VALUES (1, 'xxhixx'), (2, 'yyworldyy')",
        ],
        query: "SELECT TRIM(s, 'xy') AS t, LTRIM(s, 'xy') AS l, RTRIM(s, 'xy') AS r FROM t ORDER BY id",
    },
    // A multi-character set behaves as a bag: any of {a,b,c} at either end goes.
    Case {
        id: "trim_charset_multi",
        setup: &[
            "CREATE TABLE t (id INTEGER, s TEXT)",
            "INSERT INTO t VALUES (1, 'abcHIcba'), (2, 'aaa')",
        ],
        query: "SELECT TRIM(s, 'abc') AS r FROM t ORDER BY id",
    },
    // NULL in the trim-set propagates; the single-argument whitespace form and
    // an empty set are both regression-guarded here.
    Case {
        id: "trim_charset_null_and_edge",
        setup: &[
            "CREATE TABLE t (id INTEGER, s TEXT)",
            "INSERT INTO t VALUES (1, '  hi  '), (2, 'xhix')",
        ],
        query: "SELECT TRIM(s) AS ws, TRIM(s, NULL) AS tn, TRIM(s, '') AS te FROM t ORDER BY id",
    },
    // CONCAT joins all arguments; a NULL contributes the empty string (it does
    // not nullify the result), and integers coerce to text.
    Case {
        id: "concat",
        setup: &[
            "CREATE TABLE t (id INTEGER, a TEXT, b TEXT)",
            "INSERT INTO t VALUES (1, 'foo', 'bar'), (2, 'x', NULL)",
        ],
        query: "SELECT CONCAT(a, b, id) AS r FROM t ORDER BY id",
    },
    // CONCAT_WS joins the value arguments with a separator, SKIPPING NULLs; a
    // NULL separator makes the whole result NULL.
    Case {
        id: "concat_ws",
        setup: &[
            "CREATE TABLE t (id INTEGER, a TEXT, b TEXT, c TEXT)",
            "INSERT INTO t VALUES (1, 'a', 'b', 'c'), (2, 'a', NULL, 'c')",
        ],
        query: "SELECT CONCAT_WS('-', a, b, c) AS r FROM t ORDER BY id",
    },
    Case {
        id: "concat_ws_null_sep",
        setup: &[
            "CREATE TABLE t (id INTEGER, a TEXT)",
            "INSERT INTO t VALUES (1, 'a')",
        ],
        query: "SELECT CONCAT_WS(NULL, a, 'b') AS r FROM t ORDER BY id",
    },
    // SUBSTRING is a spelling of SUBSTR (2- and 3-argument forms).
    Case {
        id: "substring_alias",
        setup: &[
            "CREATE TABLE t (id INTEGER, s TEXT)",
            "INSERT INTO t VALUES (1, 'hello')",
        ],
        query: "SELECT SUBSTRING(s, 2) AS a, SUBSTRING(s, 2, 3) AS b FROM t ORDER BY id",
    },
    // ROUND with a NEGATIVE digit count is treated as zero digits (SQLite never
    // rounds to tens/hundreds): round(2.567,-1) = round(2.567,0) = 3.0, not 0.0.
    Case {
        id: "round_negative_digits",
        setup: &[
            "CREATE TABLE t (id INTEGER, x REAL)",
            "INSERT INTO t VALUES (1, 2.567), (2, 12.5)",
        ],
        query: "SELECT ROUND(x, -1) AS a, ROUND(x, -5) AS b, ROUND(x, 1) AS c FROM t ORDER BY id",
    },
    // UNHEX decodes hex digit pairs into a blob (inverse of HEX). Even-length hex
    // → blob; odd length or a non-hex char → NULL. Compared as blobs directly
    // (wrapping in HEX would trip a separate, pre-existing HEX(NULL) divergence).
    Case {
        id: "unhex",
        setup: &[
            "CREATE TABLE t (id INTEGER, s TEXT)",
            "INSERT INTO t VALUES (1, '414243'), (2, 'abc'), (3, 'DEADbeef')",
        ],
        query: "SELECT UNHEX(s) AS r FROM t ORDER BY id",
    },
    // The 2-argument form ignores a set of characters, but only at byte
    // boundaries: '41.42' with '.' → x'4142', '4-1-4-2' with '-' → NULL.
    Case {
        id: "unhex_ignore_set",
        setup: &[
            "CREATE TABLE t (id INTEGER, s TEXT, ig TEXT)",
            "INSERT INTO t VALUES (1, '41.42', '.'), (2, '4-1-4-2', '-')",
        ],
        query: "SELECT UNHEX(s, ig) AS r FROM t ORDER BY id",
    },
    // HEX(NULL) is the EMPTY STRING, not NULL — SQLite casts the argument to a
    // blob first, and NULL → empty blob → ''. The non-NULL cases are unchanged.
    Case {
        id: "hex_of_null",
        setup: &[
            "CREATE TABLE t (id INTEGER, s TEXT)",
            "INSERT INTO t VALUES (1, 'abc'), (2, NULL)",
        ],
        query: "SELECT HEX(s) AS h, TYPEOF(HEX(s)) AS t FROM t ORDER BY id",
    },
    // OCTET_LENGTH counts BYTES (UTF-8), where LENGTH counts characters:
    // 'héllo' is 5 characters but 6 bytes.
    Case {
        id: "octet_length",
        setup: &[
            "CREATE TABLE t (id INTEGER, s TEXT)",
            "INSERT INTO t VALUES (1, 'héllo'), (2, '日本'), (3, ''), (4, NULL)",
        ],
        query: "SELECT OCTET_LENGTH(s) AS o, LENGTH(s) AS l FROM t ORDER BY id",
    },
    // LIKELY / UNLIKELY / LIKELIHOOD are planner hints — semantically the
    // identity function on their first argument.
    Case {
        id: "likely_family",
        setup: &[
            "CREATE TABLE t (id INTEGER, n INTEGER, s TEXT)",
            "INSERT INTO t VALUES (1, 5, 'a'), (2, NULL, 'b')",
        ],
        query: "SELECT LIKELY(n) AS a, UNLIKELY(s) AS b, LIKELIHOOD(id, 0.5) AS c FROM t ORDER BY id",
    },
    // GLOB(pattern, subject) — the function form of the GLOB operator: a
    // case-sensitive wildcard match (`*`, `?`, `[...]`).
    Case {
        id: "glob_function",
        setup: &[
            "CREATE TABLE t (id INTEGER, s TEXT)",
            "INSERT INTO t VALUES (1, 'hello'), (2, 'HELLO'), (3, 'help'), (4, NULL)",
        ],
        query: "SELECT GLOB('h*o', s) AS a, GLOB('hel[lp]*', s) AS b FROM t ORDER BY id",
    },
    // PRINTF / FORMAT with integer & string conversions (width, flags,
    // precision, hex, `%%`, and coercions). Avoids `%f` (declined) and `''`
    // string literals (a separate lexer limitation).
    Case {
        id: "printf",
        setup: &[
            "CREATE TABLE t (id INTEGER, name TEXT)",
            "INSERT INTO t VALUES (5, 'ada'), (42, 'grace')",
        ],
        query: "SELECT PRINTF('[%05d] %-8s %x %%', id, name, id) AS r FROM t ORDER BY id",
    },
    // FORMAT alias + missing-argument default (0) + `%q` via a quote from CHAR.
    Case {
        id: "printf_edges",
        setup: &[
            "CREATE TABLE t (id INTEGER)",
            "INSERT INTO t VALUES (1)",
        ],
        query: "SELECT FORMAT('%d %d', id) AS a, PRINTF('%q', CHAR(39)) AS b FROM t",
    },
    // A doubled single quote (`''`) inside a string literal is SQL's escape for
    // one literal quote — `'it''s'` is the 4-character string `it's`. Exercises
    // string literals in the SELECT list AND in an INSERT'd row value.
    Case {
        id: "escaped_quote_literal",
        setup: &[
            "CREATE TABLE t (id INTEGER, s TEXT)",
            "INSERT INTO t VALUES (1, 'O''Brien'), (2, 'it''s')",
        ],
        query: "SELECT s, LENGTH(s), 'a''b' AS lit FROM t ORDER BY id",
    },
    // A bare `JOIN` (no INNER/LEFT/… keyword) is an INNER join, and must produce
    // the same rows as an explicit `INNER JOIN`.
    Case {
        id: "bare_join",
        setup: &[
            "CREATE TABLE a (id INTEGER, b_id INTEGER)",
            "CREATE TABLE b (id INTEGER, name TEXT)",
            "INSERT INTO a VALUES (1, 10), (2, 20), (3, 99)",
            "INSERT INTO b VALUES (10, 'x'), (20, 'y')",
        ],
        query: "SELECT a.id, b.name FROM a JOIN b ON a.b_id = b.id ORDER BY a.id",
    },
    // A join with NO `ON` condition is a Cartesian (cross) product — both the
    // bare `JOIN` and the explicit `CROSS JOIN` forms.
    Case {
        id: "cross_product_no_on",
        setup: &[
            "CREATE TABLE a (x INTEGER)",
            "CREATE TABLE b (y INTEGER)",
            "INSERT INTO a VALUES (1), (2)",
            "INSERT INTO b VALUES (10), (20)",
        ],
        query: "SELECT a.x, b.y FROM a CROSS JOIN b ORDER BY a.x, b.y",
    },
    Case {
        id: "join_no_on_is_cross",
        setup: &[
            "CREATE TABLE a (x INTEGER)",
            "CREATE TABLE b (y INTEGER)",
            "INSERT INTO a VALUES (1), (2)",
            "INSERT INTO b VALUES (10), (20)",
        ],
        query: "SELECT a.x, b.y FROM a JOIN b ORDER BY a.x, b.y",
    },
    // A column alias may omit the `AS` keyword: `SELECT id n` names the output
    // column `n`, exactly like `SELECT id AS n`. SQLite (and standard SQL)
    // accept both spellings. The output *column name* is what this case checks,
    // so it directly exercises alias plumbing, not just row values.
    Case {
        id: "column_alias_without_as",
        setup: &[
            "CREATE TABLE t (id INTEGER, name TEXT)",
            "INSERT INTO t VALUES (1, 'a'), (2, 'b')",
        ],
        query: "SELECT id n, name label FROM t ORDER BY id",
    },
    // A bare alias must behave identically to the explicit-AS form and must not
    // swallow the trailing FROM/ORDER BY. Pairing an expression alias with a
    // plain column alias covers both the computed and the passthrough path.
    Case {
        id: "bare_alias_matches_as",
        setup: &[
            "CREATE TABLE t (a INTEGER, b INTEGER)",
            "INSERT INTO t VALUES (2, 3), (5, 7)",
        ],
        query: "SELECT a + b total, a first FROM t ORDER BY total",
    },
    // A table alias may omit the `AS` keyword: `FROM users u` aliases the table
    // exactly like `FROM users AS u`. A qualified reference through the bare
    // alias (`u.id`) must resolve, proving the alias reached the planner.
    Case {
        id: "table_alias_without_as",
        setup: &[
            "CREATE TABLE users (id INTEGER, name TEXT)",
            "INSERT INTO users VALUES (1, 'a'), (2, 'b'), (3, 'c')",
        ],
        query: "SELECT u.id, u.name FROM users u WHERE u.id > 1 ORDER BY u.id",
    },
    // Bare table aliases across a JOIN: both sides aliased without `AS`, joined
    // on the bare-aliased columns. Must match the explicit-AS join exactly.
    Case {
        id: "join_bare_table_alias",
        setup: &[
            "CREATE TABLE a (id INTEGER, v INTEGER)",
            "CREATE TABLE b (id INTEGER, w INTEGER)",
            "INSERT INTO a VALUES (1, 10), (2, 20)",
            "INSERT INTO b VALUES (1, 100), (2, 200)",
        ],
        query: "SELECT x.v, y.w FROM a x JOIN b y ON x.id = y.id ORDER BY x.v",
    },
    // MySQL-compatible `LIMIT off, count` shorthand: `LIMIT 1, 2` skips 1 row
    // then returns 2, identical to `LIMIT 2 OFFSET 1`. The FIRST number is the
    // offset and the SECOND is the count — the reverse of the OFFSET form —
    // which the planner swaps. ORDER BY makes the row window deterministic.
    Case {
        id: "limit_comma_offset_count",
        setup: &[
            "CREATE TABLE t (n INTEGER)",
            "INSERT INTO t VALUES (1), (2), (3), (4), (5)",
        ],
        query: "SELECT n FROM t ORDER BY n LIMIT 1, 2",
    },
    // The comma form and the equivalent `LIMIT count OFFSET off` form must
    // return the identical window — here both mean "skip 2, take 3".
    Case {
        id: "limit_comma_matches_offset",
        setup: &[
            "CREATE TABLE t (n INTEGER)",
            "INSERT INTO t VALUES (10), (20), (30), (40), (50), (60)",
        ],
        query: "SELECT n FROM t ORDER BY n LIMIT 2, 3",
    },
    // The `GLOB` infix operator: case-sensitive Unix-glob matching (`*` = any
    // run, `?` = one char). `X GLOB Y` is SQLite's `glob(Y, X)`. Case matters —
    // `x*` matches `xyz` but not `Xyz` (unlike LIKE, which is case-insensitive
    // for ASCII). Exercises the operator lowering to the glob builtin.
    Case {
        id: "glob_operator",
        setup: &[
            "CREATE TABLE t (id INTEGER, s TEXT)",
            "INSERT INTO t VALUES (1,'xyz'),(2,'abc'),(3,'xen'),(4,'Xyz')",
        ],
        query: "SELECT id FROM t WHERE s GLOB 'x*' ORDER BY id",
    },
    // `NOT GLOB` is the logical negation, and `?` matches exactly one char.
    Case {
        id: "not_glob_and_question",
        setup: &[
            "CREATE TABLE t (id INTEGER, s TEXT)",
            "INSERT INTO t VALUES (1,'cat'),(2,'cot'),(3,'coat'),(4,'cut')",
        ],
        query: "SELECT id FROM t WHERE s NOT GLOB 'c?t' ORDER BY id",
    },
];

/// Documented divergences: `(case id, reason)`. Ledger cases are executed but
/// exempt from the equality gate. **Shrinking this list is the conformance
/// metric**, and it is now **empty** — every seed case matches real SQLite.
///
/// It opened at ten reproduced gaps and was driven to zero one oracle-gated
/// increment at a time, in order:
///
/// - **`inner_join`** — a qualified reference like `a.name` across a join
///   resolved to `NULL` (cursor keyed under `None` while `LoadColumn` looked it
///   up under `Some("a")`); fixed by keying cursors on the effective alias.
/// - **`left_join`/`right_join`** — outer joins now NULL-pad the unmatched side
///   via a per-outer-row match flag (`RIGHT a b` = `LEFT b a`).
/// - **`full_join`** — a LEFT pass unioned with a RIGHT anti-join pass.
/// - **`string_functions`** — `UPPER(name)` returned an integer because a
///   positional→named→positional round-trip in `sql-vm`'s Phase-4 materialize
///   collapsed same-named output columns through a `HashMap` (last value wins);
///   fixed with positional projection, plus SQLite-style function column names.
/// - **`count_star`/`sum_min_max`/`avg`/`group_by`/`having`** — the aggregate
///   *rows* always matched; the output columns are now named the SQLite way
///   (`COUNT(*)`, `SUM(n)`, `AVG(n)`) instead of the engine-internal `agg_N`.
///
/// A newly discovered divergence is added back here with a reason rather than
/// silently skipped, so the list stays an honest measure.
const LEDGER: &[(&str, &str)] = &[
    // Empty — every seed case now matches real SQLite. The ledger opened at ten
    // reproduced gaps and has been driven to zero, one oracle-gated increment at
    // a time: INNER/LEFT/RIGHT/FULL JOIN semantics, scalar-function results and
    // names, and finally aggregate computed-column names (`agg_N` → `COUNT(*)`/
    // `SUM(n)`). New divergences, when found, are added back here with a reason
    // rather than silently skipped — shrinking this list remains the metric.
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
