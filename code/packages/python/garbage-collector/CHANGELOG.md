# Changelog

## 0.1.0 — 2026-03-20

### Added

- **GarbageCollector ABC** — pluggable interface for any GC algorithm
- **HeapObject types**: `ConsCell`, `Symbol`, `LispClosure` with `references()` methods
- **MarkAndSweepGC** — McCarthy's 1960 algorithm: mark reachable objects, sweep the rest
- **SymbolTable** — symbol interning with GC-managed heap allocation
- `is_valid_address()` for safe address checking
- `stats()` for introspection (total allocations, collections, freed, heap size)
- Root scanning supports nested lists, dicts, and HeapObject references
- Heap addresses start at 0x10000 to avoid ambiguity with small integer values
- 44 tests (32 mark-sweep + 12 symbol table), 96% coverage
- Full literate programming documentation
