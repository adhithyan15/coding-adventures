# @coding-adventures/css-parser

Grammar-driven CSS parser for TypeScript.

This package tokenizes CSS with `@coding-adventures/css-lexer`, loads the shared
`code/grammars/css.grammar` parser grammar, and returns the generic AST produced
by `@coding-adventures/parser`.

```ts
import { parseCss } from "@coding-adventures/css-parser";

const ast = parseCss("h1 { color: red; }");
console.log(ast.ruleName); // "stylesheet"
```
