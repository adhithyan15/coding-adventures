# csharp/excel-lexer

Excel lexer - tokenizes Excel source text using the grammar-driven lexer infrastructure.

This package mirrors the Java and Kotlin excel-lexer package surface while reserving the dependency shape for the shared grammar-driven infrastructure.

## Usage

``csharp
var package = new ExcelLexer();
var id = package.Ping();
``