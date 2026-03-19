# Python Lexer (TypeScript)

Tokenizes Python source code using the grammar-driven lexer approach.

## What Is This?

This package is a **thin wrapper** around the generic `grammarTokenize` from the `@coding-adventures/lexer` package. It demonstrates a core principle of the grammar-driven architecture: the same lexer engine that tokenizes one language can tokenize Python by simply loading a different `.tokens` file.

No new lexer code is needed. The `python.tokens` file in `code/grammars/` declares Python's token definitions (keywords, operators, literals), and the `grammarTokenize` function reads those declarations at runtime.

## How It Fits in the Stack

```
python.tokens (grammar file)
    |
    v
parseTokenGrammar()            -- parses the .tokens file
    |
    v
grammarTokenize()              -- generic tokenization engine
    |
    v
tokenizePython()               -- thin wrapper (this package)
```

## Usage

```typescript
import { tokenizePython } from "@coding-adventures/python-lexer";

const tokens = tokenizePython("x = 1 + 2");
for (const token of tokens) {
  console.log(token);
}
// Token(NAME, "x", 1:1)
// Token(EQUALS, "=", 1:3)
// Token(NUMBER, "1", 1:5)
// Token(PLUS, "+", 1:7)
// Token(NUMBER, "2", 1:9)
// Token(EOF, "", 1:10)
```

## Dependencies

- `@coding-adventures/lexer` -- provides `grammarTokenize` and `Token`
- `@coding-adventures/grammar-tools` -- parses `.tokens` files
