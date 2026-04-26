# csharp/json-lexer

JSON lexer - tokenizes JSON source text using the grammar-driven lexer infrastructure.

This package mirrors the Java and Kotlin json-lexer package surface while reserving the dependency shape for the shared grammar-driven infrastructure.

## Usage

``csharp
var package = new JsonLexer();
var id = package.Ping();
``