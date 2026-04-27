# csharp/typescript-parser

TypeScript parser - parses TypeScript source text using the grammar-driven parser infrastructure.

This package mirrors the Java and Kotlin typescript-parser package surface while reserving the dependency shape for the shared grammar-driven infrastructure.

## Usage

``csharp
var package = new TypeScriptParser();
var id = package.Ping();
``