"""
IndexPolicy — decision interface for automatic index creation.
==============================================================

The :class:`IndexAdvisor` delegates all "should I create an index?" decisions
to an :class:`IndexPolicy` object.  Separating the policy from the advisor
keeps two orthogonal concerns apart:

- **Advisor**: observes query plans, maintains per-column hit counts,
  calls the backend when the policy says "yes".
- **Policy**: stateless (or stateful) business rules — e.g. "create after
  N hits", "create only for large tables", "never create on these columns".

Swapping a policy is a one-liner at connection time:

.. code-block:: python

    conn = mini_sqlite.connect(":memory:")
    conn.set_policy(HitCountPolicy(threshold=5))

Protocol, not ABC
-----------------

:class:`IndexPolicy` is a ``@runtime_checkable`` ``Protocol`` rather than an
abstract base class.  This means any object that implements ``should_create``
satisfies the interface — no inheritance required.  It also makes mock
policies trivial in tests: a simple ``lambda`` wrapped in a tiny class is all
you need.

HitCountPolicy
--------------

The built-in policy creates an index the first time the hit count for a
``(table, column)`` pair reaches the configured threshold.  Subsequent
queries that still use a full scan on the same column do *not* create
duplicate indexes — the advisor skips creation if an index for that column
already exists on the backend.

The default threshold of **3** is a conservative choice:

- 1 hit → might be a one-off migration query; too aggressive to index.
- 3 hits → a pattern is forming; index is likely to pay off.
- Higher thresholds → use when writes are expensive (index maintenance cost)
  or the table is small (index rarely helps anyway).

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
    """Decision interface: should we create an index on this column?

    The advisor calls :meth:`should_create` each time it observes a new hit
    for a ``(table, column)`` pair.  Return ``True`` to trigger index
    creation; return ``False`` to wait for more evidence.

    ``hit_count`` is the running total of times the advisor has seen a
    filter on ``column`` within ``table``.  It includes the current
    observation — so the first call arrives with ``hit_count=1``.
    """

    def should_create(self, table: str, column: str, hit_count: int) -> bool:
        """Return True when an index on table.column should be created."""
        ...  # pragma: no cover


class HitCountPolicy:
    """Create an index when a column's filter-hit count reaches ``threshold``.

    Example — create after 3 repeated filter observations (the default)::

        policy = HitCountPolicy(threshold=3)
        policy.should_create("orders", "user_id", 1)  # False
        policy.should_create("orders", "user_id", 2)  # False
        policy.should_create("orders", "user_id", 3)  # True  ← index created
        policy.should_create("orders", "user_id", 4)  # True  (still yes, but
                                                        #  advisor won't re-create)

    The threshold is read-only after construction — create a new policy
    instance to change it.
    """

    def __init__(self, threshold: int = 3) -> None:
        if threshold < 1:
            raise ValueError(f"threshold must be >= 1, got {threshold!r}")
        self._threshold = threshold

    @property
    def threshold(self) -> int:
        """The configured hit-count threshold."""
        return self._threshold

    def should_create(self, table: str, column: str, hit_count: int) -> bool:
        """Return True when ``hit_count`` has reached the configured threshold.

        The ``table`` and ``column`` arguments are unused by this
        implementation — the decision is purely count-based.  Custom
        policies may inspect them (e.g. to exclude system tables or
        high-churn columns).
        """
        _ = table, column  # unused — suppress lint warnings
        return hit_count >= self._threshold
