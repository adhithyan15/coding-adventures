# Changelog

## [0.1.0] - 2026-03-21

### Added

- `InterruptDescriptorTable` with 256 entries, `set_entry`, `get_entry`,
  `write_to_memory`, `load_from_memory` (little-endian binary serialization)
- `InterruptFrame` struct for saving/restoring 32 registers + PC + MStatus + MCause
- `save_context` and `restore_context` functions
- `ISRRegistry` mapping interrupt numbers to `Box<dyn FnMut>` handlers
- `InterruptController` with full lifecycle:
  - `raise_interrupt` with duplicate prevention and partition_point-sorted queue
  - `has_pending` / `next_pending` (returns `Option<usize>`)
  - `acknowledge`, `set_mask` / `is_masked`, `enable` / `disable`
  - `pending_count` / `clear_all`
- Well-known interrupt number constants
- Comprehensive test suite
