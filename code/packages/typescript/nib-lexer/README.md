# @coding-adventures/nib-lexer

Tokenizes Nib source text using the shared grammar-driven TypeScript lexer.

## What It Does

This package is intentionally thin:

- reads [`nib.tokens`](/C:/Users/adhit/Downloads/Codex/coding-adventures/code/grammars/nib.tokens)
- parses that grammar with `@coding-adventures/grammar-tools`
- tokenizes source via `@coding-adventures/lexer`
- reclassifies `KEYWORD` tokens to the concrete lowercase Nib keyword text

That keeps the language definition in one place while still giving TypeScript a
real Nib frontend entry point.

## Usage

```ts
import { tokenizeNib } from "@coding-adventures/nib-lexer";

const tokens = tokenizeNib("let x: u4 = 0xF;");
```
