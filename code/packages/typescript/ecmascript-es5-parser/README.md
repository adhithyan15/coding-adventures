# ECMAScript 5 (2009) Parser (TypeScript)

Parses ECMAScript 5 source code into abstract syntax trees (ASTs) using the grammar-driven parser approach.

## What Is This?

This package is a **thin wrapper** around the generic `GrammarParser` from the `@coding-adventures/parser` package. It loads `es5.grammar` and delegates all parsing to the generic engine.

ES5 adds getter/setter properties in object literals and the `debugger` statement to the ES3 grammar.

## Usage

```typescript
import { parseEs5 } from "@coding-adventures/ecmascript-es5-parser";

const ast = parseEs5("debugger;");
console.log(ast.ruleName); // "program"
```

## Dependencies

- `@coding-adventures/ecmascript-es5-lexer` -- tokenizes ES5 source code
- `@coding-adventures/parser` -- provides `GrammarParser` and `ASTNode`
- `@coding-adventures/grammar-tools` -- parses `.grammar` files
