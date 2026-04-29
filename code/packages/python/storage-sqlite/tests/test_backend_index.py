"""Tests for SqliteFileBackend index operations (phase IX-2).

Coverage targets:
- Schema.create_index / drop_index / find_index / list_indexes
- SqliteFileBackend.create_index — success, duplicate, bad table, bad column
- SqliteFileBackend.create_index backfills existing rows
- SqliteFileBackend.scan_index — equality, range, bounds
- SqliteFileBackend.drop_index — removes B-tree pages + schema row
- SqliteFileBackend.list_indexes — all, filtered, auto flag
- IndexAlreadyExists / IndexNotFound propagated correctly
- Persistence: index survives backend close and reopen
- Oracle test: index row visible to stdlib sqlite3
- Reverse oracle: sqlite3-created index readable by scan_index
"""

from __future__ import annotations

import sqlite3
from pathlib import Path

import pytest
from sql_backend import (
    ColumnDef,
    ColumnNotFound,
    IndexAlreadyExists,
    IndexDef,
    IndexNotFound,
    TableNotFound,
)

from storage_sqlite import SqliteFileBackend
from storage_sqlite.schema import SchemaError

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

_COLS = [
    ColumnDef(name="id", type_name="INTEGER", primary_key=True),
    ColumnDef(name="name", type_name="TEXT", not_null=True),
    ColumnDef(name="score", type_name="INTEGER"),
]


def _make_backend(tmp_path: Path) -> SqliteFileBackend:
    """Return a fresh SqliteFileBackend with a populated 'players' table."""
    b = SqliteFileBackend(str(tmp_path / "test.db"))
    b.create_table("players", _COLS, if_not_exists=False)
    h = b.begin_transaction()
    b.insert("players", {"id": 1, "name": "Alice", "score": 90})
    b.insert("players", {"id": 2, "name": "Bob", "score": 75})
    b.insert("players", {"id": 3, "name": "Carol", "score": 90})
    b.insert("players", {"id": 4, "name": "Dave", "score": None})
    b.commit(h)
    return b


# ---------------------------------------------------------------------------
# Schema-level index CRUD
# ---------------------------------------------------------------------------


class TestSchemaIndex:
    def test_create_and_find_index(self, tmp_path: Path) -> None:
        b = _make_backend(tmp_path)
        root = b._schema.create_index(  # noqa: SLF001
            "idx_score", "players", "CREATE INDEX idx_score ON players (score)"
        )
        assert isinstance(root, int) and root >= 2
        result = b._schema.find_index("idx_score")  # noqa: SLF001
        assert result is not None
        _, rp, sql = result
        assert rp == root
        assert sql is not None and "idx_score" in sql
        b.close()

    def test_create_duplicate_raises_schema_error(self, tmp_path: Path) -> None:
        b = _make_backend(tmp_path)
        b._schema.create_index("idx_score", "players", None)  # noqa: SLF001
        with pytest.raises(SchemaError, match="already exists"):
            b._schema.create_index("idx_score", "players", None)  # noqa: SLF001
        b.close()

    def test_drop_index(self, tmp_path: Path) -> None:
        b = _make_backend(tmp_path)
        b._schema.create_index("idx_score", "players", None)  # noqa: SLF001
        b._schema.drop_index("idx_score")  # noqa: SLF001
        assert b._schema.find_index("idx_score") is None  # noqa: SLF001
        b.close()

    def test_drop_nonexistent_raises(self, tmp_path: Path) -> None:
        b = _make_backend(tmp_path)
        with pytest.raises(SchemaError, match="does not exist"):
            b._schema.drop_index("no_such_index")  # noqa: SLF001
        b.close()

    def test_list_indexes_empty(self, tmp_path: Path) -> None:
        b = _make_backend(tmp_path)
        assert b._schema.list_indexes() == []  # noqa: SLF001
        b.close()

    def test_list_indexes_returns_row(self, tmp_path: Path) -> None:
        b = _make_backend(tmp_path)
        b._schema.create_index("idx_score", "players", "CREATE INDEX idx_score ON players (score)")  # noqa: SLF001
        rows = b._schema.list_indexes()  # noqa: SLF001
        assert len(rows) == 1
        name, tbl, rootpage, sql = rows[0]
        assert name == "idx_score"
        assert tbl == "players"
        assert rootpage >= 2
        assert sql is not None
        b.close()

    def test_list_indexes_filter_by_table(self, tmp_path: Path) -> None:
        b = _make_backend(tmp_path)
        b.create_table("other", [ColumnDef(name="x", type_name="INTEGER")], if_not_exists=False)
        b._schema.create_index("idx_score", "players", None)  # noqa: SLF001
        b._schema.create_index("idx_other", "other", None)  # noqa: SLF001
        players_idxs = b._schema.list_indexes("players")  # noqa: SLF001
        assert len(players_idxs) == 1
        assert players_idxs[0][0] == "idx_score"
        b.close()

    def test_find_index_returns_none_for_missing(self, tmp_path: Path) -> None:
        b = _make_backend(tmp_path)
        assert b._schema.find_index("no_such") is None  # noqa: SLF001
        b.close()

    def test_schema_cookie_bumped(self, tmp_path: Path) -> None:
        b = _make_backend(tmp_path)
        cookie_before = b._schema.get_schema_cookie()  # noqa: SLF001
        b._schema.create_index("idx_score", "players", None)  # noqa: SLF001
        assert b._schema.get_schema_cookie() == cookie_before + 1  # noqa: SLF001
        b._schema.drop_index("idx_score")  # noqa: SLF001
        assert b._schema.get_schema_cookie() == cookie_before + 2  # noqa: SLF001
        b.close()

    def test_null_sql_stored_and_retrieved(self, tmp_path: Path) -> None:
        b = _make_backend(tmp_path)
        b._schema.create_index("auto_players_score", "players", None)  # noqa: SLF001
        result = b._schema.find_index("auto_players_score")  # noqa: SLF001
        assert result is not None
        _, _, sql = result
        assert sql is None
        b.close()


# ---------------------------------------------------------------------------
# SqliteFileBackend.create_index
# ---------------------------------------------------------------------------


class TestBackendCreateIndex:
    def test_basic_create(self, tmp_path: Path) -> None:
        b = _make_backend(tmp_path)
        b.create_index(IndexDef(name="idx_score", table="players", columns=["score"]))
        assert len(b.list_indexes("players")) == 1
        b.close()

    def test_duplicate_raises(self, tmp_path: Path) -> None:
        b = _make_backend(tmp_path)
        b.create_index(IndexDef(name="idx_score", table="players", columns=["score"]))
        with pytest.raises(IndexAlreadyExists) as exc_info:
            b.create_index(IndexDef(name="idx_score", table="players", columns=["score"]))
        assert exc_info.value == IndexAlreadyExists(index="idx_score")
        b.close()

    def test_unknown_table_raises(self, tmp_path: Path) -> None:
        b = _make_backend(tmp_path)
        with pytest.raises(TableNotFound):
            b.create_index(IndexDef(name="idx", table="no_table", columns=["score"]))
        b.close()

    def test_unknown_column_raises(self, tmp_path: Path) -> None:
        b = _make_backend(tmp_path)
        with pytest.raises(ColumnNotFound) as exc_info:
            b.create_index(IndexDef(name="idx", table="players", columns=["no_col"]))
        assert exc_info.value == ColumnNotFound(table="players", column="no_col")
        b.close()

    def test_backfill_existing_rows(self, tmp_path: Path) -> None:
        """After create_index, existing rows must be findable via scan_index."""
        b = _make_backend(tmp_path)
        b.create_index(IndexDef(name="idx_score", table="players", columns=["score"]))
        # score=90 → rowids 1 (Alice) and 3 (Carol)
        rowids = list(b.scan_index("idx_score", [90], [90]))
        assert sorted(rowids) == [1, 3]
        b.close()

    def test_backfill_all_values(self, tmp_path: Path) -> None:
        b = _make_backend(tmp_path)
        b.create_index(IndexDef(name="idx_score", table="players", columns=["score"]))
        # Full scan should return all 4 rowids.
        rowids = list(b.scan_index("idx_score", None, None))
        assert sorted(rowids) == [1, 2, 3, 4]
        b.close()

    def test_auto_flag_preserved(self, tmp_path: Path) -> None:
        b = _make_backend(tmp_path)
        b.create_index(IndexDef(name="auto_players_score", table="players",
                                columns=["score"], auto=True))
        [idx] = b.list_indexes("players")
        assert idx.auto is True
        b.close()

    def test_non_auto_flag(self, tmp_path: Path) -> None:
        b = _make_backend(tmp_path)
        b.create_index(IndexDef(name="idx_score", table="players", columns=["score"]))
        [idx] = b.list_indexes("players")
        assert idx.auto is False
        b.close()


# ---------------------------------------------------------------------------
# SqliteFileBackend.scan_index
# ---------------------------------------------------------------------------


class TestBackendScanIndex:
    def _indexed_backend(self, tmp_path: Path) -> SqliteFileBackend:
        b = _make_backend(tmp_path)
        b.create_index(IndexDef(name="idx_score", table="players", columns=["score"]))
        return b

    def test_unknown_index_raises(self, tmp_path: Path) -> None:
        b = _make_backend(tmp_path)
        with pytest.raises(IndexNotFound):
            list(b.scan_index("no_index", None, None))
        b.close()

    def test_equality_lookup(self, tmp_path: Path) -> None:
        b = self._indexed_backend(tmp_path)
        rowids = list(b.scan_index("idx_score", [90], [90]))
        assert sorted(rowids) == [1, 3]
        b.close()

    def test_equality_no_match(self, tmp_path: Path) -> None:
        b = self._indexed_backend(tmp_path)
        rowids = list(b.scan_index("idx_score", [999], [999]))
        assert rowids == []
        b.close()

    def test_range_scan(self, tmp_path: Path) -> None:
        b = self._indexed_backend(tmp_path)
        # 75 ≤ score ≤ 90 → rowids 2, 1, 3
        rowids = list(b.scan_index("idx_score", [75], [90]))
        assert sorted(rowids) == [1, 2, 3]
        b.close()

    def test_range_exclusive_lo(self, tmp_path: Path) -> None:
        b = self._indexed_backend(tmp_path)
        # score > 75 → only 90-scoring rows
        rowids = list(b.scan_index("idx_score", [75], None, lo_inclusive=False))
        assert sorted(rowids) == [1, 3]
        b.close()

    def test_range_exclusive_hi(self, tmp_path: Path) -> None:
        b = self._indexed_backend(tmp_path)
        # score < 90 → only score=75 row
        rowids = list(b.scan_index("idx_score", [75], [90], hi_inclusive=False))
        assert rowids == [2]
        b.close()

    def test_null_scores_in_full_scan(self, tmp_path: Path) -> None:
        b = self._indexed_backend(tmp_path)
        # Unbounded scan includes the NULL-score row.
        rowids = list(b.scan_index("idx_score", None, None))
        assert 4 in rowids  # Dave has score=None
        b.close()

    def test_null_sorts_first(self, tmp_path: Path) -> None:
        b = self._indexed_backend(tmp_path)
        rowids = list(b.scan_index("idx_score", None, None))
        # Row 4 (score=None) should be the first yielded.
        assert rowids[0] == 4
        b.close()

    def test_ascending_key_order(self, tmp_path: Path) -> None:
        b = self._indexed_backend(tmp_path)
        rowids = list(b.scan_index("idx_score", None, None))
        # Retrieve scores in the scan order; they must be non-decreasing
        # (NULLs first, then ascending integers).
        it = b.scan("players")
        row_map = {}
        while (row := it.next()) is not None:
            row_map[row["id"]] = row["score"]
        it.close()
        scores = [row_map[r] for r in rowids]
        # None always before integers
        saw_non_null = False
        for s in scores:
            if s is not None:
                saw_non_null = True
            elif saw_non_null:
                pytest.fail("NULL appeared after non-NULL in scan_index output")
        non_null = [s for s in scores if s is not None]
        assert non_null == sorted(non_null)
        b.close()

    def test_text_column_scan(self, tmp_path: Path) -> None:
        b = _make_backend(tmp_path)
        b.create_index(IndexDef(name="idx_name", table="players", columns=["name"]))
        rowids = list(b.scan_index("idx_name", ["Bob"], ["Bob"]))
        assert rowids == [2]
        b.close()

    def test_unbounded_lo(self, tmp_path: Path) -> None:
        b = self._indexed_backend(tmp_path)
        # No lower bound, hi = 90 (inclusive)
        rowids = list(b.scan_index("idx_score", None, [90]))
        assert sorted(rowids) == [1, 2, 3, 4]
        b.close()


# ---------------------------------------------------------------------------
# SqliteFileBackend.drop_index
# ---------------------------------------------------------------------------


class TestBackendDropIndex:
    def test_basic_drop(self, tmp_path: Path) -> None:
        b = _make_backend(tmp_path)
        b.create_index(IndexDef(name="idx_score", table="players", columns=["score"]))
        b.drop_index("idx_score")
        assert b.list_indexes("players") == []
        b.close()

    def test_drop_missing_raises(self, tmp_path: Path) -> None:
        b = _make_backend(tmp_path)
        with pytest.raises(IndexNotFound) as exc_info:
            b.drop_index("no_index")
        assert exc_info.value == IndexNotFound(index="no_index")
        b.close()

    def test_drop_if_exists_no_error(self, tmp_path: Path) -> None:
        b = _make_backend(tmp_path)
        b.drop_index("no_index", if_exists=True)  # no exception
        b.close()

    def test_drop_removes_from_schema(self, tmp_path: Path) -> None:
        b = _make_backend(tmp_path)
        b.create_index(IndexDef(name="idx_score", table="players", columns=["score"]))
        b.drop_index("idx_score")
        assert b._schema.find_index("idx_score") is None  # noqa: SLF001
        b.close()

    def test_scan_after_drop_raises(self, tmp_path: Path) -> None:
        b = _make_backend(tmp_path)
        b.create_index(IndexDef(name="idx_score", table="players", columns=["score"]))
        b.drop_index("idx_score")
        with pytest.raises(IndexNotFound):
            list(b.scan_index("idx_score", None, None))
        b.close()


# ---------------------------------------------------------------------------
# SqliteFileBackend.list_indexes
# ---------------------------------------------------------------------------


class TestBackendListIndexes:
    def test_empty_initially(self, tmp_path: Path) -> None:
        b = _make_backend(tmp_path)
        assert b.list_indexes() == []
        b.close()

    def test_lists_all_indexes(self, tmp_path: Path) -> None:
        b = _make_backend(tmp_path)
        b.create_table("other", [ColumnDef(name="x", type_name="INTEGER")], if_not_exists=False)
        b.create_index(IndexDef(name="idx_score", table="players", columns=["score"]))
        b.create_index(IndexDef(name="idx_x", table="other", columns=["x"]))
        all_idxs = b.list_indexes()
        assert len(all_idxs) == 2
        assert {i.name for i in all_idxs} == {"idx_score", "idx_x"}
        b.close()

    def test_filter_by_table(self, tmp_path: Path) -> None:
        b = _make_backend(tmp_path)
        b.create_table("other", [ColumnDef(name="x", type_name="INTEGER")], if_not_exists=False)
        b.create_index(IndexDef(name="idx_score", table="players", columns=["score"]))
        b.create_index(IndexDef(name="idx_x", table="other", columns=["x"]))
        assert [i.name for i in b.list_indexes("players")] == ["idx_score"]
        assert [i.name for i in b.list_indexes("other")] == ["idx_x"]
        b.close()

    def test_columns_parsed_correctly(self, tmp_path: Path) -> None:
        b = _make_backend(tmp_path)
        b.create_index(IndexDef(name="idx_score", table="players", columns=["score"]))
        [idx] = b.list_indexes("players")
        assert idx.columns == ["score"]
        b.close()

    def test_auto_prefix_detected(self, tmp_path: Path) -> None:
        b = _make_backend(tmp_path)
        b.create_index(IndexDef(name="auto_players_score", table="players", columns=["score"]))
        [idx] = b.list_indexes("players")
        assert idx.auto is True
        b.close()

    def test_non_auto_index(self, tmp_path: Path) -> None:
        b = _make_backend(tmp_path)
        b.create_index(IndexDef(name="my_idx", table="players", columns=["score"]))
        [idx] = b.list_indexes("players")
        assert idx.auto is False
        b.close()


# ---------------------------------------------------------------------------
# Persistence: index survives backend close/reopen
# ---------------------------------------------------------------------------


class TestIndexPersistence:
    def test_index_survives_reopen(self, tmp_path: Path) -> None:
        path = str(tmp_path / "test.db")
        with SqliteFileBackend(path) as b:
            b.create_table("players", _COLS, if_not_exists=False)
            h = b.begin_transaction()
            b.insert("players", {"id": 1, "name": "Alice", "score": 90})
            b.commit(h)
            b.create_index(IndexDef(name="idx_score", table="players", columns=["score"]))

        # Reopen and verify the index still works.
        with SqliteFileBackend(path) as b2:
            assert len(b2.list_indexes("players")) == 1
            rowids = list(b2.scan_index("idx_score", [90], [90]))
            assert rowids == [1]

    def test_backfill_survives_reopen(self, tmp_path: Path) -> None:
        path = str(tmp_path / "test.db")
        with SqliteFileBackend(path) as b:
            b.create_table("players", _COLS, if_not_exists=False)
            h = b.begin_transaction()
            for i in range(1, 11):
                b.insert("players", {"id": i, "name": f"P{i}", "score": i * 10})
            b.commit(h)
            b.create_index(IndexDef(name="idx_score", table="players", columns=["score"]))

        with SqliteFileBackend(path) as b2:
            rowids = list(b2.scan_index("idx_score", [50], [70]))
            assert sorted(rowids) == [5, 6, 7]


# ---------------------------------------------------------------------------
# Oracle test: index visible to stdlib sqlite3
# ---------------------------------------------------------------------------


class TestOracleIndexVisible:
    def test_index_visible_to_sqlite3(self, tmp_path: Path) -> None:
        """An index created by SqliteFileBackend must appear in sqlite_schema
        when the file is opened by the stdlib sqlite3 module."""
        path = str(tmp_path / "oracle.db")
        b = _make_backend(tmp_path)
        # Re-route to the correct path for this test.
        b.close()
        b = SqliteFileBackend(path)
        b.create_table("players", _COLS, if_not_exists=False)
        h = b.begin_transaction()
        b.insert("players", {"id": 1, "name": "Alice", "score": 90})
        b.commit(h)
        b.create_index(IndexDef(name="idx_score", table="players", columns=["score"]))
        b.close()

        with sqlite3.connect(path) as conn:
            rows = conn.execute(
                "SELECT name, tbl_name FROM sqlite_schema WHERE type='index'"
            ).fetchall()
        names = [r[0] for r in rows]
        assert "idx_score" in names

    def test_sqlite3_index_readable_by_scan_index(self, tmp_path: Path) -> None:
        """An index created by stdlib sqlite3 must be readable by scan_index."""
        path = str(tmp_path / "reverse.db")

        # Build the database using stdlib sqlite3.
        with sqlite3.connect(path) as conn:
            conn.execute("CREATE TABLE players (id INTEGER PRIMARY KEY, score INTEGER)")
            conn.execute("CREATE INDEX idx_score ON players (score)")
            conn.execute("INSERT INTO players VALUES (1, 90)")
            conn.execute("INSERT INTO players VALUES (2, 75)")
            conn.execute("INSERT INTO players VALUES (3, 90)")

        # Read it back via SqliteFileBackend.
        with SqliteFileBackend(path) as b:
            # Index must be in list_indexes.
            idxs = b.list_indexes("players")
            assert any(i.name == "idx_score" for i in idxs)

            # scan_index must yield correct rowids.
            rowids = list(b.scan_index("idx_score", [90], [90]))
            assert sorted(rowids) == [1, 3]


# ---------------------------------------------------------------------------
# UNIQUE index enforcement
# ---------------------------------------------------------------------------


class TestUniqueIndex:
    """``IndexDef.unique=True`` is enforced at create_index and survives reopen."""

    def _setup_users(self, b: SqliteFileBackend) -> None:
        b.create_table(
            "users",
            [
                ColumnDef("id", "INTEGER", primary_key=True),
                ColumnDef("email", "TEXT"),
                ColumnDef("name", "TEXT"),
            ],
            if_not_exists=False,
        )

    def test_create_unique_index_writes_unique_keyword(self, tmp_path: Path) -> None:
        """list_indexes round-trips the unique flag."""
        path = str(tmp_path / "u1.db")
        with SqliteFileBackend(path) as b:
            self._setup_users(b)
            b.create_index(
                IndexDef(name="idx_email", table="users", columns=["email"], unique=True)
            )
            idxs = b.list_indexes("users")
            assert len(idxs) == 1
            assert idxs[0].unique is True
            assert idxs[0].name == "idx_email"

    def test_create_non_unique_index_default(self, tmp_path: Path) -> None:
        """Plain CREATE INDEX is non-unique."""
        path = str(tmp_path / "u2.db")
        with SqliteFileBackend(path) as b:
            self._setup_users(b)
            b.create_index(
                IndexDef(name="idx_name", table="users", columns=["name"])
            )
            idxs = b.list_indexes("users")
            assert idxs[0].unique is False

    def test_create_unique_rejects_existing_duplicates(self, tmp_path: Path) -> None:
        """Backfill of a UNIQUE index over duplicate data raises ConstraintViolation."""
        from sql_backend import ConstraintViolation
        path = str(tmp_path / "u3.db")
        with SqliteFileBackend(path) as b:
            self._setup_users(b)
            b.insert("users", {"id": 1, "email": "alice@example.com", "name": "Alice"})
            b.insert("users", {"id": 2, "email": "alice@example.com", "name": "Alice2"})
            with pytest.raises(ConstraintViolation, match="UNIQUE INDEX"):
                b.create_index(
                    IndexDef(
                        name="idx_email", table="users",
                        columns=["email"], unique=True,
                    )
                )
            # The schema must remain unchanged after the failure.
            assert b.list_indexes("users") == []

    def test_create_unique_succeeds_with_distinct_data(self, tmp_path: Path) -> None:
        """Backfill succeeds when existing rows have unique values."""
        path = str(tmp_path / "u4.db")
        with SqliteFileBackend(path) as b:
            self._setup_users(b)
            b.insert("users", {"id": 1, "email": "a@example.com", "name": "A"})
            b.insert("users", {"id": 2, "email": "b@example.com", "name": "B"})
            b.create_index(
                IndexDef(
                    name="idx_email", table="users",
                    columns=["email"], unique=True,
                )
            )
            rowids = list(b.scan_index("idx_email", ["a@example.com"], ["a@example.com"]))
            assert rowids == [1]

    def test_create_unique_allows_null_duplicates(self, tmp_path: Path) -> None:
        """Multiple rows with NULL in an indexed column do not conflict."""
        path = str(tmp_path / "u5.db")
        with SqliteFileBackend(path) as b:
            self._setup_users(b)
            b.insert("users", {"id": 1, "name": "alice"})  # email omitted → NULL
            b.insert("users", {"id": 2, "name": "bob"})    # email omitted → NULL
            # A UNIQUE index on email succeeds because NULLs are distinct.
            b.create_index(
                IndexDef(
                    name="idx_email", table="users",
                    columns=["email"], unique=True,
                )
            )
            assert b.list_indexes("users")[0].unique is True

    def test_unique_flag_survives_reopen(self, tmp_path: Path) -> None:
        """After close + reopen, list_indexes still reports the unique flag."""
        path = str(tmp_path / "u6.db")
        with SqliteFileBackend(path) as b:
            self._setup_users(b)
            b.create_index(
                IndexDef(
                    name="idx_email", table="users",
                    columns=["email"], unique=True,
                )
            )
        with SqliteFileBackend(path) as b2:
            idxs = b2.list_indexes("users")
            assert len(idxs) == 1
            assert idxs[0].unique is True

    def test_parse_index_unique_helper(self) -> None:
        """_parse_index_unique recognises CREATE UNIQUE INDEX (case-insensitive)."""
        from storage_sqlite.backend import _parse_index_unique
        assert _parse_index_unique('CREATE UNIQUE INDEX "x" ON "t" ("c")') is True
        assert _parse_index_unique('create unique index "x" ON "t" ("c")') is True
        assert _parse_index_unique('CREATE  UNIQUE  INDEX "x" ON "t" ("c")') is True
        assert _parse_index_unique('CREATE INDEX "x" ON "t" ("c")') is False
        assert _parse_index_unique(None) is False
        assert _parse_index_unique("") is False
