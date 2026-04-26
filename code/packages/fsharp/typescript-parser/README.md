# fsharp/typescript-parser

TypeScript parser - parses TypeScript source text using the grammar-driven parser infrastructure.

This package mirrors the Java and Kotlin typescript-parser package surface while reserving the dependency shape for the shared grammar-driven infrastructure.

## Usage

``fsharp
let package = TypeScriptParser()
let id = package.Ping()
``