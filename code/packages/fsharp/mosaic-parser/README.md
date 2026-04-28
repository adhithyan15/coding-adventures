# fsharp/mosaic-parser

Mosaic parser - parses Mosaic source text using the grammar-driven parser infrastructure.

This package mirrors the Java and Kotlin mosaic-parser package surface while reserving the dependency shape for the shared grammar-driven infrastructure.

## Usage

``fsharp
let package = MosaicParser()
let id = package.Ping()
``