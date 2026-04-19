"""
Backend conformance suite
=========================

Shared test helpers that every Backend must pass. The model:

    from sql_backend.conformance import run_required, run_read_write, run_ddl, run_transaction

    def test_my_backend():
        run_required(lambda: populate_my_backend())

Each ``run_*`` function accepts a *factory* — a zero-arg callable that returns
a fresh, pre-populated backend in a known state. Tests call the factory,
perform their assertions, and discard the backend. Any backend that passes
the relevant tiers is a drop-in replacement for :class:`InMemoryBackend`.

Why factories and not fixtures?
-------------------------------

Because some tests mutate the backend (insert, delete, rollback) — and each
test needs a clean slate. Taking a factory means the conformance suite
doesn't need to know *how* a backend resets itself (truncate? fresh object?
new temp file?) — it just asks for a fresh one.

Four tiers
----------

1. **Required** — every backend: schema introspection and read-only scan.
2. **Read-write** — backends that accept mutations.
3. **DDL** — backends that can create and drop tables.
4. **Transaction** — backends with real commit/rollback semantics.

Read-only backends (CSV, HTTP) pass tier 1 only, and raise :class:`Unsupported`
for tiers 2–4. That's fine — the conformance suite lets a test skip the
tiers the backend doesn't claim.

The golden-state fixture
------------------------

All tiers share one fixture: a ``users`` table with five pre-inserted rows.
Using one shape across tiers keeps the assertions short and means a failure
in a higher tier doesn't force the reader to context-switch to a new schema.
"""

from __future__ import annotations

from collections.abc import Callable

import pytest

from .backend import Backend
from .errors import (
    ColumnNotFound,
    ConstraintViolation,
    TableAlreadyExists,
    TableNotFound,
)
from .in_memory import InMemoryBackend
from .row import Row
from .schema import ColumnDef

BackendFactory = Callable[[], Backend]


# --- The golden-state fixture ---------------------------------------------

USERS_COLUMNS: list[ColumnDef] = [
    ColumnDef(name="id", type_name="INTEGER", primary_key=True),
    ColumnDef(name="name", type_name="TEXT", not_null=True),
    ColumnDef(name="age", type_name="INTEGER"),
    ColumnDef(name="email", type_name="TEXT", unique=True),
]

USERS_ROWS: list[Row] = [
    {"id": 1, "name": "alice", "age": 30, "email": "alice@example.com"},
    {"id": 2, "name": "bob", "age": 25, "email": "bob@example.com"},
    {"id": 3, "name": "carol", "age": 35, "email": "carol@example.com"},
    {"id": 4, "name": "dave", "age": 40, "email": None},
    {"id": 5, "name": "eve", "age": 28, "email": "eve@example.com"},
]


def make_in_memory_users() -> InMemoryBackend:
    """Build a fresh InMemoryBackend containing the golden ``users`` table.

    Every language that implements the Backend interface should ship its own
    analogue. Conformance tests run against this fixture so failures mean
    the same thing in every language.
    """
    return InMemoryBackend.from_tables(
        {"users": (USERS_COLUMNS, USERS_ROWS)},
    )


# --- Tier 1: required -----------------------------------------------------


def run_required(factory: BackendFactory) -> None:
    """Schema + read-only scan. Every backend must pass.

    Check each assertion individually so a failing test pinpoints the
    specific contract the backend violates rather than reporting a single
    opaque "conformance failed".
    """
    # 1. tables() lists registered tables
    b = factory()
    assert "users" in b.tables()

    # 2. columns(t) returns column definitions in declaration order
    b = factory()
    cols = b.columns("users")
    assert [c.name for c in cols] == ["id", "name", "age", "email"]

    # 3. columns(unknown) raises TableNotFound
    b = factory()
    with pytest.raises(TableNotFound):
        b.columns("missing")

    # 4. scan(t) returns all rows in insertion order
    b = factory()
    it = b.scan("users")
    seen: list[Row] = []
    while True:
        row = it.next()
        if row is None:
            break
        seen.append(row)
    it.close()
    assert [r["id"] for r in seen] == [1, 2, 3, 4, 5]

    # 5. scan(unknown) raises TableNotFound
    b = factory()
    with pytest.raises(TableNotFound):
        b.scan("missing")

    # 6. scan() on empty table returns zero rows
    empty = InMemoryBackend.from_tables({"empty": (USERS_COLUMNS, [])})
    it = empty.scan("empty")
    assert it.next() is None
    it.close()


# --- Tier 2: read-write ---------------------------------------------------


def run_read_write(factory: BackendFactory) -> None:
    """Inserts, updates, deletes — plus constraint enforcement."""
    # 7. insert() makes a row visible to subsequent scan()
    b = factory()
    b.insert("users", {"id": 6, "name": "frank", "age": 50, "email": "frank@example.com"})
    ids = _scan_ids(b, "users")
    assert 6 in ids

    # 8. NOT NULL violation
    b = factory()
    with pytest.raises(ConstraintViolation):
        b.insert("users", {"id": 99, "name": None, "age": 1, "email": None})

    # 9. Duplicate primary key
    b = factory()
    with pytest.raises(ConstraintViolation):
        b.insert("users", {"id": 1, "name": "dup", "age": 0, "email": None})

    # 10. Defaults applied for omitted columns
    b = factory()
    b.create_table(
        "with_default",
        [
            ColumnDef(name="id", type_name="INTEGER", primary_key=True),
            ColumnDef(name="flag", type_name="INTEGER", default=7),
        ],
        if_not_exists=False,
    )
    b.insert("with_default", {"id": 1})
    it = b.scan("with_default")
    row = it.next()
    it.close()
    assert row == {"id": 1, "flag": 7}

    # 11. update() visible in subsequent scan
    b = factory()
    cursor = b._open_cursor("users")  # type: ignore[attr-defined]
    # Advance to the second row (bob, id=2).
    cursor.next()
    cursor.next()
    b.update("users", cursor, {"age": 99})
    cursor.close()
    # Read back the row with id=2.
    it = b.scan("users")
    rows: list[Row] = []
    while True:
        r = it.next()
        if r is None:
            break
        rows.append(r)
    it.close()
    assert next(r for r in rows if r["id"] == 2)["age"] == 99

    # 12. update() NOT NULL violation
    b = factory()
    cursor = b._open_cursor("users")  # type: ignore[attr-defined]
    cursor.next()
    with pytest.raises(ConstraintViolation):
        b.update("users", cursor, {"name": None})

    # 13. delete() removes the row
    b = factory()
    cursor = b._open_cursor("users")  # type: ignore[attr-defined]
    cursor.next()  # id=1
    b.delete("users", cursor)
    assert 1 not in _scan_ids(b, "users")


# --- Tier 3: DDL ----------------------------------------------------------


def run_ddl(factory: BackendFactory) -> None:
    """CREATE TABLE and DROP TABLE, with and without IF [NOT] EXISTS."""
    # 14. create_table() makes the table visible
    b = factory()
    b.create_table(
        "widgets",
        [ColumnDef(name="id", type_name="INTEGER", primary_key=True)],
        if_not_exists=False,
    )
    assert "widgets" in b.tables()

    # 15. create_table with if_not_exists=False on existing → raise
    b = factory()
    with pytest.raises(TableAlreadyExists):
        b.create_table("users", [], if_not_exists=False)

    # 16. create_table with if_not_exists=True on existing → no-op
    b = factory()
    b.create_table("users", [], if_not_exists=True)
    assert "users" in b.tables()

    # 17. drop_table() removes the table
    b = factory()
    b.drop_table("users", if_exists=False)
    assert "users" not in b.tables()

    # 18. drop_table if_exists=False on missing → raise
    b = factory()
    with pytest.raises(TableNotFound):
        b.drop_table("ghost", if_exists=False)

    # 19. drop_table if_exists=True on missing → no-op
    b = factory()
    b.drop_table("ghost", if_exists=True)


# --- Tier 4: transactions -------------------------------------------------


def run_transaction(factory: BackendFactory) -> None:
    """Snapshot isolation: commit persists; rollback restores."""
    # 20. Committed mutations are visible
    b = factory()
    h = b.begin_transaction()
    b.insert("users", {"id": 7, "name": "grace", "age": 45, "email": "grace@example.com"})
    b.commit(h)
    assert 7 in _scan_ids(b, "users")

    # 21. Rolled-back mutations are not visible
    b = factory()
    before = _scan_ids(b, "users")
    h = b.begin_transaction()
    b.insert("users", {"id": 8, "name": "heidi", "age": 22, "email": "heidi@example.com"})
    b.rollback(h)
    after = _scan_ids(b, "users")
    assert before == after

    # 22. Rollback restores exact state
    b = factory()
    cursor = b._open_cursor("users")  # type: ignore[attr-defined]
    cursor.next()  # id=1
    h = b.begin_transaction()
    b.delete("users", cursor)
    cursor.close()
    b.rollback(h)
    assert 1 in _scan_ids(b, "users")


# --- Helpers --------------------------------------------------------------


def _scan_ids(backend: Backend, table: str) -> list[int]:
    """Pull out the id column from every row — cheap way to compare states."""
    it = backend.scan(table)
    ids: list[int] = []
    while True:
        r = it.next()
        if r is None:
            break
        v = r.get("id")
        if isinstance(v, int):
            ids.append(v)
    it.close()
    return ids


# Exported so external tests can filter the column-not-found probe
__all__ = [
    "BackendFactory",
    "USERS_COLUMNS",
    "USERS_ROWS",
    "ColumnNotFound",
    "make_in_memory_users",
    "run_ddl",
    "run_read_write",
    "run_required",
    "run_transaction",
]
