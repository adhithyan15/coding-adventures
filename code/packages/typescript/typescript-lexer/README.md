# TypeScript Lexer (TypeScript)

Tokenizes TypeScript source code using the grammar-driven lexer approach.

## What Is This?

This package is a **thin wrapper** around the generic `grammarTokenize` from the `@coding-adventures/lexer` package. It loads the `typescript.tokens` grammar file and delegates all tokenization to the generic engine.

## Usage

```typescript
import { tokenizeTypescript } from "@coding-adventures/typescript-lexer";

const tokens = tokenizeTypescript("let x: number = 1 + 2;");
for (const token of tokens) {
  console.log(token);
}
```

## Dependencies

- `@coding-adventures/lexer` -- provides `grammarTokenize` and `Token`
- `@coding-adventures/grammar-tools` -- parses `.tokens` files
