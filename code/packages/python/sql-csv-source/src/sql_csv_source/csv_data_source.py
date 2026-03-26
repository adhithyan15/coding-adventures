"""
sql_csv_source.csv_data_source
──────────────────────────────
Thin adapter that implements the DataSource ABC from ``sql_execution_engine``
using ``csv_parser`` to read CSV files from a directory.

Design
------

The adapter is intentionally minimal. All the complexity of SQL evaluation
(filtering, joining, aggregation, ordering) lives in the execution engine.
This adapter's only jobs are:

1. Map a ``table_name`` to a file path (``{directory}/{table_name}.csv``).
2. Parse the CSV text into row dicts via ``csv_parser.parse_csv``.
3. Coerce each string value to its natural Python/SQL type.
4. Report missing tables as ``TableNotFoundError``.

Directory layout assumed:

    data/
        employees.csv
        departments.csv
        orders.csv

Query: ``SELECT * FROM employees`` → reads ``data/employees.csv``.

Type Coercion
-------------

CSV is untyped — every field is a string.  The engine needs Python values
(``None``, ``int``, ``float``, ``bool``, ``str``) to evaluate expressions
like ``WHERE salary > 80000`` or ``WHERE active = true``.

Coercion rules (applied in order):

    | CSV string  | Python value    | Rationale                     |
    |-------------|-----------------|-------------------------------|
    | ""          | None            | SQL NULL                      |
    | "true"      | True            | case-insensitive boolean       |
    | "false"     | False           | case-insensitive boolean       |
    | "42"        | 42 (int)        | try int() first               |
    | "3.14"      | 3.14 (float)    | then float()                  |
    | "hello"     | "hello" (str)   | fall through                  |

Column Ordering
---------------

Python dicts (3.7+) preserve insertion order.  ``csv_parser.parse_csv``
builds each row dict by zipping the header list with field values, so
``list(row.keys())`` gives columns in header order.  That said, to be
explicit and safe, ``schema()`` reads the first raw line of the file and
splits on comma — this is a direct read of the header row with zero
ambiguity about ordering.
"""

from __future__ import annotations

from pathlib import Path
from typing import Any

from csv_parser import parse_csv
from sql_execution_engine import execute
from sql_execution_engine.data_source import DataSource
from sql_execution_engine.errors import TableNotFoundError


class CsvDataSource(DataSource):
    """A DataSource backed by CSV files in a directory.

    Each ``tablename.csv`` file in *directory* is one queryable table.
    Column names come from the CSV header row.  Values are type-coerced
    from strings to the most appropriate Python type before being handed
    to the execution engine.

    Parameters
    ----------
    directory:
        Path to the directory containing ``*.csv`` files.  May be a
        ``str`` or ``pathlib.Path``.

    Example
    -------
    ::

        source = CsvDataSource("data/")
        result = execute("SELECT * FROM employees WHERE active = true", source)
        for row in result.rows:
            print(row["name"], row["salary"])
    """

    def __init__(self, directory: str | Path) -> None:
        # Store as a Path object for convenient path manipulation.
        # Path("data") / "employees.csv"  →  "data/employees.csv"
        self.directory = Path(directory)

    def schema(self, table_name: str) -> list[str]:
        """Return column names for *table_name* in header order.

        Reads only the first line of the CSV file to extract column names.
        This is fast (no need to parse all rows) and preserves the exact
        header order from the file.

        Parameters
        ----------
        table_name:
            Bare table name (e.g. ``"employees"``).  The method appends
            ``.csv`` and looks inside ``self.directory``.

        Returns
        -------
        list[str]
            Column names in the order they appear in the CSV header row.

        Raises
        ------
        TableNotFoundError
            If ``{table_name}.csv`` does not exist in the directory.
        """
        path = self._resolve(table_name)
        # Read the full content and grab just the first line.
        # Splitting on "\n" handles both "\n" and "\r\n" line endings.
        first_line = path.read_text(encoding="utf-8").split("\n")[0].strip()
        if not first_line:
            return []
        # Split the header line on commas to get ordered column names.
        return [col.strip() for col in first_line.split(",")]

    def scan(self, table_name: str) -> list[dict[str, Any]]:
        """Return all data rows from *table_name* with type-coerced values.

        Uses ``csv_parser.parse_csv`` to handle RFC 4180 features like
        quoted fields with embedded commas and escaped double-quotes.

        Parameters
        ----------
        table_name:
            Bare table name (e.g. ``"employees"``).

        Returns
        -------
        list[dict[str, Any]]
            Each dict maps column name → coerced value.  An empty table
            (header-only CSV) returns ``[]``.

        Raises
        ------
        TableNotFoundError
            If ``{table_name}.csv`` does not exist in the directory.
        """
        path = self._resolve(table_name)
        content = path.read_text(encoding="utf-8")
        # parse_csv returns list[dict[str, str]] — all values are strings.
        str_rows = parse_csv(content)
        # Coerce each value from string to its natural Python/SQL type.
        return [{k: _coerce(v) for k, v in row.items()} for row in str_rows]

    # ── private ──────────────────────────────────────────────────────────────

    def _resolve(self, table_name: str) -> Path:
        """Build the CSV file path and raise TableNotFoundError if missing."""
        path = self.directory / f"{table_name}.csv"
        if not path.exists():
            raise TableNotFoundError(table_name)
        return path


def execute_csv(sql: str, directory: str | Path) -> Any:
    """Execute a SQL query against CSV files in *directory*.

    Convenience one-liner that constructs a :class:`CsvDataSource` for
    *directory* and runs *sql* through the execution engine.

    Parameters
    ----------
    sql:
        A SQL ``SELECT`` statement.
    directory:
        Path to the directory containing ``*.csv`` files.

    Returns
    -------
    QueryResult
        A result object with ``.columns`` (list of str) and
        ``.rows`` (list of dict).

    Example
    -------
    ::

        result = execute_csv("SELECT name FROM employees LIMIT 2", "data/")
        print(result.columns)  # ["name"]
        print(result.rows)     # [{"name": "Alice"}, {"name": "Bob"}]
    """
    source = CsvDataSource(directory)
    return execute(sql, source)


def _coerce(value: str) -> Any:
    """Coerce a CSV string to the most appropriate Python/SQL type.

    This function is the heart of the type system bridge between CSV
    (everything is a string) and SQL (values have types).

    Coercion is applied in priority order:

    1. Empty string  →  ``None``  (SQL NULL — absence of a value)
    2. ``"true"``    →  ``True``  (case-insensitive)
    3. ``"false"``   →  ``False`` (case-insensitive)
    4. Parseable int →  ``int``   (e.g., ``"42"`` → ``42``)
    5. Parseable float → ``float`` (e.g., ``"3.14"`` → ``3.14``)
    6. Anything else →  ``str``   (keep as-is)

    Why booleans before numbers?  Because ``True`` and ``False`` in Python
    are technically integers (``True == 1``, ``False == 0``), so if we
    tried ``int("true")`` first it would raise ``ValueError``.  Explicit
    boolean check first avoids that ambiguity.

    Parameters
    ----------
    value:
        A single CSV field value string.

    Returns
    -------
    None | bool | int | float | str
        The coerced Python value.
    """
    # ── NULL ─────────────────────────────────────────────────────────────────
    if value == "":
        return None

    # ── Boolean ──────────────────────────────────────────────────────────────
    # Check case-insensitively so "True", "TRUE", "true" all work.
    lower = value.lower()
    if lower == "true":
        return True
    if lower == "false":
        return False

    # ── Integer ──────────────────────────────────────────────────────────────
    # int("42") succeeds; int("3.14") raises ValueError.
    try:
        return int(value)
    except ValueError:
        pass

    # ── Float ────────────────────────────────────────────────────────────────
    # float("3.14") succeeds; float("hello") raises ValueError.
    try:
        return float(value)
    except ValueError:
        pass

    # ── String fallthrough ───────────────────────────────────────────────────
    return value
