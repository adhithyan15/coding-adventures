# Changelog

## [0.2.1] - 2026-04-02

### Fixed

- Added `.PanicOnUnexpected()` to `VirtualMachine.Execute` and `GenericVM.Execute` so that panics raised inside `Step` (e.g., `InvalidOpcodeError` for unregistered opcodes) propagate out to the caller instead of being swallowed by the outer Operations wrapper. Fixes `TestErrorHandling/unknown_opcode_panics_with_InvalidOpcodeError` and related sub-tests.

## [0.2.0] - 2026-03-31

### Changed

- Wrapped all public functions and methods (`NewVirtualMachine`, `Execute`, `Step`, `AssembleCode`, `NewGenericVM`, `RegisterOpcode`, `RegisterBuiltin`, `GetBuiltin`, `Push`, `Pop`, `Peek`, `PushFrame`, `PopFrame`, `AdvancePC`, `JumpTo`, `InjectGlobals`, `SetMaxRecursionDepth`, `MaxRecursionDepth`, `SetFrozen`, `IsFrozen`, `Reset`) with the Operations system (`StartNew[T]`), providing automatic timing, structured logging, and panic recovery. Public API signatures unchanged.

## [0.1.0] - Unreleased

### Added
- Defined `VirtualMachine` tracking dynamically allocated `interface{}` parameter Arrays mapping Type boundaries safely during execution without predefined limits.
- Extensively simulated Call boundaries routing `Variables` alongside contextual Frames mirroring how Python allocates execution scopes linearly.
- Ported unit tests documenting logical control Jumps explicitly evaluating `JumpIfFalse` utilizing generic dynamic inferencing.
