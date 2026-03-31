# Changelog — interrupt-handler (Perl)

## 0.01 — 2026-03-31

### Added
- `IDT` — Interrupt Descriptor Table with set_entry/get_entry
- `ISRRegistry` — sub registry with register/dispatch/has_handler
- `Controller` — pending queue, mask register, enable/disable, priority dispatch
- `Frame` — saved CPU context with save_context/restore_context
- 95%+ test coverage via Test2::V0
