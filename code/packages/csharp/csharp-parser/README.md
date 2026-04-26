# csharp/csharp-parser

C# parser - parses C# source text using the grammar-driven parser infrastructure.

This package mirrors the Java and Kotlin csharp-parser package surface while reserving the dependency shape for the shared grammar-driven infrastructure.

## Usage

``csharp
var package = new CSharpParser();
var id = package.Ping();
``