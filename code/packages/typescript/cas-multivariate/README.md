# @coding-adventures/cas-multivariate

Pure TypeScript multivariate polynomial arithmetic for Symbolic IR.

This package ports the Python `cas-multivariate` core to browser-safe TypeScript:

- sparse rational multivariate polynomials over Q
- lex, grlex, and grevlex monomial orders
- multivariate reduction and S-polynomials
- Buchberger Groebner basis computation with safety caps
- small rational ideal solving through lex bases and back-substitution
- conversion helpers and handlers for Symbolic IR operations

The package does not include a lexer or parser. Callers construct Symbolic IR directly with `@coding-adventures/symbolic-ir` or use the `MPoly` APIs.

## Example

```ts
import { ADD, LIST, SUB, app, int, sym } from "@coding-adventures/symbolic-ir";
import { IDEAL_SOLVE, idealSolveHandler } from "@coding-adventures/cas-multivariate";

const x = sym("x");
const y = sym("y");
const equations = app(LIST, [
  app(ADD, [x, y, int(-1)]),
  app(SUB, [x, y]),
]);
const variables = app(LIST, [x, y]);

const result = idealSolveHandler(app(IDEAL_SOLVE, [equations, variables]));
// List(List(Rule(x, 1/2), Rule(y, 1/2)))
```
