"""SQL Execution Engine — execute SELECT queries against pluggable data sources.

This package is the execution layer of the SQL stack:

.. code-block:: text

    sql-lexer  →  sql-parser  →  sql-execution-engine
                                        ↑
                                   (this package)

Public API
----------

.. code-block:: python

    from sql_execution_engine import execute, execute_all, DataSource, QueryResult
    from sql_execution_engine.errors import (
        ExecutionError,
        TableNotFoundError,
        ColumnNotFoundError,
    )

Quick Start
-----------

.. code-block:: python

    from sql_execution_engine import execute, DataSource

    class MySource(DataSource):
        def schema(self, table_name):
            ...
        def scan(self, table_name):
            ...

    result = execute("SELECT * FROM users WHERE age > 18", MySource())
    print(result.columns)  # ["id", "name", "age"]
    print(result.rows)     # [{"id": 1, "name": "Alice", "age": 30}, ...]
"""

from sql_execution_engine.data_source import DataSource
from sql_execution_engine.engine import execute, execute_all
from sql_execution_engine.errors import (
    ColumnNotFoundError,
    ExecutionError,
    TableNotFoundError,
)
from sql_execution_engine.result import QueryResult

__all__ = [
    "execute",
    "execute_all",
    "DataSource",
    "QueryResult",
    "ExecutionError",
    "TableNotFoundError",
    "ColumnNotFoundError",
]
