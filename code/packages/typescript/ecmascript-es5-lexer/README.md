# ECMAScript 5 (2009) Lexer (TypeScript)

Tokenizes ECMAScript 5 source code using the grammar-driven lexer approach.

## What Is This?

This package is a **thin wrapper** around the generic `grammarTokenize` from the `@coding-adventures/lexer` package. It loads the `es5.tokens` grammar file and delegates all tokenization to the generic engine.

ES5 landed a full decade after ES3. Its syntactic changes are modest (the `debugger` keyword, getter/setter syntax) — the real innovations were strict mode and property descriptors.

## Usage

```typescript
import { tokenizeEs5 } from "@coding-adventures/ecmascript-es5-lexer";

const tokens = tokenizeEs5("debugger;");
for (const token of tokens) {
  console.log(token);
}
```

## Dependencies

- `@coding-adventures/lexer` -- provides `grammarTokenize` and `Token`
- `@coding-adventures/grammar-tools` -- parses `.tokens` files
