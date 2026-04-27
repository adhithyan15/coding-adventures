# csharp/json-parser

JSON parser - parses JSON source text using the grammar-driven parser infrastructure.

This package mirrors the Java and Kotlin json-parser package surface while reserving the dependency shape for the shared grammar-driven infrastructure.

## Usage

``csharp
var package = new JsonParser();
var id = package.Ping();
``