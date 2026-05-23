"""Read-only introspection PRAGMAs.

This file pins the behaviour of a cluster of PRAGMAs that
applications, ORMs, and DB admin tools probe to learn what they can do
against a connection.  They're all read-only and have no side effects:

* ``PRAGMA database_list``     — attached databases (we have just ``main``)
* ``PRAGMA collation_list``    — supported collations
* ``PRAGMA compile_options``   — build-time flags
* ``PRAGMA function_list``     — registered scalar / aggregate functions
* ``PRAGMA module_list``       — virtual-table modules (we have none)
* ``PRAGMA pragma_list``       — the catalog of supported PRAGMAs
* ``PRAGMA data_version``      — write-counter (stays at 1 for in-memory)
* ``PRAGMA integrity_check``   — schema/data integrity (always 'ok')
* ``PRAGMA quick_check``       — same shape, faster variant
* ``PRAGMA page_count``,
  ``PRAGMA page_size``,
  ``PRAGMA freelist_count``,
  ``PRAGMA encoding``          — page/encoding metadata

Where the answer is meaningful for an in-memory mini-sqlite connection
(``database_list``, ``data_version``, ``page_size``, ``encoding`` and
the integrity probes) we oracle-test against stdlib sqlite3.  For the
inherently version-dependent lists (``pragma_list``,
``compile_options``, ``function_list``) we assert *shape and ordering*
rather than membership, because mini-sqlite advertises only what it
implements (which is by design a strict subset of real SQLite).
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _check_oracle(query: str) -> None:
    """Byte-for-byte against stdlib sqlite3."""
    m = list(mini_sqlite.connect(":memory:").execute(query))
    r = list(sqlite3.connect(":memory:").execute(query))
    assert m == r, f"SQL: {query!r}\n  mini: {m}\n  ref:  {r}"


# ---------------------------------------------------------------------------
# Oracle-tested PRAGMAs — same rows as stdlib sqlite3 on a fresh
# ``:memory:`` connection.
# ---------------------------------------------------------------------------


class TestOracleMatch:
    def test_database_list_default(self) -> None:
        _check_oracle("PRAGMA database_list")

    def test_data_version_default(self) -> None:
        _check_oracle("PRAGMA data_version")

    def test_page_size_default(self) -> None:
        _check_oracle("PRAGMA page_size")

    def test_page_count_empty(self) -> None:
        _check_oracle("PRAGMA page_count")

    def test_freelist_count_empty(self) -> None:
        _check_oracle("PRAGMA freelist_count")

    def test_encoding_default(self) -> None:
        _check_oracle("PRAGMA encoding")

    def test_integrity_check(self) -> None:
        _check_oracle("PRAGMA integrity_check")

    def test_quick_check(self) -> None:
        _check_oracle("PRAGMA quick_check")


# ---------------------------------------------------------------------------
# pragma_list — exists, alphabetical, advertises only what we implement.
# ---------------------------------------------------------------------------


class TestPragmaList:
    def test_returns_rows(self) -> None:
        rows = list(mini_sqlite.connect(":memory:").execute("PRAGMA pragma_list"))
        assert len(rows) > 0, "pragma_list should not be empty"

    def test_single_column_named_name(self) -> None:
        cur = mini_sqlite.connect(":memory:").execute("PRAGMA pragma_list")
        assert cur.description is not None
        assert [d[0] for d in cur.description] == ["name"]

    def test_alphabetical(self) -> None:
        rows = list(mini_sqlite.connect(":memory:").execute("PRAGMA pragma_list"))
        names = [r[0] for r in rows]
        assert names == sorted(names), "pragma_list should be alphabetical"

    def test_no_duplicates(self) -> None:
        rows = list(mini_sqlite.connect(":memory:").execute("PRAGMA pragma_list"))
        names = [r[0] for r in rows]
        assert len(names) == len(set(names)), "pragma_list should have no duplicates"

    def test_advertises_table_info(self) -> None:
        # The cornerstone introspection PRAGMA — must be present.
        rows = list(mini_sqlite.connect(":memory:").execute("PRAGMA pragma_list"))
        names = {r[0] for r in rows}
        assert "table_info" in names

    def test_advertises_self(self) -> None:
        # If pragma_list is listed, downstream tools can discover it
        # without already knowing the name.  (Honest, since we just
        # implemented it.)
        rows = list(mini_sqlite.connect(":memory:").execute("PRAGMA pragma_list"))
        names = {r[0] for r in rows}
        assert "pragma_list" in names

    def test_advertises_writable_pragmas(self) -> None:
        # Writable scalars from _PRAGMA_DEFAULTS should appear so that
        # tools can know they can set them.
        rows = list(mini_sqlite.connect(":memory:").execute("PRAGMA pragma_list"))
        names = {r[0] for r in rows}
        for expected in ("foreign_keys", "user_version", "cache_size", "synchronous"):
            assert expected in names, f"{expected} missing from pragma_list"


# ---------------------------------------------------------------------------
# compile_options — shape only (the values are intentionally
# different from real SQLite because mini-sqlite is a different
# implementation, but tools rely on the *shape*).
# ---------------------------------------------------------------------------


class TestCompileOptions:
    def test_single_column_named_compile_options(self) -> None:
        cur = mini_sqlite.connect(":memory:").execute("PRAGMA compile_options")
        assert cur.description is not None
        assert [d[0] for d in cur.description] == ["compile_options"]

    def test_returns_rows(self) -> None:
        rows = list(mini_sqlite.connect(":memory:").execute("PRAGMA compile_options"))
        assert len(rows) > 0


# ---------------------------------------------------------------------------
# function_list — every row should describe a function known to the VM.
# ---------------------------------------------------------------------------


class TestFunctionList:
    def test_shape(self) -> None:
        cur = mini_sqlite.connect(":memory:").execute("PRAGMA function_list")
        assert cur.description is not None
        cols = [d[0] for d in cur.description]
        assert cols == ["name", "builtin", "type", "enc", "narg", "flags"]

    def test_includes_common_functions(self) -> None:
        rows = list(mini_sqlite.connect(":memory:").execute("PRAGMA function_list"))
        names = {r[0] for r in rows}
        for fn in ("abs", "length", "lower", "upper", "coalesce", "count"):
            assert fn in names, f"{fn} missing from function_list"

    def test_aggregates_marked_a(self) -> None:
        rows = list(mini_sqlite.connect(":memory:").execute("PRAGMA function_list"))
        by_name = {r[0]: r for r in rows}
        for agg in ("count", "sum", "avg", "min", "max"):
            assert by_name[agg][2] == "a", (
                f"{agg} should be reported as aggregate (type='a'); got {by_name[agg]}"
            )


# ---------------------------------------------------------------------------
# module_list — empty (we have no virtual table modules); the row shape
# must still be correct so tools that iterate over it don't crash.
# ---------------------------------------------------------------------------


class TestModuleList:
    def test_empty(self) -> None:
        rows = list(mini_sqlite.connect(":memory:").execute("PRAGMA module_list"))
        assert rows == []

    def test_column_shape(self) -> None:
        cur = mini_sqlite.connect(":memory:").execute("PRAGMA module_list")
        assert cur.description is not None
        assert [d[0] for d in cur.description] == ["name"]


# ---------------------------------------------------------------------------
# collation_list — three rows, matching real SQLite's catalogue.
# ---------------------------------------------------------------------------


class TestCollationList:
    def test_rows(self) -> None:
        rows = list(mini_sqlite.connect(":memory:").execute("PRAGMA collation_list"))
        # Real SQLite returns these three; mini-sqlite mirrors the
        # naming so applications that probe for "do you have NOCASE?"
        # find a yes.
        names = {r[1] for r in rows}
        assert {"BINARY", "NOCASE", "RTRIM"} <= names
