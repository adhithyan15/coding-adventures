# C# Parser (TypeScript)

Parses C# source code into an Abstract Syntax Tree (AST) using the grammar-driven parser approach.

## What Is This?

This package is a **thin wrapper** around the generic `GrammarParser` from the `@coding-adventures/parser` package. It loads the appropriate `csharp{version}.grammar` file, tokenizes the source using `@coding-adventures/csharp-lexer`, and delegates all parsing to the generic engine.

## Usage

```typescript
import { parseCSharp, createCSharpParser } from "@coding-adventures/csharp-parser";

// Default version (C# 12.0)
const ast = parseCSharp("class Hello { }");
console.log(ast.ruleName); // "program"

// Specific version
const ast8 = parseCSharp("int x = 1 + 2;", "8.0");

// Factory function for more control
const parser = createCSharpParser("int x = 42;", "12.0");
const ast = parser.parse();
```

## Supported C# versions

| Version | Grammar file | Notable additions |
|---------|-------------|-------------------|
| `"1.0"` | `grammars/csharp/csharp1.0.grammar` | Original C# |
| `"2.0"` | `grammars/csharp/csharp2.0.grammar` | Generics, nullable types |
| `"3.0"` | `grammars/csharp/csharp3.0.grammar` | LINQ, `var`, lambda `=>` |
| `"4.0"` | `grammars/csharp/csharp4.0.grammar` | `dynamic`, named/optional params |
| `"5.0"` | `grammars/csharp/csharp5.0.grammar` | `async`/`await` |
| `"6.0"` | `grammars/csharp/csharp6.0.grammar` | String interpolation, `?.` |
| `"7.0"` | `grammars/csharp/csharp7.0.grammar` | Tuples, pattern matching |
| `"8.0"` | `grammars/csharp/csharp8.0.grammar` | Nullable reference types |
| `"9.0"` | `grammars/csharp/csharp9.0.grammar` | Records, top-level programs |
| `"10.0"` | `grammars/csharp/csharp10.0.grammar` | Global usings, file-scoped namespaces |
| `"11.0"` | `grammars/csharp/csharp11.0.grammar` | Required members, list patterns |
| `"12.0"` (default) | `grammars/csharp/csharp12.0.grammar` | Primary constructors, collection expressions |

## AST structure

The root node always has `ruleName === "program"`. Child rules include:

- `var_declaration` — typed variable declarations (`int x = 1;`)
- `expression_stmt` — standalone expressions ending in `;`
- `assignment` — assignment expressions (`x = 5;`)
- `expression` — additive expressions
- `term` — multiplicative expressions
- `factor` — literals, identifiers, and parenthesized expressions

## Dependencies

- `@coding-adventures/csharp-lexer` — tokenizes C# source using the matching tokens grammar
- `@coding-adventures/parser` — provides `GrammarParser` and `ASTNode`
- `@coding-adventures/grammar-tools` — parses `.grammar` files
