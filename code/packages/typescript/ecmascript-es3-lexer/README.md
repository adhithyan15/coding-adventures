# ECMAScript 3 (1999) Lexer (TypeScript)

Tokenizes ECMAScript 3 source code using the grammar-driven lexer approach.

## What Is This?

This package is a **thin wrapper** around the generic `grammarTokenize` from the `@coding-adventures/lexer` package. It loads the `es3.tokens` grammar file and delegates all tokenization to the generic engine.

ES3 was the version that made JavaScript a real, complete language. It added strict equality (`===`, `!==`), error handling (`try/catch/finally/throw`), regular expression literals, and the `instanceof` operator.

## Usage

```typescript
import { tokenizeEs3 } from "@coding-adventures/ecmascript-es3-lexer";

const tokens = tokenizeEs3("var x = 1 === 2;");
for (const token of tokens) {
  console.log(token);
}
```

## Dependencies

- `@coding-adventures/lexer` -- provides `grammarTokenize` and `Token`
- `@coding-adventures/grammar-tools` -- parses `.tokens` files
