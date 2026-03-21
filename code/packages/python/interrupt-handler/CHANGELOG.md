# Changelog

## [0.1.0] - 2026-03-21

### Added

- `InterruptDescriptorTable` with 256 entries, `set_entry`, `get_entry`,
  `write_to_memory`, `load_from_memory` (little-endian binary serialization)
- `InterruptFrame` dataclass for saving/restoring 32 registers + PC + MStatus + MCause
- `save_context` and `restore_context` functions with defensive copying
- `ISRRegistry` mapping interrupt numbers to Python callables with
  `register`, `dispatch`, `has_handler`
- `InterruptController` with full lifecycle support:
  - `raise_interrupt` with duplicate prevention and bisect-sorted pending queue
  - `has_pending` / `next_pending` with mask and global enable checks
  - `acknowledge`, `set_mask` / `is_masked`, `enable` / `disable`
  - `pending_count` / `clear_all`
- Well-known interrupt number constants (0-5, 32, 33, 128)
- Comprehensive test suite with pytest
