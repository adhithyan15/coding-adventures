# csharp/typescript-lexer

TypeScript lexer - tokenizes TypeScript source text using the grammar-driven lexer infrastructure.

This package mirrors the Java and Kotlin typescript-lexer package surface while reserving the dependency shape for the shared grammar-driven infrastructure.

## Usage

``csharp
var package = new TypeScriptLexer();
var id = package.Ping();
``