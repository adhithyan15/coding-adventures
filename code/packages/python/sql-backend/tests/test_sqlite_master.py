"""Tests for the synthesized ``sqlite_master`` / ``sqlite_schema`` table.

The in-memory backend exposes a virtual ``sqlite_master`` (and its alias
``sqlite_schema``) that lists tables, indexes, and triggers.  The rows
are synthesized on demand from the backend's current state — no
storage, no maintenance.  These tests pin:

* the fixed five-column schema
* row content for tables / indexes
* read-only enforcement (no INSERT/UPDATE/DROP/CREATE on the master name)
* both names route to the same data
"""

from __future__ import annotations

import pytest

from sql_backend.errors import ConstraintViolation
from sql_backend.in_memory import InMemoryBackend
from sql_backend.index import IndexDef
from sql_backend.schema import ColumnDef


def _make_backend() -> InMemoryBackend:
    """Backend with one table and one user-created index."""
    b = InMemoryBackend()
    b.create_table(
        "users",
        [
            ColumnDef(name="id", type_name="INTEGER", primary_key=True),
            ColumnDef(name="name", type_name="TEXT", not_null=True),
        ],
        if_not_exists=False,
    )
    b.create_index(IndexDef(name="ix_name", table="users", columns=("name",), unique=False))
    return b


class TestMasterSchema:
    """``columns('sqlite_master')`` and ``columns('sqlite_schema')`` return
    the fixed five-column schema regardless of backend state."""

    def test_columns_returns_fixed_schema(self) -> None:
        b = InMemoryBackend()
        cols = b.columns("sqlite_master")
        assert [c.name for c in cols] == ["type", "name", "tbl_name", "rootpage", "sql"]
        assert [c.type_name for c in cols] == ["TEXT", "TEXT", "TEXT", "INTEGER", "TEXT"]

    def test_sqlite_schema_is_alias(self) -> None:
        # Both names yield identical column lists.
        b = InMemoryBackend()
        assert b.columns("sqlite_master") == b.columns("sqlite_schema")

    def test_columns_independent_of_user_tables(self) -> None:
        b = _make_backend()
        # User tables exist but the master schema is fixed.
        assert [c.name for c in b.columns("sqlite_master")] == [
            "type", "name", "tbl_name", "rootpage", "sql"
        ]


class TestMasterRows:
    """``scan('sqlite_master')`` synthesizes one row per object."""

    def test_empty_backend_has_no_master_rows(self) -> None:
        b = InMemoryBackend()
        it = b.scan("sqlite_master")
        assert it.next() is None

    def test_one_table_one_row(self) -> None:
        b = InMemoryBackend()
        b.create_table(
            "t", [ColumnDef(name="x", type_name="INTEGER")], if_not_exists=False
        )
        rows = _collect(b.scan("sqlite_master"))
        assert len(rows) == 1
        r = rows[0]
        assert r["type"] == "table"
        assert r["name"] == "t"
        assert r["tbl_name"] == "t"
        # The first b-tree object gets rootpage 1; non-zero is SQLite's
        # convention for "exists in the b-tree".
        assert r["rootpage"] == 1
        # Identifiers are quoted to prevent second-order SQL injection
        # when a downstream consumer re-executes sqlite_master.sql.
        assert 'CREATE TABLE "t"' in r["sql"]
        assert '"x" INTEGER' in r["sql"]

    def test_index_appears_after_table(self) -> None:
        b = _make_backend()
        rows = _collect(b.scan("sqlite_master"))
        types = [r["type"] for r in rows]
        assert "table" in types
        assert "index" in types
        # Tables come before indexes (matches SQLite's insertion order).
        assert types.index("table") < types.index("index")

    def test_index_row_contents(self) -> None:
        b = _make_backend()
        rows = _collect(b.scan("sqlite_master"))
        idx_rows = [r for r in rows if r["type"] == "index"]
        assert len(idx_rows) == 1
        r = idx_rows[0]
        assert r["name"] == "ix_name"
        assert r["tbl_name"] == "users"
        assert r["sql"] == 'CREATE INDEX "ix_name" ON "users" ("name")'

    def test_unique_index_in_sql_text(self) -> None:
        b = InMemoryBackend()
        b.create_table(
            "t", [ColumnDef(name="x", type_name="INTEGER")], if_not_exists=False
        )
        b.create_index(IndexDef(name="ix_x", table="t", columns=("x",), unique=True))
        r = next(r for r in _collect(b.scan("sqlite_master")) if r["type"] == "index")
        assert "UNIQUE" in r["sql"]

    def test_autoindex_has_null_sql(self) -> None:
        # ``sqlite_autoindex_*`` names indicate engine-managed indexes —
        # their sql column should be NULL, matching real SQLite.
        b = InMemoryBackend()
        b.create_table(
            "t",
            [
                ColumnDef(name="id", type_name="INTEGER", primary_key=True),
                ColumnDef(name="email", type_name="TEXT", unique=True),
            ],
            if_not_exists=False,
        )
        b.create_index(IndexDef(
            name="sqlite_autoindex_t_1", table="t", columns=("email",), unique=True,
        ))
        idx_rows = [r for r in _collect(b.scan("sqlite_master")) if r["type"] == "index"]
        assert idx_rows[0]["sql"] is None

    def test_strict_keyword_appears_in_sql(self) -> None:
        b = InMemoryBackend()
        b.create_table(
            "t",
            [ColumnDef(name="x", type_name="INTEGER")],
            if_not_exists=False,
            strict=True,
        )
        r = _collect(b.scan("sqlite_master"))[0]
        # Trailing STRICT keyword, after the column-list closing paren.
        assert r["sql"].endswith(") STRICT")


class TestInjectionMitigation:
    """Identifier quoting in the synthesized ``sql`` column.

    The ``sql`` column of ``sqlite_master`` is data, not directly
    re-executed by mini-sqlite — but a common consumer pattern (ORMs,
    migration tools, schema-diff utilities) is to read the sql column
    and feed it back to ``execute()``.  If we emitted unquoted
    identifiers, a maliciously-named column could carry SQL syntax
    across that boundary.  These tests pin the mitigation: every
    identifier in the synthesized output is wrapped in double quotes
    with embedded ``"`` doubled.
    """

    def test_table_name_with_quote_doubled(self) -> None:
        b = InMemoryBackend()
        b.create_table(
            'weird"name',
            [ColumnDef(name="x", type_name="INTEGER")],
            if_not_exists=False,
        )
        r = _collect(b.scan("sqlite_master"))[0]
        # The embedded `"` must be doubled inside the quoted identifier.
        assert '"weird""name"' in r["sql"]

    def test_column_name_with_quote_doubled(self) -> None:
        b = InMemoryBackend()
        b.create_table(
            "t",
            [ColumnDef(name='col"with"quotes', type_name="TEXT")],
            if_not_exists=False,
        )
        r = _collect(b.scan("sqlite_master"))[0]
        assert '"col""with""quotes" TEXT' in r["sql"]

    def test_column_name_with_injection_attempt_neutralized(self) -> None:
        # A column name crafted to look like SQL must end up as a
        # quoted identifier, not as executable syntax.
        b = InMemoryBackend()
        b.create_table(
            "t",
            [ColumnDef(name='x); DROP TABLE users;--', type_name="INTEGER")],
            if_not_exists=False,
        )
        r = _collect(b.scan("sqlite_master"))[0]
        # The entire payload is inside one quoted identifier — the
        # outer parentheses we generate for the column list are still
        # the only `(` / `)` in the surrounding structure.
        assert '"x); DROP TABLE users;--" INTEGER' in r["sql"]

    def test_default_string_with_quotes_escaped(self) -> None:
        # Defaults are formatted as SQL literals with single-quote
        # doubling (SQLite's rule), not Python repr.
        b = InMemoryBackend()
        b.create_table(
            "t",
            [ColumnDef(name="x", type_name="TEXT", default="he said 'hi'")],
            if_not_exists=False,
        )
        r = _collect(b.scan("sqlite_master"))[0]
        assert "DEFAULT 'he said ''hi'''" in r["sql"]

    def test_default_bytes_as_blob_literal(self) -> None:
        # bytes defaults serialise as X'hex...' (SQLite BLOB literal),
        # not Python's b'...' repr.
        b = InMemoryBackend()
        b.create_table(
            "t",
            [ColumnDef(name="x", type_name="BLOB", default=b"\x00\xff")],
            if_not_exists=False,
        )
        r = _collect(b.scan("sqlite_master"))[0]
        assert "DEFAULT X'00FF'" in r["sql"]

    def test_unsafe_type_name_neutralized_to_numeric(self) -> None:
        # SQLite's CREATE TABLE grammar accepts almost any token sequence
        # in the column-type slot.  A programmatic caller could construct
        # a ColumnDef with a type that contains SQL syntax; the
        # synthesized sql column must NOT echo that verbatim.
        b = InMemoryBackend()
        b.create_table(
            "t",
            [ColumnDef(name="x", type_name="INTEGER); DROP TABLE u;--")],
            if_not_exists=False,
        )
        r = _collect(b.scan("sqlite_master"))[0]
        # The unsafe text is replaced with NUMERIC affinity; the
        # injection payload never reaches the synthesized sql.
        assert "DROP TABLE" not in r["sql"]
        assert " NUMERIC" in r["sql"]

    def test_unsafe_collation_neutralized_to_binary(self) -> None:
        b = InMemoryBackend()
        b.create_table(
            "t",
            [ColumnDef(name="x", type_name="TEXT", collation="X; DROP TABLE u;--")],
            if_not_exists=False,
        )
        r = _collect(b.scan("sqlite_master"))[0]
        assert "DROP TABLE" not in r["sql"]
        assert "COLLATE BINARY" in r["sql"]

    def test_parameterized_type_name_preserved(self) -> None:
        # ``VARCHAR(64)`` and ``DECIMAL(10,2)`` are common legacy types;
        # they must pass through the sanitizer unchanged.
        b = InMemoryBackend()
        b.create_table(
            "t",
            [
                ColumnDef(name="a", type_name="VARCHAR(64)"),
                ColumnDef(name="b", type_name="DECIMAL(10,2)"),
            ],
            if_not_exists=False,
        )
        r = _collect(b.scan("sqlite_master"))[0]
        assert "VARCHAR(64)" in r["sql"]
        assert "DECIMAL(10,2)" in r["sql"]

    def test_non_finite_float_default_as_null(self) -> None:
        # ``inf`` / ``nan`` are not valid SQL literals; emit NULL instead
        # of letting Python's ``repr()`` produce ``inf``.
        b = InMemoryBackend()
        b.create_table(
            "t",
            [ColumnDef(name="x", type_name="REAL", default=float("inf"))],
            if_not_exists=False,
        )
        r = _collect(b.scan("sqlite_master"))[0]
        assert "DEFAULT NULL" in r["sql"]
        assert "inf" not in r["sql"]

    def test_default_none_as_null(self) -> None:
        b = InMemoryBackend()
        b.create_table(
            "t",
            [ColumnDef(name="x", type_name="INTEGER", default=None)],
            if_not_exists=False,
        )
        r = _collect(b.scan("sqlite_master"))[0]
        assert "DEFAULT NULL" in r["sql"]


class TestSchemaAlias:
    """``sqlite_schema`` returns identical rows to ``sqlite_master``."""

    def test_both_names_yield_same_rows(self) -> None:
        b = _make_backend()
        rows_master = _collect(b.scan("sqlite_master"))
        rows_schema = _collect(b.scan("sqlite_schema"))
        assert rows_master == rows_schema


class TestReadOnly:
    """The master / schema table cannot be mutated or redefined."""

    def test_cannot_create_table_named_sqlite_master(self) -> None:
        b = InMemoryBackend()
        with pytest.raises(ConstraintViolation) as exc:
            b.create_table(
                "sqlite_master",
                [ColumnDef(name="x", type_name="TEXT")],
                if_not_exists=False,
            )
        assert "reserved" in str(exc.value).lower()

    def test_cannot_create_table_named_sqlite_schema(self) -> None:
        b = InMemoryBackend()
        with pytest.raises(ConstraintViolation):
            b.create_table(
                "sqlite_schema",
                [ColumnDef(name="x", type_name="TEXT")],
                if_not_exists=False,
            )

    def test_cannot_drop_sqlite_master(self) -> None:
        # The IF EXISTS branch would otherwise silently succeed because
        # ``sqlite_master`` isn't in ``_tables`` — the explicit guard
        # surfaces a clear error.
        b = InMemoryBackend()
        with pytest.raises(ConstraintViolation):
            b.drop_table("sqlite_master", if_exists=True)

    def test_cannot_drop_sqlite_master_no_if_exists(self) -> None:
        b = InMemoryBackend()
        with pytest.raises(ConstraintViolation):
            b.drop_table("sqlite_master", if_exists=False)

    def test_cannot_insert_into_sqlite_master(self) -> None:
        b = InMemoryBackend()
        with pytest.raises(ConstraintViolation):
            b.insert("sqlite_master", {
                "type": "table",
                "name": "fake",
                "tbl_name": "fake",
                "rootpage": 0,
                "sql": "CREATE TABLE fake (x INT)",
            })


class TestRootpage:
    """Non-zero rootpage for tables and indexes; 0 for triggers."""

    def test_first_table_gets_rootpage_1(self) -> None:
        b = InMemoryBackend()
        b.create_table(
            "t", [ColumnDef(name="x", type_name="INTEGER")], if_not_exists=False
        )
        r = _collect(b.scan("sqlite_master"))[0]
        assert r["rootpage"] == 1

    def test_pages_monotonic_in_creation_order(self) -> None:
        # SQLite assigns page numbers in creation order for fresh tables;
        # we mirror that by enumerating tables then indexes in dict order.
        b = _make_backend()
        rows = _collect(b.scan("sqlite_master"))
        pages = [r["rootpage"] for r in rows]
        assert pages == sorted(pages)  # monotonic
        assert all(p > 0 for p in pages)  # all positive (b-tree objects)

    def test_index_gets_distinct_page_from_table(self) -> None:
        b = _make_backend()  # users (table) + ix_name (index)
        rows = _collect(b.scan("sqlite_master"))
        table_pages = {r["rootpage"] for r in rows if r["type"] == "table"}
        index_pages = {r["rootpage"] for r in rows if r["type"] == "index"}
        assert not (table_pages & index_pages)


class TestTablesListingIsClean:
    """``backend.tables()`` lists user tables only — system tables are
    hidden from the listing (matching SQLite's ``.tables`` behaviour)."""

    def test_tables_does_not_include_sqlite_master(self) -> None:
        b = InMemoryBackend()
        assert "sqlite_master" not in b.tables()
        assert "sqlite_schema" not in b.tables()

    def test_tables_lists_user_tables_after_create(self) -> None:
        b = _make_backend()
        assert b.tables() == ["users"]


def _collect(it: object) -> list[dict]:
    """Drain an iterator's rows into a plain list (for ergonomic assertions)."""
    rows = []
    while True:
        r = it.next()
        if r is None:
            break
        rows.append(r)
    return rows
