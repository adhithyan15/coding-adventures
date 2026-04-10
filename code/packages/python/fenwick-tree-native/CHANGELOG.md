# Changelog

## [0.1.0] - 2026-04-10

### Added

- Initial release of the native fenwick tree extension
- `FenwickTree` class wrapping the Rust `fenwick-tree` crate via `python-bridge`
- Constructor, `from_list`, updates, prefix sums, range sums, point queries, and `find_kth`
- `FenwickError`, `IndexOutOfRangeError`, and `EmptyTreeError` exceptions
