# Ordered token alternatives

- Use one lookahead for a leading run of direct token choices in compiled
  alternations. Preserve ordered token-name and legacy enum matching,
  newline-skipping rollback, failure diagnostics, and resource refusals.
- Add finite old-engine comparisons and an opt-in SAT miter driven by fresh Go
  observations; production Go remains independent of Node and the solver.
