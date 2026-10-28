### Harden Haskell and Swift Bezier flattening

- Replaced midpoint-only recursion with bounded, iterative finite-segment
  subdivision in the established Haskell and Swift Bezier2D lanes.
- Added typed atomic failure for invalid or non-finite inputs, derived numeric
  overflow, and the pinned depth or 65,535-subdivision budgets.
- Added dynamic readers for all nine neutral cases and retained native
  Bezier2D plus downstream Arc2D regression coverage.
