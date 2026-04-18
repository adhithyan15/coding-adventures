# @coding-adventures/nib-lexer

Tokenizes Nib source text using the shared grammar-driven TypeScript lexer.

## What It Does

This package is intentionally thin:

- reads `code/grammars/nib.tokens`
- parses that grammar with `@coding-adventures/grammar-tools`
- tokenizes source via `@coding-adventures/lexer`
- reclassifies `KEYWORD` tokens to the concrete lowercase Nib keyword text
- optionally preserves source offsets and skip-stream trivia for formatter paths

That keeps the language definition in one place while still giving TypeScript a
real Nib frontend entry point.

## Usage

```ts
import { tokenizeNib } from "@coding-adventures/nib-lexer";

const tokens = tokenizeNib("let x: u4 = 0xF;");
```

Formatter-oriented callers can opt into richer source information:

```ts
const richTokens = tokenizeNib("// lead\nconst MAX: u4 = 10;", {
  preserveSourceInfo: true,
});

console.log(richTokens[0]?.leadingTrivia);
```
