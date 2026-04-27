# csharp/mosaic-parser

Mosaic parser - parses Mosaic source text using the grammar-driven parser infrastructure.

This package mirrors the Java and Kotlin mosaic-parser package surface while reserving the dependency shape for the shared grammar-driven infrastructure.

## Usage

``csharp
var package = new MosaicParser();
var id = package.Ping();
``