# csharp/java-parser

Java parser - parses Java source text using the grammar-driven parser infrastructure.

This package mirrors the Java and Kotlin java-parser package surface while reserving the dependency shape for the shared grammar-driven infrastructure.

## Usage

``csharp
var package = new JavaParser();
var id = package.Ping();
``