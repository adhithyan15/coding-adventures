# C# Parser (Go)

Parses C# source code into ASTs using the grammar-driven parser engine. A
thin wrapper that loads the appropriate `.grammar` file and delegates parsing
to the generic `GrammarParser`.

## Usage

```go
import csharpparser "github.com/adhithyan15/coding-adventures/code/packages/go/csharp-parser"

// Default grammar (C# 12.0) — best when you don't know the exact C# version.
ast, err := csharpparser.ParseCSharp("int x = 1 + 2;", "")

// Versioned grammar — pin to a specific C# release.
ast, err := csharpparser.ParseCSharp("var x = 1;", "3.0")
```

## Supported versions

| Version  | Release    | Year | Notable features                           |
|----------|------------|------|--------------------------------------------|
| `""`     | Default (12.0) | —  | —                                         |
| `"1.0"`  | C# 1.0     | 2002 | Original release with .NET 1.0             |
| `"2.0"`  | C# 2.0     | 2005 | Generics, iterators, nullable value types  |
| `"3.0"`  | C# 3.0     | 2007 | LINQ, lambdas, auto-properties, var        |
| `"4.0"`  | C# 4.0     | 2010 | dynamic, named/optional parameters         |
| `"5.0"`  | C# 5.0     | 2012 | async/await                                |
| `"6.0"`  | C# 6.0     | 2015 | String interpolation, null-conditional     |
| `"7.0"`  | C# 7.0     | 2017 | Tuples, pattern matching, local functions  |
| `"8.0"`  | C# 8.0     | 2019 | Nullable reference types, async streams    |
| `"9.0"`  | C# 9.0     | 2020 | Records, top-level statements              |
| `"10.0"` | C# 10.0    | 2021 | Global usings, file-scoped namespaces      |
| `"11.0"` | C# 11.0    | 2022 | Required members, raw string literals      |
| `"12.0"` | C# 12.0    | 2023 | Primary constructors, collection expressions |

Passing an unrecognised version string returns a descriptive error immediately,
preventing silent fallback to the wrong grammar. Note that all C# version strings
use the "X.0" format (e.g. `"12.0"`, not `"12"`).
