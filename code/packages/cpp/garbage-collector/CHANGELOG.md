# Changelog

All notable changes to the C++ `garbage-collector` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-13

### Added

- Initial header-only pure-ISO C++17 port of the Rust `garbage-collector` crate
  (namespace `ca::garbage_collector`) — a language-agnostic mark-and-sweep
  tracing GC with correct cycle handling.
- `HeapObject` base class with `ConsCell` / `Symbol` / `LispClosure` overriding
  `references()` / `type_name()`; the abstract `GarbageCollector` interface and
  its `MarkAndSweepGC` implementation (`allocate` owns objects via
  `std::unique_ptr` in an address-keyed `std::unordered_map`; `deref`,
  `collect`, `heap_size`, `is_valid_address`, `stats`).
- `Value` (a `std::variant` of int / address / str / bool / nil / list) with
  factory methods; `SymbolTable` (`intern` / `lookup` → `std::optional` /
  `all_symbols`).
- Addresses increase monotonically from `0x10000` and are never reused.
- 32 checks mirroring the Rust crate's own unit tests, run under every available
  C++ compiler via the shared `iso-harness`; the suite also passes clean under
  AddressSanitizer + UndefinedBehaviorSanitizer.
