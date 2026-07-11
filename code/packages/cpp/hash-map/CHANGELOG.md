# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C++17, header-only port of the Rust `hash-map` crate (DT18): a generic
  `ca::hash_map<K, V>`.
- Two collision-resolution strategies — separate chaining (resize > 1.0 load) and
  open addressing with tombstones (resize > 0.75) — selectable per map.
- Four selectable hash functions in `ca::detail`: SipHash-2-4 (default),
  FNV-1a-32, MurmurHash3-32, and djb2.
- API: `set`, `get` (returns `std::optional<V>`), `has`, `remove`, `size`,
  `capacity`, `empty`, `load_factor`, `strategy`, `hash_function`, `entries`,
  `keys`.
- Keys may be `std::string` (hashed by characters) or any trivially-copyable type
  (hashed by object representation); open-addressing slots use
  `std::optional<entry>` so `K`/`V` need not be default-constructible.
- Tests covering all eight (strategy × hash) combinations, resize stress with
  string and integer keys, and a `keys()` completeness check, run under GCC and
  Clang via `iso-harness`.
