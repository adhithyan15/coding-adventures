"""
Schema types
============

A table's schema is an ordered list of :class:`ColumnDef`. Each ``ColumnDef``
captures everything the VM and backend need to know about one column:

======================  ====================================================
Field                   Meaning
======================  ====================================================
``name``                Column name, as written in CREATE TABLE
``type_name``           Type token as a string ("INTEGER", "TEXT", ...)
``not_null``            True if NULLs are rejected on insert/update
``primary_key``         True if this column is the primary key. Implies
                        ``not_null`` and ``unique``. The constraint-enforcer
                        in InMemoryBackend keys off these three flags — it
                        does not look at ``primary_key`` alone.
``unique``              True if duplicate values are rejected
``default``             Value supplied when an insert omits this column.
                        ``None`` here is genuinely "no default" — we can't
                        use ``None`` for both "no default" and "default is
                        NULL", so the absence-of-default case uses a
                        sentinel (see :data:`NO_DEFAULT`).
======================  ====================================================

Why a string for ``type_name`` and not an enum?
-----------------------------------------------

SQLite — the original — uses *type affinity*: the type name you write in
CREATE TABLE is advisory, not a hard constraint. ``INTEGER PRIMARY KEY`` and
``INT PRIMARY KEY`` behave identically; a column declared ``VARCHAR(255)``
cheerfully stores integers. We preserve this spirit by keeping the token as a
string and leaving interpretation to the backend. A CSV backend might ignore
types entirely; a future strict backend could reject writes that don't match.
Leaving the format open is what makes the interface portable.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Final, Literal

from .values import SqlValue


# Sentinel used by :class:`ColumnDef.default` to distinguish "the column has
# no default" from "the column's default is SQL NULL". We can't use Python
# ``None`` for both because NULL is itself a valid SqlValue. The sentinel is
# an object whose identity is unique — ``is`` comparisons against it are
# cheap and unambiguous.
class _NoDefault:
    """Sentinel type for :data:`NO_DEFAULT`."""

    _instance: _NoDefault | None = None

    def __new__(cls) -> _NoDefault:
        if cls._instance is None:
            cls._instance = super().__new__(cls)
        return cls._instance

    def __repr__(self) -> str:
        return "NO_DEFAULT"

    def __bool__(self) -> bool:
        return False


NO_DEFAULT: Final[_NoDefault] = _NoDefault()


# A column default is either a SqlValue (including None for SQL NULL) or the
# :data:`NO_DEFAULT` sentinel meaning "no default clause was specified".
ColumnDefault = SqlValue | _NoDefault


@dataclass(eq=True)
class ColumnDef:
    """One column in a table schema.

    Example::

        ColumnDef(
            name="email",
            type_name="TEXT",
            not_null=True,
            unique=True,
        )

    Primary key columns should set ``primary_key=True`` *in addition to*
    whatever NOT NULL / UNIQUE flags apply — InMemoryBackend enforces
    ``primary_key`` as (implicit) NOT NULL + UNIQUE, but explicit flags make
    the intent readable in test fixtures.
    """

    name: str
    type_name: str
    not_null: bool = False
    primary_key: bool = False
    unique: bool = False
    autoincrement: bool = False
    default: ColumnDefault = field(default=NO_DEFAULT)
    check_expr: object = field(default=None, compare=False, hash=False)
    # (ref_table, ref_col_or_None) — None ref_col means "reference the PK".
    # Typed as object to avoid circular import with planner types.
    foreign_key: object = field(default=None, compare=False, hash=False)

    def effective_not_null(self) -> bool:
        """PRIMARY KEY implies NOT NULL. Convenience for the constraint enforcer."""
        return self.not_null or self.primary_key

    def effective_unique(self) -> bool:
        """PRIMARY KEY implies UNIQUE. Convenience for the constraint enforcer."""
        return self.unique or self.primary_key

    def has_default(self) -> bool:
        """True iff a DEFAULT clause was specified (even if that default is NULL)."""
        return self.default is not NO_DEFAULT


@dataclass(eq=True)
class TriggerDef:
    """Definition of a CREATE TRIGGER object stored in the backend.

    ``body`` is the raw SQL text of the trigger body statements (without
    the outer BEGIN…END wrapper), with individual statements separated by
    semicolons.  The VM re-parses and re-compiles the body on each firing.

    ``timing`` is ``"BEFORE"`` or ``"AFTER"``.
    ``event`` is ``"INSERT"``, ``"UPDATE"``, or ``"DELETE"``.
    """

    name: str
    table: str
    timing: Literal["BEFORE", "AFTER"]
    event: Literal["INSERT", "UPDATE", "DELETE"]
    body: str
