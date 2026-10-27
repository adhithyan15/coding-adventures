### Harden C# and F# Bezier flattening

- Aligned the C# and F# `bezier2d` packages with the shared G2D02 flattening
  corpus using bounded iterative subdivision and finite-segment error checks.
- Added fail-closed validation for non-positive tolerances, non-finite inputs,
  derived overflow, and depth or subdivision exhaustion.
- Added cross-platform 95% line-coverage gates and exact build-tool selection
  so a shared fixture edit schedules both native .NET consumers.
