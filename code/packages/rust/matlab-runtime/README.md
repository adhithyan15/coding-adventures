# MATLAB Runtime

A tree-walking evaluator for the [MATLAB](https://en.wikipedia.org/wiki/MATLAB)
language, over [`array-runtime`](../array-runtime). Item **MA-3d** of the MATLAB
frontend (spec [`MA01`](../../../specs/MA01-matlab-language.md)): the piece that
makes the MATLAB lexer/parser executable.

## The payoff

A matrix product `A * B` lowers to `array_runtime::execute(MatMul, …)`, which
**plans the operation and runs it on the cheapest available backend** — CPU
today, a GPU executor the moment one is registered. So matrix acceleration is
automatic and *by cost*, with no `gpuArray` and no language-level GPU code: the
whole MA-1 → MA-2 substrate lights up through real MATLAB syntax.

```rust
use coding_adventures_matlab_runtime::eval;
// A * A is executed through the array-runtime planner → [[7 10], [15 22]].
let out = eval("A = [1 2; 3 4]; A * A\n").unwrap();
assert!(out.contains('7') && out.contains("22"));
```

Unlike R (which reuses the shared S evaluator), MATLAB has its **own** evaluator,
because its value model is the `array-runtime` `Array` and its semantics are
MATLAB's: 1-based, column-major, matrix-first.

## What it evaluates

- Scalars, **matrix literals** `[1 2; 3 4]`, **ranges** `a:b`/`a:b:c`.
- Operators: `+ - .* ./ .^` (element-wise, broadcasting, via `array_runtime::ops`),
  `*` (matrix product, via `execute`; scalar `*` is element-wise), unary `- ~`,
  transpose `'`/`.'`, comparisons (`== ~= < > <= >=` → `0`/`1`), `& | && ||`.
- **Variables and assignment**, with `;`-suppressed display and the `x =` / `ans =`
  echo.
- **1-based indexing**: `A(i)` (linear, column-major), `A(i,j)`, `A(:,k)`,
  `A(i,:)`, `A(:)`, and the `A(end)` sentinel.
- **Control flow**: `if/elseif/else`, `for`, `while`.
- **Builtins**: `zeros`, `ones`, `eye`, `size`, `length`, `numel`, `sum`, `mean`,
  `max`, `min`, `abs`, `sqrt`, `transpose`, `disp`.

### Deferred (documented)

User `function` definitions, anonymous `@(x)`, cell arrays, `switch`/`try`
execution, matrix solve `\`/`/`, matrix power `^`, indexed assignment
(`A(i)=…`), and multi-assign `[a,b]=…`. Execution is `f32`-precision for `*`
(the `array-runtime` executor's dtype); element-wise ops stay exact `f64`.

## Testing

```sh
cargo test -p coding-adventures-matlab-runtime
```
