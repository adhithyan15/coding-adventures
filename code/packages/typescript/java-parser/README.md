# Java Parser (TypeScript)

Parses Java source code into abstract syntax trees (ASTs) using the grammar-driven parser approach.

## What Is This?

This package is a **thin wrapper** around the generic `GrammarParser` from the `@coding-adventures/parser` package. It loads the appropriate `java{version}.grammar` and delegates all parsing to the generic engine.

## Usage

```typescript
import { parseJava, createJavaParser } from "@coding-adventures/java-parser";

// Default version (Java 21)
const ast = parseJava("class Hello { }");
console.log(ast.ruleName); // "program"

// Specific version
const ast8 = parseJava("int x = 1 + 2;", "8");

// Factory function for more control
const parser = createJavaParser("int x = 42;", "21");
const ast = parser.parse();
```

## Supported Java versions

| Version | Grammar files |
|---------|--------------|
| `"1.0"` | `grammars/java/java1.0.{tokens,grammar}` |
| `"1.1"` | `grammars/java/java1.1.{tokens,grammar}` |
| `"1.4"` | `grammars/java/java1.4.{tokens,grammar}` |
| `"5"` | `grammars/java/java5.{tokens,grammar}` |
| `"7"` | `grammars/java/java7.{tokens,grammar}` |
| `"8"` | `grammars/java/java8.{tokens,grammar}` |
| `"10"` | `grammars/java/java10.{tokens,grammar}` |
| `"14"` | `grammars/java/java14.{tokens,grammar}` |
| `"17"` | `grammars/java/java17.{tokens,grammar}` |
| `"21"` (default) | `grammars/java/java21.{tokens,grammar}` |

## Dependencies

- `@coding-adventures/java-lexer` -- tokenizes Java source code
- `@coding-adventures/parser` -- provides `GrammarParser` and `ASTNode`
- `@coding-adventures/grammar-tools` -- parses `.grammar` files
