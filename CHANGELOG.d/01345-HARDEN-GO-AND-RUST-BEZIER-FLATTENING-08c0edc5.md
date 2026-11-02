### Harden Go and Rust Bezier flattening

- Replaced midpoint-only recursive flattening with bounded, iterative
  finite-segment subdivision in the Go and Rust Bezier2D lanes.
- Added explicit atomic errors for invalid or non-finite input, derived numeric
  overflow, and the pinned depth or 65,535-subdivision ceilings.
- Added dynamic readers for all nine neutral cases and retained native
  Bezier2D plus downstream Arc2D regression coverage.
