# coding_adventures.sql_csv_source

Lua port of the mini-sqlite CSV data source adapter. It maps table names to
`<table>.csv` files, reads headers as schemas, parses rows with
`coding_adventures.csv_parser`, coerces CSV strings to SQL-friendly Lua values,
and can execute SELECT queries through `coding_adventures.sql_execution_engine`.
