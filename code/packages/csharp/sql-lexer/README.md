# csharp/sql-lexer

SQL lexer - tokenizes SQL source text using the grammar-driven lexer infrastructure.

This package mirrors the Java and Kotlin sql-lexer package surface while reserving the dependency shape for the shared grammar-driven infrastructure.

## Usage

``csharp
var package = new SqlLexer();
var id = package.Ping();
``