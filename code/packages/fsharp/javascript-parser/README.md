# fsharp/javascript-parser

JavaScript parser - parses JavaScript source text using the grammar-driven parser infrastructure.

This package mirrors the Java and Kotlin javascript-parser package surface while reserving the dependency shape for the shared grammar-driven infrastructure.

## Usage

``fsharp
let package = JavaScriptParser()
let id = package.Ping()
``