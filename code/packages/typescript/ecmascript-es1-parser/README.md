# ECMAScript 1 (1997) Parser (TypeScript)

Parses ECMAScript 1 source code into abstract syntax trees (ASTs) using the grammar-driven parser approach.

## What Is This?

This package is a **thin wrapper** around the generic `GrammarParser` from the `@coding-adventures/parser` package. It loads `es1.grammar` and delegates all parsing to the generic engine.

ES1 was the first standardized JavaScript. It supports `var` declarations, function declarations/expressions, all basic statement types (no try/catch), and the full expression precedence chain.

## Usage

```typescript
import { parseEs1 } from "@coding-adventures/ecmascript-es1-parser";

const ast = parseEs1("var x = 1 + 2;");
console.log(ast.ruleName); // "program"
```

## Dependencies

- `@coding-adventures/ecmascript-es1-lexer` -- tokenizes ES1 source code
- `@coding-adventures/parser` -- provides `GrammarParser` and `ASTNode`
- `@coding-adventures/grammar-tools` -- parses `.grammar` files
