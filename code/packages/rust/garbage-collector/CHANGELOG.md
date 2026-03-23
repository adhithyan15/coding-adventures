# Changelog

## 0.1.0 — 2026-03-21

### Added
- `HeapObject` trait for anything that lives on the managed heap
- `ConsCell`, `Symbol`, `LispClosure` heap object types
- `GarbageCollector` trait (abstract interface for all GC algorithms)
- `MarkAndSweepGC` implementation: mark phase (DFS from roots), sweep phase (delete unmarked)
- Address space starting at 0x10000 to avoid ambiguity with small integers
- `SymbolTable` for interning symbols (identity-based equality)
- Cycle detection (handled naturally by mark-and-sweep)
- Stats tracking: total allocations, collections, freed, heap size
- Comprehensive test suite covering allocation, collection, cycles, and symbol interning
