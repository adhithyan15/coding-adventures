# fsharp/python-lexer

Python lexer - tokenizes Python source text using the grammar-driven lexer infrastructure.

This package mirrors the Java and Kotlin python-lexer package surface while reserving the dependency shape for the shared grammar-driven infrastructure.

## Usage

``fsharp
let package = PythonLexer()
let id = package.Ping()
``