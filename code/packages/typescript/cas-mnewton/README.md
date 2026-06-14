# @coding-adventures/cas-mnewton

Pure TypeScript Newton-Raphson root finding over
`@coding-adventures/symbolic-ir`.

This package mirrors Python and Rust `cas-mnewton` with a VM-agnostic API:
callers provide an evaluator callback and a symbolic derivative callback.

`mnewtonSolve(f, x, x0, evalFn, diffFn)` returns a float IR root on
convergence, returns the original function expression when numeric evaluation
cannot proceed, and throws `MNewtonError` when the derivative is zero before a
Newton step.

For VM integration, `mnewtonHandler(expr, evalFn, diffFn)` handles
`MNewton(f, x, x0)` and `MNewton(f, x, x0, tol)` without depending on a
specific VM implementation. `buildMNewtonHandlerTable()` returns the dispatch
map keyed by the `MNewton` head name.
