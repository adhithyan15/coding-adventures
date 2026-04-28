# csharp/lattice-parser

Lattice parser - parses Lattice source text using the grammar-driven parser infrastructure.

This package mirrors the Java and Kotlin lattice-parser package surface while reserving the dependency shape for the shared grammar-driven infrastructure.

## Usage

``csharp
var package = new LatticeParser();
var id = package.Ping();
``