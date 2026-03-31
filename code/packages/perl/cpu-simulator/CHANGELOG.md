# Changelog — CodingAdventures::CpuSimulator (Perl)

## 0.01 — 2026-03-31

Initial release.

- `CpuSimulator::Memory` — fixed-size byte-addressable RAM (little-endian)
  - `read_byte` / `write_byte` — single byte access with bounds checking
  - `read_word` / `write_word` — 32-bit little-endian word access
  - `load_bytes` — bulk load from arrayref
  - `dump` — dump range to arrayref
- `CpuSimulator::SparseMemory` — sparse address space (hash-backed)
  - Same API as Memory; stores only non-zero bytes
  - Writing 0 removes the entry to stay sparse
- `CpuSimulator::RegisterFile` — fast CPU register storage
  - `read($i)` / `write($i, $v)` — 0-based index access
  - Writes masked to configured bit width (default 32-bit)
  - `dump()` — hashref `{ "R0" => value, ... }`
