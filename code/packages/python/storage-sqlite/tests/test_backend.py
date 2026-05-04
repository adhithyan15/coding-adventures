"""
Tests for SqliteFileBackend (phase 7).

Structure
---------
The test file is split into three sections:

1. **Conformance tests** — the four tiers from ``sql_backend.conformance``.
   These are the same assertions that InMemoryBackend passes; a failure here
   means our file backend deviates from the contract.

2. **File-persistence tests** — round-trips that open the database in a
   second backend instance and verify that committed data is still there.

3. **Unit tests** for the SQL helper functions and the row encode/decode
   helpers that cannot be reached through the Backend interface.

Factory pattern
---------------
Each conformance test calls a *factory*: a zero-arg callable that returns a
fresh, pre-populated backend with the ``users`` table and its five rows.

For the file backend we use ``tmp_path`` (pytest's built-in temporary
directory fixture) to get a throwaway file.  Because the conformance suite
calls the factory multiple times (once per sub-test) we build a fresh file
for each call rather than sharing state across calls.

Transaction note
----------------
The conformance fixtures (rows 1–5) are committed to disk before the
factory returns.  This is necessary for the rollback tests (22) to work:
``rollback()`` reverts dirty pages to the *last committed* state, so the
five rows must already be committed when the test starts.
"""

from __future__ import annotations

import os
from pathlib import Path

import pytest
from sql_backend import ColumnDef, ConstraintViolation, TableAlreadyExists, TableNotFound
from sql_backend.conformance import (
    USERS_COLUMNS,
    USERS_ROWS,
    run_ddl,
    run_read_write,
    run_required,
    run_transaction,
)

from storage_sqlite import record
from storage_sqlite.backend import (
    SqliteFileBackend,
    _apply_defaults,
    _check_not_null,
    _choose_rowid,
    _columns_to_sql,
    _decode_row,
    _encode_row,
    _find_max_rowid,
    _format_literal,
    _is_ipk,
    _parse_literal,
    _parse_one_column,
    _sql_to_columns,
    _sql_to_trigger_def,
    _tokenize,
    _trigger_to_sql,
)
from storage_sqlite.btree import BTree
from storage_sqlite.pager import Pager
from storage_sqlite.schema import initialize_new_database

# ---------------------------------------------------------------------------
# Factory helpers
# ---------------------------------------------------------------------------


def _make_file_backend(path: str) -> SqliteFileBackend:
    """Create a SqliteFileBackend at *path* pre-populated with the users table
    and committed to disk.

    The commit is essential for rollback tests: ``pager.rollback()`` reverts
    to the last *committed* checkpoint, so the five user rows must be durably
    persisted before any test that calls ``rollback()`` starts.
    """
    b = SqliteFileBackend(path)
    b.create_table("users", USERS_COLUMNS, if_not_exists=False)
    for row in USERS_ROWS:
        b.insert("users", row)
    h = b.begin_transaction()
    b.commit(h)
    return b


def _factory_for(tmp_path: Path):
    """Return a factory callable bound to *tmp_path*.

    Each call to the factory creates a brand-new ``.db`` file so tests
    never share state.
    """
    counter = {"n": 0}

    def factory() -> SqliteFileBackend:
        counter["n"] += 1
        path = str(tmp_path / f"test_{counter['n']}.db")
        return _make_file_backend(path)

    return factory


# ---------------------------------------------------------------------------
# Conformance: tier 1 (required)
# ---------------------------------------------------------------------------


def test_conformance_required(tmp_path: Path) -> None:
    """Every backend must satisfy schema introspection and read-only scan."""
    run_required(_factory_for(tmp_path))


# ---------------------------------------------------------------------------
# Conformance: tier 2 (read-write)
# ---------------------------------------------------------------------------


def test_conformance_read_write(tmp_path: Path) -> None:
    """Inserts, updates, deletes, NOT NULL, duplicate PK, defaults."""
    run_read_write(_factory_for(tmp_path))


# ---------------------------------------------------------------------------
# Conformance: tier 3 (DDL)
# ---------------------------------------------------------------------------


def test_conformance_ddl(tmp_path: Path) -> None:
    """CREATE TABLE, DROP TABLE, IF [NOT] EXISTS variants."""
    run_ddl(_factory_for(tmp_path))


# ---------------------------------------------------------------------------
# Conformance: tier 4 (transactions)
# ---------------------------------------------------------------------------


def test_conformance_transaction(tmp_path: Path) -> None:
    """Commit persists; rollback restores."""
    run_transaction(_factory_for(tmp_path))


# ---------------------------------------------------------------------------
# File persistence — round-trip via a second backend instance
# ---------------------------------------------------------------------------


def test_data_survives_close_and_reopen(tmp_path: Path) -> None:
    """Rows committed in one backend instance are visible in a new instance
    opened on the same file.

    This is the key property that distinguishes SqliteFileBackend from
    InMemoryBackend: data actually reaches the disk.
    """
    path = str(tmp_path / "persist.db")
    b = _make_file_backend(path)
    b.close()  # explicit close (commits nothing extra; just closes pager)

    # Open a fresh instance on the same file.
    b2 = SqliteFileBackend(path)
    assert "users" in b2.tables()
    it = b2.scan("users")
    rows = []
    while (row := it.next()) is not None:
        rows.append(row)
    it.close()
    b2.close()

    assert [r["id"] for r in rows] == [1, 2, 3, 4, 5]
    assert rows[0]["name"] == "alice"
    assert rows[1]["name"] == "bob"


def test_new_file_created_automatically(tmp_path: Path) -> None:
    """Opening a non-existent path creates a valid SQLite database file."""
    path = str(tmp_path / "brand_new.db")
    assert not os.path.exists(path)

    b = SqliteFileBackend(path)
    b.close()

    assert os.path.exists(path)
    # The file must be readable by a second instance.
    b2 = SqliteFileBackend(path)
    assert b2.tables() == []
    b2.close()


def test_existing_file_opened(tmp_path: Path) -> None:
    """An existing file is opened rather than overwritten."""
    path = str(tmp_path / "existing.db")
    b = _make_file_backend(path)
    b.close()

    b2 = SqliteFileBackend(path)
    cols = b2.columns("users")
    assert [c.name for c in cols] == ["id", "name", "age", "email"]
    b2.close()


def test_schema_cookie_persists(tmp_path: Path) -> None:
    """The schema cookie is bumped and persisted with each CREATE TABLE."""
    path = str(tmp_path / "cookie.db")
    b = SqliteFileBackend(path)
    cookie_0 = b._schema.get_schema_cookie()

    id_col = [ColumnDef(name="id", type_name="INTEGER", primary_key=True)]
    b.create_table("t1", id_col, if_not_exists=False)
    h = b.begin_transaction()
    b.commit(h)
    cookie_1 = b._schema.get_schema_cookie()

    b.create_table("t2", id_col, if_not_exists=False)
    h = b.begin_transaction()
    b.commit(h)
    cookie_2 = b._schema.get_schema_cookie()
    b.close()

    assert cookie_1 == cookie_0 + 1
    assert cookie_2 == cookie_0 + 2


def test_multiple_tables_persist(tmp_path: Path) -> None:
    """Multiple tables survive a close-and-reopen cycle."""
    path = str(tmp_path / "multi.db")
    b = SqliteFileBackend(path)
    id_col = [ColumnDef(name="id", type_name="INTEGER", primary_key=True)]
    b.create_table("a", id_col, if_not_exists=False)
    b.create_table("b", [ColumnDef(name="x", type_name="TEXT")], if_not_exists=False)
    b.insert("a", {"id": 1})
    b.insert("b", {"x": "hello"})
    h = b.begin_transaction()
    b.commit(h)
    b.close()

    b2 = SqliteFileBackend(path)
    assert sorted(b2.tables()) == ["a", "b"]
    b2.close()


def test_drop_table_persists(tmp_path: Path) -> None:
    """Dropping a table and committing makes it absent in the next session."""
    path = str(tmp_path / "drop.db")
    b = _make_file_backend(path)
    b.drop_table("users", if_exists=False)
    h = b.begin_transaction()
    b.commit(h)
    b.close()

    b2 = SqliteFileBackend(path)
    assert "users" not in b2.tables()
    b2.close()


# ---------------------------------------------------------------------------
# Transaction semantics
# ---------------------------------------------------------------------------


def test_rollback_reverts_insert(tmp_path: Path) -> None:
    """An un-committed insert disappears after rollback."""
    path = str(tmp_path / "rollback_insert.db")
    b = _make_file_backend(path)

    h = b.begin_transaction()
    b.insert("users", {"id": 99, "name": "ghost", "age": 0, "email": None})
    b.rollback(h)

    ids = []
    it = b.scan("users")
    while (row := it.next()) is not None:
        ids.append(row["id"])
    it.close()
    b.close()

    assert 99 not in ids


def test_commit_then_rollback_still_sees_committed(tmp_path: Path) -> None:
    """A row committed in txn-1 remains visible after txn-2 is rolled back."""
    path = str(tmp_path / "commit_rollback.db")
    b = _make_file_backend(path)

    # Commit id=10.
    h = b.begin_transaction()
    b.insert("users", {"id": 10, "name": "ten", "age": 10, "email": None})
    b.commit(h)

    # Roll back id=11.
    h = b.begin_transaction()
    b.insert("users", {"id": 11, "name": "eleven", "age": 11, "email": None})
    b.rollback(h)

    ids = []
    it = b.scan("users")
    while (row := it.next()) is not None:
        ids.append(row["id"])
    it.close()
    b.close()

    assert 10 in ids
    assert 11 not in ids


def test_context_manager_rollback_on_exception(tmp_path: Path) -> None:
    """Using the backend as a context manager rolls back on unhandled exit."""
    path = str(tmp_path / "ctx.db")
    # Create and commit initial state outside the context manager.
    b_init = SqliteFileBackend(path)
    id_col = [ColumnDef(name="id", type_name="INTEGER", primary_key=True)]
    b_init.create_table("t", id_col, if_not_exists=False)
    h = b_init.begin_transaction()
    b_init.commit(h)
    b_init.close()

    with SqliteFileBackend(path) as b:
        b.insert("t", {"id": 1})
        # Exit without commit — close() rolls back.

    b2 = SqliteFileBackend(path)
    it = b2.scan("t")
    assert it.next() is None  # nothing was committed
    it.close()
    b2.close()


# ---------------------------------------------------------------------------
# Constraint enforcement
# ---------------------------------------------------------------------------


def test_not_null_on_insert(tmp_path: Path) -> None:
    """NOT NULL constraint is enforced on insert."""
    path = str(tmp_path / "nn.db")
    b = _make_file_backend(path)
    with pytest.raises(ConstraintViolation):
        b.insert("users", {"id": 100, "name": None, "age": 1, "email": None})
    b.close()


def test_duplicate_pk_on_insert(tmp_path: Path) -> None:
    """Duplicate INTEGER PRIMARY KEY raises ConstraintViolation."""
    path = str(tmp_path / "dupk.db")
    b = _make_file_backend(path)
    with pytest.raises(ConstraintViolation):
        b.insert("users", {"id": 1, "name": "dup", "age": 0, "email": None})
    b.close()


def test_unique_constraint_on_insert(tmp_path: Path) -> None:
    """UNIQUE constraint is enforced on insert for non-NULL values."""
    path = str(tmp_path / "uniq.db")
    b = _make_file_backend(path)
    with pytest.raises(ConstraintViolation):
        b.insert("users", {"id": 99, "name": "x", "age": 0, "email": "alice@example.com"})
    b.close()


def test_unique_null_is_not_a_conflict(tmp_path: Path) -> None:
    """NULL values in UNIQUE columns never conflict with each other (SQL semantics)."""
    path = str(tmp_path / "null_uniq.db")
    b = _make_file_backend(path)
    # dave (id=4) already has email=None; another NULL should be fine.
    b.insert("users", {"id": 99, "name": "no-email", "age": 0, "email": None})
    # Both rows should be present.
    it = b.scan("users")
    nulls = 0
    while (row := it.next()) is not None:
        if row["email"] is None:
            nulls += 1
    it.close()
    b.close()
    assert nulls == 2


def test_not_null_on_update(tmp_path: Path) -> None:
    """NOT NULL constraint is enforced on update."""
    path = str(tmp_path / "nn_update.db")
    b = _make_file_backend(path)
    cursor = b._open_cursor("users")
    cursor.next()  # alice, id=1
    with pytest.raises(ConstraintViolation):
        b.update("users", cursor, {"name": None})
    cursor.close()
    b.close()


# ---------------------------------------------------------------------------
# DDL edge cases
# ---------------------------------------------------------------------------


def test_create_table_if_not_exists(tmp_path: Path) -> None:
    """create_table with if_not_exists=True is idempotent."""
    path = str(tmp_path / "ine.db")
    b = SqliteFileBackend(path)
    cols = [ColumnDef(name="id", type_name="INTEGER", primary_key=True)]
    b.create_table("t", cols, if_not_exists=False)
    b.create_table("t", cols, if_not_exists=True)  # no-op, no exception
    assert b.tables() == ["t"]
    b.close()


def test_create_table_already_exists_raises(tmp_path: Path) -> None:
    """create_table with if_not_exists=False raises TableAlreadyExists."""
    path = str(tmp_path / "ae.db")
    b = SqliteFileBackend(path)
    cols = [ColumnDef(name="id", type_name="INTEGER", primary_key=True)]
    b.create_table("t", cols, if_not_exists=False)
    with pytest.raises(TableAlreadyExists):
        b.create_table("t", cols, if_not_exists=False)
    b.close()


def test_drop_table_if_exists(tmp_path: Path) -> None:
    """drop_table with if_exists=True on a missing table is a no-op."""
    path = str(tmp_path / "drop_ie.db")
    b = SqliteFileBackend(path)
    b.drop_table("ghost", if_exists=True)  # no exception
    b.close()


def test_drop_table_not_found_raises(tmp_path: Path) -> None:
    """drop_table with if_exists=False on a missing table raises TableNotFound."""
    path = str(tmp_path / "drop_nf.db")
    b = SqliteFileBackend(path)
    with pytest.raises(TableNotFound):
        b.drop_table("ghost", if_exists=False)
    b.close()


def test_scan_missing_table_raises(tmp_path: Path) -> None:
    """scan() on an unknown table raises TableNotFound."""
    path = str(tmp_path / "scan_nf.db")
    b = SqliteFileBackend(path)
    with pytest.raises(TableNotFound):
        b.scan("missing")
    b.close()


def test_columns_missing_table_raises(tmp_path: Path) -> None:
    """columns() on an unknown table raises TableNotFound."""
    path = str(tmp_path / "cols_nf.db")
    b = SqliteFileBackend(path)
    with pytest.raises(TableNotFound):
        b.columns("missing")
    b.close()


# ---------------------------------------------------------------------------
# Large table — exercises splits and overflow
# ---------------------------------------------------------------------------


def test_large_table_insert_and_scan(tmp_path: Path) -> None:
    """Inserting many rows forces B-tree splits; scan still returns all rows."""
    path = str(tmp_path / "large.db")
    b = SqliteFileBackend(path)
    b.create_table(
        "items",
        [
            ColumnDef(name="id", type_name="INTEGER", primary_key=True),
            ColumnDef(name="label", type_name="TEXT"),
        ],
        if_not_exists=False,
    )
    n = 500
    for i in range(1, n + 1):
        b.insert("items", {"id": i, "label": f"item-{i:05d}"})

    h = b.begin_transaction()
    b.commit(h)
    b.close()

    # Re-open and verify all rows are there in order.
    b2 = SqliteFileBackend(path)
    it = b2.scan("items")
    seen = []
    while (row := it.next()) is not None:
        seen.append(row["id"])
    it.close()
    b2.close()

    assert seen == list(range(1, n + 1))


def test_overflow_row_round_trips(tmp_path: Path) -> None:
    """A row with a large text value (>4 KB) is stored across overflow pages
    and decoded correctly on re-read.
    """
    path = str(tmp_path / "overflow.db")
    b = SqliteFileBackend(path)
    b.create_table(
        "blobs",
        [
            ColumnDef(name="id", type_name="INTEGER", primary_key=True),
            ColumnDef(name="data", type_name="TEXT"),
        ],
        if_not_exists=False,
    )
    big = "x" * 6000  # well above the 4 061-byte max_local threshold
    b.insert("blobs", {"id": 1, "data": big})
    h = b.begin_transaction()
    b.commit(h)
    b.close()

    b2 = SqliteFileBackend(path)
    it = b2.scan("blobs")
    row = it.next()
    it.close()
    b2.close()

    assert row is not None
    assert row["id"] == 1
    assert row["data"] == big


# ---------------------------------------------------------------------------
# Helper unit tests: _format_literal
# ---------------------------------------------------------------------------


def test_format_literal_null() -> None:
    assert _format_literal(None) == "NULL"


def test_format_literal_int() -> None:
    assert _format_literal(42) == "42"
    assert _format_literal(-7) == "-7"


def test_format_literal_float() -> None:
    assert _format_literal(3.14) == repr(3.14)


def test_format_literal_bool() -> None:
    # bool is a subclass of int; must emit 1/0 not True/False.
    assert _format_literal(True) == "1"
    assert _format_literal(False) == "0"


def test_format_literal_str() -> None:
    assert _format_literal("hello") == "'hello'"
    assert _format_literal("it's") == "'it''s'"


def test_format_literal_bytes() -> None:
    assert _format_literal(b"\xde\xad") == "X'dead'"


# ---------------------------------------------------------------------------
# Helper unit tests: _columns_to_sql
# ---------------------------------------------------------------------------


def test_columns_to_sql_simple() -> None:
    cols = [
        ColumnDef(name="id", type_name="INTEGER", primary_key=True),
        ColumnDef(name="name", type_name="TEXT", not_null=True),
    ]
    sql = _columns_to_sql("users", cols)
    assert sql == "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)"


def test_columns_to_sql_unique() -> None:
    cols = [
        ColumnDef(name="id", type_name="INTEGER", primary_key=True),
        ColumnDef(name="email", type_name="TEXT", unique=True),
    ]
    sql = _columns_to_sql("t", cols)
    assert "UNIQUE" in sql
    assert "email TEXT UNIQUE" in sql


def test_columns_to_sql_default() -> None:
    cols = [
        ColumnDef(name="id", type_name="INTEGER", primary_key=True),
        ColumnDef(name="score", type_name="INTEGER", default=0),
    ]
    sql = _columns_to_sql("t", cols)
    assert "DEFAULT 0" in sql


def test_columns_to_sql_primary_key_no_not_null() -> None:
    """PRIMARY KEY implies NOT NULL — we should not emit both."""
    cols = [ColumnDef(name="id", type_name="INTEGER", primary_key=True, not_null=True)]
    sql = _columns_to_sql("t", cols)
    assert "NOT NULL" not in sql


# ---------------------------------------------------------------------------
# Helper unit tests: _tokenize and _parse_literal
# ---------------------------------------------------------------------------


def test_tokenize_basic() -> None:
    tokens = _tokenize("id INTEGER PRIMARY KEY")
    assert tokens == ["id", "INTEGER", "PRIMARY", "KEY"]


def test_tokenize_strips_comments() -> None:
    tokens = _tokenize("id INTEGER -- this is a comment\nPRIMARY KEY")
    assert "this" not in tokens
    assert "PRIMARY" in tokens


def test_parse_literal_null() -> None:
    assert _parse_literal("NULL") is None
    assert _parse_literal("null") is None


def test_parse_literal_int() -> None:
    assert _parse_literal("42") == 42
    assert _parse_literal("-7") == -7


def test_parse_literal_float() -> None:
    assert isinstance(_parse_literal("3.14"), float)
    assert abs(_parse_literal("3.14") - 3.14) < 1e-9  # type: ignore[operator]


def test_parse_literal_string() -> None:
    assert _parse_literal("'hello'") == "hello"
    assert _parse_literal("'it''s'") == "it's"


def test_parse_literal_bytes() -> None:
    assert _parse_literal("X'deadbeef'") == bytes.fromhex("deadbeef")


# ---------------------------------------------------------------------------
# Helper unit tests: _parse_one_column
# ---------------------------------------------------------------------------


def test_parse_one_column_simple() -> None:
    col = _parse_one_column("id INTEGER PRIMARY KEY")
    assert col is not None
    assert col.name == "id"
    assert col.type_name == "INTEGER"
    assert col.primary_key is True


def test_parse_one_column_not_null() -> None:
    col = _parse_one_column("name TEXT NOT NULL")
    assert col is not None
    assert col.not_null is True


def test_parse_one_column_unique() -> None:
    col = _parse_one_column("email TEXT UNIQUE")
    assert col is not None
    assert col.unique is True


def test_parse_one_column_default() -> None:
    col = _parse_one_column("score INTEGER DEFAULT 7")
    assert col is not None
    assert col.default == 7


def test_parse_one_column_table_constraint_skipped() -> None:
    """Table-level PRIMARY KEY(…) constraints return None."""
    assert _parse_one_column("PRIMARY KEY(id)") is None
    assert _parse_one_column("UNIQUE(email)") is None


def test_parse_one_column_too_short() -> None:
    assert _parse_one_column("id") is None
    assert _parse_one_column("") is None


# ---------------------------------------------------------------------------
# Helper unit tests: _sql_to_columns
# ---------------------------------------------------------------------------


def test_sql_to_columns_roundtrip() -> None:
    """_sql_to_columns(_columns_to_sql(t, cols)) == cols."""
    original = [
        ColumnDef(name="id", type_name="INTEGER", primary_key=True),
        ColumnDef(name="name", type_name="TEXT", not_null=True),
        ColumnDef(name="score", type_name="INTEGER", default=0),
        ColumnDef(name="email", type_name="TEXT", unique=True),
    ]
    sql = _columns_to_sql("t", original)
    parsed = _sql_to_columns(sql)
    assert [c.name for c in parsed] == [c.name for c in original]
    assert [c.type_name for c in parsed] == [c.type_name for c in original]
    assert [c.primary_key for c in parsed] == [c.primary_key for c in original]
    assert [c.not_null for c in parsed] == [c.not_null for c in original]
    assert [c.unique for c in parsed] == [c.unique for c in original]
    assert [c.default for c in parsed] == [c.default for c in original]


def test_sql_to_columns_no_parens() -> None:
    with pytest.raises(ValueError):
        _sql_to_columns("CREATE TABLE t")


# ---------------------------------------------------------------------------
# Helper unit tests: _is_ipk
# ---------------------------------------------------------------------------


def test_is_ipk_true() -> None:
    assert _is_ipk(ColumnDef(name="id", type_name="INTEGER", primary_key=True))
    assert _is_ipk(ColumnDef(name="id", type_name="int", primary_key=True))
    assert _is_ipk(ColumnDef(name="id", type_name="INT", primary_key=True))


def test_is_ipk_false_not_pk() -> None:
    assert not _is_ipk(ColumnDef(name="id", type_name="INTEGER"))


def test_is_ipk_false_text_type() -> None:
    assert not _is_ipk(ColumnDef(name="id", type_name="TEXT", primary_key=True))


# ---------------------------------------------------------------------------
# Helper unit tests: _encode_row / _decode_row
# ---------------------------------------------------------------------------


def test_encode_decode_round_trip() -> None:
    """_decode_row(_encode_row(rowid, row, cols), cols) == row."""
    cols = [
        ColumnDef(name="id", type_name="INTEGER", primary_key=True),
        ColumnDef(name="name", type_name="TEXT"),
        ColumnDef(name="age", type_name="INTEGER"),
    ]
    row = {"id": 42, "name": "alice", "age": 30}
    payload = _encode_row(42, row, cols)
    decoded = _decode_row(42, payload, cols)
    assert decoded == row


def test_encode_ipk_as_null_placeholder() -> None:
    """IPK column is stored as NULL in the payload (matching real sqlite3).

    Real SQLite writes NULL for the INTEGER PRIMARY KEY column in the record
    payload; the actual value lives in the B-tree cell key (the rowid).  We
    replicate that convention so files produced by this backend are readable
    by the stdlib ``sqlite3`` module, and vice versa.
    """
    cols = [
        ColumnDef(name="id", type_name="INTEGER", primary_key=True),
        ColumnDef(name="x", type_name="TEXT"),
    ]
    row = {"id": 5, "x": "hi"}
    payload = _encode_row(5, row, cols)
    # Payload contains two values: NULL for the IPK slot and 'hi' for x.
    values, _ = record.decode(payload)
    assert len(values) == 2
    assert values[0] is None  # IPK slot → NULL
    assert values[1] == "hi"


def test_decode_injects_rowid_for_ipk() -> None:
    """IPK column's value comes from rowid; the NULL slot in payload is consumed."""
    cols = [
        ColumnDef(name="id", type_name="INTEGER", primary_key=True),
        ColumnDef(name="x", type_name="TEXT"),
    ]
    # Payload has NULL for IPK and 'world' for x — same layout as real sqlite3.
    payload = record.encode([None, "world"])
    decoded = _decode_row(99, payload, cols)
    assert decoded["id"] == 99
    assert decoded["x"] == "world"


# ---------------------------------------------------------------------------
# Helper unit tests: _apply_defaults
# ---------------------------------------------------------------------------


def test_apply_defaults_fills_missing() -> None:
    cols = [
        ColumnDef(name="id", type_name="INTEGER", primary_key=True),
        ColumnDef(name="flag", type_name="INTEGER", default=7),
    ]
    row = {"id": 1}
    result = _apply_defaults(row, cols)
    assert result["flag"] == 7


def test_apply_defaults_absent_no_default_is_null() -> None:
    cols = [
        ColumnDef(name="id", type_name="INTEGER", primary_key=True),
        ColumnDef(name="x", type_name="TEXT"),  # no default
    ]
    row = {"id": 1}
    result = _apply_defaults(row, cols)
    assert result["x"] is None


def test_apply_defaults_existing_value_not_overwritten() -> None:
    cols = [
        ColumnDef(name="id", type_name="INTEGER", primary_key=True),
        ColumnDef(name="flag", type_name="INTEGER", default=7),
    ]
    row = {"id": 1, "flag": 99}
    result = _apply_defaults(row, cols)
    assert result["flag"] == 99


# ---------------------------------------------------------------------------
# Helper unit tests: _check_not_null
# ---------------------------------------------------------------------------


def test_check_not_null_raises_on_null() -> None:
    cols = [ColumnDef(name="name", type_name="TEXT", not_null=True)]
    with pytest.raises(ConstraintViolation):
        _check_not_null("t", {"name": None}, cols)


def test_check_not_null_ok_with_value() -> None:
    cols = [ColumnDef(name="name", type_name="TEXT", not_null=True)]
    _check_not_null("t", {"name": "alice"}, cols)  # should not raise


def test_check_not_null_ipk_implied() -> None:
    """INTEGER PRIMARY KEY implies NOT NULL."""
    cols = [ColumnDef(name="id", type_name="INTEGER", primary_key=True)]
    with pytest.raises(ConstraintViolation):
        _check_not_null("t", {"id": None}, cols)


# ---------------------------------------------------------------------------
# Helper unit tests: _find_max_rowid / _choose_rowid
# ---------------------------------------------------------------------------


def test_find_max_rowid_empty(tmp_path: Path) -> None:
    path = str(tmp_path / "empty.db")
    with Pager.create(path) as pager:
        initialize_new_database(pager)
        tree = BTree.create(pager)
        assert _find_max_rowid(tree) == 0


def test_find_max_rowid_with_rows(tmp_path: Path) -> None:
    path = str(tmp_path / "max.db")
    with Pager.create(path) as pager:
        initialize_new_database(pager)
        tree = BTree.create(pager)
        for rid in [3, 1, 7, 2]:
            tree.insert(rid, record.encode([rid]))
        assert _find_max_rowid(tree) == 7


def test_choose_rowid_from_ipk(tmp_path: Path) -> None:
    """When the row supplies an IPK value, it IS the rowid."""
    cols = [ColumnDef(name="id", type_name="INTEGER", primary_key=True)]
    path = str(tmp_path / "ipk.db")
    with Pager.create(path) as pager:
        initialize_new_database(pager)
        tree = BTree.create(pager)
        rowid = _choose_rowid({"id": 42}, cols, tree)
        assert rowid == 42


def test_choose_rowid_auto_assign(tmp_path: Path) -> None:
    """Without an IPK, rowid = max_existing + 1."""
    cols = [ColumnDef(name="x", type_name="TEXT")]
    path = str(tmp_path / "auto.db")
    with Pager.create(path) as pager:
        initialize_new_database(pager)
        tree = BTree.create(pager)
        tree.insert(3, record.encode(["three"]))
        rowid = _choose_rowid({"x": "four"}, cols, tree)
        assert rowid == 4


# ---------------------------------------------------------------------------
# add_column
# ---------------------------------------------------------------------------


def test_add_column_basic(tmp_path: Path) -> None:
    """add_column appends a new column; existing rows read NULL for it."""
    path = str(tmp_path / "ac.db")
    with SqliteFileBackend(path) as b:
        b.create_table("t", [ColumnDef("id", "INTEGER", primary_key=True)], if_not_exists=False)
        b.insert("t", {"id": 1})
        b.add_column("t", ColumnDef("name", "TEXT"))
        cols = [c.name for c in b.columns("t")]
        assert "name" in cols
        cursor = b.scan("t")
        row = cursor.next()
        assert row is not None
        assert row["name"] is None


def test_add_column_with_default(tmp_path: Path) -> None:
    """Existing rows get the default value for a new column on read."""
    path = str(tmp_path / "acd.db")
    with SqliteFileBackend(path) as b:
        b.create_table("t", [ColumnDef("id", "INTEGER", primary_key=True)], if_not_exists=False)
        b.insert("t", {"id": 1})
        b.add_column("t", ColumnDef("score", "INTEGER", default=0))
        cursor = b.scan("t")
        row = cursor.next()
        assert row is not None
        assert row["score"] == 0


def test_add_column_unknown_table_raises(tmp_path: Path) -> None:
    """add_column raises TableNotFound for an unknown table."""
    path = str(tmp_path / "act.db")
    with SqliteFileBackend(path) as b, pytest.raises(TableNotFound):
        b.add_column("ghost", ColumnDef("x", "TEXT"))


def test_add_column_duplicate_raises(tmp_path: Path) -> None:
    """add_column raises ColumnAlreadyExists if the column name already exists."""
    from sql_backend import ColumnAlreadyExists
    path = str(tmp_path / "acd2.db")
    with SqliteFileBackend(path) as b:
        b.create_table("t", [ColumnDef("id", "INTEGER", primary_key=True)], if_not_exists=False)
        with pytest.raises(ColumnAlreadyExists):
            b.add_column("t", ColumnDef("id", "TEXT"))


# ---------------------------------------------------------------------------
# current_transaction
# ---------------------------------------------------------------------------


def test_current_transaction_none_when_inactive(tmp_path: Path) -> None:
    """current_transaction returns None when no transaction is open."""
    path = str(tmp_path / "ct.db")
    with SqliteFileBackend(path) as b:
        assert b.current_transaction() is None


def test_current_transaction_returns_handle_when_active(tmp_path: Path) -> None:
    """current_transaction returns the active handle."""
    path = str(tmp_path / "ct2.db")
    with SqliteFileBackend(path) as b:
        h = b.begin_transaction()
        assert b.current_transaction() == h
        b.rollback(h)
        assert b.current_transaction() is None


# ---------------------------------------------------------------------------
# Savepoints
# ---------------------------------------------------------------------------


def test_savepoint_rollback_undoes_writes(tmp_path: Path) -> None:
    """rollback_to_savepoint restores data written after the savepoint."""
    path = str(tmp_path / "sp.db")
    with SqliteFileBackend(path) as b:
        b.create_table("t", [ColumnDef("id", "INTEGER", primary_key=True)], if_not_exists=False)
        h = b.begin_transaction()
        b.insert("t", {"id": 1})
        b.create_savepoint("s1")
        b.insert("t", {"id": 2})
        b.rollback_to_savepoint("s1")
        # After rollback, only id=1 should remain.
        ids = [r["id"] for r in _scan_all(b, "t")]
        assert ids == [1]
        b.commit(h)


def test_savepoint_release_keeps_data(tmp_path: Path) -> None:
    """release_savepoint keeps current data; only destroys the savepoint entry."""
    path = str(tmp_path / "spr.db")
    with SqliteFileBackend(path) as b:
        b.create_table("t", [ColumnDef("id", "INTEGER", primary_key=True)], if_not_exists=False)
        h = b.begin_transaction()
        b.create_savepoint("s1")
        b.insert("t", {"id": 10})
        b.release_savepoint("s1")
        # Data is still there after release.
        ids = [r["id"] for r in _scan_all(b, "t")]
        assert ids == [10]
        b.commit(h)


def test_savepoint_unknown_name_raises(tmp_path: Path) -> None:
    """rollback_to_savepoint and release_savepoint raise Unsupported for unknown names."""
    from sql_backend import Unsupported
    path = str(tmp_path / "spu.db")
    with SqliteFileBackend(path) as b:
        b.create_table("t", [ColumnDef("id", "INTEGER")], if_not_exists=False)
        h = b.begin_transaction()
        with pytest.raises(Unsupported):
            b.rollback_to_savepoint("no_such")
        with pytest.raises(Unsupported):
            b.release_savepoint("no_such")
        b.rollback(h)


def test_savepoint_nested(tmp_path: Path) -> None:
    """Nested savepoints roll back independently."""
    path = str(tmp_path / "spn.db")
    with SqliteFileBackend(path) as b:
        b.create_table("t", [ColumnDef("id", "INTEGER", primary_key=True)], if_not_exists=False)
        h = b.begin_transaction()
        b.insert("t", {"id": 1})
        b.create_savepoint("outer")
        b.insert("t", {"id": 2})
        b.create_savepoint("inner")
        b.insert("t", {"id": 3})
        b.rollback_to_savepoint("inner")  # undo id=3
        ids = [r["id"] for r in _scan_all(b, "t")]
        assert ids == [1, 2]
        b.rollback_to_savepoint("outer")  # undo id=2
        ids = [r["id"] for r in _scan_all(b, "t")]
        assert ids == [1]
        b.commit(h)


# ---------------------------------------------------------------------------
# Triggers
# ---------------------------------------------------------------------------


def test_create_and_list_trigger(tmp_path: Path) -> None:
    """create_trigger stores a trigger; list_triggers returns it."""
    from sql_backend.schema import TriggerDef
    path = str(tmp_path / "trg.db")
    id_col = [ColumnDef("id", "INTEGER", primary_key=True)]
    with SqliteFileBackend(path) as b:
        b.create_table("orders", id_col, if_not_exists=False)
        defn = TriggerDef(
            name="trg_ai", table="orders", timing="AFTER", event="INSERT", body="SELECT 1;"
        )
        b.create_trigger(defn)
        triggers = b.list_triggers("orders")
        assert len(triggers) == 1
        assert triggers[0].name == "trg_ai"
        assert triggers[0].timing == "AFTER"
        assert triggers[0].event == "INSERT"
        assert triggers[0].table == "orders"


def test_list_triggers_empty_table(tmp_path: Path) -> None:
    """list_triggers returns [] when no triggers exist for the table."""
    path = str(tmp_path / "trg2.db")
    with SqliteFileBackend(path) as b:
        b.create_table("t", [ColumnDef("id", "INTEGER")], if_not_exists=False)
        assert b.list_triggers("t") == []


def test_drop_trigger(tmp_path: Path) -> None:
    """drop_trigger removes the trigger from list_triggers."""
    from sql_backend.schema import TriggerDef
    path = str(tmp_path / "trg3.db")
    with SqliteFileBackend(path) as b:
        b.create_table("t", [ColumnDef("id", "INTEGER", primary_key=True)], if_not_exists=False)
        defn = TriggerDef(name="trg", table="t", timing="BEFORE", event="DELETE", body="SELECT 1;")
        b.create_trigger(defn)
        b.drop_trigger("trg")
        assert b.list_triggers("t") == []


def test_drop_trigger_if_exists(tmp_path: Path) -> None:
    """drop_trigger(if_exists=True) is a no-op for a nonexistent trigger."""
    path = str(tmp_path / "trg4.db")
    with SqliteFileBackend(path) as b:
        b.drop_trigger("ghost", if_exists=True)  # must not raise


def test_drop_trigger_not_found_raises(tmp_path: Path) -> None:
    """drop_trigger raises TriggerNotFound for an unknown name."""
    from sql_backend import TriggerNotFound
    path = str(tmp_path / "trg5.db")
    with SqliteFileBackend(path) as b, pytest.raises(TriggerNotFound):
        b.drop_trigger("ghost")


def test_create_trigger_duplicate_raises(tmp_path: Path) -> None:
    """create_trigger raises TriggerAlreadyExists for a duplicate name."""
    from sql_backend import TriggerAlreadyExists
    from sql_backend.schema import TriggerDef
    path = str(tmp_path / "trg6.db")
    with SqliteFileBackend(path) as b:
        b.create_table("t", [ColumnDef("id", "INTEGER")], if_not_exists=False)
        defn = TriggerDef(name="trg", table="t", timing="AFTER", event="INSERT", body="SELECT 1;")
        b.create_trigger(defn)
        with pytest.raises(TriggerAlreadyExists):
            b.create_trigger(defn)


def test_trigger_roundtrip_timing_event(tmp_path: Path) -> None:
    """All timing/event combinations round-trip correctly."""
    from sql_backend.schema import TriggerDef
    path = str(tmp_path / "trg7.db")
    with SqliteFileBackend(path) as b:
        b.create_table("t", [ColumnDef("id", "INTEGER")], if_not_exists=False)
        combos = [
            ("BEFORE", "INSERT"),
            ("AFTER", "INSERT"),
            ("BEFORE", "DELETE"),
            ("AFTER", "DELETE"),
            ("BEFORE", "UPDATE"),
            ("AFTER", "UPDATE"),
        ]
        for timing, event in combos:
            name = f"trg_{timing.lower()}_{event.lower()}"
            b.create_trigger(
                TriggerDef(name=name, table="t", timing=timing, event=event, body="SELECT 1;")  # type: ignore[arg-type]
            )
        triggers = b.list_triggers("t")
        assert len(triggers) == 6
        for trg in triggers:
            assert trg.timing in ("BEFORE", "AFTER")
            assert trg.event in ("INSERT", "DELETE", "UPDATE")


def test_trigger_sql_helpers() -> None:
    """_trigger_to_sql and _sql_to_trigger_def round-trip correctly."""
    from sql_backend.schema import TriggerDef
    defn = TriggerDef(
        name="t1", table="orders", timing="AFTER", event="INSERT",
        body="INSERT INTO log VALUES (1);"
    )
    sql = _trigger_to_sql(defn)
    assert "AFTER" in sql and "INSERT" in sql and "orders" in sql
    back = _sql_to_trigger_def("t1", "orders", sql)
    assert back.timing == "AFTER"
    assert back.event == "INSERT"
    assert "INSERT INTO log" in back.body


# ---------------------------------------------------------------------------
# Helpers used by savepoint tests
# ---------------------------------------------------------------------------


def _scan_all(backend: SqliteFileBackend, table: str) -> list[dict]:
    """Collect all rows from *table* into a list."""
    cursor = backend.scan(table)
    rows = []
    while True:
        row = cursor.next()
        if row is None:
            break
        rows.append(row)
    return rows


# ---------------------------------------------------------------------------
# AUTOINCREMENT — sqlite_sequence high-water tracking
# ---------------------------------------------------------------------------


class TestAutoincrement:
    """``INTEGER PRIMARY KEY AUTOINCREMENT`` never reuses deleted rowids."""

    def _autoinc_columns(self) -> list[ColumnDef]:
        return [
            ColumnDef(
                name="id", type_name="INTEGER",
                primary_key=True, autoincrement=True,
            ),
            ColumnDef(name="name", type_name="TEXT"),
        ]

    def test_creating_autoincrement_table_creates_sqlite_sequence(
        self, tmp_path: Path
    ) -> None:
        path = str(tmp_path / "ai1.db")
        with SqliteFileBackend(path) as b:
            assert "sqlite_sequence" not in b.tables()
            b.create_table("items", self._autoinc_columns(), if_not_exists=False)
            assert "sqlite_sequence" in b.tables()

    def test_no_sqlite_sequence_for_plain_ipk(self, tmp_path: Path) -> None:
        """Plain INTEGER PRIMARY KEY (no AUTOINCREMENT) does NOT bootstrap."""
        path = str(tmp_path / "ai2.db")
        with SqliteFileBackend(path) as b:
            b.create_table(
                "items",
                [
                    ColumnDef(name="id", type_name="INTEGER", primary_key=True),
                    ColumnDef(name="name", type_name="TEXT"),
                ],
                if_not_exists=False,
            )
            assert "sqlite_sequence" not in b.tables()

    def test_autoinc_assigns_rowids_sequentially(self, tmp_path: Path) -> None:
        path = str(tmp_path / "ai3.db")
        with SqliteFileBackend(path) as b:
            b.create_table("items", self._autoinc_columns(), if_not_exists=False)
            b.insert("items", {"name": "a"})
            b.insert("items", {"name": "b"})
            b.insert("items", {"name": "c"})
            rows = sorted(_scan_all(b, "items"), key=lambda r: r["id"])
            assert [r["id"] for r in rows] == [1, 2, 3]

    def test_autoinc_does_not_reuse_deleted_rowid(self, tmp_path: Path) -> None:
        """The whole point of AUTOINCREMENT vs plain IPK."""
        path = str(tmp_path / "ai4.db")
        with SqliteFileBackend(path) as b:
            b.create_table("items", self._autoinc_columns(), if_not_exists=False)
            b.insert("items", {"name": "a"})  # id=1
            b.insert("items", {"name": "b"})  # id=2
            b.insert("items", {"name": "c"})  # id=3

            cursor = b.scan("items")
            while (r := cursor.next()) is not None:
                if r["id"] == 3:
                    b.delete("items", cursor)
                    break

            b.insert("items", {"name": "d"})  # MUST be id=4, not id=3
            rows = sorted(_scan_all(b, "items"), key=lambda r: r["id"])
            assert [r["id"] for r in rows] == [1, 2, 4]

    def test_plain_ipk_does_reuse_deleted_rowid_at_max(self, tmp_path: Path) -> None:
        """Without AUTOINCREMENT, deleting the max rowid lets it be reused.

        This is the legacy SQLite behaviour for plain INTEGER PRIMARY KEY —
        and is the contrast that motivates AUTOINCREMENT.
        """
        path = str(tmp_path / "ai5.db")
        with SqliteFileBackend(path) as b:
            b.create_table(
                "items",
                [
                    ColumnDef(name="id", type_name="INTEGER", primary_key=True),
                    ColumnDef(name="name", type_name="TEXT"),
                ],
                if_not_exists=False,
            )
            b.insert("items", {"name": "a"})  # id=1
            b.insert("items", {"name": "b"})  # id=2
            cursor = b.scan("items")
            while (r := cursor.next()) is not None:
                if r["id"] == 2:
                    b.delete("items", cursor)
                    break
            b.insert("items", {"name": "c"})  # id=2 (reused)
            rows = sorted(_scan_all(b, "items"), key=lambda r: r["id"])
            assert [r["id"] for r in rows] == [1, 2]

    def test_autoinc_explicit_rowid_bumps_sequence(self, tmp_path: Path) -> None:
        """Inserting an explicit rowid above the sequence bumps it."""
        path = str(tmp_path / "ai6.db")
        with SqliteFileBackend(path) as b:
            b.create_table("items", self._autoinc_columns(), if_not_exists=False)
            b.insert("items", {"id": 100, "name": "explicit"})
            b.insert("items", {"name": "next"})  # MUST be id=101
            rows = sorted(_scan_all(b, "items"), key=lambda r: r["id"])
            assert [r["id"] for r in rows] == [100, 101]

    def test_autoinc_persists_across_reopen(self, tmp_path: Path) -> None:
        """The high-water seq is stored in sqlite_sequence so it survives reopen."""
        path = str(tmp_path / "ai7.db")
        with SqliteFileBackend(path) as b:
            b.create_table("items", self._autoinc_columns(), if_not_exists=False)
            b.insert("items", {"name": "a"})
            b.insert("items", {"name": "b"})
            cursor = b.scan("items")
            while (r := cursor.next()) is not None:
                if r["id"] == 2:
                    b.delete("items", cursor)
                    break
            h = b.begin_transaction()
            b.commit(h)

        with SqliteFileBackend(path) as b2:
            b2.insert("items", {"name": "after-reopen"})
            rows = sorted(_scan_all(b2, "items"), key=lambda r: r["id"])
            # First row (id=1) survives, deleted id=2 is NOT reused.
            assert [r["id"] for r in rows] == [1, 3]

    def test_autoincrement_round_trips_through_create_table_sql(
        self, tmp_path: Path
    ) -> None:
        """Closing and reopening must preserve the AUTOINCREMENT flag in the schema."""
        path = str(tmp_path / "ai8.db")
        with SqliteFileBackend(path) as b:
            b.create_table("items", self._autoinc_columns(), if_not_exists=False)
            h = b.begin_transaction()
            b.commit(h)

        with SqliteFileBackend(path) as b2:
            cols = b2.columns("items")
            id_col = next(c for c in cols if c.name == "id")
            assert id_col.primary_key is True
            assert id_col.autoincrement is True

    def test_columns_to_sql_emits_autoincrement(self) -> None:
        """The CREATE TABLE SQL emitter wraps AUTOINCREMENT after PRIMARY KEY."""
        from storage_sqlite.backend import _columns_to_sql
        sql = _columns_to_sql(
            "t",
            [
                ColumnDef(
                    name="id", type_name="INTEGER",
                    primary_key=True, autoincrement=True,
                ),
            ],
        )
        assert "PRIMARY KEY AUTOINCREMENT" in sql

    def test_parse_one_column_recognises_autoincrement(self) -> None:
        from storage_sqlite.backend import _parse_one_column
        col = _parse_one_column("id INTEGER PRIMARY KEY AUTOINCREMENT")
        assert col is not None
        assert col.primary_key is True
        assert col.autoincrement is True
