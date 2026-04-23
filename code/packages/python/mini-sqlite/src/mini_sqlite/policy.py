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

The protocol has two methods:

- :meth:`should_create` — called each time a new hit is observed for a
  ``(table, column)`` pair in a full-table-scan filter position.
- :meth:`should_drop` — called periodically to check whether an
  auto-created index has gone cold and should be removed.

Backward compatibility
----------------------

Policies that only implement :meth:`should_create` (v2-style) remain valid.
The advisor checks for :meth:`should_drop` via ``hasattr`` before calling it,
so a policy without the method is simply never asked to drop anything.

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
    hit for a ``(table, column)`` pair, and :meth:`should_drop` (when
    implemented) after each :class:`~sql_vm.QueryEvent` to check whether
    a cold auto-created index should be removed.

    ``hit_count`` in :meth:`should_create` is the running total of times the
    advisor has seen a filter on ``column`` within ``table``.  It includes
    the current observation — so the first call arrives with ``hit_count=1``.

    Implementations only need to provide :meth:`should_create`.
    :meth:`should_drop` is optional; the advisor skips drop checks when the
    method is absent.
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
