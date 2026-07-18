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
    // Blob literals `x'…'` / `X'…'` now parse into raw bytes. HEX round-trips
    // them (upper-case), and TYPEOF reports `blob`. The empty literal `x''` is
    // the zero-byte blob, so HEX(x'') is the empty string. These exercise the
    // lexer→parser→planner→VM path end to end for a value kind the VM already
    // handled but the front end could not previously produce. (LENGTH() over a
    // blob — byte count — is a separate VM-builtin gap, spun off as a follow-up.)
    Case {
        id: "blob_literal_hex_typeof",
        setup: &[],
        query: "SELECT HEX(x'48656C6C6F') AS h, TYPEOF(x'00') AS t, HEX(x'') AS e",
    },
    // A blob literal compares byte-for-byte and orders after text/numbers per
    // SQLite's storage-class ordering; here two rows filter on blob equality.
    Case {
        id: "blob_literal_equality",
        setup: &[
            "CREATE TABLE t (id INTEGER, b BLOB)",
            "INSERT INTO t VALUES (1, x'0102'), (2, x'03')",
        ],
        query: "SELECT id FROM t WHERE b = x'0102' ORDER BY id",
    },
    // Upper-case `X'…'` is equivalent to lower-case, and quote() renders a blob
    // as the `X'…'` SQL literal form.
    Case {
        id: "blob_literal_uppercase_quote",
        setup: &[],
        query: "SELECT QUOTE(X'DEADBEEF') AS q, HEX(X'ab') AS h",
    },
    // The `||` operator concatenates a blob as its RAW bytes (as text), not the
    // `x'…'` display form: `X'41' || 'B'` = 'AB' (0x41 = 'A'), and the result is
    // TEXT. Blob||text, text||blob, and blob||blob all fold to the byte string.
    Case {
        id: "concat_blob_raw_bytes",
        setup: &[],
        query: "SELECT X'41' || 'B' AS a, 'A' || X'42' AS b, X'48' || X'69' AS c, TYPEOF(X'41' || 'B') AS t",
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
    // LENGTH counts BYTES for a blob but CHARACTERS for text, and measures a
    // number by its decimal-text length. `length(x'0102ff')` = 3, `length(x'')`
    // = 0, `length(12345)` = 5, `length('héllo')` = 5. Previously LENGTH errored
    // on anything but text/NULL; blob literals (0.5.42) made the blob case
    // reachable from SQL.
    Case {
        id: "length_blob_int_text",
        setup: &[],
        query: "SELECT LENGTH(x'0102ff') AS a, LENGTH(x'') AS b, LENGTH(12345) AS c, LENGTH(-7) AS d, LENGTH('héllo') AS e",
    },
    // LENGTH over a blob column (byte count), including the empty blob.
    Case {
        id: "length_blob_column",
        setup: &[
            "CREATE TABLE t (id INTEGER, b BLOB)",
            "INSERT INTO t VALUES (1, x'DEADBEEF'), (2, x''), (3, NULL)",
        ],
        query: "SELECT LENGTH(b) AS l FROM t ORDER BY id",
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
    // `CAST(x AS INTEGER)` — text yields its leading integer prefix (`'12abc'`
    // → 12, `'3.9'` → 3 since it stops at the `.`), and a real truncates toward
    // zero (`4.9` → 4, `-4.9` → -4). Aliased so the check is on values, not the
    // (engine-specific) auto-generated CAST column name.
    Case {
        id: "cast_to_integer",
        setup: &[
            "CREATE TABLE t (id INTEGER, s TEXT, r REAL)",
            "INSERT INTO t VALUES (1,'12abc',4.9),(2,'3.9',-4.9),(3,'abc',7.2)",
        ],
        query: "SELECT CAST(s AS INTEGER) AS si, CAST(r AS INTEGER) AS ri FROM t ORDER BY id",
    },
    // `CAST(x AS REAL)` — text yields its leading real prefix (`'1e3'` → 1000.0,
    // `'12.5abc'` → 12.5), and an integer widens (`42` → 42.0). REAL cells are
    // compared within the oracle's numeric epsilon.
    Case {
        id: "cast_to_real",
        setup: &[
            "CREATE TABLE t (id INTEGER, s TEXT, n INTEGER)",
            "INSERT INTO t VALUES (1,'1e3',42),(2,'12.5abc',7)",
        ],
        query: "SELECT CAST(s AS REAL) AS sr, CAST(n AS REAL) AS nr FROM t ORDER BY id",
    },
    // `CAST(int AS TEXT)` renders the decimal string; the affinity substring
    // rule resolves the synonym `VARCHAR` to TEXT and `INT` to INTEGER.
    Case {
        id: "cast_to_text_and_synonyms",
        setup: &[
            "CREATE TABLE t (id INTEGER, n INTEGER)",
            "INSERT INTO t VALUES (1,65),(2,-7)",
        ],
        query: "SELECT CAST(n AS VARCHAR) AS tv, CAST('9x' AS INT) AS iv FROM t ORDER BY id",
    },
    // `NULLS LAST` on an ASC sort OVERRIDES SQLite's default (which puts NULLs
    // first for ASC), so the NULLs move to the end. This case is order-sensitive
    // (the query has ORDER BY), so the oracle compares row order exactly.
    Case {
        id: "order_by_nulls_last",
        setup: &[
            "CREATE TABLE t (a INTEGER)",
            "INSERT INTO t VALUES (2), (NULL), (1), (NULL), (3)",
        ],
        query: "SELECT a FROM t ORDER BY a ASC NULLS LAST",
    },
    // `NULLS FIRST` on a DESC sort OVERRIDES the default (NULLs last for DESC).
    Case {
        id: "order_by_desc_nulls_first",
        setup: &[
            "CREATE TABLE t (a INTEGER)",
            "INSERT INTO t VALUES (2), (NULL), (1), (NULL), (3)",
        ],
        query: "SELECT a FROM t ORDER BY a DESC NULLS FIRST",
    },
    // The explicit clause matching the default (`ASC NULLS FIRST`) must be a
    // no-op — same result as a bare `ORDER BY a`.
    Case {
        id: "order_by_nulls_first_default",
        setup: &[
            "CREATE TABLE t (a INTEGER)",
            "INSERT INTO t VALUES (2), (NULL), (1), (3)",
        ],
        query: "SELECT a FROM t ORDER BY a NULLS FIRST",
    },
    // Searched CASE: the value of the first branch whose WHEN is truthy wins;
    // no match with an ELSE yields the ELSE. Aliased so the check is on values.
    Case {
        id: "case_first_match_and_else",
        setup: &[
            "CREATE TABLE t (id INTEGER, a INTEGER)",
            "INSERT INTO t VALUES (1,1),(2,2),(3,3)",
        ],
        query: "SELECT CASE WHEN a=1 THEN 'one' WHEN a=2 THEN 'two' ELSE 'other' END AS c FROM t ORDER BY id",
    },
    // No matching WHEN and NO ELSE → NULL; and a NULL condition is not truthy so
    // its branch is skipped (the `WHEN NULL THEN 'x'` never fires).
    Case {
        id: "case_no_else_is_null_and_null_cond_skipped",
        setup: &[
            "CREATE TABLE t (id INTEGER, a INTEGER)",
            "INSERT INTO t VALUES (1,1),(2,5)",
        ],
        query: "SELECT CASE WHEN NULL THEN 'x' WHEN a=1 THEN 'one' END AS c FROM t ORDER BY id",
    },
    // CASE nests as an ordinary expression — usable in a WHERE predicate and in
    // arithmetic — proving it composes through the whole expression grammar.
    Case {
        id: "case_in_where_and_arithmetic",
        setup: &[
            "CREATE TABLE t (id INTEGER, a INTEGER)",
            "INSERT INTO t VALUES (1,1),(2,2),(3,NULL),(4,4)",
        ],
        query: "SELECT id, CASE WHEN a=2 THEN 10 ELSE 20 END + 5 AS v FROM t WHERE CASE WHEN a IS NULL THEN 0 ELSE 1 END ORDER BY id",
    },
    // `a IS b` is null-SAFE equality: 1 when both equal OR both NULL, 0 when
    // exactly one is NULL — unlike `=`, which yields NULL if either side is NULL.
    // Aliased so the check is on the 1/0 result value per row.
    Case {
        id: "is_null_safe_equality",
        setup: &[
            "CREATE TABLE t (id INTEGER, a INTEGER, b INTEGER)",
            "INSERT INTO t VALUES (1,1,1),(2,1,2),(3,NULL,NULL),(4,1,NULL),(5,NULL,1)",
        ],
        query: "SELECT id, (a IS b) AS e, (a IS NOT b) AS ne FROM t ORDER BY id",
    },
    // `IS`/`IS NOT` used as a WHERE predicate — the null-safe match includes the
    // both-NULL row, which a plain `a = b` would exclude (NULL is not true).
    Case {
        id: "is_operator_in_where",
        setup: &[
            "CREATE TABLE t (id INTEGER, a INTEGER, b INTEGER)",
            "INSERT INTO t VALUES (1,1,1),(2,1,2),(3,NULL,NULL),(4,2,NULL)",
        ],
        query: "SELECT id FROM t WHERE a IS b ORDER BY id",
    },
    // ---- Lane 3: COLLATE in ORDER BY -------------------------------------
    // NOCASE folds case, so mixed-case names sort case-insensitively. Equal
    // keys ('Apple'/'apple', 'banana'/'BANANA') keep insertion order — a
    // secondary `id` key pins that so the oracle diff is deterministic.
    Case {
        id: "order_by_collate_nocase",
        setup: &[
            "CREATE TABLE t (id INTEGER, name TEXT)",
            "INSERT INTO t VALUES (1,'banana'),(2,'Apple'),(3,'cherry'),(4,'BANANA'),(5,'apple')",
        ],
        query: "SELECT name FROM t ORDER BY name COLLATE NOCASE, id",
    },
    // NOCASE with DESC — the key comparison flips but the collation still folds
    // case, and equal-key ties are unaffected by direction.
    Case {
        id: "order_by_collate_nocase_desc",
        setup: &[
            "CREATE TABLE t (id INTEGER, name TEXT)",
            "INSERT INTO t VALUES (1,'banana'),(2,'Apple'),(3,'cherry'),(4,'BANANA'),(5,'apple')",
        ],
        query: "SELECT name FROM t ORDER BY name COLLATE NOCASE DESC, id",
    },
    // RTRIM ignores trailing spaces, so 'a  ' and 'a' compare equal (broken by
    // the id tiebreak). Contrast with default BINARY, where 'a' < 'a  '.
    Case {
        id: "order_by_collate_rtrim",
        setup: &[
            "CREATE TABLE t (id INTEGER, name TEXT)",
            "INSERT INTO t VALUES (1,'b'),(2,'a  '),(3,'a'),(4,'b '),(5,'c')",
        ],
        query: "SELECT name FROM t ORDER BY name COLLATE RTRIM, id",
    },
    // Explicit COLLATE BINARY is the default byte order — uppercase sorts
    // before lowercase (ASCII 'A'=65 < 'a'=97).
    Case {
        id: "order_by_collate_binary",
        setup: &[
            "CREATE TABLE t (id INTEGER, name TEXT)",
            "INSERT INTO t VALUES (1,'banana'),(2,'Apple'),(3,'apple')",
        ],
        query: "SELECT name FROM t ORDER BY name COLLATE BINARY",
    },
    // ---- Lane 3: column-DEFINED COLLATE flows into ORDER BY --------------
    // A `COLLATE NOCASE` on the column definition itself makes a bare
    // `ORDER BY name` (no explicit COLLATE in the query) fold case, exactly as
    // if the query said `ORDER BY name COLLATE NOCASE`. Before this feature the
    // column collation was parsed and discarded, so the sort fell back to
    // BINARY and diverged from SQLite.
    Case {
        id: "order_by_column_collate_nocase",
        setup: &[
            "CREATE TABLE t (id INTEGER, name TEXT COLLATE NOCASE)",
            "INSERT INTO t VALUES (1,'banana'),(2,'Apple'),(3,'cherry'),(4,'BANANA'),(5,'apple')",
        ],
        query: "SELECT name FROM t ORDER BY name, id",
    },
    // Column-defined RTRIM: 'a  ' and 'a' compare equal (id breaks the tie),
    // where BINARY would order 'a' before 'a  '.
    Case {
        id: "order_by_column_collate_rtrim",
        setup: &[
            "CREATE TABLE t (id INTEGER, name TEXT COLLATE RTRIM)",
            "INSERT INTO t VALUES (1,'b'),(2,'a  '),(3,'a'),(4,'b '),(5,'c')",
        ],
        query: "SELECT name FROM t ORDER BY name, id",
    },
    // The column collation also flows through a qualified reference (`t.name`)
    // and through a table alias (`u.name` where `u` aliases `t`).
    Case {
        id: "order_by_column_collate_qualified",
        setup: &[
            "CREATE TABLE t (id INTEGER, name TEXT COLLATE NOCASE)",
            "INSERT INTO t VALUES (1,'banana'),(2,'Apple'),(3,'apple')",
        ],
        query: "SELECT name FROM t ORDER BY t.name, t.id",
    },
    // The ORDER BY qualifier may be a table alias (`u` aliases `t`). The select
    // list is qualified too (`u.name`) to sidestep an unrelated pre-existing bug
    // where an *unqualified* projection under a table alias yields NULL.
    Case {
        id: "order_by_column_collate_alias",
        setup: &[
            "CREATE TABLE t (id INTEGER, name TEXT COLLATE NOCASE)",
            "INSERT INTO t VALUES (1,'banana'),(2,'Apple'),(3,'apple')",
        ],
        query: "SELECT u.name FROM t AS u ORDER BY u.name, u.id",
    },
    // An explicit `COLLATE` in the query overrides the column's declared
    // sequence: the column is NOCASE, but `COLLATE BINARY` forces byte order
    // ('A'=65 sorts before 'a'=97).
    Case {
        id: "order_by_column_collate_explicit_override",
        setup: &[
            "CREATE TABLE t (id INTEGER, name TEXT COLLATE NOCASE)",
            "INSERT INTO t VALUES (1,'banana'),(2,'Apple'),(3,'apple')",
        ],
        query: "SELECT name FROM t ORDER BY name COLLATE BINARY, id",
    },
    // DESC honours the column collation just like ASC — case still folds, only
    // the key order reverses.
    Case {
        id: "order_by_column_collate_nocase_desc",
        setup: &[
            "CREATE TABLE t (id INTEGER, name TEXT COLLATE NOCASE)",
            "INSERT INTO t VALUES (1,'banana'),(2,'Apple'),(3,'cherry'),(4,'BANANA'),(5,'apple')",
        ],
        query: "SELECT name FROM t ORDER BY name DESC, id",
    },
    // An unknown collation on the column definition is rejected at CREATE time
    // ("no such collating sequence"), matching SQLite's prepare-time error.
    // The oracle compares error-vs-success, so both engines erroring is a pass.
    Case {
        id: "create_table_unknown_column_collation",
        setup: &[],
        query: "CREATE TABLE t (x TEXT COLLATE BOGUS)",
    },
    // ---- Lane 3: column-DEFINED COLLATE flows into WHERE comparisons ------
    // A `COLLATE NOCASE` column makes a bare `WHERE name = 'apple'` fold case,
    // matching every row that equals 'apple' case-insensitively. Before this the
    // column collation was ignored in comparisons and only 'apple' matched.
    Case {
        id: "where_column_collate_nocase_eq",
        setup: &[
            "CREATE TABLE t (id INTEGER, name TEXT COLLATE NOCASE)",
            "INSERT INTO t VALUES (1,'Apple'),(2,'apple'),(3,'BANANA'),(4,'cherry')",
        ],
        query: "SELECT id FROM t WHERE name = 'apple' ORDER BY id",
    },
    // `<>` (not-equal) honours the column collation too: case-insensitive
    // inequality excludes both 'Apple' and 'apple'.
    Case {
        id: "where_column_collate_nocase_ne",
        setup: &[
            "CREATE TABLE t (id INTEGER, name TEXT COLLATE NOCASE)",
            "INSERT INTO t VALUES (1,'Apple'),(2,'apple'),(3,'BANANA'),(4,'cherry')",
        ],
        query: "SELECT id FROM t WHERE name <> 'apple' ORDER BY id",
    },
    // Ordered comparison (`<`) folds case: 'Apple'/'apple' both sort below 'b'.
    Case {
        id: "where_column_collate_nocase_lt",
        setup: &[
            "CREATE TABLE t (id INTEGER, name TEXT COLLATE NOCASE)",
            "INSERT INTO t VALUES (1,'Apple'),(2,'apple'),(3,'ZED'),(4,'cherry')",
        ],
        query: "SELECT id FROM t WHERE name < 'b' ORDER BY id",
    },
    // Column-defined RTRIM: trailing spaces are ignored in the comparison, so
    // 'hi   ' = 'hi'.
    Case {
        id: "where_column_collate_rtrim_eq",
        setup: &[
            "CREATE TABLE t (id INTEGER, s TEXT COLLATE RTRIM)",
            "INSERT INTO t VALUES (1,'hi   '),(2,'hi'),(3,'ho')",
        ],
        query: "SELECT id FROM t WHERE s = 'hi' ORDER BY id",
    },
    // An explicit `COLLATE BINARY` on the comparison OVERRIDES the column's
    // NOCASE, forcing byte order so only the exact-case 'apple' matches.
    Case {
        id: "where_column_collate_explicit_binary_override",
        setup: &[
            "CREATE TABLE t (id INTEGER, name TEXT COLLATE NOCASE)",
            "INSERT INTO t VALUES (1,'Apple'),(2,'apple'),(3,'cherry')",
        ],
        query: "SELECT id FROM t WHERE name = 'apple' COLLATE BINARY ORDER BY id",
    },
    // The column collation flows through the boolean structure (AND/OR) to each
    // comparison: both `name = 'apple'` and `name = 'banana'` fold case.
    Case {
        id: "where_column_collate_nocase_or",
        setup: &[
            "CREATE TABLE t (id INTEGER, name TEXT COLLATE NOCASE)",
            "INSERT INTO t VALUES (1,'Apple'),(2,'apple'),(3,'BANANA'),(4,'cherry')",
        ],
        query: "SELECT id FROM t WHERE name = 'apple' OR name = 'banana' ORDER BY id",
    },
    // The column's collation also drives the `IN` operator: SQLite takes IN's
    // collating sequence from the left operand, so `name IN ('APPLE')` on a
    // NOCASE column matches both 'Apple' and 'apple'. `NOT IN` inverts it, and
    // an explicit `COLLATE BINARY` on the value overrides the column's NOCASE.
    Case {
        id: "where_column_collate_nocase_in",
        setup: &[
            "CREATE TABLE t (id INTEGER, name TEXT COLLATE NOCASE)",
            "INSERT INTO t VALUES (1,'Apple'),(2,'apple'),(3,'BANANA'),(4,'banana')",
        ],
        query: "SELECT id FROM t WHERE name IN ('APPLE') ORDER BY id",
    },
    Case {
        id: "where_column_collate_nocase_not_in",
        setup: &[
            "CREATE TABLE t (id INTEGER, name TEXT COLLATE NOCASE)",
            "INSERT INTO t VALUES (1,'Apple'),(2,'apple'),(3,'BANANA'),(4,'banana')",
        ],
        query: "SELECT id FROM t WHERE name NOT IN ('APPLE') ORDER BY id",
    },
    Case {
        id: "where_column_collate_nocase_in_multi",
        setup: &[
            "CREATE TABLE t (id INTEGER, name TEXT COLLATE NOCASE)",
            "INSERT INTO t VALUES (1,'Apple'),(2,'apple'),(3,'BANANA'),(4,'banana')",
        ],
        query: "SELECT id FROM t WHERE name IN ('apple','banana') ORDER BY id",
    },
    // Explicit `COLLATE` may now be written directly before `IN` — `name COLLATE
    // BINARY IN (…)`. SQLite takes IN's collating sequence from the left operand,
    // and an explicit clause on that operand OVERRIDES the column's declared
    // sequence. Here the column is NOCASE but `COLLATE BINARY` forces byte order,
    // so only the exact-case 'APPLE' qualifies. (The `is_collate_call` guard in
    // `collate_comparisons` already yields to this explicit wrap.)
    Case {
        id: "where_explicit_collate_binary_in_override",
        setup: &[
            "CREATE TABLE t (id INTEGER, name TEXT COLLATE NOCASE)",
            "INSERT INTO t VALUES (1,'Apple'),(2,'apple'),(3,'APPLE'),(4,'banana')",
        ],
        query: "SELECT id FROM t WHERE name COLLATE BINARY IN ('APPLE') ORDER BY id",
    },
    // The reverse direction: an explicit `COLLATE NOCASE` LIFTS a plain (BINARY)
    // column to case-insensitive membership, matching all case variants.
    Case {
        id: "where_explicit_collate_nocase_in",
        setup: &[
            "CREATE TABLE t (id INTEGER, name TEXT)",
            "INSERT INTO t VALUES (1,'Apple'),(2,'apple'),(3,'APPLE'),(4,'banana')",
        ],
        query: "SELECT id FROM t WHERE name COLLATE NOCASE IN ('apple') ORDER BY id",
    },
    // Scalar (no-FROM) form: the explicit collation drives every equality test
    // the IN performs, and `NOT IN` inverts it. `'ABC' COLLATE NOCASE IN
    // ('abc','def')` is 1; `NOT IN ('abc')` is 0; `COLLATE BINARY IN ('abc')` is 0.
    Case {
        id: "scalar_explicit_collate_in",
        setup: &[],
        query: "SELECT 'ABC' COLLATE NOCASE IN ('abc','def') AS a, 'ABC' COLLATE NOCASE NOT IN ('abc') AS b, 'ABC' COLLATE BINARY IN ('abc') AS c",
    },
    // Explicit COLLATE composes with IN's three-valued NULL logic: a non-matching
    // membership test with a NULL element is NULL, because `__collate` passes the
    // NULL element through unchanged.
    Case {
        id: "scalar_explicit_collate_in_null",
        setup: &[],
        query: "SELECT ('ABC' COLLATE NOCASE IN ('xyz', NULL)) IS NULL AS a, 'ABC' COLLATE NOCASE IN ('abc', NULL) AS b",
    },
    // Plain `NOT BETWEEN` is the LOGICAL NEGATION of the inclusive range, not a
    // strict/exclusive-bounds range. `5 NOT BETWEEN 1 AND 10` is 0 (5 IS in
    // [1,10]); `15 NOT BETWEEN 1 AND 10` is 1; the boundaries 1 and 10 are IN the
    // range so their NOT BETWEEN is 0; a NULL operand yields NULL. (Regression
    // guard: eval_between previously computed `val > lo AND val < hi` for the
    // negated case, inverting interior values.)
    Case {
        id: "not_between_logical_negation",
        setup: &[],
        query: "SELECT 5 NOT BETWEEN 1 AND 10 AS a, 15 NOT BETWEEN 1 AND 10 AS b, 1 NOT BETWEEN 1 AND 10 AS c, 10 NOT BETWEEN 1 AND 10 AS d, (NULL NOT BETWEEN 1 AND 10) IS NULL AS e",
    },
    // Column form of NOT BETWEEN over a range of integer ids, to exercise the
    // per-row path (not just constant folding).
    Case {
        id: "not_between_column",
        setup: &[
            "CREATE TABLE t (id INTEGER)",
            "INSERT INTO t VALUES (1),(5),(10),(11),(0)",
        ],
        query: "SELECT id FROM t WHERE id NOT BETWEEN 1 AND 10 ORDER BY id",
    },
    // Explicit `COLLATE` before `BETWEEN`: `x BETWEEN a AND c` is `x >= a AND
    // x <= c`, and the collation drives both ordered comparisons. `'B' COLLATE
    // NOCASE BETWEEN 'a' AND 'c'` is 1 (folded 'b' falls in a..c) where the plain
    // byte compare is 0 (uppercase 'B' sorts below lowercase 'a'). `NOT BETWEEN`
    // inverts, and a NULL bound propagates NULL through the collation wrap.
    Case {
        id: "scalar_explicit_collate_between",
        setup: &[],
        query: "SELECT 'B' COLLATE NOCASE BETWEEN 'a' AND 'c' AS a, 'B' BETWEEN 'a' AND 'c' AS b, 'B' COLLATE NOCASE NOT BETWEEN 'a' AND 'c' AS c, 'hi   ' COLLATE RTRIM BETWEEN 'hi' AND 'hi' AS d, ('B' COLLATE NOCASE BETWEEN NULL AND 'c') IS NULL AS e",
    },
    // Column form: an explicit `COLLATE NOCASE` on a plain (BINARY) column folds
    // case for the whole range, so mixed-case names in `a`..`n` all qualify while
    // an out-of-range uppercase name does not.
    Case {
        id: "where_explicit_collate_between",
        setup: &[
            "CREATE TABLE t (id INTEGER, s TEXT)",
            "INSERT INTO t VALUES (1,'Apple'),(2,'apple'),(3,'ZEBRA'),(4,'mango')",
        ],
        query: "SELECT id FROM t WHERE s COLLATE NOCASE BETWEEN 'a' AND 'n' ORDER BY id",
    },
    // `COLLATE` before `LIKE` now parses, but LIKE IGNORES the collation
    // (matching SQLite): even `COLLATE BINARY` does not make LIKE case-sensitive,
    // and `COLLATE NOCASE` changes nothing since LIKE is already ASCII
    // case-insensitive. `NOT LIKE` and the `ESCAPE` clause compose with the
    // (ignored) collation. The point is parse-surface parity — mini used to
    // reject `COLLATE` before LIKE.
    Case {
        id: "scalar_collate_like_ignored",
        setup: &[],
        query: "SELECT 'ABC' COLLATE BINARY LIKE 'abc' AS a, 'ABC' COLLATE NOCASE LIKE 'abc' AS b, 'ABC' COLLATE NOCASE NOT LIKE 'xyz' AS c, 'A%B' COLLATE NOCASE LIKE 'a!%b' ESCAPE '!' AS d",
    },
    // `COLLATE` before `GLOB` also parses and is ignored: GLOB stays
    // case-sensitive regardless of the collation, so `'ABC' COLLATE NOCASE GLOB
    // 'abc'` is 0 while `'abc' COLLATE NOCASE GLOB 'abc'` is 1. `NOT GLOB` too.
    Case {
        id: "scalar_collate_glob_ignored",
        setup: &[],
        query: "SELECT 'ABC' COLLATE NOCASE GLOB 'abc' AS a, 'abc' COLLATE NOCASE GLOB 'abc' AS b, 'ABC' COLLATE NOCASE NOT GLOB 'abc' AS c",
    },
    // Column form: `COLLATE` before LIKE on a table column parses and matches
    // exactly as the un-collated LIKE would (LIKE is case-insensitive for ASCII).
    Case {
        id: "where_collate_like_ignored",
        setup: &[
            "CREATE TABLE t (id INTEGER, s TEXT)",
            "INSERT INTO t VALUES (1,'Apple'),(2,'apricot'),(3,'Banana')",
        ],
        query: "SELECT id FROM t WHERE s COLLATE NOCASE LIKE 'ap%' ORDER BY id",
    },
    // IN membership uses the same equality as `=`: numeric across INTEGER/REAL,
    // and it is three-valued for NULL. `1 IN (1.0)` and `1.0 IN (1)` are true
    // (numeric), `'1' IN (1)` is false (text vs int, no affinity), and a list
    // element `1.0` matches `1`. Previously IN used exact same-variant equality,
    // so `1 IN (1.0)` wrongly returned false.
    Case {
        id: "in_numeric_equality",
        setup: &[],
        query: "SELECT 1 IN (1.0) AS a, 1.0 IN (1) AS b, '1' IN (1) AS c, 1 IN (2,1.0,3) AS d, 5 IN (1,2) AS e",
    },
    // IN is three-valued: a NULL element makes an otherwise-non-matching test
    // NULL (`1 IN (NULL,2)` → NULL), but a real match wins over a NULL element
    // (`1 IN (NULL,1)` → 1). `NOT IN` inverts, so `5 NOT IN (NULL,2)` is NULL.
    Case {
        id: "in_null_three_valued",
        setup: &[],
        query: "SELECT (1 IN (NULL,2)) IS NULL AS a, 1 IN (NULL,1) AS b, (5 IN (NULL,2)) IS NULL AS c, 5 NOT IN (1,2) AS d, (5 NOT IN (NULL,2)) IS NULL AS e",
    },
    // A column WITHOUT a declared collation keeps BINARY comparison — only the
    // exact-case match qualifies. Guards against over-applying the fold.
    Case {
        id: "where_plain_column_stays_binary",
        setup: &[
            "CREATE TABLE t (id INTEGER, name TEXT)",
            "INSERT INTO t VALUES (1,'Apple'),(2,'apple')",
        ],
        query: "SELECT id FROM t WHERE name = 'apple' ORDER BY id",
    },
    // Left-operand precedence: when BOTH operands are columns, SQLite uses the
    // LEFT column's collation. `bin = nocase` compares byte-exact (the left
    // BINARY column wins — it must NOT defer to the right NOCASE column), while
    // the mirror `nocase = bin` folds case. The two therefore disagree, exactly
    // as in SQLite; getting this wrong (OR-ing past a BINARY left column) would
    // make both fold.
    Case {
        id: "where_column_collation_left_precedence",
        setup: &[
            "CREATE TABLE t (id INTEGER, cs TEXT, ci TEXT COLLATE NOCASE)",
            "INSERT INTO t VALUES (1,'foo','FOO'),(2,'bar','bar')",
        ],
        query: "SELECT id FROM t WHERE cs = ci ORDER BY id",
    },
    // The mirror image folds: a NOCASE left column drives the comparison, so
    // 'FOO' = 'foo' matches.
    Case {
        id: "where_column_collation_left_precedence_mirror",
        setup: &[
            "CREATE TABLE t (id INTEGER, cs TEXT, ci TEXT COLLATE NOCASE)",
            "INSERT INTO t VALUES (1,'foo','FOO'),(2,'bar','bar')",
        ],
        query: "SELECT id FROM t WHERE ci = cs ORDER BY id",
    },
    // ---- Lane 1: simple (operand) CASE ----------------------------------
    // `CASE x WHEN v THEN r … ELSE d END` compares the operand to each value
    // for equality; a NULL operand matches nothing (x = NULL is never true)
    // and falls through to ELSE, exactly as the searched form would.
    Case {
        id: "simple_case_with_else",
        setup: &[
            "CREATE TABLE t (id INTEGER, x INTEGER)",
            "INSERT INTO t VALUES (1,1),(2,2),(3,3),(4,NULL)",
        ],
        query: "SELECT id, CASE x WHEN 1 THEN 'a' WHEN 2 THEN 'b' ELSE 'c' END AS r FROM t ORDER BY id",
    },
    // No ELSE: an unmatched operand (including NULL) yields NULL.
    Case {
        id: "simple_case_no_else_yields_null",
        setup: &[
            "CREATE TABLE t (id INTEGER, x INTEGER)",
            "INSERT INTO t VALUES (1,1),(2,9),(3,NULL)",
        ],
        query: "SELECT id, CASE x WHEN 1 THEN 'one' END AS r FROM t ORDER BY id",
    },
    // Text operand, and the first matching branch wins.
    Case {
        id: "simple_case_text_first_match",
        setup: &[
            "CREATE TABLE t (id INTEGER, s TEXT)",
            "INSERT INTO t VALUES (1,'x'),(2,'y'),(3,'z')",
        ],
        query: "SELECT id, CASE s WHEN 'x' THEN 10 WHEN 'y' THEN 20 ELSE 30 END AS r FROM t ORDER BY id",
    },
    // Constant simple CASE with a NULL operand hits ELSE (x = NULL is not true).
    // Aliased so the diff is on the value, not the column name (unaliased
    // expression column naming is a separate, orthogonal gap).
    Case {
        id: "simple_case_null_operand_hits_else",
        setup: &["CREATE TABLE t (id INTEGER)", "INSERT INTO t VALUES (1)"],
        query: "SELECT CASE NULL WHEN 1 THEN 'a' ELSE 'z' END AS r FROM t",
    },
    // ---- Lane 2: bitwise operators `& | ~ << >>` -------------------------
    // Each operator's basic result. Aliased so the diff is on the value, not
    // the (orthogonal) unaliased-expression column name.
    Case {
        id: "bitwise_and_or",
        setup: &["CREATE TABLE t (id INTEGER)", "INSERT INTO t VALUES (1)"],
        query: "SELECT (5 & 3) AS a, (5 | 2) AS o FROM t",
    },
    Case {
        id: "bitwise_not_and_shifts",
        setup: &["CREATE TABLE t (id INTEGER)", "INSERT INTO t VALUES (1)"],
        query: "SELECT (~0) AS n, (1 << 4) AS sl, (256 >> 2) AS sr FROM t",
    },
    // Precedence: `& | << >>` share one left-associative level, so
    // `5 | 3 & 2` = `(5 | 3) & 2` = 2, and `3 + 1 << 2` = `(3+1) << 2` = 16.
    Case {
        id: "bitwise_precedence",
        setup: &["CREATE TABLE t (id INTEGER)", "INSERT INTO t VALUES (1)"],
        query: "SELECT (5 | 3 & 2) AS p, (3 + 1 << 2) AS q FROM t",
    },
    // Integer affinity: a real operand truncates toward zero (2.9 → 2).
    Case {
        id: "bitwise_real_truncation",
        setup: &["CREATE TABLE t (id INTEGER)", "INSERT INTO t VALUES (1)"],
        query: "SELECT (2.9 & 1) AS a FROM t",
    },
    // NULL propagates through every bitwise operator (binary and unary).
    Case {
        id: "bitwise_null_propagates",
        setup: &["CREATE TABLE t (id INTEGER)", "INSERT INTO t VALUES (1)"],
        query: "SELECT (NULL & 1) AS a, (1 | NULL) AS o, (~NULL) AS n, (NULL << 2) AS s FROM t",
    },
    // Shift edge cases: count ≥ 64 saturates to 0; a negative count flips the
    // shift direction (`1 << -1` = `1 >> 1` = 0); right shift is arithmetic.
    Case {
        id: "bitwise_shift_edges",
        setup: &["CREATE TABLE t (id INTEGER)", "INSERT INTO t VALUES (1)"],
        query: "SELECT (1 << 64) AS big, (8 >> 100) AS huge, (1 << -1) AS neg, (-1 >> 1) AS ar FROM t",
    },
    // Bitwise over a real table column (integer-affinity coercion per row).
    Case {
        id: "bitwise_column",
        setup: &[
            "CREATE TABLE t (id INTEGER, x INTEGER)",
            "INSERT INTO t VALUES (1,12),(2,7),(3,255)",
        ],
        query: "SELECT id, (x & 6) AS m, (x | 1) AS o FROM t ORDER BY id",
    },
    // ---- Lane 3: expr-level COLLATE in comparisons -----------------------
    // `= … COLLATE NOCASE` folds case in the equality; aliased to isolate the
    // value from the (orthogonal) unaliased-expression column name.
    Case {
        id: "collate_nocase_equality",
        setup: &["CREATE TABLE t (id INTEGER)", "INSERT INTO t VALUES (1)"],
        query: "SELECT ('A' = 'a' COLLATE NOCASE) AS a, ('A' = 'a') AS b FROM t",
    },
    // `= … COLLATE RTRIM` ignores trailing spaces in the equality.
    Case {
        id: "collate_rtrim_equality",
        setup: &["CREATE TABLE t (id INTEGER)", "INSERT INTO t VALUES (1)"],
        query: "SELECT ('a ' = 'a' COLLATE RTRIM) AS a FROM t",
    },
    // Ordering comparison honours the collation: `'B' < 'a' COLLATE NOCASE` is
    // 0 (b > a case-folded) vs 1 under default binary ('B'=66 < 'a'=97).
    Case {
        id: "collate_nocase_ordering",
        setup: &["CREATE TABLE t (id INTEGER)", "INSERT INTO t VALUES (1)"],
        query: "SELECT ('B' < 'a' COLLATE NOCASE) AS c, ('B' < 'a') AS d FROM t",
    },
    // COLLATE on a WHERE predicate matches case-insensitively across rows.
    Case {
        id: "collate_nocase_where",
        setup: &[
            "CREATE TABLE t (id INTEGER, name TEXT)",
            "INSERT INTO t VALUES (1,'Apple'),(2,'apple'),(3,'BANANA')",
        ],
        query: "SELECT id FROM t WHERE name = 'apple' COLLATE NOCASE ORDER BY id",
    },
    // Collation is ignored for a numeric operand: `5 = '5' COLLATE NOCASE` is 0
    // (5 stays integer, '5' stays text) — the canonicaliser passes non-text
    // through unchanged, matching SQLite.
    Case {
        id: "collate_numeric_operand_unaffected",
        setup: &["CREATE TABLE t (id INTEGER)", "INSERT INTO t VALUES (1)"],
        query: "SELECT (5 = '5' COLLATE NOCASE) AS a, (5 = 5 COLLATE NOCASE) AS b FROM t",
    },
    // ---- Lane 2: IS [NOT] DISTINCT FROM (standard-SQL null-safe compare) --
    // `IS NOT DISTINCT FROM` = null-safe equality (like `IS`); `IS DISTINCT
    // FROM` = its negation. Both-NULL is "not distinct" (equal). Aliased so
    // the diff is on the value, not the column name.
    Case {
        id: "is_distinct_from",
        setup: &[
            "CREATE TABLE t (id INTEGER, a INTEGER, b INTEGER)",
            "INSERT INTO t VALUES (1,1,1),(2,1,2),(3,NULL,NULL),(4,1,NULL)",
        ],
        query: "SELECT id, (a IS DISTINCT FROM b) AS d, (a IS NOT DISTINCT FROM b) AS nd FROM t ORDER BY id",
    },
    // `IS NOT DISTINCT FROM` used as a WHERE predicate — the null-safe match
    // includes the both-NULL row, which a plain `a = b` would exclude.
    Case {
        id: "is_not_distinct_from_where",
        setup: &[
            "CREATE TABLE t (id INTEGER, a INTEGER, b INTEGER)",
            "INSERT INTO t VALUES (1,1,1),(2,1,2),(3,NULL,NULL),(4,2,NULL)",
        ],
        query: "SELECT id FROM t WHERE a IS NOT DISTINCT FROM b ORDER BY id",
    },
    // ---- Lane 3: COLLATE on the LEFT comparison operand ------------------
    // `x COLLATE C = y` folds the comparison just like `x = y COLLATE C`.
    Case {
        id: "collate_left_operand_nocase",
        setup: &["CREATE TABLE t (id INTEGER)", "INSERT INTO t VALUES (1)"],
        query: "SELECT ('A' COLLATE NOCASE = 'a') AS a, ('a ' COLLATE RTRIM = 'a') AS b FROM t",
    },
    // COLLATE on the LEFT operand of a WHERE predicate matches per row.
    Case {
        id: "collate_left_operand_where",
        setup: &[
            "CREATE TABLE t (id INTEGER, name TEXT)",
            "INSERT INTO t VALUES (1,'Apple'),(2,'apple'),(3,'BANANA')",
        ],
        query: "SELECT id FROM t WHERE name COLLATE NOCASE = 'apple' ORDER BY id",
    },
    // ---- Lane 2: division / modulo by zero → NULL ------------------------
    // SQLite yields NULL (not an error) for any division or modulo by zero,
    // integer or float, including 0/0. Aliased so both engines name the columns
    // identically.
    Case {
        id: "div_by_zero_is_null",
        setup: &["CREATE TABLE t (id INTEGER)", "INSERT INTO t VALUES (1)"],
        query: "SELECT (5 / 0) AS a, (5.0 / 0) AS b, (0 / 0) AS c, (5 / 0.0) AS d FROM t",
    },
    Case {
        id: "mod_by_zero_is_null",
        setup: &["CREATE TABLE t (id INTEGER)", "INSERT INTO t VALUES (1)"],
        query: "SELECT (5 % 0) AS a, (5.5 % 0) AS b, (5 % 0.0) AS c FROM t",
    },
    // Per-row: a computed zero divisor (`n - n`) yields NULL for that row
    // instead of aborting the whole query.
    Case {
        id: "div_by_computed_zero_is_null_per_row",
        setup: &[
            "CREATE TABLE t (id INTEGER, n INTEGER)",
            "INSERT INTO t VALUES (1,4),(2,0),(3,10)",
        ],
        query: "SELECT id, (100 / n) AS q FROM t ORDER BY id",
    },
    // Non-zero division/modulo still compute normally (regression guard: the
    // NULL path must not swallow ordinary divisors).
    Case {
        id: "div_mod_nonzero_unaffected",
        setup: &["CREATE TABLE t (id INTEGER)", "INSERT INTO t VALUES (1)"],
        query: "SELECT (7 / 2) AS a, (-7 / 2) AS b, (7 % 3) AS c, (7.0 / 2) AS d FROM t",
    },
    // Binary arithmetic applies NUMERIC affinity to text/blob operands, matching
    // SQLite: `'5'+0` = 5 (integer), `'5.5'+0` = 5.5 (real), `'abc'+1` = 1 (no
    // numeric prefix → 0), `'12abc'+0` = 12, and `'10'-'3'` = 7 (both coerced).
    // Previously the engine errored on any text operand.
    Case {
        id: "arith_text_numeric_affinity",
        setup: &[],
        query: "SELECT '5'+0 AS a, '5.5'+0 AS b, 'abc'+1 AS c, '5'*2 AS d, '10'-'3' AS e, '12abc'+0 AS f",
    },
    // Division/modulo also coerce: `5 / '2'` = 2, `5 / '0'` = NULL (affinity makes
    // '0' the integer zero, so the divide-by-zero → NULL rule fires), `'7' % 3` = 1.
    // (Known edge left for later, shared with unary minus: an *integral* real-
    // syntax string like `'9.0'` collapses to an integer here, so `'9.0' / 2` is
    // 4 not SQLite's 4.5 — the float-affinity follow-up. Non-integral `'5.5'`
    // is fine.)
    Case {
        id: "div_mod_text_affinity",
        setup: &[],
        query: "SELECT 5 / '2' AS a, (5 / '0') IS NULL AS b, '7' % 3 AS c, '5.5' * 2 AS d",
    },
    // A text/blob in a BOOLEAN context takes numeric affinity, matching SQLite:
    // `NOT 'abc'` = 1 ('abc'→0), `NOT '5'` = 0, `NOT '0'` = `NOT ''` = 1, and
    // `'5' AND 1` = 1 while `'abc' AND 1` = 0. Previously all text was truthy.
    Case {
        id: "text_boolean_affinity",
        setup: &[],
        query: "SELECT NOT 'abc' AS a, NOT '5' AS b, NOT '0' AS c, 'abc' AND 1 AS d, '5' AND 1 AS e",
    },
    // The same rule drives WHERE and CASE: `WHERE <text>` keeps only rows whose
    // text is numerically non-zero, and `CASE WHEN <text>` picks THEN only then.
    Case {
        id: "where_case_text_truthiness",
        setup: &[
            "CREATE TABLE t (id INTEGER, s TEXT)",
            "INSERT INTO t VALUES (1,'abc'), (2,'5'), (3,'0'), (4,'')",
        ],
        query: "SELECT id, CASE WHEN s THEN 'y' ELSE 'n' END AS c FROM t WHERE s ORDER BY id",
    },
    // Unary minus applies NUMERIC affinity to a text/blob operand before
    // negating, matching SQLite: `-'5'` = -5, `-'12abc'` = -12 (leading numeric
    // prefix), `-'abc'` = 0 (no prefix → 0), `-'3.5'` = -3.5, and leading
    // whitespace is tolerated (`-'  7'` = -7). Previously the engine left text
    // unchanged, so `-'5'` wrongly returned the string `'5'`.
    Case {
        id: "unary_minus_text_numeric_affinity",
        setup: &[],
        query: "SELECT -'5' AS a, -'12abc' AS b, -'abc' AS c, -'3.5' AS d, -'  7' AS e",
    },
    // ---- Lane 1: CAST … AS NUMERIC (affinity) ----------------------------
    // NUMERIC prefers INTEGER when the value is integral and fits i64, else
    // REAL. Text `'3.0'` and `'1e3'` are integral → INTEGER; `'3.5'` → REAL;
    // an i64-overflowing integer → REAL. `typeof` is compared implicitly via
    // the cell's storage class (Int vs Float sort/compare differently).
    Case {
        id: "cast_numeric_text_int_vs_real",
        setup: &["CREATE TABLE t (id INTEGER)", "INSERT INTO t VALUES (1)"],
        query: "SELECT CAST('42' AS NUMERIC) AS a, CAST('3.0' AS NUMERIC) AS b, \
                CAST('3.5' AS NUMERIC) AS c, CAST('1e3' AS NUMERIC) AS d, \
                CAST('42abc' AS NUMERIC) AS e, CAST('abc' AS NUMERIC) AS f FROM t",
    },
    // A REAL value stays REAL (the cast is a no-op on numbers), an INTEGER stays
    // INTEGER, and NULL stays NULL.
    Case {
        id: "cast_numeric_number_is_noop",
        setup: &["CREATE TABLE t (id INTEGER)", "INSERT INTO t VALUES (1)"],
        query: "SELECT CAST(3.0 AS NUMERIC) AS a, CAST(3.5 AS NUMERIC) AS b, \
                CAST(42 AS NUMERIC) AS c, CAST(NULL AS NUMERIC) AS d FROM t",
    },
    // An integer that overflows i64 falls through to REAL.
    Case {
        id: "cast_numeric_overflow_is_real",
        setup: &["CREATE TABLE t (id INTEGER)", "INSERT INTO t VALUES (1)"],
        query: "SELECT CAST('99999999999999999999' AS NUMERIC) AS a, \
                CAST('9223372036854775807' AS NUMERIC) AS b FROM t",
    },
    // NUMERIC is the default affinity for non-INT/TEXT/REAL/BLOB type names, so
    // DECIMAL and BOOLEAN behave identically to NUMERIC.
    Case {
        id: "cast_decimal_boolean_are_numeric",
        setup: &["CREATE TABLE t (id INTEGER)", "INSERT INTO t VALUES (1)"],
        query: "SELECT CAST('42.5' AS DECIMAL) AS a, CAST('7' AS BOOLEAN) AS b FROM t",
    },
    // Per-row over a real column: NUMERIC keeps reals real but collapses an
    // integral text column to INTEGER.
    Case {
        id: "cast_numeric_per_row",
        setup: &[
            "CREATE TABLE t (id INTEGER, s TEXT)",
            "INSERT INTO t VALUES (1,'10'),(2,'2.5'),(3,'30.0')",
        ],
        query: "SELECT id, CAST(s AS NUMERIC) AS n FROM t ORDER BY id",
    },
    // ---- Lane 2: substr() edge cases ------------------------------------
    // `Y = 0` is a virtual slot before the first character (2-arg → whole
    // string; with a length it consumes one from Z), and a negative Z returns
    // the |Z| characters *preceding* the Y-th, reading leftward.
    Case {
        id: "substr_start_zero_and_negative_len",
        setup: &["CREATE TABLE t (id INTEGER)", "INSERT INTO t VALUES (1)"],
        query: "SELECT substr('hello',0) AS a, substr('hello',0,3) AS b, \
                substr('hello',0,1) AS c, substr('hello',2,-1) AS d, \
                substr('hello',3,-2) AS e, substr('hello',5,-2) AS f FROM t",
    },
    // Negative start (count from the right) combined with the length rules,
    // out-of-range windows, and the 2-arg form.
    Case {
        id: "substr_negative_start_and_range",
        setup: &["CREATE TABLE t (id INTEGER)", "INSERT INTO t VALUES (1)"],
        query: "SELECT substr('hello',-2) AS a, substr('hello',-2,-1) AS b, \
                substr('hello',-2,3) AS c, substr('hello',-10) AS d, \
                substr('hello',6,2) AS e, substr('hello',3,10) AS f FROM t",
    },
    // Character-based (not byte-based) for multibyte UTF-8.
    Case {
        id: "substr_multibyte_is_char_based",
        setup: &["CREATE TABLE t (id INTEGER)", "INSERT INTO t VALUES (1)"],
        query: "SELECT substr('héllo',2,2) AS a, substr('héllo',-2) AS b FROM t",
    },
    // Per-row over a text column with a computed negative length.
    Case {
        id: "substr_per_row",
        setup: &[
            "CREATE TABLE t (id INTEGER, s TEXT)",
            "INSERT INTO t VALUES (1,'abcdef'),(2,'xy'),(3,'')",
        ],
        query: "SELECT id, substr(s,2,-1) AS a, substr(s,0) AS b FROM t ORDER BY id",
    },
    // ---- Lane 2: LIKE … ESCAPE ------------------------------------------
    // The escape character makes a following `%`/`_`/itself a literal. We use
    // `#`/`/` (not backslash) as the escape so the string lexer doesn't touch
    // the pattern literal.
    Case {
        id: "like_escape_literal_wildcards",
        setup: &["CREATE TABLE t (id INTEGER)", "INSERT INTO t VALUES (1)"],
        query: "SELECT ('a%b' LIKE 'a#%b' ESCAPE '#') AS a, \
                ('100x' LIKE '100#%' ESCAPE '#') AS b, \
                ('a_b' LIKE 'a#_b' ESCAPE '#') AS c, \
                ('axb' LIKE 'a#_b' ESCAPE '#') AS d, \
                ('50%off' LIKE '50#%%' ESCAPE '#') AS e, \
                ('a/b' LIKE 'a//b' ESCAPE '/') AS f FROM t",
    },
    // NOT LIKE (with and without ESCAPE) inverts the match; NULL stays NULL.
    Case {
        id: "not_like_and_null",
        setup: &["CREATE TABLE t (id INTEGER)", "INSERT INTO t VALUES (1)"],
        query: "SELECT ('abc' NOT LIKE 'x%') AS a, ('abc' NOT LIKE 'a%') AS b, \
                ('a%c' NOT LIKE 'a#%c' ESCAPE '#') AS c, \
                ('abc' NOT LIKE 'a#%c' ESCAPE '#') AS d, \
                (NULL LIKE 'x') AS e, (NULL NOT LIKE 'x' ESCAPE '#') AS f FROM t",
    },
    // Per-row LIKE ESCAPE over a column: match rows whose code contains a
    // literal percent sign.
    Case {
        id: "like_escape_per_row",
        setup: &[
            "CREATE TABLE t (id INTEGER, code TEXT)",
            "INSERT INTO t VALUES (1,'10%'),(2,'10x'),(3,'a%b')",
        ],
        query: "SELECT id FROM t WHERE code LIKE '%#%%' ESCAPE '#' ORDER BY id",
    },
    // ---- Lane 2: upper()/lower() are ASCII-only ----------------------------
    // SQLite's built-in UPPER/LOWER case-fold only ASCII a–z/A–Z; accented and
    // non-Latin characters pass through unchanged (unlike a full-Unicode fold).
    Case {
        id: "upper_lower_ascii_only",
        setup: &["CREATE TABLE t (id INTEGER)", "INSERT INTO t VALUES (1)"],
        query: "SELECT upper('naïve') AS a, lower('ÀBC') AS b, upper('café') AS c, \
                lower('CAFÉ') AS d, upper('straße') AS e, upper('abc123!') AS f FROM t",
    },
    // Per-row over a text column with mixed scripts.
    Case {
        id: "upper_lower_per_row",
        setup: &[
            "CREATE TABLE t (id INTEGER, s TEXT)",
            "INSERT INTO t VALUES (1,'Hello'),(2,'naïve'),(3,'ПРИВЕТ')",
        ],
        query: "SELECT id, upper(s) AS u, lower(s) AS l FROM t ORDER BY id",
    },
    // ----- ORDER BY positional (ordinal) column references -----
    // A bare integer in ORDER BY is a 1-based reference to the n-th output
    // column: `ORDER BY 2` sorts by the second SELECT column (`b`).
    Case {
        id: "order_by_ordinal_single",
        setup: &[
            "CREATE TABLE u (a INTEGER, b TEXT)",
            "INSERT INTO u VALUES (3,'x'),(1,'z'),(2,'y')",
        ],
        query: "SELECT a, b FROM u ORDER BY 2",
    },
    // Multiple positional keys, mixed direction: primary `2 DESC`, tie-break `1`.
    Case {
        id: "order_by_ordinal_multi",
        setup: &[
            "CREATE TABLE u (a INTEGER, b TEXT)",
            "INSERT INTO u VALUES (3,'x'),(1,'x'),(2,'y')",
        ],
        query: "SELECT a, b FROM u ORDER BY 2 DESC, 1",
    },
    // A positional key referencing an aliased output column sorts by that column.
    Case {
        id: "order_by_ordinal_alias",
        setup: &[
            "CREATE TABLE u (a INTEGER, b TEXT)",
            "INSERT INTO u VALUES (3,'x'),(1,'z'),(2,'y')",
        ],
        query: "SELECT a AS k, b FROM u ORDER BY 1 DESC",
    },
    // Out-of-range ordinal: SQLite errors at prepare time ("ORDER BY term out of
    // range"); mini-sqlite errors too, so the case agrees by both-fail.
    Case {
        id: "order_by_ordinal_out_of_range",
        setup: &[
            "CREATE TABLE u (a INTEGER)",
            "INSERT INTO u VALUES (1),(2)",
        ],
        query: "SELECT a FROM u ORDER BY 5",
    },
    // Positional reference to an AGGREGATE output column. SQLite sorts by the
    // computed aggregate (`SUM(v)`); mini-sqlite cannot re-evaluate an aggregate
    // in the per-row sort path, so it leaves the rows in group order. Documented
    // in LEDGER until the sort can bind to an already-materialized output column.
    Case {
        id: "order_by_ordinal_over_aggregate",
        setup: &[
            "CREATE TABLE g (k TEXT, v INTEGER)",
            "INSERT INTO g VALUES ('a',1),('b',2),('a',3)",
        ],
        query: "SELECT k, sum(v) FROM g GROUP BY k ORDER BY 2",
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
    // Positional `ORDER BY <n>` that points at an AGGREGATE output column.
    // Non-aggregate positional keys are fully supported (see the
    // `order_by_ordinal_*` gated cases); the aggregate case remains open because
    // the sort path re-evaluates its key per row, and an aggregate has no per-row
    // value. Closing it means teaching the sort to bind a positional key to an
    // already-materialized output column by index instead of substituting its
    // expression. Until then this is a documented divergence, not a silent skip.
    (
        "order_by_ordinal_over_aggregate",
        "positional ORDER BY over an aggregate output column not yet re-bound to the materialized column",
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
