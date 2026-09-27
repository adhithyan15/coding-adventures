### Python build-tool portable glob hardening

- Replaced recursive globstar suffix exploration and host `fnmatch` delegation
  with an explicit Unicode-scalar parser and iterative dynamic programming.
- Added adversarial state-bound coverage plus exact portable character-class
  tests for literals, negation, ranges, astral scalars, ambiguous operators,
  and stable rejection of descending ranges.
