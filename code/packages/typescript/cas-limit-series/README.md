# @coding-adventures/cas-limit-series

Pure TypeScript direct limits and polynomial Taylor expansion over
`@coding-adventures/symbolic-ir`.

This package mirrors the current Rust `cas-limit-series` behavior:

- `limitDirect(expr, variable, point)` performs structural substitution and
  returns an unevaluated `Limit(expr, variable, point)` for literal `Div(0, 0)`.
- `taylorPolynomial(expr, variable, point, order)` expands polynomial IR around
  a numeric literal point using exact rational coefficient arithmetic.

The implementation is browser-safe TypeScript and depends only on the symbolic
IR and CAS substitution packages.
