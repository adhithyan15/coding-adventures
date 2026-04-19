"""
QueryResult — what :func:`execute` returns
==========================================

A single dataclass describing the output of a query. The shape matches what
PEP 249 drivers expose via ``cursor.description`` and ``cursor.fetchall()``
without committing to that API here — that translation lives in the façade.

Three fields:

- ``columns`` — output column names, ordered. Empty for DML/DDL.
- ``rows`` — list of tuples of SqlValue. Each tuple is ordered to match
  ``columns``. Empty for DML/DDL, or for a SELECT with no matching rows.
- ``rows_affected`` — set for INSERT, UPDATE, DELETE, CREATE, DROP; ``None``
  for SELECT so callers can distinguish "zero rows in the result set" from
  "this was a mutation that touched zero rows".
"""

from __future__ import annotations

from dataclasses import dataclass, field

from sql_backend.values import SqlValue

Row = tuple[SqlValue, ...]


@dataclass(frozen=True, slots=True)
class QueryResult:
    """Canonical result of ``execute(program, backend)``."""

    columns: tuple[str, ...] = ()
    rows: tuple[Row, ...] = ()
    rows_affected: int | None = None

    def to_dicts(self) -> list[dict[str, SqlValue]]:
        """Re-zip rows into dicts keyed by column name — useful in tests."""
        return [dict(zip(self.columns, row, strict=True)) for row in self.rows]


@dataclass(slots=True)
class _MutableResult:
    """Internal result builder used by the VM while execution is in progress.

    The VM mutates this in place (appends rows, sets schema). When execution
    completes, we freeze it into a :class:`QueryResult`.
    """

    columns: tuple[str, ...] = ()
    rows: list[Row] = field(default_factory=list)
    rows_affected: int | None = None

    def freeze(self) -> QueryResult:
        return QueryResult(
            columns=self.columns,
            rows=tuple(self.rows),
            rows_affected=self.rows_affected,
        )
