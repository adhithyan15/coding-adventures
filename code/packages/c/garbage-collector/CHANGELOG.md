# Changelog

All notable changes to the C `garbage-collector` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-13

### Added

- Initial pure-ISO C17 port of the Rust `garbage-collector` crate — a
  language-agnostic mark-and-sweep tracing GC (with correct cycle handling).
- Heap objects (`gc_cons_new`, `gc_symbol_new`, `gc_closure_new`) as a tagged
  `GcObject` union with `gc_object_type_name` / `gc_object_references` /
  `gc_object_free`.
- Collector: `gc_new` / `gc_free`, `gc_allocate` (takes ownership → monotonic
  address from `0x10000`), `gc_deref`, `gc_collect(roots)`, `gc_heap_size`,
  `gc_is_valid_address`, `gc_stats`; root `GcValue` constructors and
  `gc_value_free`.
- `GcSymbolTable` (`intern` / `lookup` / `count` / `contains`) for identity-based
  symbol equality.
- Slot-array heap (address `A` ↔ slot `A - 0x10000`; never reused), matching the
  Rust crate's incrementing `next_address` without a hash map; overflow-guarded
  growth.
- 37 checks mirroring the Rust crate's own unit tests, run under every available
  C compiler via the shared `iso-harness`; the suite also passes clean under
  AddressSanitizer + UndefinedBehaviorSanitizer.
