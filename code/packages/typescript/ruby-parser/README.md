# Ruby Parser (TypeScript)

Parses Ruby source code into abstract syntax trees (ASTs) using the grammar-driven parser approach.

## What Is This?

This package is a **thin wrapper** around the generic `GrammarParser` from the `@coding-adventures/parser` package. It loads `ruby.grammar` and delegates all parsing to the generic engine.

## Usage

```typescript
import { parseRuby } from "@coding-adventures/ruby-parser";

const ast = parseRuby("x = 1 + 2");
console.log(ast.ruleName); // "program"
```

## Dependencies

- `@coding-adventures/ruby-lexer` -- tokenizes Ruby source code
- `@coding-adventures/parser` -- provides `GrammarParser` and `ASTNode`
- `@coding-adventures/grammar-tools` -- parses `.grammar` files
