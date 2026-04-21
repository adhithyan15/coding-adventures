"""Tests for the index interface — IndexDef, IndexAlreadyExists,
IndexNotFound, and the four index methods on InMemoryBackend.

Coverage targets:
- IndexDef dataclass (equality, repr, defaults)
- IndexAlreadyExists / IndexNotFound error types
- InMemoryBackend.create_index — success, duplicate, bad table, bad column
- InMemoryBackend.drop_index — success, missing, if_exists
- InMemoryBackend.list_indexes — all, filtered by table
- InMemoryBackend.scan_index — equality, range, missing index, bounds
- Transaction rollback of create_index / drop_index
"""

from __future__ import annotations

import pytest

from sql_backend import (
    ColumnDef,
    IndexAlreadyExists,
    IndexDef,
    IndexNotFound,
    InMemoryBackend,
    TableNotFound,
)
from sql_backend.errors import ColumnNotFound

# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

_USERS_COLS = [
    ColumnDef(name="id", type_name="INTEGER", primary_key=True),
    ColumnDef(name="name", type_name="TEXT", not_null=True),
    ColumnDef(name="age", type_name="INTEGER"),
]

_ORDERS_COLS = [
    ColumnDef(name="id", type_name="INTEGER", primary_key=True),
    ColumnDef(name="user_id", type_name="INTEGER"),
    ColumnDef(name="total", type_name="REAL"),
]


def _backend_with_data() -> InMemoryBackend:
    """Return an InMemoryBackend with two tables and a handful of rows."""
    b = InMemoryBackend()
    b.create_table("users", _USERS_COLS, if_not_exists=False)
    b.create_table("orders", _ORDERS_COLS, if_not_exists=False)

    b.insert("users", {"id": 1, "name": "Alice", "age": 30})
    b.insert("users", {"id": 2, "name": "Bob", "age": 25})
    b.insert("users", {"id": 3, "name": "Carol", "age": 30})
    b.insert("users", {"id": 4, "name": "Dave", "age": None})

    b.insert("orders", {"id": 1, "user_id": 1, "total": 99.9})
    b.insert("orders", {"id": 2, "user_id": 2, "total": 49.5})
    b.insert("orders", {"id": 3, "user_id": 1, "total": 19.0})

    return b


# ---------------------------------------------------------------------------
# IndexDef dataclass
# ---------------------------------------------------------------------------


class TestIndexDef:
    def test_basic_construction(self) -> None:
        idx = IndexDef(name="idx_users_age", table="users", columns=["age"])
        assert idx.name == "idx_users_age"
        assert idx.table == "users"
        assert idx.columns == ["age"]
        assert idx.unique is False
        assert idx.auto is False

    def test_defaults(self) -> None:
        idx = IndexDef(name="x", table="t")
        assert idx.columns == []
        assert idx.unique is False
        assert idx.auto is False

    def test_equality(self) -> None:
        a = IndexDef(name="idx", table="t", columns=["c"], unique=False, auto=False)
        b = IndexDef(name="idx", table="t", columns=["c"], unique=False, auto=False)
        assert a == b

    def test_inequality_name(self) -> None:
        a = IndexDef(name="idx1", table="t", columns=["c"])
        b = IndexDef(name="idx2", table="t", columns=["c"])
        assert a != b

    def test_inequality_columns(self) -> None:
        a = IndexDef(name="idx", table="t", columns=["c1"])
        b = IndexDef(name="idx", table="t", columns=["c2"])
        assert a != b

    def test_auto_flag(self) -> None:
        idx = IndexDef(name="auto_users_age", table="users", columns=["age"], auto=True)
        assert idx.auto is True

    def test_unique_flag(self) -> None:
        idx = IndexDef(name="uq_users_name", table="users", columns=["name"], unique=True)
        assert idx.unique is True


# ---------------------------------------------------------------------------
# IndexAlreadyExists / IndexNotFound errors
# ---------------------------------------------------------------------------


class TestIndexErrors:
    def test_index_already_exists_is_backend_error(self) -> None:
        from sql_backend import BackendError

        err = IndexAlreadyExists(index="idx")
        assert isinstance(err, BackendError)

    def test_index_not_found_is_backend_error(self) -> None:
        from sql_backend import BackendError

        err = IndexNotFound(index="idx")
        assert isinstance(err, BackendError)

    def test_index_already_exists_str(self) -> None:
        assert str(IndexAlreadyExists(index="idx")) == "index already exists: 'idx'"

    def test_index_not_found_str(self) -> None:
        assert str(IndexNotFound(index="idx")) == "index not found: 'idx'"

    def test_index_already_exists_equality(self) -> None:
        assert IndexAlreadyExists(index="x") == IndexAlreadyExists(index="x")
        assert IndexAlreadyExists(index="x") != IndexAlreadyExists(index="y")

    def test_index_not_found_equality(self) -> None:
        assert IndexNotFound(index="x") == IndexNotFound(index="x")
        assert IndexNotFound(index="x") != IndexNotFound(index="y")


# ---------------------------------------------------------------------------
# create_index
# ---------------------------------------------------------------------------


class TestCreateIndex:
    def test_basic_create(self) -> None:
        b = _backend_with_data()
        b.create_index(IndexDef(name="idx_users_age", table="users", columns=["age"]))
        # Should now appear in list_indexes.
        idxs = b.list_indexes("users")
        assert len(idxs) == 1
        assert idxs[0].name == "idx_users_age"
        assert idxs[0].columns == ["age"]

    def test_duplicate_name_raises(self) -> None:
        b = _backend_with_data()
        b.create_index(IndexDef(name="idx_users_age", table="users", columns=["age"]))
        with pytest.raises(IndexAlreadyExists) as exc_info:
            b.create_index(IndexDef(name="idx_users_age", table="users", columns=["age"]))
        assert exc_info.value == IndexAlreadyExists(index="idx_users_age")

    def test_unknown_table_raises(self) -> None:
        b = _backend_with_data()
        with pytest.raises(TableNotFound):
            b.create_index(IndexDef(name="idx", table="no_such_table", columns=["x"]))

    def test_unknown_column_raises(self) -> None:
        b = _backend_with_data()
        with pytest.raises(ColumnNotFound) as exc_info:
            b.create_index(IndexDef(name="idx", table="users", columns=["no_col"]))
        assert exc_info.value == ColumnNotFound(table="users", column="no_col")

    def test_auto_flag_preserved(self) -> None:
        b = _backend_with_data()
        b.create_index(
            IndexDef(name="auto_users_age", table="users", columns=["age"], auto=True)
        )
        [idx] = b.list_indexes("users")
        assert idx.auto is True

    def test_unique_flag_preserved(self) -> None:
        b = _backend_with_data()
        b.create_index(
            IndexDef(name="uq_users_name", table="users", columns=["name"], unique=True)
        )
        [idx] = b.list_indexes("users")
        assert idx.unique is True


# ---------------------------------------------------------------------------
# drop_index
# ---------------------------------------------------------------------------


class TestDropIndex:
    def test_basic_drop(self) -> None:
        b = _backend_with_data()
        b.create_index(IndexDef(name="idx_users_age", table="users", columns=["age"]))
        b.drop_index("idx_users_age")
        assert b.list_indexes("users") == []

    def test_drop_missing_raises(self) -> None:
        b = _backend_with_data()
        with pytest.raises(IndexNotFound) as exc_info:
            b.drop_index("no_such_index")
        assert exc_info.value == IndexNotFound(index="no_such_index")

    def test_drop_if_exists_no_error(self) -> None:
        b = _backend_with_data()
        # Should not raise even though the index does not exist.
        b.drop_index("no_such_index", if_exists=True)

    def test_drop_if_exists_true_removes(self) -> None:
        b = _backend_with_data()
        b.create_index(IndexDef(name="idx_users_age", table="users", columns=["age"]))
        b.drop_index("idx_users_age", if_exists=True)
        assert b.list_indexes("users") == []

    def test_double_drop_raises(self) -> None:
        b = _backend_with_data()
        b.create_index(IndexDef(name="idx_users_age", table="users", columns=["age"]))
        b.drop_index("idx_users_age")
        with pytest.raises(IndexNotFound):
            b.drop_index("idx_users_age")


# ---------------------------------------------------------------------------
# list_indexes
# ---------------------------------------------------------------------------


class TestListIndexes:
    def test_empty_initially(self) -> None:
        b = _backend_with_data()
        assert b.list_indexes() == []

    def test_list_all(self) -> None:
        b = _backend_with_data()
        b.create_index(IndexDef(name="idx_users_age", table="users", columns=["age"]))
        b.create_index(IndexDef(name="idx_orders_user", table="orders", columns=["user_id"]))
        all_idxs = b.list_indexes()
        assert len(all_idxs) == 2
        names = [i.name for i in all_idxs]
        assert "idx_users_age" in names
        assert "idx_orders_user" in names

    def test_filter_by_table(self) -> None:
        b = _backend_with_data()
        b.create_index(IndexDef(name="idx_users_age", table="users", columns=["age"]))
        b.create_index(IndexDef(name="idx_orders_user", table="orders", columns=["user_id"]))
        assert [i.name for i in b.list_indexes("users")] == ["idx_users_age"]
        assert [i.name for i in b.list_indexes("orders")] == ["idx_orders_user"]

    def test_filter_returns_empty_for_no_match(self) -> None:
        b = _backend_with_data()
        b.create_index(IndexDef(name="idx_users_age", table="users", columns=["age"]))
        assert b.list_indexes("orders") == []

    def test_creation_order_preserved(self) -> None:
        b = _backend_with_data()
        for i in range(5):
            b.create_index(IndexDef(name=f"idx_{i}", table="users", columns=["age"]))
        names = [i.name for i in b.list_indexes("users")]
        assert names == [f"idx_{i}" for i in range(5)]


# ---------------------------------------------------------------------------
# scan_index
# ---------------------------------------------------------------------------


class TestScanIndex:
    def _make(self) -> InMemoryBackend:
        """Backend with an age index on users (rows age: 30, 25, 30, None)."""
        b = _backend_with_data()
        b.create_index(IndexDef(name="idx_age", table="users", columns=["age"]))
        return b

    def test_unknown_index_raises(self) -> None:
        b = _backend_with_data()
        with pytest.raises(IndexNotFound):
            list(b.scan_index("no_such_index", None, None))

    def test_full_scan_returns_all(self) -> None:
        b = self._make()
        rowids = list(b.scan_index("idx_age", None, None))
        # 4 rows — all returned (including the NULL-age row).
        assert len(rowids) == 4

    def test_null_sorts_first(self) -> None:
        """NULL < all numeric values — the NULL-age row must come first."""
        b = self._make()
        rowids = list(b.scan_index("idx_age", None, None))
        # Row at index 3 (age=None) should be first (NULL sorts lowest).
        first_row = b._tables["users"].rows[rowids[0]]  # noqa: SLF001
        assert first_row["age"] is None

    def test_equality_lookup(self) -> None:
        b = self._make()
        # age = 30 → should match rows 0 (Alice) and 2 (Carol).
        rowids = list(b.scan_index("idx_age", [30], [30]))
        assert len(rowids) == 2
        ages = [b._tables["users"].rows[r]["age"] for r in rowids]  # noqa: SLF001
        assert all(a == 30 for a in ages)

    def test_equality_no_match(self) -> None:
        b = self._make()
        rowids = list(b.scan_index("idx_age", [99], [99]))
        assert rowids == []

    def test_range_scan(self) -> None:
        b = self._make()
        # 25 ≤ age ≤ 30 → 3 rows (ages 25, 30, 30).
        rowids = list(b.scan_index("idx_age", [25], [30]))
        assert len(rowids) == 3

    def test_range_exclusive_lo(self) -> None:
        b = self._make()
        # age > 25 → only the two age=30 rows.
        rowids = list(b.scan_index("idx_age", [25], None, lo_inclusive=False))
        assert len(rowids) == 2

    def test_range_exclusive_hi(self) -> None:
        b = self._make()
        # age < 30 → age=None (sorts before 30) and age=25.
        # NULL sorts before all integers, so it is included in the range
        # [−∞, 30) alongside age=25.
        rowids = list(b.scan_index("idx_age", None, [30], hi_inclusive=False))
        ages = [b._tables["users"].rows[r]["age"] for r in rowids]  # noqa: SLF001
        # All returned ages must be NULL or < 30 (not equal to 30).
        assert all(a is None or a < 30 for a in ages)
        assert 30 not in ages

    def test_unbounded_lo(self) -> None:
        b = self._make()
        # No lower bound, hi = 30 → rows with age ≤ 30 (NULL, 25, 30, 30).
        rowids = list(b.scan_index("idx_age", None, [30]))
        assert len(rowids) == 4  # NULL sorts before 30

    def test_ascending_order(self) -> None:
        """Results must be in ascending key order."""
        b = self._make()
        rowids = list(b.scan_index("idx_age", None, None))
        ages = [b._tables["users"].rows[r]["age"] for r in rowids]  # noqa: SLF001
        # NULL (as None) is the smallest; compare sortable values.
        for i in range(len(ages) - 1):
            a, b_val = ages[i], ages[i + 1]
            if a is None:
                continue  # None < anything
            if b_val is None:
                pytest.fail("NULL should sort before non-NULL")
            assert a <= b_val

    def test_text_column_scan(self) -> None:
        b = _backend_with_data()
        b.create_index(IndexDef(name="idx_name", table="users", columns=["name"]))
        # name = "Bob" → exactly row 1.
        rowids = list(b.scan_index("idx_name", ["Bob"], ["Bob"]))
        assert len(rowids) == 1
        assert b._tables["users"].rows[rowids[0]]["name"] == "Bob"  # noqa: SLF001

    def test_text_ordering(self) -> None:
        """Text values must be ordered by UTF-8 byte comparison."""
        b = _backend_with_data()
        b.create_index(IndexDef(name="idx_name", table="users", columns=["name"]))
        rowids = list(b.scan_index("idx_name", None, None))
        names = [b._tables["users"].rows[r]["name"] for r in rowids]  # noqa: SLF001
        assert names == sorted(names)  # ASCII / UTF-8 order

    def test_orders_by_user_id(self) -> None:
        b = _backend_with_data()
        b.create_index(IndexDef(name="idx_uid", table="orders", columns=["user_id"]))
        rowids = list(b.scan_index("idx_uid", [1], [1]))
        assert len(rowids) == 2  # orders 1 and 3 both have user_id=1


# ---------------------------------------------------------------------------
# Transaction interaction
# ---------------------------------------------------------------------------


class TestIndexTransactions:
    def test_rollback_undoes_create(self) -> None:
        b = _backend_with_data()
        h = b.begin_transaction()
        b.create_index(IndexDef(name="idx_users_age", table="users", columns=["age"]))
        assert b.list_indexes("users") != []
        b.rollback(h)
        assert b.list_indexes("users") == []

    def test_rollback_undoes_drop(self) -> None:
        b = _backend_with_data()
        b.create_index(IndexDef(name="idx_users_age", table="users", columns=["age"]))
        h = b.begin_transaction()
        b.drop_index("idx_users_age")
        assert b.list_indexes("users") == []
        b.rollback(h)
        assert len(b.list_indexes("users")) == 1

    def test_commit_persists_create(self) -> None:
        b = _backend_with_data()
        h = b.begin_transaction()
        b.create_index(IndexDef(name="idx_users_age", table="users", columns=["age"]))
        b.commit(h)
        assert len(b.list_indexes("users")) == 1
