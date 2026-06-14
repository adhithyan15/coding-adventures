# CodingAdventures::SqlCsvSource

Perl port of the mini-sqlite CSV data source adapter. It maps table names to
`<table>.csv` files, reads headers as schemas, parses rows with
`CodingAdventures::CsvParser`, coerces CSV strings to SQL-friendly Perl values,
and can execute SELECT queries through `CodingAdventures::SqlExecutionEngine`.
