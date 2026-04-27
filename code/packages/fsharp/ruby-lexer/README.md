# fsharp/ruby-lexer

Ruby lexer - tokenizes Ruby source text using the grammar-driven lexer infrastructure.

This package mirrors the Java and Kotlin ruby-lexer package surface while reserving the dependency shape for the shared grammar-driven infrastructure.

## Usage

``fsharp
let package = RubyLexer()
let id = package.Ping()
``