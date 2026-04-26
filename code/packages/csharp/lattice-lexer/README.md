# csharp/lattice-lexer

Lattice lexer - tokenizes Lattice source text using the grammar-driven lexer infrastructure.

This package mirrors the Java and Kotlin lattice-lexer package surface while reserving the dependency shape for the shared grammar-driven infrastructure.

## Usage

``csharp
var package = new LatticeLexer();
var id = package.Ping();
``