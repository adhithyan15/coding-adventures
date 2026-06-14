# TypeScript CAS MNewton Parity

## Status

Initial TypeScript parity slice for Python and Rust `cas-mnewton`.

## Scope

- `mnewtonSolve` implements Newton-Raphson iteration over `symbolic-ir`.
- Evaluation and differentiation are callback parameters, matching the
  import-cycle-free Python package design.
- Numeric literals accepted for the initial guess and evaluated values:
  integer, rational, and float.
- Zero derivatives are reported as `MNewtonError`.
- Non-numeric starting points or non-numeric evaluated function values return
  the original expression unevaluated.

## Follow-up

- Add VM handler wiring once the TypeScript symbolic VM exposes derivative
  helpers equivalent to Python `symbolic_vm.derivative._diff`.
