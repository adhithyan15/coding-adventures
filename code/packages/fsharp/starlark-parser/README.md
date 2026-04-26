# fsharp/starlark-parser

Starlark parser - parses Starlark source text using the grammar-driven parser infrastructure.

This package mirrors the Java and Kotlin starlark-parser package surface while reserving the dependency shape for the shared grammar-driven infrastructure.

## Usage

``fsharp
let package = StarlarkParser()
let id = package.Ping()
``