# Changelog

## [0.1.0] — 2026-08-31

### Added

- Exact 525,473-DFF PowerPC 601 persistent topology.
- Combinational fixed-word decode and gate datapaths for the complete Spec 07u
  integer surface, including fixed-round multiply and signed/unsigned divide.
- Shared typed, atomic, transactional functional lifecycle.
- Topology, gate-network, direct-access, lifecycle, fault, trace, workload, and
  239-vector Python full-state differential coverage.
- Normative Spec 07u2 and strict formatting, Clippy, rustdoc, and line-coverage
  verification.
- Replace the host-language conditional-branch predicate with BO-controlled
  CTR-decrement, CR-selection, bypass-mux, polarity, and final-AND gate
  networks, exhaustively checked against an independent architectural table.
- Pass three gate/topology suites, three lifecycle suites, all 239 differential
  vectors, strict formatting/Clippy/rustdoc, and 94.58% line coverage
  (838/886).
- Make subtractor-derived signed/unsigned relation wires, the CR-field decoder,
  CR0 write-enable, `mtcrf` field muxes, and XER[CA] mux the sole status-update
  authority while preserving stable-Q SO/OV/opaque XER state.
- Reject all excluded OE/arithmetic-Rc modifier pairs atomically before result
  or state mutation, while retaining X-form record operations.
- Correct the bounded six-bit `sraw` CA edge for counts 32 through 63 without
  absorbing shift movement, count-leading-zero, or divide datapaths.
- Add exhaustive truth-table, field-isolation, carry/borrow, modifier,
  producer-to-branch, lifecycle, differential, and `syn`-parsed hostile AST
  coverage for the complete decoder-to-status-to-commit chain.
- Pass all 23 gate-package test functions, all 239 differential vectors,
  strict formatting/Clippy/rustdoc, and 95.15% package-source line coverage
  (962/1,011).
