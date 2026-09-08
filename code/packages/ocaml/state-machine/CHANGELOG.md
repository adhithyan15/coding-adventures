# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-09-05

### Added

- Added typed, traceable DFA execution with actions, validation, reachability,
  deterministic renderers, completeness checks, and atomic sequence preflight.
- Added iterative NFA epsilon closure, bounded traces, non-mutating acceptance,
  deterministic subset construction, DOT output, and DFA minimization.
- Added deterministic PDA execution with bottom-to-top stacks, explicit
  end-of-input epsilon processing, and stack, trace, and epsilon ceilings.
- Added explicit modal switching backed by a labeled directed graph, including
  reset-on-entry behavior and bounded mode traces.
- Added Alcotest coverage for representative machines, constructor failures,
  non-mutation guarantees, deterministic snapshots, and every configured
  resource ceiling.
