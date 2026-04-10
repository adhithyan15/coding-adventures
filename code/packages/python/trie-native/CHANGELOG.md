# Changelog

## [0.1.0] - 2026-04-10

### Added

- Initial release of the native trie extension
- `Trie` class wrapping the Rust `trie` crate via `python-bridge`
- Arbitrary Python object values stored in the Rust trie with Python refcount management
- Prefix queries, longest-prefix match, deletion, iteration, and dict-like item access
- `TrieError` and `KeyNotFoundError` exceptions
