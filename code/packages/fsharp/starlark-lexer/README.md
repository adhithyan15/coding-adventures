# fsharp/starlark-lexer

Starlark lexer - tokenizes Starlark source text using the grammar-driven lexer infrastructure.

This package mirrors the Java and Kotlin starlark-lexer package surface while reserving the dependency shape for the shared grammar-driven infrastructure.

## Usage

``fsharp
let package = StarlarkLexer()
let id = package.Ping()
``