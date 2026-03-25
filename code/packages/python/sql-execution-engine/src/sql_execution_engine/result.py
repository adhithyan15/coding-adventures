"""QueryResult — the output of a SQL SELECT execution.

A ``QueryResult`` bundles the column names and the data rows produced by
a SELECT query.  It is intentionally simple: a dataclass with two fields.

Why separate columns from rows?
--------------------------------

SQL SELECT allows renaming columns with AS aliases and reordering them.
The column names in ``columns`` are the *output* names after aliases are
applied — not the original table column names.

Example
-------

.. code-block:: python

    result = execute("SELECT name AS employee_name, salary FROM employees", src)
    result.columns   # ["employee_name", "salary"]
    result.rows      # [{"employee_name": "Alice", "salary": 90000}, ...]

The ``rows`` list preserves the ordering specified by ORDER BY (or insertion
order if no ORDER BY is present). Each row is a plain ``dict`` mapping the
output column names to their values.  Values may be ``None`` (SQL NULL),
``int``, ``float``, ``str``, or ``bool``.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any


@dataclass
class QueryResult:
    """The output of a successfully executed SELECT query.

    Attributes:
        columns: The output column names in SELECT order.  If the query
                 used ``AS`` aliases those are the names here.
        rows: The result rows, each a ``dict`` mapping column name to value.
              Rows are in the order produced by ORDER BY (or scan order if
              no ORDER BY was requested).
    """

    columns: list[str] = field(default_factory=list)
    rows: list[dict[str, Any]] = field(default_factory=list)

    def __repr__(self) -> str:
        return (
            f"QueryResult(columns={self.columns!r}, "
            f"rows={len(self.rows)} row{'s' if len(self.rows) != 1 else ''})"
        )
