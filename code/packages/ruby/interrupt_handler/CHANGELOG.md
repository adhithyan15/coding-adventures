# Changelog

## [0.1.0] - 2026-03-21

### Added

- `IDT` with 256 entries, `set_entry`, `get_entry`, `write_to_memory`,
  `load_from_memory` (little-endian binary serialization)
- `InterruptFrame` for saving/restoring 32 registers + PC + MStatus + MCause
- `InterruptHandler.save_context` and `.restore_context` module methods
  with defensive copying
- `ISRRegistry` mapping interrupt numbers to Ruby lambdas/procs
- `InterruptController` with full lifecycle support:
  - `raise_interrupt` with duplicate prevention and sorted pending queue
  - `has_pending?` / `next_pending` with mask and global enable checks
  - `acknowledge`, `set_mask` / `masked?`, `enable` / `disable`
  - `pending_count` / `clear_all`
- Well-known interrupt number constants
- Comprehensive minitest test suite
