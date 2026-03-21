# JavaScript Parser (TypeScript)

Parses JavaScript source code into abstract syntax trees (ASTs) using the grammar-driven parser approach.

## What Is This?

This package is a **thin wrapper** around the generic `GrammarParser` from the `@coding-adventures/parser` package. It loads `javascript.grammar` and delegates all parsing to the generic engine.

## Usage

```typescript
import { parseJavascript } from "@coding-adventures/javascript-parser";

const ast = parseJavascript("let x = 1 + 2;");
console.log(ast.ruleName); // "program"
```

## Dependencies

- `@coding-adventures/javascript-lexer` -- tokenizes JavaScript source code
- `@coding-adventures/parser` -- provides `GrammarParser` and `ASTNode`
- `@coding-adventures/grammar-tools` -- parses `.grammar` files
