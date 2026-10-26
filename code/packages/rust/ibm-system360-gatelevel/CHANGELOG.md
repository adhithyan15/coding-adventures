# Changelog

All notable changes to this package will be documented in this file.

## [0.1.1] - 2026-10-26

### Fixed

- Route every modeled branch predicate through gate reductions, muxes,
  comparator condition-code wires, and one-hot opcode lines.
- Add exhaustive mask, condition-code, register-field, count-wire, and
  index-branch truth tables plus token-normalized exact-body structural
  coverage with adversarial host-predicate, indirection, dead-anchor, and
  primitive/helper-shadowing fixtures. Exact locks cover the complete branch
  select, fixed-width adder, signed-comparator, and top-level dispatch data
  flow, including the four-byte router, RR/RX/RS dispatcher shells, and
  complete validation router, branch-validator shell, and public
  `step`/`run`/`execute` bridge; adversarial helper rewrites, dead dispatch,
  oracle substitution, expectation shadowing, and post-dispatch overwrites are
  rejected.
- Lock the crate's file-backed `executor` module binding and reject attributed,
  aliased, duplicate, or inline oracle-backed routing modules that leave the
  reviewed executor source dead.

## [0.1.0] - 2026-10-24

### Added

- Exact D-flip-flop storage for all 8,389,403 persistent architectural bits.
- Independent gate-network RR, RX, RS, SI, SS, and hexadecimal-floating
  transitions with complete-state parity against the checked functional model.
- Exhaustive first-byte classification, branch-path, overlap, lifecycle,
  atomic-fault, and strict-check validation with 92.86% package line coverage
  (1,626/1,751).
