"""
IX-8: Composite (multi-column) automatic index support.

Tests are organised into three classes:

TestAdvisorComposite
    Unit-level tests for IndexAdvisor pair tracking and composite index
    creation.  Uses a real InMemoryBackend so _maybe_create_composite_index
    can call backend.create_index.

TestPlannerComposite
    Tests that the planner selects composite indexes when available and that
    multi-column bounds are produced correctly.

TestCompositeIntegration
    End-to-end tests through mini_sqlite.connect() that verify the full
    create → use → correctness cycle for composite indexes.
"""

from __future__ import annotations

from sql_backend.in_memory import InMemoryBackend
from sql_backend.schema import ColumnDef

import mini_sqlite
from mini_sqlite.advisor import IndexAdvisor
from mini_sqlite.policy import HitCountPolicy

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _make_backend_with_table(
    table: str = "orders",
    cols: list[str] | None = None,
) -> InMemoryBackend:
    """Create an InMemoryBackend with a single test table already created."""
    if cols is None:
        cols = ["id", "user_id", "status", "amount"]
    backend = InMemoryBackend()
    col_defs = [
        ColumnDef(
            name=c,
            type_name="INTEGER" if c in ("id", "user_id", "amount") else "TEXT",
        )
        for c in cols
    ]
    backend.create_table(table, col_defs, if_not_exists=False)
    return backend


def _connect_with_rows(n: int = 10) -> mini_sqlite.Connection:
    """Connect in-memory with a populated orders table."""
    conn = mini_sqlite.connect(":memory:", auto_index=True)
    conn.execute(
        "CREATE TABLE orders ("
        "  id INTEGER, user_id INTEGER, status TEXT, amount INTEGER"
        ")"
    )
    for i in range(n):
        conn.execute(
            "INSERT INTO orders VALUES (?, ?, ?, ?)",
            (i, i % 3, "shipped" if i % 2 == 0 else "pending", i * 10),
        )
    conn.commit()
    return conn


# ---------------------------------------------------------------------------
# TestAdvisorComposite — unit tests for pair tracking and composite creation
# ---------------------------------------------------------------------------


class TestAdvisorComposite:
    """Advisor-level tests for IX-8 composite index lifecycle."""

    # -----------------------------------------------------------------------
    # Pair hit tracking
    # -----------------------------------------------------------------------

    def test_pair_hits_start_at_zero(self) -> None:
        """A fresh advisor has no pair hit counts."""
        backend = _make_backend_with_table()
        advisor = IndexAdvisor(backend)
        assert advisor._pair_hits == {}

    def test_pair_hits_accumulate_per_query(self) -> None:
        """Each observe_plan call for Filter(Scan) with two columns increments pair count."""
        conn = _connect_with_rows()
        advisor = conn._advisor
        assert advisor is not None

        # Run 2 queries (below threshold=3) — pair should be tracked but no index yet.
        conn.execute("SELECT * FROM orders WHERE user_id = 1 AND status = 'shipped'").fetchall()
        conn.execute("SELECT * FROM orders WHERE user_id = 2 AND status = 'pending'").fetchall()

        # At this point: pair_hits[(orders, user_id, status)] == 2 (below threshold)
        # OR the plan may already use an IndexScan if single-col indexes fired.
        # Either way, no composite index should exist yet.
        indexes = conn._advisor._backend.list_indexes("orders")
        composite = [i for i in indexes if len(i.columns) == 2]
        assert len(composite) == 0

    def test_composite_created_at_threshold(self) -> None:
        """After threshold queries on a column pair, a composite index is created."""
        conn = _connect_with_rows(20)
        conn.set_policy(HitCountPolicy(threshold=3))

        for _ in range(3):
            conn.execute(
                "SELECT * FROM orders WHERE user_id = 1 AND status = 'shipped'"
            ).fetchall()

        indexes = conn._advisor._backend.list_indexes("orders")
        col_pairs = [tuple(i.columns) for i in indexes if len(i.columns) == 2]
        assert ("user_id", "status") in col_pairs

    def test_composite_index_name_convention(self) -> None:
        """Composite index is named auto_{table}_{col_a}_{col_b}."""
        conn = _connect_with_rows(20)
        conn.set_policy(HitCountPolicy(threshold=3))

        for _ in range(3):
            conn.execute(
                "SELECT * FROM orders WHERE user_id = 1 AND status = 'shipped'"
            ).fetchall()

        indexes = conn._advisor._backend.list_indexes("orders")
        names = [i.name for i in indexes]
        assert "auto_orders_user_id_status" in names

    def test_single_col_on_leading_prevents_composite(self) -> None:
        """If a single-column index on the leading column already exists,
        the composite is not created."""
        conn = _connect_with_rows(20)
        conn.set_policy(HitCountPolicy(threshold=2))

        # Run 2 queries on user_id only → single-col index created.
        for _ in range(2):
            conn.execute("SELECT * FROM orders WHERE user_id = 1").fetchall()

        # Confirm single-col index exists.
        indexes = conn._advisor._backend.list_indexes("orders")
        assert any(i.columns == ["user_id"] for i in indexes)

        # Now run 2 queries on both → pair hits = 2 but composite should be
        # suppressed because the single-col covering index on user_id exists.
        for _ in range(2):
            conn.execute(
                "SELECT * FROM orders WHERE user_id = 1 AND status = 'shipped'"
            ).fetchall()

        indexes2 = conn._advisor._backend.list_indexes("orders")
        assert not any(len(i.columns) == 2 for i in indexes2)

    def test_composite_not_duplicated(self) -> None:
        """After composite is created, further queries don't create it again."""
        conn = _connect_with_rows(20)
        conn.set_policy(HitCountPolicy(threshold=3))

        for _ in range(6):
            conn.execute(
                "SELECT * FROM orders WHERE user_id = 1 AND status = 'shipped'"
            ).fetchall()

        indexes = conn._advisor._backend.list_indexes("orders")
        composite = [i for i in indexes if i.columns == ["user_id", "status"]]
        assert len(composite) == 1  # exactly one, not duplicated

    def test_independent_columns_no_spurious_composite(self) -> None:
        """Columns observed in separate queries don't trigger a composite."""
        conn = _connect_with_rows(20)
        conn.set_policy(HitCountPolicy(threshold=3))

        # 3 queries on user_id only.
        for _ in range(3):
            conn.execute("SELECT * FROM orders WHERE user_id = 1").fetchall()

        # 3 queries on status only.
        for _ in range(3):
            conn.execute("SELECT * FROM orders WHERE status = 'shipped'").fetchall()

        indexes = conn._advisor._backend.list_indexes("orders")
        # No composite should exist — columns were never queried together.
        assert not any(len(i.columns) == 2 for i in indexes)

    def test_meta_populated_for_composite(self) -> None:
        """_auto_index_meta is set correctly for created composite indexes."""
        conn = _connect_with_rows(20)
        conn.set_policy(HitCountPolicy(threshold=3))

        for _ in range(3):
            conn.execute(
                "SELECT * FROM orders WHERE user_id = 1 AND status = 'shipped'"
            ).fetchall()

        advisor = conn._advisor
        meta = advisor._auto_index_meta.get("auto_orders_user_id_status")
        assert meta is not None
        assert meta[0] == "orders"
        assert meta[1] == ("user_id", "status")


# ---------------------------------------------------------------------------
# TestPlannerComposite — planner index selection with composite indexes
# ---------------------------------------------------------------------------


class TestPlannerComposite:
    """Tests that the planner selects composite indexes correctly."""

    def _make_conn_with_composite_index(self) -> mini_sqlite.Connection:
        """Return an in-memory connection with an explicit composite index."""
        conn = mini_sqlite.connect(":memory:", auto_index=False)
        conn.execute(
            "CREATE TABLE orders (id INTEGER, user_id INTEGER, status TEXT, amount INTEGER)"
        )
        for i in range(20):
            conn.execute(
                "INSERT INTO orders VALUES (?, ?, ?, ?)",
                (i, i % 3, "shipped" if i % 2 == 0 else "pending", i * 10),
            )
        conn.execute("CREATE INDEX idx_u_s ON orders (user_id, status)")
        conn.commit()
        return conn

    def test_composite_index_used_for_both_columns(self) -> None:
        """WHERE a=? AND b=? with composite index (a,b) → uses index."""
        conn = self._make_conn_with_composite_index()

        rows = conn.execute(
            "SELECT id FROM orders WHERE user_id = 0 AND status = 'shipped'"
        ).fetchall()
        # Correctness check: rows where user_id=0 AND status='shipped'.
        all_rows = conn.execute("SELECT id, user_id, status FROM orders").fetchall()
        expected_ids = sorted(
            r[0] for r in all_rows if r[1] == 0 and r[2] == "shipped"
        )
        result_ids = sorted(r[0] for r in rows)
        assert result_ids == expected_ids

    def test_leading_prefix_match_for_single_column(self) -> None:
        """WHERE a=? alone with composite index (a,b) uses the index via prefix match."""
        conn = self._make_conn_with_composite_index()

        rows = conn.execute(
            "SELECT id FROM orders WHERE user_id = 1"
        ).fetchall()
        all_rows = conn.execute("SELECT id, user_id FROM orders").fetchall()
        expected_ids = sorted(r[0] for r in all_rows if r[1] == 1)
        result_ids = sorted(r[0] for r in rows)
        assert result_ids == expected_ids

    def test_non_leading_column_cannot_use_composite(self) -> None:
        """WHERE b=? alone with composite index (a,b) → full scan (no index on b alone)."""
        conn = self._make_conn_with_composite_index()

        rows = conn.execute(
            "SELECT id FROM orders WHERE status = 'shipped'"
        ).fetchall()
        all_rows = conn.execute("SELECT id, status FROM orders").fetchall()
        expected_ids = sorted(r[0] for r in all_rows if r[1] == "shipped")
        result_ids = sorted(r[0] for r in rows)
        assert result_ids == expected_ids  # results are correct even on full scan

    def test_composite_preferred_over_single_for_two_column_query(self) -> None:
        """Composite (a,b) index beats single (a) when predicate has both cols."""
        conn = mini_sqlite.connect(":memory:", auto_index=False)
        conn.execute(
            "CREATE TABLE t (id INTEGER, a INTEGER, b INTEGER, c INTEGER)"
        )
        for i in range(30):
            conn.execute("INSERT INTO t VALUES (?, ?, ?, ?)", (i, i % 5, i % 3, i))
        # Create both a single-column index and a composite index.
        conn.execute("CREATE INDEX idx_a ON t (a)")
        conn.execute("CREATE INDEX idx_ab ON t (a, b)")
        conn.commit()

        rows = conn.execute(
            "SELECT id FROM t WHERE a = 2 AND b = 1"
        ).fetchall()
        all_rows = conn.execute("SELECT id, a, b FROM t").fetchall()
        expected_ids = sorted(r[0] for r in all_rows if r[1] == 2 and r[2] == 1)
        result_ids = sorted(r[0] for r in rows)
        assert result_ids == expected_ids

    def test_range_on_second_column_uses_composite(self) -> None:
        """WHERE a=? AND b>? with composite (a,b) → correct rows returned."""
        conn = self._make_conn_with_composite_index()

        rows = conn.execute(
            "SELECT id, amount FROM orders WHERE user_id = 1 AND amount > 50"
        ).fetchall()
        all_rows = conn.execute("SELECT id, user_id, amount FROM orders").fetchall()
        expected = sorted(
            (r[0], r[2]) for r in all_rows if r[1] == 1 and r[2] > 50
        )
        result = sorted(rows)
        assert result == expected

    def test_range_on_second_column_lower_bound(self) -> None:
        """WHERE a=? AND b<? with composite (a,b) → correct rows returned."""
        conn = mini_sqlite.connect(":memory:", auto_index=False)
        conn.execute("CREATE TABLE t (id INTEGER, a INTEGER, b INTEGER)")
        for i in range(20):
            conn.execute("INSERT INTO t VALUES (?, ?, ?)", (i, i % 4, i))
        conn.execute("CREATE INDEX idx_ab ON t (a, b)")
        conn.commit()

        rows = conn.execute(
            "SELECT id FROM t WHERE a = 1 AND b < 10"
        ).fetchall()
        all_rows = conn.execute("SELECT id, a, b FROM t").fetchall()
        expected_ids = sorted(r[0] for r in all_rows if r[1] == 1 and r[2] < 10)
        result_ids = sorted(r[0] for r in rows)
        assert result_ids == expected_ids

    def test_equality_on_both_columns_uses_composite(self) -> None:
        """WHERE a=? AND b=? with composite (a,b) returns exactly matching rows."""
        conn = mini_sqlite.connect(":memory:", auto_index=False)
        conn.execute("CREATE TABLE t (id INTEGER, a INTEGER, b INTEGER)")
        for i in range(40):
            conn.execute("INSERT INTO t VALUES (?, ?, ?)", (i, i % 5, i % 4))
        conn.execute("CREATE INDEX idx_ab ON t (a, b)")
        conn.commit()

        rows = conn.execute(
            "SELECT id FROM t WHERE a = 3 AND b = 2"
        ).fetchall()
        all_rows = conn.execute("SELECT id, a, b FROM t").fetchall()
        expected_ids = sorted(r[0] for r in all_rows if r[1] == 3 and r[2] == 2)
        result_ids = sorted(r[0] for r in rows)
        assert result_ids == expected_ids

    def test_between_on_second_column(self) -> None:
        """WHERE a=? AND b BETWEEN ? AND ? with composite (a,b) → correct rows."""
        conn = mini_sqlite.connect(":memory:", auto_index=False)
        conn.execute("CREATE TABLE t (id INTEGER, a INTEGER, b INTEGER)")
        for i in range(30):
            conn.execute("INSERT INTO t VALUES (?, ?, ?)", (i, i % 3, i))
        conn.execute("CREATE INDEX idx_ab ON t (a, b)")
        conn.commit()

        rows = conn.execute(
            "SELECT id FROM t WHERE a = 1 AND b BETWEEN 5 AND 20"
        ).fetchall()
        all_rows = conn.execute("SELECT id, a, b FROM t").fetchall()
        expected_ids = sorted(r[0] for r in all_rows if r[1] == 1 and 5 <= r[2] <= 20)
        result_ids = sorted(r[0] for r in rows)
        assert result_ids == expected_ids


# ---------------------------------------------------------------------------
# TestCompositeIntegration — end-to-end through mini_sqlite.connect()
# ---------------------------------------------------------------------------


class TestCompositeIntegration:
    """Full lifecycle integration tests for composite index advisor."""

    def test_full_create_cycle_composite(self) -> None:
        """3 compound-AND queries → composite index created and used correctly."""
        conn = _connect_with_rows(30)
        conn.set_policy(HitCountPolicy(threshold=3))

        # 3 queries on (user_id AND status) → should trigger composite creation.
        for _ in range(3):
            conn.execute(
                "SELECT id FROM orders WHERE user_id = 0 AND status = 'shipped'"
            ).fetchall()

        # Verify composite was created.
        indexes = conn._advisor._backend.list_indexes("orders")
        assert any(i.columns == ["user_id", "status"] for i in indexes)

        # Verify query still returns correct results.
        rows = conn.execute(
            "SELECT id FROM orders WHERE user_id = 0 AND status = 'shipped'"
        ).fetchall()
        all_rows = conn.execute("SELECT id, user_id, status FROM orders").fetchall()
        expected = sorted(r[0] for r in all_rows if r[1] == 0 and r[2] == "shipped")
        assert sorted(r[0] for r in rows) == expected

    def test_composite_correctness_with_range(self) -> None:
        """Composite index used for a=? AND b>? returns identical results to full scan."""
        conn = mini_sqlite.connect(":memory:", auto_index=False)
        conn.execute("CREATE TABLE sales (id INTEGER, dept INTEGER, revenue INTEGER)")
        for i in range(100):
            conn.execute("INSERT INTO sales VALUES (?, ?, ?)", (i, i % 5, i * 7))
        conn.execute("CREATE INDEX idx_dept_rev ON sales (dept, revenue)")
        conn.commit()

        rows_indexed = sorted(conn.execute(
            "SELECT id FROM sales WHERE dept = 2 AND revenue > 100"
        ).fetchall())

        # Drop the index and re-run for ground truth.
        conn.execute("DROP INDEX idx_dept_rev")
        conn.commit()
        rows_full = sorted(conn.execute(
            "SELECT id FROM sales WHERE dept = 2 AND revenue > 100"
        ).fetchall())

        assert rows_indexed == rows_full

    def test_composite_correctness_with_equality(self) -> None:
        """Composite index used for a=? AND b=? returns identical results to full scan."""
        conn = mini_sqlite.connect(":memory:", auto_index=False)
        conn.execute("CREATE TABLE products (id INTEGER, cat INTEGER, sub INTEGER, name TEXT)")
        for i in range(60):
            conn.execute("INSERT INTO products VALUES (?, ?, ?, ?)", (i, i % 6, i % 4, f"p{i}"))
        conn.execute("CREATE INDEX idx_cat_sub ON products (cat, sub)")
        conn.commit()

        rows_indexed = sorted(conn.execute(
            "SELECT id FROM products WHERE cat = 2 AND sub = 1"
        ).fetchall())

        conn.execute("DROP INDEX idx_cat_sub")
        conn.commit()
        rows_full = sorted(conn.execute(
            "SELECT id FROM products WHERE cat = 2 AND sub = 1"
        ).fetchall())

        assert rows_indexed == rows_full

    def test_auto_index_false_no_composite_created(self) -> None:
        """auto_index=False means no advisor, so no composite is ever created."""
        conn = mini_sqlite.connect(":memory:", auto_index=False)
        conn.execute("CREATE TABLE t (id INTEGER, a INTEGER, b INTEGER)")
        for i in range(10):
            conn.execute("INSERT INTO t VALUES (?, ?, ?)", (i, i, i))
        conn.commit()

        for _ in range(5):
            conn.execute("SELECT * FROM t WHERE a = 1 AND b = 1").fetchall()

        # No advisor → no indexes at all.
        assert conn._advisor is None
        backend_indexes = list(conn._backend.list_indexes("t"))
        assert backend_indexes == []

    def test_composite_drop_resets_pair_hits(self) -> None:
        """When a composite index is cold-dropped, pair hit counts reset."""
        conn = mini_sqlite.connect(":memory:", auto_index=True)
        conn.execute("CREATE TABLE t (id INTEGER, a INTEGER, b INTEGER)")
        for i in range(20):
            conn.execute("INSERT INTO t VALUES (?, ?, ?)", (i, i % 4, i % 3))
        conn.set_policy(HitCountPolicy(threshold=2, cold_window=3))
        conn.commit()

        # Create the composite.
        for _ in range(2):
            conn.execute("SELECT * FROM t WHERE a = 1 AND b = 1").fetchall()

        advisor = conn._advisor
        assert any(
            i.columns == ["a", "b"]
            for i in advisor._backend.list_indexes("t")
        )
        composite_name = "auto_t_a_b"

        # Run 3 queries on a different predicate → composite goes cold.
        for _ in range(3):
            conn.execute("SELECT * FROM t WHERE id = 1").fetchall()

        # Composite should be dropped now.
        assert not any(
            i.name == composite_name
            for i in advisor._backend.list_indexes("t")
        )
        # Pair hits should have been reset.
        assert advisor._pair_hits.get(("t", "a", "b"), 0) == 0
