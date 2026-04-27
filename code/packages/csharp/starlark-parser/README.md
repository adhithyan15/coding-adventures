# csharp/starlark-parser

Starlark parser - parses Starlark source text using the grammar-driven parser infrastructure.

This package mirrors the Java and Kotlin starlark-parser package surface while reserving the dependency shape for the shared grammar-driven infrastructure.

## Usage

``csharp
var package = new StarlarkParser();
var id = package.Ping();
``