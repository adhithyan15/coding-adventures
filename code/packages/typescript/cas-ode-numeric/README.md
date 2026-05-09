# @coding-adventures/cas-ode-numeric

Pure TypeScript fixed-step fourth-order Runge-Kutta integration for symbolic-IR
right hand sides.

The package mirrors Python `cas-ode-numeric` while remaining browser-safe and
runtime-neutral: `rk4Solve` owns RK4 stepping and passes temporary `(time, state)`
bindings to a user-provided evaluator.
