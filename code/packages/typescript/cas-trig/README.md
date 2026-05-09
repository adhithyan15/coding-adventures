# cas-trig (TypeScript)

Pure TypeScript trigonometric operations over symbolic IR expressions. The
package mirrors the Rust `cas-trig` crate and runs without native bindings.

## Operations

| Function | Description |
|---|---|
| `sinEval`, `cosEval`, `tanEval` | Exact special values, numeric evaluation, or unevaluated trig nodes |
| `atanEval`, `asinEval`, `acosEval` | Numeric inverse trig evaluation with symbolic fallback |
| `trigSimplify(expr)` | Bottom-up simplification of trig nodes |
| `expandTrig(expr)` | Angle-addition and double-angle expansion for `Sin`/`Cos` |
| `powerReduce(expr)` | Half-angle reduction for `Sin(x)^2` and `Cos(x)^2` |
| `extractPiMultiple(expr)` | Recognize `(n/d) * Pi` shapes |

Special values are exact for rational multiples of `Pi` with reduced
denominators in `{1, 2, 3, 4, 6}`.
