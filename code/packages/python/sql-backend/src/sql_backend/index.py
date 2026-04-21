"""
Index definition type
=====================

:class:`IndexDef` describes a single B-tree index that a backend holds.
It carries all the information needed to create, drop, or reason about an
index without touching the storage layer.

Design note
-----------

``IndexDef`` is a plain data class — no methods beyond equality and repr.
The *backend* is responsible for turning an ``IndexDef`` into actual B-tree
pages; the *planner* and *advisor* interact with indexes through this
descriptor only.

Two flags worth explaining:

``unique``
    When ``True``, the backend must reject inserts/updates that would
    produce a duplicate key.  In v2 all automatically-created indexes are
    non-unique (the advisor builds them for read acceleration, not for
    constraint enforcement), so this flag is always ``False`` for v2
    auto-created indexes.  It is reserved for ``CREATE UNIQUE INDEX``
    support in a future release.

``auto``
    When ``True``, this index was created by the :class:`IndexAdvisor` in
    response to observed query patterns, not by an explicit ``CREATE INDEX``
    statement from the application.  Auto-created indexes may be dropped
    automatically when the policy decides they are no longer warranted.
    User-created indexes (``auto=False``) are never dropped automatically.

Examples::

    # Single-column index on orders.user_id (auto-created by the advisor):
    IndexDef(
        name="auto_orders_user_id",
        table="orders",
        columns=["user_id"],
        unique=False,
        auto=True,
    )

    # Explicit unique index on users.email:
    IndexDef(
        name="idx_users_email",
        table="users",
        columns=["email"],
        unique=True,
        auto=False,
    )
"""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass(eq=True)
class IndexDef:
    """Descriptor for a single B-tree index.

    Parameters
    ----------
    name:
        Unique index name within the database.  The advisor uses the
        ``auto_{table}_{column}`` naming convention; user-created indexes
        may use any name.
    table:
        Name of the table this index covers.
    columns:
        Column names in sort order, left to right.  For a single-column
        index pass a one-element list, e.g. ``["user_id"]``.  Composite
        indexes are deferred to v3.
    unique:
        ``True`` if the index enforces uniqueness.  v2 only ships
        non-unique indexes — this field is reserved for future use.
    auto:
        ``True`` if this index was created automatically by the
        :class:`~mini_sqlite.advisor.IndexAdvisor`, ``False`` if it was
        created by an explicit ``CREATE INDEX`` statement.

    Equality
    --------

    Two ``IndexDef`` instances compare equal when all five fields match.
    This makes assertions in tests concise::

        assert backend.list_indexes("orders") == [
            IndexDef(
                name="auto_orders_user_id",
                table="orders",
                columns=["user_id"],
            )
        ]
    """

    name: str
    table: str
    columns: list[str] = field(default_factory=list)
    unique: bool = False
    auto: bool = False
