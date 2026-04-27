"""End-to-end round trips through the full pipeline."""

import mini_sqlite


def test_full_round_trip_select():
    conn = mini_sqlite.connect(":memory:")
    conn.execute("""
        CREATE TABLE employees (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            dept TEXT,
            salary INTEGER
        )
    """)
    conn.executemany(
        "INSERT INTO employees (id, name, dept, salary) VALUES (?, ?, ?, ?)",
        [
            (1, "Alice", "eng", 90000),
            (2, "Bob", "eng", 75000),
            (3, "Carol", "sales", 65000),
            (4, "Dave", "sales", 55000),
        ],
    )
    conn.commit()
    cur = conn.execute(
        "SELECT name, salary FROM employees WHERE dept = ? ORDER BY salary DESC",
        ("eng",),
    )
    assert cur.fetchall() == [("Alice", 90000), ("Bob", 75000)]


def test_aggregate_group_by():
    conn = mini_sqlite.connect(":memory:")
    conn.execute("CREATE TABLE sales (dept TEXT, amount INTEGER)")
    conn.executemany(
        "INSERT INTO sales (dept, amount) VALUES (?, ?)",
        [("eng", 100), ("eng", 200), ("sales", 50), ("sales", 25)],
    )
    conn.commit()
    rows = conn.execute(
        "SELECT dept, SUM(amount) FROM sales GROUP BY dept ORDER BY dept"
    ).fetchall()
    assert rows == [("eng", 300), ("sales", 75)]


def test_update_and_delete():
    conn = mini_sqlite.connect(":memory:")
    conn.execute("CREATE TABLE t (id INTEGER, v INTEGER)")
    conn.executemany("INSERT INTO t VALUES (?, ?)", [(1, 10), (2, 20), (3, 30)])
    conn.commit()
    conn.execute("UPDATE t SET v = ? WHERE id = ?", (99, 2))
    conn.execute("DELETE FROM t WHERE id = ?", (1,))
    conn.commit()
    rows = conn.execute("SELECT * FROM t ORDER BY id").fetchall()
    assert rows == [(2, 99), (3, 30)]


def test_insert_without_column_list_defaults_to_all():
    conn = mini_sqlite.connect(":memory:")
    conn.execute("CREATE TABLE t (a INTEGER, b TEXT)")
    conn.execute("INSERT INTO t VALUES (1, 'x')")
    conn.commit()
    rows = conn.execute("SELECT * FROM t").fetchall()
    assert rows == [(1, "x")]


def test_distinct():
    conn = mini_sqlite.connect(":memory:")
    conn.execute("CREATE TABLE t (x INTEGER)")
    conn.executemany("INSERT INTO t VALUES (?)", [(1,), (1,), (2,), (2,), (3,)])
    conn.commit()
    rows = conn.execute("SELECT DISTINCT x FROM t ORDER BY x").fetchall()
    assert rows == [(1,), (2,), (3,)]


def test_limit_offset():
    conn = mini_sqlite.connect(":memory:")
    conn.execute("CREATE TABLE t (x INTEGER)")
    conn.executemany("INSERT INTO t VALUES (?)", [(i,) for i in range(10)])
    conn.commit()
    rows = conn.execute("SELECT x FROM t ORDER BY x LIMIT 3 OFFSET 2").fetchall()
    assert rows == [(2,), (3,), (4,)]


def test_drop_table():
    conn = mini_sqlite.connect(":memory:")
    conn.execute("CREATE TABLE t (x INTEGER)")
    conn.execute("DROP TABLE t")
    # Second drop without IF EXISTS should fail.
    import pytest

    with pytest.raises(mini_sqlite.OperationalError):
        conn.execute("DROP TABLE t")
    # With IF EXISTS it's fine.
    conn.execute("DROP TABLE IF EXISTS t")


def test_join_inner():
    conn = mini_sqlite.connect(":memory:")
    conn.execute("CREATE TABLE a (id INTEGER, v TEXT)")
    conn.execute("CREATE TABLE b (id INTEGER, w TEXT)")
    conn.executemany("INSERT INTO a VALUES (?, ?)", [(1, "x"), (2, "y")])
    conn.executemany("INSERT INTO b VALUES (?, ?)", [(1, "p"), (2, "q")])
    conn.commit()
    rows = conn.execute(
        "SELECT a.id, a.v, b.w FROM a INNER JOIN b ON a.id = b.id ORDER BY a.id"
    ).fetchall()
    assert rows == [(1, "x", "p"), (2, "y", "q")]


def test_parameter_types_roundtrip():
    conn = mini_sqlite.connect(":memory:")
    conn.execute("CREATE TABLE t (i INTEGER, s TEXT, f INTEGER)")
    conn.execute("INSERT INTO t VALUES (?, ?, ?)", (42, "hello", None))
    conn.commit()
    rows = conn.execute("SELECT * FROM t").fetchall()
    assert rows == [(42, "hello", None)]


def test_string_with_quote_roundtrip():
    conn = mini_sqlite.connect(":memory:")
    conn.execute("CREATE TABLE t (s TEXT)")
    conn.execute("INSERT INTO t VALUES (?)", ("it's a \"test\"",))
    conn.commit()
    row = conn.execute("SELECT s FROM t").fetchone()
    assert row == ("it's a \"test\"",)
