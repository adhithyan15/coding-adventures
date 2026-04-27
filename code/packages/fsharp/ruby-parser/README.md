# fsharp/ruby-parser

Ruby parser - parses Ruby source text using the grammar-driven parser infrastructure.

This package mirrors the Java and Kotlin ruby-parser package surface while reserving the dependency shape for the shared grammar-driven infrastructure.

## Usage

``fsharp
let package = RubyParser()
let id = package.Ping()
``