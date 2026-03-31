# Changelog — CodingAdventures::Core (Perl)

## 0.01 — 2026-03-31

Initial release.

- `Core::Core` — complete CPU core
  - `new($config, $decoder)` — wires pipeline + memory + register file
  - `load_program(\@bytes, $addr)` — loads machine code
  - `step()` — one clock cycle, returns pipeline snapshot
  - `run($max_cycles)` — runs until halt or max_cycles
  - `read_register($i)` / `write_register($i, $v)` — register access
  - `read_memory_word($addr)` / `write_memory_word($addr, $v)`
  - `get_trace()` — snapshot history
- `Core::CoreConfig` — micro-architecture parameters
  - `simple()` — 5-stage, 16 registers, 64KB
  - `performance()` — 13-stage, 31 registers
- `Core::CoreStats` — IPC tracking
- `Core::MemoryController` — memory wrapper with latency model
- ISA Decoder protocol: `decode()`, `execute()`, `instruction_size()`
- Depends on: `CodingAdventures::CpuPipeline`, `CodingAdventures::CpuSimulator`
