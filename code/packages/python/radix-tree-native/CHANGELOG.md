# Changelog

## [0.1.0] - 2026-04-10

### Added

- Initial release of the native radix tree extension
- `RadixTree` class wrapping the Rust `radix-tree` crate via `python-bridge`
- Arbitrary Python object values stored in the Rust radix tree with Python refcount management
- Prefix queries, longest-prefix match, iteration, and `to_dict`
