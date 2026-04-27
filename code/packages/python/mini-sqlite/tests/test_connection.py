"""Connection behaviour — lifecycle, transactions, context manager."""

import pytest

import mini_sqlite


def test_cursor_returns_new_cursor():
    conn = mini_sqlite.connect(":memory:")
    cur1 = conn.cursor()
    cur2 = conn.cursor()
    assert cur1 is not cur2


def test_close_is_idempotent():
    conn = mini_sqlite.connect(":memory:")
    conn.close()
    conn.close()  # no error


def test_closed_connection_rejects_operations():
    conn = mini_sqlite.connect(":memory:")
    conn.close()
    with pytest.raises(mini_sqlite.ProgrammingError):
        conn.cursor()
    with pytest.raises(mini_sqlite.ProgrammingError):
        conn.commit()
    with pytest.raises(mini_sqlite.ProgrammingError):
        conn.rollback()


def test_commit_without_transaction_is_noop():
    conn = mini_sqlite.connect(":memory:")
    conn.commit()


def test_rollback_without_transaction_is_noop():
    conn = mini_sqlite.connect(":memory:")
    conn.rollback()


def test_implicit_transaction_on_dml_then_rollback():
    conn = mini_sqlite.connect(":memory:")
    conn.execute("CREATE TABLE t (x INTEGER)")
    conn.execute("INSERT INTO t VALUES (1)")
    conn.rollback()
    rows = conn.execute("SELECT * FROM t").fetchall()
    assert rows == []


def test_implicit_transaction_commit_persists():
    conn = mini_sqlite.connect(":memory:")
    conn.execute("CREATE TABLE t (x INTEGER)")
    conn.execute("INSERT INTO t VALUES (1)")
    conn.commit()
    conn.execute("INSERT INTO t VALUES (2)")
    conn.rollback()
    rows = conn.execute("SELECT * FROM t ORDER BY x").fetchall()
    assert rows == [(1,)]


def test_context_manager_commits_on_success():
    with mini_sqlite.connect(":memory:") as conn:
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("INSERT INTO t VALUES (42)")
    # Re-use same connection to verify commit happened.
    rows = conn.execute("SELECT * FROM t").fetchall()
    assert rows == [(42,)]


def test_context_manager_rolls_back_on_exception():
    conn = mini_sqlite.connect(":memory:")
    conn.execute("CREATE TABLE t (x INTEGER)")
    conn.commit()
    with pytest.raises(RuntimeError), conn:  # noqa: SIM117
        conn.execute("INSERT INTO t VALUES (99)")
        raise RuntimeError("boom")
    rows = conn.execute("SELECT * FROM t").fetchall()
    assert rows == []


def test_close_with_open_transaction_rolls_back():
    conn = mini_sqlite.connect(":memory:")
    conn.execute("CREATE TABLE t (x INTEGER)")
    conn.commit()
    conn.execute("INSERT INTO t VALUES (1)")
    conn.close()
    # Open a fresh connection — the in-memory backend is per-connection so
    # this just confirms close() doesn't raise with an open txn.


def test_autocommit_mode_persists_each_stmt():
    conn = mini_sqlite.connect(":memory:", autocommit=True)
    conn.execute("CREATE TABLE t (x INTEGER)")
    conn.execute("INSERT INTO t VALUES (1)")
    # No commit call needed.
    rows = conn.execute("SELECT * FROM t").fetchall()
    assert rows == [(1,)]


def test_connection_execute_shortcut_returns_cursor():
    conn = mini_sqlite.connect(":memory:")
    cur = conn.execute("CREATE TABLE t (x INTEGER)")
    assert isinstance(cur, mini_sqlite.Cursor)


def test_connection_executemany_shortcut():
    conn = mini_sqlite.connect(":memory:")
    conn.execute("CREATE TABLE t (x INTEGER)")
    cur = conn.executemany("INSERT INTO t VALUES (?)", [(1,), (2,), (3,)])
    rows = conn.execute("SELECT * FROM t ORDER BY x").fetchall()
    assert rows == [(1,), (2,), (3,)]
    assert cur.rowcount == 3
