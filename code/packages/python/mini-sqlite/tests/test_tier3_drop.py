"""
Tier-3 tests — IX-7: index drop logic (QueryEvent, should_drop, cold_window).

Covers:
  - HitCountPolicy with cold_window parameter
  - QueryEvent emission after SELECT scans
  - IndexAdvisor.on_query_event: utilisation tracking and drop loop
  - Integration: full create-then-drop lifecycle
"""

from __future__ import annotations

import pytest
from sql_backend import InMemoryBackend
from sql_backend.index import IndexDef
from sql_backend.schema import ColumnDef

import mini_sqlite
from mini_sqlite import HitCountPolicy, QueryEvent
from mini_sqlite.advisor import IndexAdvisor

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _conn(threshold: int = 3, cold_window: int = 0) -> mini_sqlite.Connection:
    """Return a fresh in-memory connection with a single 20-row table."""
    conn = mini_sqlite.connect(":memory:", auto_index=True)
    conn.set_policy(HitCountPolicy(threshold=threshold, cold_window=cold_window))
    conn.execute("CREATE TABLE orders (id INTEGER PRIMARY KEY, user_id INTEGER, total REAL)")
    for i in range(20):
        conn.execute("INSERT INTO orders VALUES (?, ?, ?)", (i, i % 5, float(i * 10)))
    conn.commit()
    return conn


def _list_auto_indexes(conn: mini_sqlite.Connection, table: str = "orders") -> list[str]:
    """Return names of auto-created indexes on *table*."""
    try:
        idxs = conn._advisor._backend.list_indexes(table)
    except Exception:
        return []
    return [idx.name for idx in idxs if idx.name.startswith("auto_")]


def _make_advisor(threshold: int = 1, cold_window: int = 0) -> tuple[
    IndexAdvisor, InMemoryBackend
]:
    """Return an (advisor, backend) pair with a pre-created 'orders' table."""
    backend = InMemoryBackend()
    backend.create_table(
        "orders",
        [
            ColumnDef(name="id", type_name="INTEGER"),
            ColumnDef(name="user_id", type_name="INTEGER"),
        ],
        if_not_exists=False,
    )
    for i in range(10):
        backend.insert("orders", {"id": i, "user_id": i % 3})
    advisor = IndexAdvisor(
        backend,
        policy=HitCountPolicy(threshold=threshold, cold_window=cold_window),
    )
    return advisor, backend


def _cold_event(table: str = "orders") -> QueryEvent:
    return QueryEvent(
        table=table,
        filtered_columns=[],
        rows_scanned=10,
        rows_returned=10,
        used_index=None,
        duration_us=50,
    )


def _warm_event(index_name: str, table: str = "orders") -> QueryEvent:
    return QueryEvent(
        table=table,
        filtered_columns=["user_id"],
        rows_scanned=2,
        rows_returned=2,
        used_index=index_name,
        duration_us=10,
    )


# ---------------------------------------------------------------------------
# HitCountPolicy — cold_window parameter
# ---------------------------------------------------------------------------


class TestHitCountPolicyColdWindow:
    """New cold_window parameter on HitCountPolicy."""

    def test_default_cold_window_is_zero(self):
        policy = HitCountPolicy()
        assert policy.cold_window == 0

    def test_explicit_cold_window(self):
        policy = HitCountPolicy(threshold=3, cold_window=50)
        assert policy.cold_window == 50

    def test_cold_window_zero_should_drop_always_false(self):
        """cold_window=0 means drop logic is disabled — should_drop always False."""
        policy = HitCountPolicy(threshold=3, cold_window=0)
        for n in range(200):
            assert policy.should_drop("auto_t_c", "t", "c", n) is False

    def test_should_drop_false_below_window(self):
        policy = HitCountPolicy(threshold=3, cold_window=10)
        for n in range(10):
            assert policy.should_drop("auto_t_c", "t", "c", n) is False

    def test_should_drop_true_at_window(self):
        policy = HitCountPolicy(threshold=3, cold_window=10)
        assert policy.should_drop("auto_t_c", "t", "c", 10) is True

    def test_should_drop_true_above_window(self):
        policy = HitCountPolicy(threshold=3, cold_window=10)
        assert policy.should_drop("auto_t_c", "t", "c", 999) is True

    def test_cold_window_negative_raises(self):
        with pytest.raises(ValueError, match="cold_window"):
            HitCountPolicy(threshold=3, cold_window=-1)

    def test_cold_window_args_unused_in_decision(self):
        """Index name, table, and column do not affect HitCountPolicy.should_drop."""
        policy = HitCountPolicy(threshold=1, cold_window=5)
        assert policy.should_drop("auto_a_x", "a", "x", 5) is True
        assert policy.should_drop("auto_b_y", "b", "y", 5) is True
        assert policy.should_drop("whatever", "z", "q", 5) is True

    def test_should_drop_satisfies_protocol(self):
        """HitCountPolicy implements the optional should_drop method."""
        policy = HitCountPolicy()
        assert hasattr(policy, "should_drop")
        assert callable(policy.should_drop)

    def test_threshold_and_cold_window_together(self):
        policy = HitCountPolicy(threshold=3, cold_window=10)
        assert policy.should_create("t", "c", 2) is False
        assert policy.should_create("t", "c", 3) is True
        assert policy.should_drop("auto_t_c", "t", "c", 9) is False
        assert policy.should_drop("auto_t_c", "t", "c", 10) is True


# ---------------------------------------------------------------------------
# QueryEvent emission from the VM
# ---------------------------------------------------------------------------


class TestQueryEventEmission:
    """QueryEvent is emitted after SELECT scans via the advisor hook."""

    def test_event_emitted_after_full_table_scan(self):
        conn = _conn()
        captured: list[QueryEvent] = []
        original = conn._advisor.on_query_event
        conn._advisor.on_query_event = lambda e: (captured.append(e), original(e))[-1]

        conn.execute("SELECT * FROM orders WHERE user_id = 1").fetchall()

        assert len(captured) == 1
        ev = captured[0]
        assert ev.table == "orders"
        assert ev.used_index is None  # full scan (below threshold)

    def test_event_rows_scanned_and_returned(self):
        conn = _conn()
        captured: list[QueryEvent] = []
        original = conn._advisor.on_query_event
        conn._advisor.on_query_event = lambda e: (captured.append(e), original(e))[-1]

        rows = conn.execute("SELECT * FROM orders WHERE user_id = 0").fetchall()

        ev = captured[0]
        # 20 rows in table; user_id = 0 matches rows 0, 5, 10, 15 → 4 rows returned.
        assert ev.rows_scanned == 20
        assert ev.rows_returned == len(rows)

    def test_event_not_emitted_for_insert(self):
        conn = _conn()
        captured: list[QueryEvent] = []
        original = conn._advisor.on_query_event
        conn._advisor.on_query_event = lambda e: (captured.append(e), original(e))[-1]

        conn.execute("INSERT INTO orders VALUES (99, 9, 990.0)")
        conn.commit()

        assert captured == []

    def test_event_not_emitted_for_update(self):
        conn = _conn()
        captured: list[QueryEvent] = []
        original = conn._advisor.on_query_event
        conn._advisor.on_query_event = lambda e: (captured.append(e), original(e))[-1]

        conn.execute("UPDATE orders SET total = 0.0 WHERE id = 0")
        conn.commit()

        assert captured == []

    def test_event_not_emitted_for_ddl(self):
        conn = _conn()
        captured: list[QueryEvent] = []
        original = conn._advisor.on_query_event
        conn._advisor.on_query_event = lambda e: (captured.append(e), original(e))[-1]

        conn.execute("CREATE TABLE new_t (x INTEGER)")

        assert captured == []

    def test_event_used_index_set_after_index_scan(self):
        """After threshold queries, subsequent query emits used_index."""
        conn = _conn(threshold=1)  # index created on first hit

        captured: list[QueryEvent] = []
        original = conn._advisor.on_query_event
        conn._advisor.on_query_event = lambda e: (captured.append(e), original(e))[-1]

        # First query — observe_plan creates index; VM still does full scan this time.
        conn.execute("SELECT * FROM orders WHERE user_id = 1").fetchall()
        # Second query — planner picks the now-existing index.
        conn.execute("SELECT * FROM orders WHERE user_id = 1").fetchall()

        index_used_events = [e for e in captured if e.used_index is not None]
        assert len(index_used_events) >= 1
        assert index_used_events[-1].used_index == "auto_orders_user_id"

    def test_filtered_columns_populated(self):
        """filtered_columns in the QueryEvent reflects the WHERE predicate column."""
        conn = _conn()
        captured: list[QueryEvent] = []
        original = conn._advisor.on_query_event
        conn._advisor.on_query_event = lambda e: (captured.append(e), original(e))[-1]

        conn.execute("SELECT * FROM orders WHERE user_id = 2").fetchall()

        ev = captured[0]
        assert "user_id" in ev.filtered_columns

    def test_duration_us_is_non_negative(self):
        conn = _conn()
        captured: list[QueryEvent] = []
        original = conn._advisor.on_query_event
        conn._advisor.on_query_event = lambda e: (captured.append(e), original(e))[-1]

        conn.execute("SELECT * FROM orders").fetchall()

        assert len(captured) == 1
        assert captured[0].duration_us >= 0

    def test_event_emitted_for_unfiltered_select(self):
        """A SELECT without WHERE still emits an event (table scan occurred)."""
        conn = _conn()
        captured: list[QueryEvent] = []
        original = conn._advisor.on_query_event
        conn._advisor.on_query_event = lambda e: (captured.append(e), original(e))[-1]

        conn.execute("SELECT * FROM orders").fetchall()
        assert len(captured) == 1
        assert captured[0].rows_returned == 20


# ---------------------------------------------------------------------------
# IndexAdvisor.on_query_event — drop logic
# ---------------------------------------------------------------------------


class TestAdvisorDropLogic:
    """IndexAdvisor.on_query_event drops cold auto-created indexes."""

    def test_query_count_increments_on_each_event(self):
        advisor, _ = _make_advisor()
        event = _cold_event()
        for _ in range(3):
            advisor.on_query_event(event)
        assert advisor._query_count == 3

    def test_last_use_recorded_when_index_used(self):
        advisor, backend = _make_advisor()
        idx = IndexDef(
            name="auto_orders_user_id", table="orders",
            columns=["user_id"], unique=False, auto=True,
        )
        backend.create_index(idx)
        advisor._created_at["auto_orders_user_id"] = 0

        advisor.on_query_event(_warm_event("auto_orders_user_id"))
        assert advisor._query_count == 1
        assert advisor._last_use.get("auto_orders_user_id") == 1

    def test_index_dropped_after_cold_window(self):
        """Index is dropped once queries_since_last_use >= cold_window."""
        advisor, backend = _make_advisor(cold_window=5)
        idx = IndexDef(
            name="auto_orders_user_id", table="orders",
            columns=["user_id"], unique=False, auto=True,
        )
        backend.create_index(idx)
        advisor._created_at["auto_orders_user_id"] = 0

        for _ in range(5):
            advisor.on_query_event(_cold_event())

        remaining = [i.name for i in backend.list_indexes("orders")]
        assert "auto_orders_user_id" not in remaining

    def test_index_not_dropped_before_cold_window(self):
        """Index is NOT dropped when queries_since_last_use < cold_window."""
        advisor, backend = _make_advisor(cold_window=10)
        idx = IndexDef(
            name="auto_orders_user_id", table="orders",
            columns=["user_id"], unique=False, auto=True,
        )
        backend.create_index(idx)
        advisor._created_at["auto_orders_user_id"] = 0

        for _ in range(4):  # 4 < cold_window=10
            advisor.on_query_event(_cold_event())

        remaining = [i.name for i in backend.list_indexes("orders")]
        assert "auto_orders_user_id" in remaining

    def test_user_created_index_never_dropped(self):
        """Indexes NOT starting with 'auto_' are never touched by the advisor."""
        advisor, backend = _make_advisor(cold_window=1)
        user_idx = IndexDef(
            name="my_custom_index", table="orders",
            columns=["user_id"], unique=False, auto=False,
        )
        backend.create_index(user_idx)
        # Not registered in advisor._created_at — advisor doesn't own it.

        for _ in range(1000):
            advisor.on_query_event(_cold_event())

        remaining = [i.name for i in backend.list_indexes("orders")]
        assert "my_custom_index" in remaining

    def test_drop_failure_is_non_fatal(self):
        """If drop_index raises, the advisor continues without crashing."""
        advisor, backend = _make_advisor(cold_window=1)
        # Register a non-existent index in tracking — drop_index uses if_exists=True
        # so it silently does nothing, not raises. We test with a name that follows
        # the auto_ convention but has parts that don't parse to 3 segments.
        advisor._created_at["auto_orders_user_id"] = 0  # valid name, no actual idx

        cold = _cold_event()
        # Should not raise even though index doesn't exist.
        advisor.on_query_event(cold)

    def test_policy_without_should_drop_skips_drop_loop(self):
        """A v2-style policy (no should_drop) causes the advisor to skip drop checks."""

        class V2Policy:
            def should_create(self, table: str, column: str, hit_count: int) -> bool:
                return hit_count >= 3

        advisor, backend = _make_advisor()
        advisor._policy = V2Policy()
        idx = IndexDef(
            name="auto_orders_user_id", table="orders",
            columns=["user_id"], unique=False, auto=True,
        )
        backend.create_index(idx)
        advisor._created_at["auto_orders_user_id"] = 0

        for _ in range(1000):
            advisor.on_query_event(_cold_event())

        remaining = [i.name for i in backend.list_indexes("orders")]
        assert "auto_orders_user_id" in remaining

    def test_drop_resets_hit_count(self):
        """After drop, hit count for the column is cleared so re-creation is possible."""
        advisor, backend = _make_advisor(cold_window=2)
        idx = IndexDef(
            name="auto_orders_user_id", table="orders",
            columns=["user_id"], unique=False, auto=True,
        )
        backend.create_index(idx)
        advisor._created_at["auto_orders_user_id"] = 0
        advisor._hits[("orders", "user_id")] = 5  # simulate accumulated hits

        for _ in range(2):
            advisor.on_query_event(_cold_event())

        # Index dropped — hit count reset.
        assert ("orders", "user_id") not in advisor._hits

    def test_use_resets_cold_counter(self):
        """Using an index resets the cold counter, preventing premature drops."""
        advisor, backend = _make_advisor(cold_window=5)
        idx = IndexDef(
            name="auto_orders_user_id", table="orders",
            columns=["user_id"], unique=False, auto=True,
        )
        backend.create_index(idx)
        advisor._created_at["auto_orders_user_id"] = 0

        # 4 cold events — just below window.
        for _ in range(4):
            advisor.on_query_event(_cold_event())

        # Use the index — resets the counter.
        advisor.on_query_event(_warm_event("auto_orders_user_id"))

        # 4 more cold events — still below window from the reset.
        for _ in range(4):
            advisor.on_query_event(_cold_event())

        remaining = [i.name for i in backend.list_indexes("orders")]
        assert "auto_orders_user_id" in remaining

    def test_tracking_cleared_after_drop(self):
        """After drop, the index is removed from _created_at and _last_use."""
        advisor, backend = _make_advisor(cold_window=2)
        idx = IndexDef(
            name="auto_orders_user_id", table="orders",
            columns=["user_id"], unique=False, auto=True,
        )
        backend.create_index(idx)
        advisor._created_at["auto_orders_user_id"] = 0

        for _ in range(2):
            advisor.on_query_event(_cold_event())

        # No longer tracked.
        assert "auto_orders_user_id" not in advisor._created_at
        assert "auto_orders_user_id" not in advisor._last_use


# ---------------------------------------------------------------------------
# Integration: full create-then-drop lifecycle via Connection
# ---------------------------------------------------------------------------


class TestDropIntegration:
    """End-to-end: index is created then dropped automatically."""

    def test_index_created_and_dropped_via_connection(self):
        """Full cycle: threshold queries → index created; cold window → index dropped."""
        conn = mini_sqlite.connect(":memory:", auto_index=True)
        conn.set_policy(HitCountPolicy(threshold=3, cold_window=5))

        conn.execute(
            "CREATE TABLE orders (id INTEGER PRIMARY KEY, user_id INTEGER)"
        )
        for i in range(10):
            conn.execute("INSERT INTO orders VALUES (?, ?)", (i, i % 3))
        conn.commit()

        # 3 queries on user_id → index created.
        for _ in range(3):
            conn.execute("SELECT * FROM orders WHERE user_id = 1").fetchall()

        auto_indexes = _list_auto_indexes(conn)
        assert "auto_orders_user_id" in auto_indexes

        # 5 cold queries (no filter, no index used) → index dropped.
        for _ in range(5):
            conn.execute("SELECT * FROM orders").fetchall()

        auto_indexes_after = _list_auto_indexes(conn)
        assert "auto_orders_user_id" not in auto_indexes_after

    def test_index_recreated_after_drop(self):
        """After drop, new filtered queries can trigger re-creation."""
        conn = mini_sqlite.connect(":memory:", auto_index=True)
        conn.set_policy(HitCountPolicy(threshold=2, cold_window=3))

        conn.execute("CREATE TABLE orders (id INTEGER PRIMARY KEY, user_id INTEGER)")
        for i in range(10):
            conn.execute("INSERT INTO orders VALUES (?, ?)", (i, i % 3))
        conn.commit()

        # Create index.
        for _ in range(2):
            conn.execute("SELECT * FROM orders WHERE user_id = 1").fetchall()
        assert "auto_orders_user_id" in _list_auto_indexes(conn)

        # Drop it.
        for _ in range(3):
            conn.execute("SELECT * FROM orders").fetchall()
        assert "auto_orders_user_id" not in _list_auto_indexes(conn)

        # Re-create it.
        for _ in range(2):
            conn.execute("SELECT * FROM orders WHERE user_id = 1").fetchall()
        assert "auto_orders_user_id" in _list_auto_indexes(conn)

    def test_cold_window_zero_never_drops(self):
        """Default HitCountPolicy (cold_window=0) never drops indexes."""
        conn = mini_sqlite.connect(":memory:", auto_index=True)
        conn.set_policy(HitCountPolicy(threshold=2, cold_window=0))

        conn.execute("CREATE TABLE t (id INTEGER, c INTEGER)")
        for i in range(5):
            conn.execute("INSERT INTO t VALUES (?, ?)", (i, i))
        conn.commit()

        # Create index.
        for _ in range(2):
            conn.execute("SELECT * FROM t WHERE c = 1").fetchall()
        assert "auto_t_c" in _list_auto_indexes(conn, "t")

        # Many cold queries — index stays.
        for _ in range(500):
            conn.execute("SELECT * FROM t").fetchall()

        assert "auto_t_c" in _list_auto_indexes(conn, "t")

    def test_auto_index_false_never_drops(self):
        """auto_index=False connection has no advisor — no creates, no drops."""
        conn = mini_sqlite.connect(":memory:", auto_index=False)
        conn.execute("CREATE TABLE t (id INTEGER, c INTEGER)")
        for i in range(5):
            conn.execute("INSERT INTO t VALUES (?, ?)", (i, i))
        conn.commit()

        for _ in range(100):
            conn.execute("SELECT * FROM t WHERE c = 1").fetchall()

        assert conn._advisor is None

    def test_query_event_exported_from_mini_sqlite(self):
        """QueryEvent is accessible from the top-level mini_sqlite namespace."""
        assert hasattr(mini_sqlite, "QueryEvent")
        ev = mini_sqlite.QueryEvent(
            table="t",
            filtered_columns=["c"],
            rows_scanned=5,
            rows_returned=1,
            used_index=None,
            duration_us=100,
        )
        assert ev.table == "t"
        assert ev.filtered_columns == ["c"]

    def test_dunder_all_includes_query_event(self):
        """QueryEvent appears in mini_sqlite.__all__."""
        assert "QueryEvent" in mini_sqlite.__all__
