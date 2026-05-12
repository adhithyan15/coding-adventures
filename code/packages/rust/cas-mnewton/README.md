# cas-mnewton

Rust port of the MACSYMA `MNewton` numeric root finder over `symbolic-ir`.

The core algorithm stays VM-agnostic: callers provide an evaluator callback and
a symbolic derivative callback. That mirrors the Python implementation and
keeps this crate reusable from `symbolic-vm`, WASM bindings, and standalone
tests.

`mnewton_solve(f, x, x0, eval, diff, options)` returns `IRNode::Float(root)` on
convergence, returns the original function expression when the input cannot be
evaluated numerically, and reports `MNewtonError::ZeroDerivative` when Newton's
step is undefined.

`mnewton_handler(expr, eval, diff)` is the VM-neutral handler layer for
`MNewton(f, x, x0)` and `MNewton(f, x, x0, tol)`. It validates the IR shape,
injects evaluator and derivative callbacks, returns malformed calls unchanged,
and catches `MNewtonError` as an unevaluated expression.

`build_mnewton_handler_table()` returns a callable table keyed by `MNEWTON`.
