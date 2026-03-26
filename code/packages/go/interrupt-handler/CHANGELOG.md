# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- `InterruptDescriptorTable` with 256 entries, `SetEntry`, `GetEntry`,
  `WriteToMemory`, `LoadFromMemory` (little-endian binary serialization)
- `InterruptFrame` for saving/restoring all 32 registers + PC + MStatus + MCause
- `SaveContext` and `RestoreContext` functions for interrupt frame management
- `ISRRegistry` mapping interrupt numbers to Go handler functions with
  `Register`, `Dispatch`, `HasHandler`
- `InterruptController` with full lifecycle support:
  - `RaiseInterrupt` with duplicate prevention and sorted pending queue
  - `HasPending` / `NextPending` with mask and global enable checks
  - `Acknowledge` to clear handled interrupts
  - `SetMask` / `IsMasked` for per-interrupt masking (bits 0-31)
  - `Enable` / `Disable` for global interrupt control
  - `PendingCount` / `ClearAll` for queue management
- Well-known interrupt number constants (0-5, 32, 33, 128)
- Interrupt type enum: Fault, Timer, Keyboard, Syscall
- Comprehensive test suite covering IDT, ISR, controller, priority, masking,
  context save/restore, and full lifecycle
