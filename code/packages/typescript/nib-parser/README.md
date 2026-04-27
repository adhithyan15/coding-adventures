# @coding-adventures/nib-parser

Parses Nib source text into ASTs using the shared grammar-driven TypeScript
parser.

## What It Does

This package wires together:

- `code/grammars/nib.grammar`
- `@coding-adventures/nib-lexer`
- `@coding-adventures/parser`

The result is a thin, language-specific entry point that produces the same kind
of grammar-driven AST shape as the other parser packages in the repo.

## Usage

```ts
import { parseNib } from "@coding-adventures/nib-parser";

const ast = parseNib("fn main() { let x: u4 = 5; }");
console.log(ast.ruleName); // "program"
```

Formatter-oriented callers can preserve source info on AST nodes:

```ts
const ast = parseNib("// lead\nfn main() { return 0; }", {
  preserveSourceInfo: true,
});

console.log(ast.leadingTrivia);
```

When a formatter also needs EOF trivia, use `parseNibDocument()`:

```ts
import { parseNibDocument } from "@coding-adventures/nib-parser";

const document = parseNibDocument("fn main() { return 0; } // tail", {
  preserveSourceInfo: true,
});

console.log(document.tokens.at(-1)?.leadingTrivia);
```
