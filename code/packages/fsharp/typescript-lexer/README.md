# fsharp/typescript-lexer

TypeScript lexer - tokenizes TypeScript source text using the grammar-driven lexer infrastructure.

This package mirrors the Java and Kotlin typescript-lexer package surface while reserving the dependency shape for the shared grammar-driven infrastructure.

## Usage

``fsharp
let package = TypeScriptLexer()
let id = package.Ping()
``