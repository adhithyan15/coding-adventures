# cas-ode-numeric

Fixed-step fourth-order Runge-Kutta integration for symbolic-IR right hand sides.

The crate mirrors the Python `cas-ode-numeric` package while staying VM-neutral:
`rk4_solve` owns RK4 stepping and passes temporary `(time, state)` bindings to an
evaluation callback. A symbolic VM, MACSYMA runtime, or WASM host can provide the
callback without coupling this package to one backend.
