# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Fixed

- Reject oversized byte transports before decoding them into a second buffer.

## [0.1.0] - 2026-09-19

### Added

- PDP-8/E base-processor state in 49,205 simulated D flip-flops.
- Gate-decoded opcodes, page selection, indirect/auto-index addressing,
  ripple-carry TAD/ISZ/IAC/PC updates, Boolean OPR predicates, and wired
  rotate/swap networks.
- Complete state/trace differential coverage against `pdp8-simulator`, plus
  lifecycle, fault atomicity, interrupt, IOT, and topology checks.
