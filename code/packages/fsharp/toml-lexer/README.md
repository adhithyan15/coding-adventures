# fsharp/toml-lexer

TOML lexer - tokenizes TOML source text using the grammar-driven lexer infrastructure.

This package mirrors the Java and Kotlin toml-lexer package surface while reserving the dependency shape for the shared grammar-driven infrastructure.

## Usage

``fsharp
let package = TomlLexer()
let id = package.Ping()
``