"""Cursor API — execute, fetch, iteration, description, rowcount."""

import pytest

import mini_sqlite


def _seeded():
    conn = mini_sqlite.connect(":memory:")
    conn.execute("CREATE TABLE employees (id INTEGER PRIMARY KEY, name TEXT, dept TEXT)")
    conn.executemany(
        "INSERT INTO employees (id, name, dept) VALUES (?, ?, ?)",
        [(1, "Alice", "eng"), (2, "Bob", "eng"), (3, "Carol", "sales")],
    )
    conn.commit()
    return conn


def test_description_after_select():
    conn = _seeded()
    cur = conn.execute("SELECT id, name FROM employees")
    assert cur.description == (
        ("id", None, None, None, None, None, None),
        ("name", None, None, None, None, None, None),
    )


def test_description_none_for_dml():
    conn = _seeded()
    cur = conn.execute("INSERT INTO employees (id, name, dept) VALUES (4, 'D', 'eng')")
    assert cur.description is None


def test_rowcount_after_select():
    conn = _seeded()
    cur = conn.execute("SELECT * FROM employees")
    assert cur.rowcount == 3


def test_rowcount_after_insert():
    conn = _seeded()
    cur = conn.execute("INSERT INTO employees (id, name, dept) VALUES (4, 'D', 'eng')")
    assert cur.rowcount == 1


def test_rowcount_after_update():
    conn = _seeded()
    cur = conn.execute("UPDATE employees SET dept = 'eng' WHERE dept = 'sales'")
    assert cur.rowcount == 1


def test_fetchone():
    conn = _seeded()
    cur = conn.execute("SELECT id FROM employees ORDER BY id")
    assert cur.fetchone() == (1,)
    assert cur.fetchone() == (2,)


def test_fetchone_returns_none_when_exhausted():
    conn = _seeded()
    cur = conn.execute("SELECT id FROM employees WHERE id = 999")
    assert cur.fetchone() is None


def test_fetchmany_default_arraysize():
    conn = _seeded()
    cur = conn.execute("SELECT id FROM employees ORDER BY id")
    assert cur.fetchmany() == [(1,)]  # default arraysize = 1


def test_fetchmany_with_size():
    conn = _seeded()
    cur = conn.execute("SELECT id FROM employees ORDER BY id")
    assert cur.fetchmany(2) == [(1,), (2,)]
    assert cur.fetchmany(99) == [(3,)]  # remaining


def test_fetchall():
    conn = _seeded()
    cur = conn.execute("SELECT id FROM employees ORDER BY id")
    assert cur.fetchall() == [(1,), (2,), (3,)]


def test_iteration_protocol():
    conn = _seeded()
    cur = conn.execute("SELECT id FROM employees ORDER BY id")
    ids = [row[0] for row in cur]
    assert ids == [1, 2, 3]


def test_cursor_close_blocks_further_use():
    conn = _seeded()
    cur = conn.cursor()
    cur.close()
    with pytest.raises(mini_sqlite.ProgrammingError):
        cur.execute("SELECT 1")


def test_closed_connection_blocks_cursor_use():
    conn = _seeded()
    cur = conn.cursor()
    conn.close()
    with pytest.raises(mini_sqlite.ProgrammingError):
        cur.execute("SELECT 1")


def test_boolean_output_coercion():
    conn = mini_sqlite.connect(":memory:")
    conn.execute("CREATE TABLE t (x INTEGER)")
    conn.execute("INSERT INTO t VALUES (?)", (True,))
    conn.execute("INSERT INTO t VALUES (?)", (False,))
    conn.commit()
    rows = conn.execute("SELECT x FROM t ORDER BY x").fetchall()
    # True → 1, False → 0.
    assert rows == [(0,), (1,)]


def test_setinputsizes_and_setoutputsize_are_noops():
    conn = _seeded()
    cur = conn.cursor()
    cur.setinputsizes([1, 2, 3])
    cur.setoutputsize(10, column=0)


def test_executemany_sets_total_rowcount():
    conn = mini_sqlite.connect(":memory:")
    conn.execute("CREATE TABLE t (x INTEGER)")
    cur = conn.executemany("INSERT INTO t VALUES (?)", [(1,), (2,)])
    assert cur.rowcount == 2


def test_parameterised_select_with_qmark():
    conn = _seeded()
    cur = conn.execute("SELECT name FROM employees WHERE dept = ?", ("eng",))
    names = sorted(row[0] for row in cur)
    assert names == ["Alice", "Bob"]


def test_parameterised_select_with_named_params():
    """End-to-end: ``:name`` placeholders bound from a dict via execute."""
    conn = _seeded()
    cur = conn.execute(
        "SELECT name FROM employees WHERE dept = :d", {"d": "eng"},
    )
    names = sorted(row[0] for row in cur)
    assert names == ["Alice", "Bob"]


def test_parameterised_insert_with_named_params():
    conn = _seeded()
    conn.execute(
        "INSERT INTO employees (id, name, dept) VALUES (:id, :name, :dept)",
        {"id": 99, "name": "Dan", "dept": "ops"},
    )
    cur = conn.execute("SELECT name FROM employees WHERE id = ?", (99,))
    assert cur.fetchone() == ("Dan",)


def test_named_params_missing_key_raises_programming_error():
    conn = _seeded()
    with pytest.raises(mini_sqlite.ProgrammingError, match=":dept"):
        conn.execute(
            "SELECT * FROM employees WHERE dept = :dept", {"other": "eng"},
        )


def test_named_param_reused_in_same_statement():
    conn = _seeded()
    cur = conn.execute(
        "SELECT name FROM employees WHERE dept = :d OR name = :d",
        {"d": "eng"},
    )
    names = sorted(row[0] for row in cur)
    assert names == ["Alice", "Bob"]
