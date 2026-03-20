# Starlark Lexer (TypeScript)

Tokenizes Starlark source code using the grammar-driven lexer approach.

## What Is This?

This package is a **thin wrapper** around the generic `grammarTokenize` from the `@coding-adventures/lexer` package. It loads the `starlark.tokens` grammar file and delegates all tokenization to the generic engine.

## What Is Starlark?

Starlark is a deterministic, hermetic dialect of Python designed by Google for the Bazel build system. It is the language used in BUILD files, .bzl files, and other build system configuration. Starlark intentionally omits Python features like `class`, `import`, `while`, `try/except`, and recursion to guarantee termination and reproducibility.

## Usage

```typescript
import { tokenizeStarlark } from "@coding-adventures/starlark-lexer";

const tokens = tokenizeStarlark("x = 1 + 2");
for (const token of tokens) {
  console.log(token);
}
```

## Key Features

- **Indentation tracking** — emits INDENT/DEDENT tokens for significant whitespace
- **Reserved keyword detection** — `class`, `import`, `while`, etc. cause immediate errors
- **Full operator support** — `**`, `//`, `+=`, `<<=`, and all other Starlark operators
- **String literals** — single, double, triple-quoted, raw, and bytes strings
- **Comment skipping** — `#` comments are consumed but produce no tokens

## Dependencies

- `@coding-adventures/lexer` -- provides `grammarTokenize` and `Token`
- `@coding-adventures/grammar-tools` -- parses `.tokens` files
