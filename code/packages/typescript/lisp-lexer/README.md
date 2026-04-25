# Lisp Lexer (TypeScript)

Grammar-driven Lisp lexer for TypeScript.

This package loads `code/grammars/lisp.tokens` and delegates tokenization to `@coding-adventures/lexer`.

```ts
import { tokenizeLisp } from "@coding-adventures/lisp-lexer";

const tokens = tokenizeLisp("(define x 42)");
```
