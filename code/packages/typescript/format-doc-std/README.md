# @coding-adventures/format-doc-std

Reusable syntax templates for `@coding-adventures/format-doc`.

This package is the first shared template layer on top of the document algebra.
It covers the recurring syntax shapes that most language-specific formatters
need: delimited lists, call-like forms, block-like forms, and infix chains.

## Usage

```ts
import { text } from "@coding-adventures/format-doc";
import { callLike, delimitedList, infixChain } from "@coding-adventures/format-doc-std";

const args = delimitedList({
  open: text("["),
  close: text("]"),
  items: [text("a"), text("b"), text("c")],
});

const call = callLike(text("sum"), [text("left"), text("right")]);

const chain = infixChain({
  operands: [text("a"), text("b"), text("c")],
  operators: [text("+"), text("+")],
});
```

## Exports

- `delimitedList()` — shared list formatting
- `callLike()` — callee plus delimited arguments
- `blockLike()` — opener/body/closer blocks
- `infixChain()` — operand/operator chains

## Related packages

- `@coding-adventures/format-doc` — core document algebra
- future language formatter packages — AST-to-`Doc` printers that reuse these helpers
