# ECMAScript 3 (1999) Parser (TypeScript)

Parses ECMAScript 3 source code into abstract syntax trees (ASTs) using the grammar-driven parser approach.

## What Is This?

This package is a **thin wrapper** around the generic `GrammarParser` from the `@coding-adventures/parser` package. It loads `es3.grammar` and delegates all parsing to the generic engine.

ES3 added try/catch/finally, throw, strict equality (===, !==), instanceof, and regex literals to the ES1 grammar.

## Usage

```typescript
import { parseEs3 } from "@coding-adventures/ecmascript-es3-parser";

const ast = parseEs3("try { x; } catch (e) { y; }");
console.log(ast.ruleName); // "program"
```

## Dependencies

- `@coding-adventures/ecmascript-es3-lexer` -- tokenizes ES3 source code
- `@coding-adventures/parser` -- provides `GrammarParser` and `ASTNode`
- `@coding-adventures/grammar-tools` -- parses `.grammar` files
