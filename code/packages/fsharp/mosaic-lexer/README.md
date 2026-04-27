# fsharp/mosaic-lexer

Mosaic lexer - tokenizes Mosaic source text using the grammar-driven lexer infrastructure.

This package mirrors the Java and Kotlin mosaic-lexer package surface while reserving the dependency shape for the shared grammar-driven infrastructure.

## Usage

``fsharp
let package = MosaicLexer()
let id = package.Ping()
``