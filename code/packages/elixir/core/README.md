# core

Complete processor core simulator in Elixir, integrating all D-series micro-architectural components.

## Overview

This package implements a complete processor core by composing:

- **Pipeline** (D04): moves instructions through configurable stages (IF, ID, EX, MEM, WB)
- **Register File**: fast storage for operands and results, with configurable width and count
- **Memory Controller**: latency-aware access to main memory with async request support
- **ISA Decoder**: pluggable instruction set via Elixir behaviours
- **Interrupt Controller**: interrupt routing for multi-core systems
- **Multi-Core CPU**: multiple independent cores sharing memory

The Core is ISA-independent. Any instruction set can be plugged in by implementing the `CodingAdventures.Core.Decoder` behaviour. A `MockDecoder` is provided for testing.

## Layer Position

```
MultiCoreCPU
  Core (D05) <- THIS PACKAGE
    Pipeline (D04) <- cpu_pipeline dependency
      IF -> ID -> EX -> MEM -> WB
    Register File
    Memory Controller
    ISA Decoder (injected via behaviour)
```

## Usage

```elixir
alias CodingAdventures.Core.{Config, MockDecoder}
alias CodingAdventures.Core.Core, as: CoreModule

# Create a simple core with the mock decoder.
{:ok, core_tuple} = CoreModule.new(Config.simple_config(), MockDecoder)

# Load a program.
program = MockDecoder.encode_program([
  MockDecoder.encode_addi(1, 0, 42),  # R1 = 42
  MockDecoder.encode_halt()
])
core_tuple = CoreModule.load_program(core_tuple, program, 0)

# Run until halt or max cycles.
{core_tuple, stats} = CoreModule.run(core_tuple, 1000)

# Read results.
IO.puts("R1 = #{CoreModule.read_register(core_tuple, 1)}")
IO.puts("IPC = #{CodingAdventures.Core.Stats.ipc(stats)}")

# Clean up.
CoreModule.stop(core_tuple)
```

## Configuration Presets

| Preset | Pipeline | Registers | Inspired By |
|--------|----------|-----------|-------------|
| `simple_config/0` | 5-stage | 16x32-bit | MIPS R2000 (1985) |
| `cortex_a78_like_config/0` | 13-stage | 31x64-bit | ARM Cortex-A78 (2020) |
| `default_config/0` | 5-stage | 16x32-bit | Teaching default |

## ISA Decoder Behaviour

Implement `CodingAdventures.Core.Decoder` to create a custom instruction set:

```elixir
defmodule MyDecoder do
  @behaviour CodingAdventures.Core.Decoder

  @impl true
  def decode(raw, token), do: # ... fill in token fields

  @impl true
  def execute(token, reg_file), do: # ... compute ALU result

  @impl true
  def instruction_size(), do: 4
end
```

## Dependencies

- `coding_adventures_cpu_pipeline` -- the D04 pipeline package

## Testing

```bash
mix test --cover
```
