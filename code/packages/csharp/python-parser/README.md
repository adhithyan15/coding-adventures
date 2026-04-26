# csharp/python-parser

Python parser - parses Python source text using the grammar-driven parser infrastructure.

This package mirrors the Java and Kotlin python-parser package surface while reserving the dependency shape for the shared grammar-driven infrastructure.

## Usage

``csharp
var package = new PythonParser();
var id = package.Ping();
``