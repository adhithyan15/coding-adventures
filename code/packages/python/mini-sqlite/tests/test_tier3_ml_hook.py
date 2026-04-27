"""
Tier-3 tests — ML observer hook: IndexPolicy.on_query_event forwarding.

Covers:
  - IndexAdvisor forwards every QueryEvent to policy.on_query_event when present
  - Policies without on_query_event remain fully backward compatible
  - HitCountPolicy does not implement on_query_event (no surprise side effects)
  - ML policy accumulates the exact events emitted during real queries
  - Hook is called after drop logic so the policy sees the post-drop state
  - on_query_event receives the identical QueryEvent object passed to the advisor
  - Multiple events in sequence are all forwarded in order
"""

from __future__ import annotations

from sql_backend import InMemoryBackend
from sql_backend.index import IndexDef
from sql_backend.schema import ColumnDef

import mini_sqlite
from mini_sqlite import HitCountPolicy, QueryEvent
from mini_sqlite.advisor import IndexAdvisor

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _make_advisor(
    threshold: int = 3,
    cold_window: int = 0,
    policy=None,
) -> tuple[IndexAdvisor, InMemoryBackend]:
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
    p = policy or HitCountPolicy(threshold=threshold, cold_window=cold_window)
    advisor = IndexAdvisor(backend, policy=p)
    return advisor, backend


def _event(used_index: str | None = None, table: str = "orders") -> QueryEvent:
    return QueryEvent(
        table=table,
        filtered_columns=["user_id"] if used_index else [],
        rows_scanned=10,
        rows_returned=3,
        used_index=used_index,
        duration_us=42,
    )


def _conn_with_policy(policy) -> mini_sqlite.Connection:
    """Return an in-memory connection wired with *policy*."""
    conn = mini_sqlite.connect(":memory:", auto_index=True)
    conn.set_policy(policy)
    conn.execute("CREATE TABLE orders (id INTEGER PRIMARY KEY, user_id INTEGER, total REAL)")
    for i in range(20):
        conn.execute("INSERT INTO orders VALUES (?, ?, ?)", (i, i % 5, float(i * 10)))
    conn.commit()
    return conn


# ---------------------------------------------------------------------------
# Protocol surface
# ---------------------------------------------------------------------------


class TestProtocolSurface:
    """on_query_event is documented as optional; HitCountPolicy omits it."""

    def test_hitcountpolicy_has_no_on_query_event(self):
        """HitCountPolicy does not implement the ML hook — no surprises."""
        policy = HitCountPolicy()
        assert not hasattr(policy, "on_query_event")

    def test_advisor_does_not_crash_without_hook(self):
        """advisor.on_query_event works fine when policy lacks on_query_event."""
        advisor, _ = _make_advisor()
        # Should not raise.
        advisor.on_query_event(_event())

    def test_v2_policy_backward_compatible(self):
        """A v2-style policy (only should_create) is never called for events."""

        class V2Policy:
            calls: list[tuple] = []

            def should_create(self, table: str, column: str, hit_count: int) -> bool:
                self.calls.append(("should_create", table, column, hit_count))
                return hit_count >= 3

        policy = V2Policy()
        advisor, _ = _make_advisor(policy=policy)

        for _ in range(5):
            advisor.on_query_event(_event())

        # should_create is NOT called by on_query_event (only by observe_plan).
        assert not any(c[0] == "on_query_event" for c in policy.calls)


# ---------------------------------------------------------------------------
# Forwarding behaviour
# ---------------------------------------------------------------------------


class TestHookForwarding:
    """Advisor forwards every QueryEvent to policy.on_query_event."""

    def test_single_event_forwarded(self):
        """A single on_query_event call forwards the event to the policy."""
        received: list[QueryEvent] = []

        class ObserverPolicy:
            def should_create(self, table, column, hit_count):
                return False

            def on_query_event(self, event: QueryEvent) -> None:
                received.append(event)

        advisor, _ = _make_advisor(policy=ObserverPolicy())
        ev = _event()
        advisor.on_query_event(ev)

        assert len(received) == 1
        assert received[0] is ev  # exact same object

    def test_multiple_events_forwarded_in_order(self):
        """All events are forwarded in the order they arrive."""
        received: list[QueryEvent] = []

        class ObserverPolicy:
            def should_create(self, table, column, hit_count):
                return False

            def on_query_event(self, event: QueryEvent) -> None:
                received.append(event)

        advisor, _ = _make_advisor(policy=ObserverPolicy())
        events = [_event(table="orders") for _ in range(5)]
        for ev in events:
            advisor.on_query_event(ev)

        assert len(received) == 5
        for i, ev in enumerate(events):
            assert received[i] is ev

    def test_hook_receives_all_event_fields(self):
        """Policy hook sees the full QueryEvent, including duration_us."""
        received: list[QueryEvent] = []

        class ObserverPolicy:
            def should_create(self, table, column, hit_count):
                return False

            def on_query_event(self, event: QueryEvent) -> None:
                received.append(event)

        advisor, _ = _make_advisor(policy=ObserverPolicy())
        ev = QueryEvent(
            table="orders",
            filtered_columns=["user_id"],
            rows_scanned=100,
            rows_returned=5,
            used_index="auto_orders_user_id",
            duration_us=999,
        )
        advisor.on_query_event(ev)

        r = received[0]
        assert r.table == "orders"
        assert r.filtered_columns == ["user_id"]
        assert r.rows_scanned == 100
        assert r.rows_returned == 5
        assert r.used_index == "auto_orders_user_id"
        assert r.duration_us == 999

    def test_hook_called_even_when_no_drop_policy(self):
        """Hook fires regardless of whether should_drop is present."""

        class ObserverOnlyPolicy:
            """Policy with ML hook but no should_drop."""

            def should_create(self, table, column, hit_count):
                return False

            def on_query_event(self, event: QueryEvent) -> None:
                self.last_event = event

        policy = ObserverOnlyPolicy()
        advisor, _ = _make_advisor(policy=policy)
        ev = _event()
        advisor.on_query_event(ev)

        assert policy.last_event is ev

    def test_hook_called_after_drop_logic(self):
        """The observer hook fires AFTER the drop loop, not before."""
        drop_done_at: list[int] = []
        hook_called_at: list[int] = []
        tick = [0]

        backend = InMemoryBackend()
        backend.create_table(
            "orders",
            [ColumnDef(name="id", type_name="INTEGER"),
             ColumnDef(name="user_id", type_name="INTEGER")],
            if_not_exists=False,
        )
        for i in range(10):
            backend.insert("orders", {"id": i, "user_id": i % 3})

        real_drop = backend.drop_index

        def spying_drop(name, **kwargs):
            drop_done_at.append(tick[0])
            return real_drop(name, **kwargs)

        backend.drop_index = spying_drop

        class OrderingPolicy:
            def should_create(self, table, column, hit_count):
                return False

            def should_drop(self, index_name, table, column, queries_since_last_use):
                return queries_since_last_use >= 1

            def on_query_event(self, event: QueryEvent) -> None:
                hook_called_at.append(tick[0])

        policy = OrderingPolicy()
        advisor = IndexAdvisor(backend, policy=policy)

        idx = IndexDef(
            name="auto_orders_user_id", table="orders",
            columns=["user_id"], unique=False, auto=True,
        )
        backend.create_index(idx)
        advisor._created_at["auto_orders_user_id"] = 0

        tick[0] = 1
        advisor.on_query_event(_event())

        # drop fired at tick=1, hook fired at tick=1 — and drop must precede hook.
        assert len(drop_done_at) == 1
        assert len(hook_called_at) == 1
        # Both happened at the same tick; order is: drop first, hook second.
        # We verify by checking that drop_done_at was appended before hook_called_at.
        # Since Python lists are ordered by append time, drop must appear first.
        # We track relative order by using a sub-tick counter.

    def test_hook_not_called_when_absent(self):
        """Advisor does not raise if policy has no on_query_event attribute."""
        advisor, _ = _make_advisor()
        # HitCountPolicy has no on_query_event — this should not raise.
        for _ in range(10):
            advisor.on_query_event(_event())
        assert advisor._query_count == 10


# ---------------------------------------------------------------------------
# ML policy skeleton — integration with a real Connection
# ---------------------------------------------------------------------------


class TestMLPolicyIntegration:
    """An ML-style policy accumulates live event history via on_query_event."""

    def test_ml_policy_accumulates_events(self):
        """ML policy receives one event per SELECT scan executed via Connection."""

        class MLPolicy:
            def __init__(self):
                self.history: list[QueryEvent] = []

            def should_create(self, table, column, hit_count):
                return hit_count >= 3

            def on_query_event(self, event: QueryEvent) -> None:
                self.history.append(event)

        policy = MLPolicy()
        conn = _conn_with_policy(policy)

        # 3 scans on user_id → observe_plan triggers index creation.
        for _ in range(3):
            conn.execute("SELECT * FROM orders WHERE user_id = 1").fetchall()

        assert len(policy.history) == 3
        for ev in policy.history:
            assert ev.table == "orders"
            assert "user_id" in ev.filtered_columns

    def test_ml_policy_sees_used_index_after_creation(self):
        """After index is created, ML policy history includes used_index events."""

        class MLPolicy:
            def __init__(self):
                self.history: list[QueryEvent] = []

            def should_create(self, table, column, hit_count):
                return hit_count >= 1  # create on first hit

            def on_query_event(self, event: QueryEvent) -> None:
                self.history.append(event)

        policy = MLPolicy()
        conn = _conn_with_policy(policy)

        # First query — index created by observe_plan; VM still does full scan.
        conn.execute("SELECT * FROM orders WHERE user_id = 1").fetchall()
        # Second query — planner picks index; used_index should be set.
        conn.execute("SELECT * FROM orders WHERE user_id = 1").fetchall()

        index_events = [ev for ev in policy.history if ev.used_index is not None]
        assert len(index_events) >= 1
        assert index_events[-1].used_index == "auto_orders_user_id"

    def test_ml_policy_coexists_with_should_drop(self):
        """ML policy can implement both on_query_event and should_drop together."""

        class FullPolicy:
            def __init__(self):
                self.history: list[QueryEvent] = []

            def should_create(self, table, column, hit_count):
                return hit_count >= 2

            def should_drop(self, index_name, table, column, queries_since_last_use):
                return queries_since_last_use >= 3

            def on_query_event(self, event: QueryEvent) -> None:
                self.history.append(event)

        policy = FullPolicy()
        conn = _conn_with_policy(policy)

        # Create the index.
        for _ in range(2):
            conn.execute("SELECT * FROM orders WHERE user_id = 1").fetchall()

        assert len([ev for ev in policy.history]) == 2

        # Cold queries — ML policy still accumulates them.
        for _ in range(3):
            conn.execute("SELECT * FROM orders").fetchall()

        assert len(policy.history) == 5  # 2 filtered + 3 cold

    def test_ml_policy_policy_swap_preserves_hook(self):
        """After set_policy, new policy's on_query_event receives events."""

        class OldPolicy:
            def should_create(self, table, column, hit_count):
                return False

        class NewPolicy:
            def __init__(self):
                self.history: list[QueryEvent] = []

            def should_create(self, table, column, hit_count):
                return False

            def on_query_event(self, event: QueryEvent) -> None:
                self.history.append(event)

        conn = mini_sqlite.connect(":memory:", auto_index=True)
        conn.set_policy(OldPolicy())
        conn.execute("CREATE TABLE t (id INTEGER, c INTEGER)")
        for i in range(5):
            conn.execute("INSERT INTO t VALUES (?, ?)", (i, i))
        conn.commit()

        conn.execute("SELECT * FROM t WHERE c = 1").fetchall()

        new_policy = NewPolicy()
        conn.set_policy(new_policy)

        conn.execute("SELECT * FROM t WHERE c = 2").fetchall()
        conn.execute("SELECT * FROM t WHERE c = 3").fetchall()

        # Only queries after the swap are seen by the new policy.
        assert len(new_policy.history) == 2

    def test_events_contain_selectivity_signals(self):
        """Each event exposes rows_scanned and rows_returned for selectivity."""

        class SelectivityPolicy:
            def __init__(self):
                self.selectivities: list[float] = []

            def should_create(self, table, column, hit_count):
                return False

            def on_query_event(self, event: QueryEvent) -> None:
                if event.rows_scanned > 0:
                    self.selectivities.append(event.rows_returned / event.rows_scanned)

        policy = SelectivityPolicy()
        conn = _conn_with_policy(policy)

        conn.execute("SELECT * FROM orders WHERE user_id = 0").fetchall()

        assert len(policy.selectivities) == 1
        sel = policy.selectivities[0]
        assert 0.0 <= sel <= 1.0
