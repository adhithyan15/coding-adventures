# cas-ode

Pure TypeScript symbolic ODE solver for the `@coding-adventures/symbolic-ir`
expression tree.

The package is browser-safe: it does not call Python, a VM, a parser, Node-only
APIs, or dynamic evaluation. Public APIs accept existing symbolic IR nodes and
return symbolic IR nodes.

## Supported families

| Family | Form |
| --- | --- |
| First-order linear | `D(y,x) + P(x)*y = Q(x)` |
| Separable | `D(y,x) = f(x)*g(y)` |
| Bernoulli | `D(y,x) + P(x)*y = Q(x)*y^n` |
| Exact | `M(x,y) + N(x,y)*D(y,x) = 0` |
| Homogeneous type | `D(y,x) = f(y/x)` |
| Second-order constant-coefficient homogeneous | `a*y'' + b*y' + c*y = 0` |
| Second-order constant-coefficient nonhomogeneous | polynomial, exponential, sine, and cosine forcing, with variation-of-parameters fallback |
| Euler-Cauchy | `a*x^2*y'' + b*x*y' + c*y = 0` |

When an exact primitive is outside the built-in integration table, the solver
returns symbolic `Integrate(expr, x)` nodes rather than failing the whole ODE.
For homogeneous-type equations this includes implicit solutions such as
`Equal(Integrate(1/(f(y/x)-y/x), y/x), Log(x)+%c)`.

## Usage

```ts
import { ADD, D, MUL, SUB, app, int, sym } from "@coding-adventures/symbolic-ir";
import { ode2 } from "@coding-adventures/cas-ode";

const x = sym("x");
const y = sym("y");
const yPrime = app(D, [y, x]);

const equation = app(SUB, [yPrime, app(MUL, [int(2), y])]);
const result = ode2(equation, y, x);
```

`ode2(equation, y, x)` returns `Equal(y, ...)` when a supported solver matches,
or the unevaluated `ODE2(equation, y, x)` node otherwise.
