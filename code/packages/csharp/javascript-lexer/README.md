# csharp/javascript-lexer

JavaScript lexer - tokenizes JavaScript source text using the grammar-driven lexer infrastructure.

This package mirrors the Java and Kotlin javascript-lexer package surface while reserving the dependency shape for the shared grammar-driven infrastructure.

## Usage

``csharp
var package = new JavaScriptLexer();
var id = package.Ping();
``