# coding-adventures-macsyma-runtime

Rust MACSYMA runtime session facade over the statically linked MACSYMA compiler
and `symbolic-vm`.

This first slice is intentionally small: it compiles MACSYMA source, evaluates
statements through the Rust symbolic VM, preserves `;` versus `$` display
metadata, and records in-memory `%i`/`%o`-style history for a REPL/WASM facade.

History lookup is implemented in this runtime layer with a structural pre-eval
replacement pass. `%` resolves to the previous output, `%iN` resolves to the
N-th recorded input expression, and `%oN` resolves to the N-th recorded output.
The symbolic VM backend remains unchanged, so direct backend lookup of history
names is intentionally out of scope for this Rust parity slice.

The runtime also wires MACSYMA `linsolve`/`Solve(List(...), List(...))` calls to
the Rust `cas-solve` exact linear-system solver. Supported systems return
`List(Rule(variable, value), ...)`; unsupported or non-linear systems remain as
unevaluated `Solve(...)` IR nodes.

`Solve(inequality, variable)` also delegates to the Rust `cas-solve` polynomial
inequality solver. Supported one-variable polynomial inequalities return
`List(...)` interval predicates such as `Greater(x, 1)` or
`And(GreaterEqual(x, -1), LessEqual(x, 1))`; unsupported inequalities remain
unevaluated.

Direct `Solve(f(linear) = constant, variable)` transcendental equations delegate
to the Rust `cas-solve` inverse-family solver. Supported `Exp`, `Log`, trig,
and hyperbolic forms return `List(...)` symbolic inverse or periodic-family
solutions; unsupported nested forms remain unevaluated.
