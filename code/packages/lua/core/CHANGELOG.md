# Changelog — coding-adventures-core (Lua)

## 0.1.0 — 2026-03-31

Initial release.

- `Core` — full CPU core integrating pipeline + memory + registers
  - `new(config, decoder)` — constructs the wired core
  - `load_program(bytes, addr)` — loads machine code into memory
  - `step()` — advances one clock cycle, returns pipeline snapshot
  - `run(max_cycles)` — runs until halt or max_cycles
  - `read_register(i)` / `write_register(i, v)` — register access
  - `read_memory_word(addr)` / `write_memory_word(addr, v)` — memory access
  - `get_trace()` — full snapshot history for visualization
- `CoreConfig` — configurable micro-architecture parameters
  - `simple()` — 5-stage, 16 registers, 64KB memory (teaching core)
  - `performance()` — 13-stage, 31 registers (ARM Cortex-A78-inspired)
- `CoreStats` — execution statistics with IPC/CPI
- `MemoryController` — wraps Memory with latency model
- ISA Decoder protocol: implement `decode()`, `execute()`, `instruction_size()`
