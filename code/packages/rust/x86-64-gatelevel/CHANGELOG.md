# Changelog

## [0.1.1] — 2026-10-26

### Fixed

- Route signed and unsigned multiply overflow and divide quotient-fit
  qualification through gate reductions over the complete 128-bit result.
- Add exact boundary differentials, per-wire qualifier witnesses, atomic fault
  coverage, and a structural guard against host-wide comparisons returning to
  instruction execution.
- Lock the normalized qualifier, entry-point, and consumer bodies and exercise
  adversarial helper-shadow, iterator, high-word, and dead-anchor fixtures so
  alternate spellings cannot bypass that structural guard.
- Parse the production Rust syntax so locked functions must be unique
  top-level definitions and the instruction consumers must belong to the
  direct `exec_one` dispatch; reject inactive `cfg` decoys and later active
  host-wide duplicates.
- Bind those checks to the unique attribute-free external `execution` and
  `gate_networks` modules, rejecting path aliases, inline replacements,
  duplicate routing, and inactive reviewed-file decoys.
- Document the reproducible all-target coverage command and its 88.40%
  (1,357/1,535) package result.

## [0.1.0] — 2026-08-31

### Added

- Exact 525,382-DFF x86-64 persistent topology.
- Independent integer execution path with gate-derived arithmetic, Boolean,
  shift/rotate, address, condition, multiply, and divide networks.
- Shared typed, atomic, transactional functional lifecycle.
- Topology, gate-network, lifecycle, fault, trace, workload, and 262-vector
  Python full-state differential coverage.
- Normative Spec 07w2 and strict formatting, Clippy, rustdoc, and line-coverage
  verification.
