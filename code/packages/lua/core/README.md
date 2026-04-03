# coding-adventures-core (Lua)

Complete CPU core that integrates pipeline, register file, memory controller,
and an ISA decoder into a single working processor.
Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) project.

## What is the Core?

The Core is the integration point — it wires together all the components:

```
┌─────────────────────────────────────────────────────────┐
│  Core                                                    │
│                                                          │
│  ┌──────────┐  fetch_fn  ┌────────────────────────────┐ │
│  │ Memory   │←──────────→│  Pipeline (D04)            │ │
│  │ Controller│ memory_fn │                            │ │
│  └──────────┘            │  IF → ID → EX → MEM → WB  │ │
│                          └────────────────────────────┘ │
│  ┌──────────┐                         ↑ writeback_fn    │
│  │Register  │←────────────────────────┘                 │
│  │ File     │←── decode_fn / execute_fn                  │
│  └──────────┘           ↑                               │
│  ┌──────────┐            │                               │
│  │ ISA      │────────────┘  (injected)                   │
│  │ Decoder  │                                            │
│  └──────────┘                                            │
└─────────────────────────────────────────────────────────┘
```

The Core itself has NO knowledge of instruction semantics — that is entirely
delegated to the injected ISA decoder. The same Core can run ARM, RISC-V,
or any custom instruction set.

## Usage

```lua
local core_mod   = require("coding_adventures.core")
local Core       = core_mod.Core
local CoreConfig = core_mod.CoreConfig

-- Define an ISA decoder
local MyDecoder = {}
function MyDecoder.decode(raw, token)
    if raw == 0xFF then token.opcode = "HALT"; token.is_halt = true
    else token.opcode = "NOP" end
    return token
end
function MyDecoder.execute(token, rf) return token end
function MyDecoder.instruction_size() return 4 end

-- Build the core
local result = Core.new(CoreConfig.simple(), MyDecoder)
assert(result.ok, result.err)
local core = result.core

-- Load a program (halt after 1 instruction)
core:load_program({ 0xFF, 0x00, 0x00, 0x00 }, 0)

-- Run until halt
core:run(100)
print(core:is_halted())              -- true
print(core:get_stats():to_string())  -- CoreStats{...}
```

## Configuration Presets

| Preset | Pipeline | Registers | Notes |
|--------|----------|-----------|-------|
| `CoreConfig.simple()` | 5-stage | 16 × 32-bit | Teaching core |
| `CoreConfig.performance()` | 13-stage | 31 × 64-bit | ARM Cortex-A78-inspired |

## Layer Position

```
ISA Simulators (Layer 7) — inject decoder here
         ↓
Core (D05) ← this package
         ↓
Pipeline (D04) + CPU Simulator (cpu_simulator)
```

## Dependencies

- `coding-adventures-cpu-pipeline` — provides Pipeline and PipelineConfig
- `coding-adventures-cpu-simulator` — provides Memory and RegisterFile
