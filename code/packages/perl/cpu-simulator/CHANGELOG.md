# Changelog — CodingAdventures::CpuSimulator (Perl)

## 0.01 — 2026-03-31

Initial release.

- `Memory` — dense array-backed byte-addressable RAM
  - `new($size)` / `read_byte` / `write_byte` / `read_word` / `write_word`
  - `load_bytes($addr, \@bytes)` / `dump($start, $length)`
- `SparseMemory` — hash-backed sparse RAM (same API as Memory)
  - Writing 0 removes the entry to maintain sparsity
- `RegisterFile` — 0-indexed register file with bit-width masking
  - `new($num_registers, $bit_width)` / `read($idx)` / `write($idx, $val)`
  - `dump()` → hashref {R0 => v, R1 => v, …}
