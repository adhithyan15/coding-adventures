# fsharp/json-lexer

JSON lexer - tokenizes JSON source text using the grammar-driven lexer infrastructure.

This package mirrors the Java and Kotlin json-lexer package surface while reserving the dependency shape for the shared grammar-driven infrastructure.

## Usage

``fsharp
let package = JsonLexer()
let id = package.Ping()
``