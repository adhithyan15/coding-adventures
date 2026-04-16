# Changelog

## [0.1.0] - 2026-04-16

### Added

- Initial implementation of `coding-adventures-beam-vm-simulator`.
- `BeamVMSimulator` with profile-driven loading, stepping, and execution.
- Support for a small executable BEAM subset including local calls, external calls, jumps, moves, and returns.
- Initial `erlang` BIF support for arithmetic operations.
- Integration with `coding-adventures-simulator-protocol`.
- Unit tests covering stepping, halting, helper-function calls, and external BIF dispatch.
