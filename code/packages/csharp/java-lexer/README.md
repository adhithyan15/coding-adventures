# csharp/java-lexer

Java lexer - tokenizes Java source text using the grammar-driven lexer infrastructure.

This package mirrors the Java and Kotlin java-lexer package surface while reserving the dependency shape for the shared grammar-driven infrastructure.

## Usage

``csharp
var package = new JavaLexer();
var id = package.Ping();
``