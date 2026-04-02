# Changelog

## [0.2.1] - 2026-04-02

### Fixed

- Added `.PanicOnUnexpected()` to `CompileNode`, `ExitScope`, and `PatchJump` operations so intentional panics (`UnhandledRuleError`, scope mismatch, invalid jump index) propagate correctly instead of being swallowed by the Operations panic-recovery wrapper.

## [0.2.0] - 2026-03-31

### Changed

- Wrapped all public functions and methods with the Operations system for automatic timing, structured logging, and panic recovery.

## [0.1.0] - Unreleased

### Added
- Decoupled explicitly shared `parser` AST structs outside compiler module to prevent recursive dependencies organically.
- Designed structured Compiler execution loop evaluating `Statement` bounds recursively translating strings inherently mapping cleanly internally without evaluating explicit contexts.
- Added explicit mapping evaluating byte bounds securely natively representing JVM logic correctly mapped securely matching Java standards exactly.
