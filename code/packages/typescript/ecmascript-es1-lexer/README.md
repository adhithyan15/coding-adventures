# ECMAScript 1 (1997) Lexer (TypeScript)

Tokenizes ECMAScript 1 source code using the grammar-driven lexer approach.

## What Is This?

This package is a **thin wrapper** around the generic `grammarTokenize` from the `@coding-adventures/lexer` package. It loads the `es1.tokens` grammar file and delegates all tokenization to the generic engine.

ES1 was the first standardized version of JavaScript (June 1997). It has `var` declarations, basic operators (no `===`), and no try/catch or regex literals.

## Usage

```typescript
import { tokenizeEs1 } from "@coding-adventures/ecmascript-es1-lexer";

const tokens = tokenizeEs1("var x = 1 + 2;");
for (const token of tokens) {
  console.log(token);
}
```

## Dependencies

- `@coding-adventures/lexer` -- provides `grammarTokenize` and `Token`
- `@coding-adventures/grammar-tools` -- parses `.tokens` files
