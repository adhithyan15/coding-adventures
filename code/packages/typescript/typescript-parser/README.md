# TypeScript Parser (TypeScript)

Parses TypeScript source code into abstract syntax trees (ASTs) using the grammar-driven parser approach.

## What Is This?

This package is a **thin wrapper** around the generic `GrammarParser` from the `@coding-adventures/parser` package. It loads `typescript.grammar` and delegates all parsing to the generic engine.

## Usage

```typescript
import { parseTypescript } from "@coding-adventures/typescript-parser";

const ast = parseTypescript("let x = 1 + 2;");
console.log(ast.ruleName); // "program"
```

## Dependencies

- `@coding-adventures/typescript-lexer` -- tokenizes TypeScript source code
- `@coding-adventures/parser` -- provides `GrammarParser` and `ASTNode`
- `@coding-adventures/grammar-tools` -- parses `.grammar` files
