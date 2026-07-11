# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C17 port of the Rust `hash-map` crate (DT18): a byte-key → byte-value
  hash map with owned key/value copies.
- Two collision-resolution strategies — separate chaining (resize > 1.0 load)
  and open addressing with tombstones (resize > 0.75) — selectable per map.
- Four selectable hash functions reproduced inline: SipHash-2-4 (default),
  FNV-1a-32, MurmurHash3-32, and djb2.
- API: `hashmap_new` / `hashmap_free`, `hashmap_set` / `hashmap_get` /
  `hashmap_has` / `hashmap_delete`, and accessors (`hashmap_size`,
  `hashmap_capacity`, `hashmap_load_factor`, `hashmap_get_strategy`,
  `hashmap_get_hash`).
- Allocation-free resize (relinks chaining nodes / moves open-addressing slots,
  no key/value re-duplication) that degrades gracefully on allocation failure;
  overflow-guarded capacity doubling.
- Tests covering all eight (strategy × hash) combinations, resize stress, and
  empty-key/value edge cases, run under GCC and Clang via `iso-harness`.
