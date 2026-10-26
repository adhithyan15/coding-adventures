# Changelog

All notable changes to this package will be documented in this file.

## [0.1.2] - 2026-10-26

### Fixed

- Make the single 256-line gate decoder the sole authority for instruction
  transport width, legality, format family, operation execution, independent
  validation, and commit. The functional simulator remains a semantic oracle
  but can no longer classify or dispatch the fetched opcode first.
- Preserve truncation-before-unknown precedence for reserved opcode holes by
  reducing transport membership separately from legal-operation selection.
- Remove secondary raw-opcode dispatch, including the BXH/BXLE decode inside
  the branch network, and carry the original settled select wires throughout
  the transition.
- Add a 256-opcode by 2/4/6-byte fail-closed matrix, inactive-storage poison
  witness, exact decode-to-commit structural lock, and hostile raw dispatch,
  duplicate/dead/secondary decode, oracle routing, storage-access, shadowing,
  and post-commit-overwrite fixtures.

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
  reviewed executor source dead. Top-level delimiter-aware module discovery
  also rejects macro-discarded declarations paired with macro-generated live
  `executor` or `gates` modules. Routing identity is parsed from the Rust AST,
  so character literals containing delimiters cannot confuse module scope.

## [0.1.0] - 2026-10-24

### Added

- Exact D-flip-flop storage for all 8,389,403 persistent architectural bits.
- Independent gate-network RR, RX, RS, SI, SS, and hexadecimal-floating
  transitions with complete-state parity against the checked functional model.
- Exhaustive first-byte classification, branch-path, overlap, lifecycle,
  atomic-fault, and strict-check validation with 92.86% package line coverage
  (1,626/1,751).
