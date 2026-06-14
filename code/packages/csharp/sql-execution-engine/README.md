# CodingAdventures.SqlExecutionEngine

C# SELECT-only SQL execution engine for mini-sqlite packages.

The package exposes an `IDataSource` abstraction, an in-memory data source, and
helpers for executing SQL against any source that can provide table schemas and
row dictionaries.
