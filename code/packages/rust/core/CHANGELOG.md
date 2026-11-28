# Changelog

## Unreleased

### Added

- Checked atomic byte-range RAM initialization and copied byte observation for
  `MemoryController` and individual `Core` instances. Existing loading and
  memory access APIs are unchanged.
- `Core::new_with_operand_forwarding` and the default-compatible decoder hook
  for per-source EX/MEM values; `Core::new` keeps the existing scalar path.

### Fixed

- The opt-in operand-indexed forwarding constructor now rejects split,
  reordered, or non-scalar pipeline topologies whose producer-age semantics
  are not defined by the current resolver. The default constructor is
  unaffected.
- Rust 1.100 clippy (`-D warnings`) drift in `src/register_file.rs`: `needless_range_loop`: the `Display` impl iterates the register values with `iter().enumerate().take(count)`; behaviour unchanged.
