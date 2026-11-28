# Changelog

## Unreleased

### Added

- `Core::new_with_operand_forwarding` and the default-compatible decoder hook
  for per-source EX/MEM values; `Core::new` keeps the existing scalar path.

### Fixed

- Rust 1.100 clippy (`-D warnings`) drift in `src/register_file.rs`: `needless_range_loop`: the `Display` impl iterates the register values with `iter().enumerate().take(count)`; behaviour unchanged.
