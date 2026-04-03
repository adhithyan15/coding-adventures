# Changelog — coding-adventures-cpu-simulator (Lua)

## 0.1.0 — 2026-03-31

Initial release.

- `Memory` — fixed-size byte-addressable RAM
  - `read_byte(addr)` / `write_byte(addr, val)` — single byte access
  - `read_word(addr)` / `write_word(addr, val)` — 32-bit little-endian word
  - `load_bytes(addr, bytes)` — bulk load from table
  - `dump(start, length)` — dump range to table
- `SparseMemory` — sparse address space (stores only non-zero bytes)
  - Same API as Memory; ideal for large address spaces
  - Writing 0 removes the entry to maintain sparsity
- `RegisterFile` — fast CPU register storage
  - `read(index)` / `write(index, value)` — 0-based register access
  - Writes are masked to the configured bit width
  - `dump()` — snapshot of all registers as `{ "R0" → value, ... }`
