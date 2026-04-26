# Lisp Parser (TypeScript)

Grammar-driven Lisp parser for TypeScript.

This package tokenizes source with `@coding-adventures/lisp-lexer`, loads `code/grammars/lisp.grammar`, and delegates parsing to `@coding-adventures/parser`.

```ts
import { parseLisp } from "@coding-adventures/lisp-parser";

const ast = parseLisp("(+ 1 2)");
```
