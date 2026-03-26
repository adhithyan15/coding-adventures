"""
sql_csv_source — SQL CSV DataSource adapter.

This package is a thin adapter that connects the SQL execution engine to
CSV files on disk. It implements the ``DataSource`` abstract base class
from ``sql_execution_engine`` using ``csv_parser`` to read CSV files.

Public API
----------

``CsvDataSource(directory)``
    A ``DataSource`` backed by ``*.csv`` files in *directory*.
    Each ``tablename.csv`` is one queryable table.

``execute_csv(sql, directory)``
    Convenience one-liner: build a ``CsvDataSource`` for *directory*
    and execute *sql* against it.

Example
-------

    >>> from sql_csv_source import CsvDataSource, execute_csv
    >>> source = CsvDataSource("tests/fixtures")
    >>> source.schema("employees")
    ['id', 'name', 'dept_id', 'salary', 'active']
    >>> result = execute_csv("SELECT name FROM employees WHERE active = true", "tests/fixtures")
    >>> [r["name"] for r in result.rows]
    ['Alice', 'Bob', 'Dave']
"""

from sql_csv_source.csv_data_source import CsvDataSource, execute_csv

__all__ = ["CsvDataSource", "execute_csv"]
