"""
IndexPolicy — decision interface for automatic index lifecycle management.
=========================================================================

The :class:`IndexAdvisor` delegates all index creation and drop decisions
to an :class:`IndexPolicy` object.  Separating the policy from the advisor
keeps two orthogonal concerns apart:

- **Advisor**: observes query plans, maintains per-column hit counts,
  tracks index utilisation, calls the backend when the policy says "yes".
- **Policy**: stateless (or stateful) business rules — e.g. "create after
  N hits, drop after M cold queries", "only create on large tables", etc.

Swapping a policy is a one-liner at connection time:

.. code-block:: python

    conn = mini_sqlite.connect(":memory:")
    conn.set_policy(HitCountPolicy(threshold=5, cold_window=50))

Protocol, not ABC
-----------------

:class:`IndexPolicy` is a ``@runtime_checkable`` ``Protocol`` rather than an
abstract base class.  This means any object that implements the required
methods satisfies the interface — no inheritance required.

The protocol has three methods, of which only one is required:

- :meth:`should_create` (**required**) — called each time a new hit is
  observed for a ``(table, column)`` pair in a full-table-scan filter
  position.
- :meth:`should_drop` (*optional*) — called periodically to check whether
  an auto-created index has gone cold and should be removed.
- :meth:`on_query_event` (*optional*) — called after every
  :class:`~sql_vm.QueryEvent` so that the policy can observe raw runtime
  signals (selectivity, duration, index usage) for its own bookkeeping.
  Intended for ML-based or adaptive policies that want to build a feature
  history from live query data.

Backward compatibility
----------------------

Policies that only implement :meth:`should_create` (v2-style) remain valid.
The advisor checks for :meth:`should_drop` and :meth:`on_query_event` via
``hasattr`` before calling them, so a policy without those methods is simply
never asked to drop anything or observe query events.

ML / adaptive observer hook
----------------------------

:meth:`on_query_event` is the recommended extension point for data-driven
policies.  Each call receives the full :class:`~sql_vm.QueryEvent` for one
SELECT scan, including:

- ``table`` — which table was scanned
- ``filtered_columns`` — which columns appeared in the predicate
- ``rows_scanned`` / ``rows_returned`` — raw cardinality signals
- ``used_index`` — which auto-created index was selected (or ``None``)
- ``duration_us`` — wall-clock cost in microseconds

A minimal ML policy skeleton::

    class MLPolicy:
        def __init__(self):
            self._history: list[QueryEvent] = []

        def should_create(self, table, column, hit_count):
            return hit_count >= 3  # baseline rule while model warms up

        def on_query_event(self, event):
            self._history.append(event)
            # retrain / update model here as needed

HitCountPolicy
--------------

The built-in policy creates an index the first time the hit count for a
``(table, column)`` pair reaches the configured ``threshold``.  When
``cold_window > 0``, it also requests a drop of any auto-created index
that has not appeared in ``QueryEvent.used_index`` for at least
``cold_window`` consecutive SELECT scans.

Thread safety
-------------

Neither :class:`HitCountPolicy` nor :class:`IndexAdvisor` is thread-safe.
Multi-threaded applications should use per-connection instances (the default)
or add their own locking.
"""

from __future__ import annotations

from typing import Protocol, runtime_checkable


@runtime_checkable
class IndexPolicy(Protocol):
    """Decision interface for automatic index lifecycle management.

    The advisor calls :meth:`should_create` each time it observes a filter
    hit for a ``(table, column)`` pair, :meth:`should_drop` (when
    implemented) after each :class:`~sql_vm.QueryEvent` to check whether
    a cold auto-created index should be removed, and :meth:`on_query_event`
    (when implemented) to deliver the raw event to the policy so it can
    maintain its own feature history.

    ``hit_count`` in :meth:`should_create` is the running total of times the
    advisor has seen a filter on ``column`` within ``table``.  It includes
    the current observation — so the first call arrives with ``hit_count=1``.

    Implementations only need to provide :meth:`should_create`.
    :meth:`should_drop` and :meth:`on_query_event` are optional; the advisor
    detects their presence via ``hasattr`` and skips the respective logic
    when absent.
    """

    def should_create(self, table: str, column: str, hit_count: int) -> bool:
        """Return True when an index on table.column should be created."""
        ...  # pragma: no cover


class HitCountPolicy:
    """Create an index when a column's filter-hit count reaches ``threshold``.
    Optionally drop auto-created indexes that go cold.

    Parameters
    ----------
    threshold : int
        Number of full-table scans on a column before requesting an index.
        Must be ≥ 1.  Default 3.
    cold_window : int
        Number of SELECT scans (across any table) without seeing a given
        auto-created index used before requesting a drop.  0 disables
        automatic dropping — the default.

        When ``cold_window > 0``, :meth:`should_drop` returns ``True`` when
        ``queries_since_last_use >= cold_window``.  The advisor resets the
        counter each time an index is observed in a
        :class:`~sql_vm.QueryEvent`.

    Example — create after 3 hits, drop after 50 cold queries::

        policy = HitCountPolicy(threshold=3, cold_window=50)
        # After threshold full scans on user_id → index created.
        # After 50 queries where auto_orders_user_id is not used → drop.

    The threshold and cold_window are read-only after construction — create
    a new :class:`HitCountPolicy` instance to change either value.
    """

    def __init__(self, threshold: int = 3, *, cold_window: int = 0) -> None:
        if threshold < 1:
            raise ValueError(f"threshold must be >= 1, got {threshold!r}")
        if cold_window < 0:
            raise ValueError(f"cold_window must be >= 0, got {cold_window!r}")
        self._threshold = threshold
        self._cold_window = cold_window

    @property
    def threshold(self) -> int:
        """The configured hit-count threshold (read-only)."""
        return self._threshold

    @property
    def cold_window(self) -> int:
        """The configured cold-window size in queries (read-only).

        0 means automatic dropping is disabled.
        """
        return self._cold_window

    def should_create(self, table: str, column: str, hit_count: int) -> bool:
        """Return True when ``hit_count`` has reached the configured threshold.

        The ``table`` and ``column`` arguments are unused by this
        implementation — the decision is purely count-based.  Custom
        policies may inspect them (e.g. to exclude system tables or
        high-churn columns).
        """
        _ = table, column  # unused — suppress lint warnings
        return hit_count >= self._threshold

    def should_drop(
        self,
        index_name: str,
        table: str,
        column: str,
        queries_since_last_use: int,
    ) -> bool:
        """Return True when the index has been idle for ``cold_window`` queries.

        Returns ``False`` unconditionally when ``cold_window`` is 0
        (drop logic disabled).

        The ``index_name``, ``table``, and ``column`` arguments are unused
        by this implementation — the decision is purely count-based.  Custom
        policies may inspect them (e.g. to protect certain indexes from
        automatic removal).
        """
        _ = index_name, table, column  # unused — suppress lint warnings
        if self._cold_window == 0:
            return False
        return queries_since_last_use >= self._cold_window
