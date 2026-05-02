# sql-csv-source (Java)

A thin adapter that lets the Java `sql-execution-engine` query directories of CSV files.

Each `tablename.csv` file is exposed as one SQL table. Values are parsed through
`csv-parser`, coerced into Java SQL-friendly values, and handed to the execution
engine through its `DataSource` interface.
