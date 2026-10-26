# Changelog

## Unreleased

- Give architectural illegal-slot detection precedence after a valid delay-slot
  fetch, including undefined code and excluded RTE, TRAPA, and SR-load forms,
  while preserving ordinary decode and transport-fault classifications.
- Corrected `DIV1` quotient and T-bit updates for both incoming-Q=1
  quadrants and added an independent exhaustive boundary truth table.
- Add the bounded Spec 07av SH-2 integer simulator with complete checked state,
  big-endian transport, typed transactional lifecycle, exact traces, delayed
  control flow, integer/memory families, and multiply/divide behavior.
