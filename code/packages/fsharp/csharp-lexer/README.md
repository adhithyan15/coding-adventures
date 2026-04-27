# fsharp/csharp-lexer

C# lexer - tokenizes C# source text using the grammar-driven lexer infrastructure.

This package mirrors the Java and Kotlin csharp-lexer package surface while reserving the dependency shape for the shared grammar-driven infrastructure.

## Usage

``fsharp
let package = CSharpLexer()
let id = package.Ping()
``