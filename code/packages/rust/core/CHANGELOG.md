# Changelog

## Unreleased

### Fixed

- Rust 1.100 clippy (`-D warnings`) drift in `src/register_file.rs`: `needless_range_loop`: the `Display` impl iterates the register values with `iter().enumerate().take(count)`; behaviour unchanged.
