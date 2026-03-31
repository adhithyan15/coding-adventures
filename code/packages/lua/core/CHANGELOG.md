# Changelog — coding-adventures-core (Lua)

## 0.1.0 — 2026-03-31

Initial release.

- `MemoryController` — Memory wrapper with latency metadata
  - `new(size, latency)` — creates backing RAM
  - `read_word` / `write_word` / `load_program`
- `CoreConfig` — micro-architecture parameters
  - `simple()` — 5-stage, 16 registers, 64KB
  - `performance()` — 13-stage, 31 registers, 256KB
- `CoreStats` — aggregate statistics
  - `ipc()` — instructions per cycle
  - `cpi()` — cycles per instruction
  - `to_string()` — human-readable summary
- `Core` — complete processor core
  - `new(config, decoder)` — returns {ok, core}
  - `load_program(bytes, start_address)` — loads machine code
  - `step()` — one clock cycle, returns Snapshot
  - `run(max_cycles)` — runs until halt or max_cycles
  - `read_register(i)` / `write_register(i, v)` — register access
  - `read_memory_word(addr)` / `write_memory_word(addr, v)`
  - `get_stats()` — CoreStats
  - `get_trace()` — snapshot history
- ISA Decoder protocol: `decode(raw, token)`, `execute(token, reg_file)`, `instruction_size()`
- Depends on: `coding-adventures-cpu-pipeline`, `coding-adventures-cpu-simulator`
