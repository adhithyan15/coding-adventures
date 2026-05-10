# Java SQL Execution Engine

SELECT-only SQL execution engine for Java mini-sqlite packages.

The package exposes a small `DataSource` interface with `schema` and `scan`
methods, executes SQL against any implementation of that interface, and includes
an in-memory data source for tests and examples.
