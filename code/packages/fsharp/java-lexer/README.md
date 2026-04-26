# fsharp/java-lexer

Java lexer - tokenizes Java source text using the grammar-driven lexer infrastructure.

This package mirrors the Java and Kotlin java-lexer package surface while reserving the dependency shape for the shared grammar-driven infrastructure.

## Usage

``fsharp
let package = JavaLexer()
let id = package.Ping()
``