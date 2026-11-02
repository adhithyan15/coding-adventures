# Changelog

## Unreleased

- Add the bounded Spec 07ar Data General Nova functional simulator.
- Base direct and indirect PC-relative effective addresses on the instruction
  address, including correct 15-bit wraparound at address zero.
- Cover all twelve memory-reference operation/accumulator codes, all 32,768
  operate-field words, direct/indirect/auto-index addressing, typed atomic
  lifecycle faults, and complete state with 91.53% line coverage (540 of 590
  lines).
