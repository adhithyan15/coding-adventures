# Rust CAS MNewton Parity

## Status

Initial Rust parity slice for Python `cas-mnewton`.

## Scope

- `mnewton_solve` implements Newton-Raphson iteration over `symbolic-ir`.
- Evaluation and differentiation are callback parameters, matching the Python
  package's import-cycle-free design.
- Numeric literals accepted for the initial guess and evaluated values:
  integer, rational, and float.
- Zero derivatives are reported as `MNewtonError::ZeroDerivative`.
- Non-numeric starting points or non-numeric evaluated function values return
  the original expression unevaluated.

## Follow-up

- Add `symbolic-vm` handler wiring once the Rust VM exposes derivative helpers
  equivalent to Python `symbolic_vm.derivative._diff`.
- Port the same package to TypeScript with the same callback-shaped API.
