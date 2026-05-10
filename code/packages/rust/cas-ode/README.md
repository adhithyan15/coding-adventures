# cas-ode

Pure Rust symbolic ODE solver for the shared `symbolic-ir` CAS tree.

The crate ports the Python `code/packages/python/cas-ode` package's core
`ODE2(eqn, y, x)` behavior into a focused IR-to-IR Rust API:

- first-order linear ODEs
- separable ODEs
- Bernoulli ODEs
- exact first-order ODEs when the exactness check can be proven structurally
- second-order constant-coefficient homogeneous and non-homogeneous ODEs
- Euler-Cauchy homogeneous ODEs
- variation-of-parameters fallback, emitted with symbolic `Integrate(...)`
  nodes when primitives are not available in Rust

The public entrypoints are `solve_ode`, `ode2_handler`, and
`build_ode_handler_table`.
