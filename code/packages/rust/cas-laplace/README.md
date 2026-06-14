# cas-laplace

Pure Rust Laplace transform support for the symbolic IR stack.

This package mirrors the Python `cas-laplace` layer with table-driven forward
and inverse transforms. It is designed for native Rust consumers and for the
WASM pathway: no Python runtime, parser, or host services are required.

## Exports

- `laplace_transform(f, t, s)` returns `L{f(t)}`
- `inverse_laplace(f, s, t)` returns `L^-1{F(s)}`
- `laplace_handler`, `ilt_handler`, `dirac_delta_handler`, and
  `unit_step_handler` provide VM-friendly entry points
- `LAPLACE`, `ILT`, `DIRAC_DELTA`, and `UNIT_STEP` are canonical head names

Unknown forms fall through to unevaluated IR nodes.
