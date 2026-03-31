# Changelog — interrupt_handler (Lua)

## 0.1.0 — 2026-03-31

### Added
- `IDT` — Interrupt Descriptor Table mapping numbers 0..255 to ISR entry structs
- `ISRRegistry` — function registry mapping interrupt numbers to Lua handlers
- `Controller` — full interrupt controller with pending queue, mask register, global enable/disable, and priority dispatch
- `Frame` — saved CPU context (PC, registers, mstatus, mcause)
- Immutable (functional-style) API — every mutating operation returns a new struct
- 95%+ test coverage via busted test suite
