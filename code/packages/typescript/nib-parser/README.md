# @coding-adventures/nib-parser

Parses Nib source text into ASTs using the shared grammar-driven TypeScript
parser.

## What It Does

This package wires together:

- [`nib.grammar`](/C:/Users/adhit/Downloads/Codex/coding-adventures/code/grammars/nib.grammar)
- [`@coding-adventures/nib-lexer`](C:/Users/adhit/Downloads/Codex/coding-adventures/code/packages/typescript/nib-lexer)
- `@coding-adventures/parser`

The result is a thin, language-specific entry point that produces the same kind
of grammar-driven AST shape as the other parser packages in the repo.

## Usage

```ts
import { parseNib } from "@coding-adventures/nib-parser";

const ast = parseNib("fn main() { let x: u4 = 5; }");
console.log(ast.ruleName); // "program"
```
