## Unreleased - 2026-11-09 - Like floor-root sums

### Added

- Combine repeated positive static square roots with the same radicand into a
  checked coefficient before applying `entier`, avoiding binary64 widening.
