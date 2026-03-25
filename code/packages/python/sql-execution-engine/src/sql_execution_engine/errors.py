"""Error classes for the SQL execution engine.

All errors inherit from ``ExecutionError`` so callers can catch the full
family with a single ``except ExecutionError`` clause, or catch specific
sub-classes for more precise error handling.

Error Hierarchy
---------------

.. code-block:: text

    ExecutionError          (base — something went wrong during execution)
    ├── TableNotFoundError  (the DataSource doesn't know the table name)
    └── ColumnNotFoundError (a column name is not in the row/schema)
"""

from __future__ import annotations


class ExecutionError(Exception):
    """Base class for all SQL execution engine errors.

    Every error raised by this package inherits from ``ExecutionError``.
    This lets callers write::

        try:
            result = execute(sql, source)
        except ExecutionError as e:
            print(f"SQL failed: {e}")

    rather than having to enumerate every possible error type.
    """


class TableNotFoundError(ExecutionError):
    """Raised when the DataSource does not recognize a table name.

    Example::

        # Source only has "employees", not "departments"
        execute("SELECT * FROM departments", source)
        # raises TableNotFoundError("departments")

    Attributes:
        table_name: The table name that was not found.
    """

    def __init__(self, table_name: str) -> None:
        self.table_name = table_name
        super().__init__(f"Table not found: {table_name!r}")


class ColumnNotFoundError(ExecutionError):
    """Raised when a column reference cannot be resolved.

    This happens when a query references a column that doesn't exist in
    the current row context — either the table doesn't have that column,
    or a qualified reference like ``t.col`` refers to an unknown alias.

    Example::

        # employees table has no "department" column (it's "dept_id")
        execute("SELECT department FROM employees", source)
        # raises ColumnNotFoundError("department")

    Attributes:
        column_name: The column name that was not found.
    """

    def __init__(self, column_name: str) -> None:
        self.column_name = column_name
        super().__init__(f"Column not found: {column_name!r}")
