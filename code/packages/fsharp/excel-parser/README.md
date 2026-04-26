# fsharp/excel-parser

Excel parser - parses Excel source text using the grammar-driven parser infrastructure.

This package mirrors the Java and Kotlin excel-parser package surface while reserving the dependency shape for the shared grammar-driven infrastructure.

## Usage

``fsharp
let package = ExcelParser()
let id = package.Ping()
``