# Java Parser (Go)

Parses Java source code into ASTs using the grammar-driven parser engine. A
thin wrapper that loads the appropriate `.grammar` file and delegates parsing
to the generic `GrammarParser`.

## Usage

```go
import javaparser "github.com/adhithyan15/coding-adventures/code/packages/go/java-parser"

// Default grammar (Java 21) — best when you don't know the exact Java version.
ast, err := javaparser.ParseJava("int x = 1 + 2;", "")

// Versioned grammar — pin to a specific Java release.
ast, err := javaparser.ParseJava("var x = 1;", "10")
```

## Supported versions

| Version | Release         | Year |
|---------|-----------------|------|
| `""`    | Default (21)    | —    |
| `"1.0"` | Java 1.0       | 1996 |
| `"1.1"` | Java 1.1       | 1997 |
| `"1.4"` | Java 1.4       | 2002 |
| `"5"`   | Java 5         | 2004 |
| `"7"`   | Java 7         | 2011 |
| `"8"`   | Java 8         | 2014 |
| `"10"`  | Java 10        | 2018 |
| `"14"`  | Java 14        | 2020 |
| `"17"`  | Java 17        | 2021 |
| `"21"`  | Java 21        | 2023 |

Passing an unrecognised version string returns a descriptive error immediately,
preventing silent fallback to the wrong grammar.
