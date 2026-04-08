# Changelog — coding-adventures-hash-map

## [0.1.0] — 2026-04-08

### Added

- Initial implementation of `HashMap[K, V]` (DT18).
- Two collision strategies:
  - `"chaining"` — separate chaining with lists; resize threshold: load factor > 1.0.
  - `"open_addressing"` — linear probing with tombstone deletion; resize threshold: load factor > 0.75.
- Three pluggable hash functions: `"fnv1a"` (default), `"murmur3"`, `"djb2"` — all from `coding-adventures-hash-functions` (DT17).
- Automatic resize (capacity × 2) when load factor exceeds strategy threshold.
- Public API: `set`, `get`, `delete`, `has`, `keys`, `values`, `entries`, `size`, `load_factor`, `capacity`.
- Python protocol support: `__len__`, `__contains__`, `__iter__`, `__repr__`.
- Module-level utilities: `from_entries`, `merge`.
- 95%+ test coverage across chaining and open addressing strategies.
- Property-based test comparing HashMap against Python's `dict` for random set/delete sequences.
- Literate inline documentation throughout source code.
