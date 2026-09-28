- Nested `select` and `input` start tags processed with an open `select` now
  report the required select insertion-mode parse error, covering 4 previously
  silent malformed corpus cases without changing DOM recovery or
  undeclared-diagnostic coverage.
