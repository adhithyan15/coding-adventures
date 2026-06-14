# @coding-adventures/symbolic-ir

Pure TypeScript symbolic expression IR for computer algebra systems.

The IR has six node forms:

| Kind | Meaning |
|---|---|
| `symbol` | Named atom such as `x`, `%pi`, or `Add` |
| `integer` | Exact arbitrary-precision integer backed by `bigint` |
| `rational` | Exact reduced fraction backed by `bigint` |
| `float` | JavaScript double |
| `string` | String literal |
| `apply` | Uniform compound expression: `head(args...)` |

The package has no host dependencies and is safe to use in browsers.

```ts
import { ADD, POW, app, int, sym, toDisplayString } from "@coding-adventures/symbolic-ir";

const x = sym("x");
const expr = app(ADD, [app(POW, [x, int(2)]), int(1)]);

toDisplayString(expr); // "Add(Pow(x, 2), 1)"
```
