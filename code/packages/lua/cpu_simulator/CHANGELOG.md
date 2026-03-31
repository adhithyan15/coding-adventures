# Changelog — coding-adventures-cpu-simulator (Lua)

## 0.1.0 — 2026-03-31

Initial release.

- `Memory` — dense array-backed byte-addressable RAM
  - `new(size)` — zero-filled memory
  - `read_byte` / `write_byte` — single-byte access
  - `read_word` / `write_word` — 32-bit little-endian access
  - `load_bytes(address, bytes)` — bulk load
  - `dump(start, length)` — return byte list
- `SparseMemory` — hash-backed sparse memory (same API as Memory)
  - Efficient for large, mostly-empty address spaces
  - Writes of 0 remove the entry to maintain sparsity
- `RegisterFile` — configurable-width register file
  - `new(num_registers, bit_width)` — 0-indexed, all zeros initially
  - `read(index)` / `write(index, value)` — with bit-width masking
  - `dump()` — returns {R0=v, R1=v, ...} table
