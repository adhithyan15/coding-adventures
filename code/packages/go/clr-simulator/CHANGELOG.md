# Changelog

## [0.1.2] - 2026-04-02

### Fixed
- Added `.PanicOnUnexpected()` to `Step` and `Run` operations so intentional panics (division by zero, uninitialized local, unknown opcode, halted simulator) propagate correctly instead of being swallowed by the Operations panic-recovery wrapper.

## [0.1.1] - 2026-04-02

### Changed
- Wrapped all public functions (`NewCLRSimulator`, `Load`, `Step`, `Run`, `EncodeLdcI4`, `EncodeStloc`, `EncodeLdloc`, `AssembleClr`) with the Operations system via `StartNew[T]`, providing automatic timing, structured logging, and panic recovery for every public call.

## [0.1.0] - Unreleased

### Added
- Created `CLRSimulator` isolating the specific Inferencing behaviors decoupled structurally from generic Virtual Machine models.
- Established rigorous `CLRTrace` bounds mapping the specific parameters mapping generic executions across the underlying Array architectures explicitly detailing variable bounds structurally.
- Modeled 2-Byte prefixed instructions parsing native logical conditions bounds checking internally (`0xFE`).
- Explained within README mechanisms why Microsoft decoupled Type mapping dynamically resolving bytes at runtime.
