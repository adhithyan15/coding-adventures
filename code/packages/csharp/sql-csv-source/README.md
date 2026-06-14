# sql-csv-source (C#)

A thin adapter that lets the C# `sql-execution-engine` query directories of CSV
files. Each `tablename.csv` file is exposed as a table, parsed through
`csv-parser`, coerced into SQL-friendly .NET values, and executed through the
engine's `IDataSource` interface.
