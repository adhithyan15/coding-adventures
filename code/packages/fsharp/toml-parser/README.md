# fsharp/toml-parser

TOML parser - parses TOML source text using the grammar-driven parser infrastructure.

This package mirrors the Java and Kotlin toml-parser package surface while reserving the dependency shape for the shared grammar-driven infrastructure.

## Usage

``fsharp
let package = TomlParser()
let id = package.Ping()
``