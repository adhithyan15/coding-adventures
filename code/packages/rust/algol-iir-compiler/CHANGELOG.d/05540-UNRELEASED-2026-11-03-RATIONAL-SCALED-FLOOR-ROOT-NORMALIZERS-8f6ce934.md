## 0.554.0 - 2026-11-03

### Added

- Preserve one exact static square root through checked integer multiplication
  and division even when the reduced coefficient is rational, computing the
  enclosing `entier` floor or negative ceiling without binary64 widening.
- Keep rooted or zero divisors, checked overflow, multiple roots, overrides,
  and dynamic roots outside the proof.
