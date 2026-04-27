# csharp/sql-parser

SQL parser - parses SQL source text using the grammar-driven parser infrastructure.

This package mirrors the Java and Kotlin sql-parser package surface while reserving the dependency shape for the shared grammar-driven infrastructure.

## Usage

``csharp
var package = new SqlParser();
var id = package.Ping();
``