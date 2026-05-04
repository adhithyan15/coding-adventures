# sql-execution-engine (F#)

A small SELECT execution engine for mini-sqlite data sources.

The package exposes an `IDataSource` abstraction, an in-memory implementation,
and query helpers for scans, filters, joins, grouping, aggregates, ordering,
distinct, limit, and offset.
