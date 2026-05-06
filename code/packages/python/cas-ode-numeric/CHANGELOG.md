# Changelog

## 0.1.0 — 2026-05-04

**Initial release — fixed-step RK4 numeric ODE integrator.**

### What's new

- `rk4_solve(f_ir, y0, t_span, dt, vm, *, state_names, t_name)` — integrates a
  system of first-order ODEs whose right-hand sides are IR trees evaluated
  through the symbolic VM at each stage.

### Design notes

- Implements the classical RK4 method with 4 stages per step (O(h⁴) global
  error).
- State variables and time are temporarily bound as `IRFloat` in the VM
  environment at each stage; saved and restored so caller's environment is
  unaffected.
- Accepts IR trees that reference any mix of state variables and the time
  variable — enables both autonomous (f(y)) and non-autonomous (f(y, t)) ODEs.
- The final step is clamped to land exactly on `t_end`.

### SPICE motivation

This package is the numeric integrator for the SPICE transient analysis
pipeline. When device equations are nonlinear (e.g. diode exponential I-V
curve), the ODE cannot be solved symbolically and RK4 is used instead.
