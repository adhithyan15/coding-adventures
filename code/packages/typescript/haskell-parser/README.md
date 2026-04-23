# Haskell Parser (TypeScript)

Parses Haskell source code into abstract syntax trees (ASTs) using the grammar-driven parser approach.

## What Is This?

This package is a **thin wrapper** around the generic `GrammarParser` from the `@coding-adventures/parser` package. It loads the appropriate `haskell{version}.grammar` and delegates all parsing to the generic engine.

## Usage

```typescript
import { parseHaskell, createHaskellParser } from "@coding-adventures/haskell-parser";

// Default version (Haskell 21)
const ast = parseHaskell("class Hello { }");
console.log(ast.ruleName); // "program"

// Specific version
const ast8 = parseHaskell("int x = 1 + 2;", "8");

// Factory function for more control
const parser = createHaskellParser("int x = 42;", "21");
const ast = parser.parse();
```

## Supported Haskell versions

| Version | Grammar files |
|---------|--------------|
| `"1.0"` | `grammars/haskell/haskell1.0.{tokens,grammar}` |
| `"1.1"` | `grammars/haskell/haskell1.1.{tokens,grammar}` |
| `"1.4"` | `grammars/haskell/haskell1.4.{tokens,grammar}` |
| `"5"` | `grammars/haskell/haskell5.{tokens,grammar}` |
| `"7"` | `grammars/haskell/haskell7.{tokens,grammar}` |
| `"8"` | `grammars/haskell/haskell8.{tokens,grammar}` |
| `"10"` | `grammars/haskell/haskell10.{tokens,grammar}` |
| `"14"` | `grammars/haskell/haskell14.{tokens,grammar}` |
| `"17"` | `grammars/haskell/haskell17.{tokens,grammar}` |
| `"21"` (default) | `grammars/haskell/haskell21.{tokens,grammar}` |

## Dependencies

- `@coding-adventures/haskell-lexer` -- tokenizes Haskell source code
- `@coding-adventures/parser` -- provides `GrammarParser` and `ASTNode`
- `@coding-adventures/grammar-tools` -- parses `.grammar` files
