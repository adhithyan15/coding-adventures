# coding_adventures_core

A configurable processor core simulator that integrates all D-series micro-architectural components into a complete CPU core.

## What This Package Does

The Core is a composition layer -- it wires together independently designed sub-components into a working processor:

- **Pipeline** (D04): Moves instructions through stages (IF, ID, EX, MEM, WB)
- **Branch Predictor** (D02): Guesses branch directions to avoid stalls
- **Hazard Detection** (D03): Detects data, control, and structural hazards
- **Cache Hierarchy** (D01): L1I, L1D, optional L2 for fast memory access
- **Register File**: Fast storage for operands and results
- **Clock**: Drives everything in lockstep

The Core itself defines no new micro-architectural behavior. Like a motherboard that connects CPU, RAM, and peripherals, it connects the sub-components via callback wiring.

## How It Fits in the Stack

```
Layer D05 (this package)
  |
  +-- D04: cpu_pipeline (instruction pipeline)
  +-- D03: hazard_detection (hazard unit)
  +-- D02: branch_predictor (direction + BTB)
  +-- D01: cache (L1I/L1D/L2 hierarchy)
  +-- D00: clock (system clock)
```

## ISA Independence

The Core does not know what instructions mean. An ISA decoder is injected from outside, providing `decode` and `execute` methods. The same Core can run ARM, RISC-V, or any custom ISA by swapping the decoder.

A `MockDecoder` is included for testing, supporting NOP, ADD, SUB, ADDI, LOAD, STORE, BRANCH, and HALT instructions.

## Usage

### Single Core

```ruby
require "coding_adventures_core"

# Create a simple core (MIPS R2000-like)
config = CodingAdventures::Core.simple_config
decoder = CodingAdventures::Core::MockDecoder.new
core = CodingAdventures::Core::Core.new(config, decoder)

# Build a program: R1 = 42, then halt
program = CodingAdventures::Core.encode_program(
  CodingAdventures::Core.encode_addi(1, 0, 42),
  CodingAdventures::Core.encode_halt
)

core.load_program(program, 0)
stats = core.run(1000)

puts "R1 = #{core.read_register(1)}"  # => 42
puts "IPC: #{stats.ipc}"
puts stats.to_s
```

### Multi-Core

```ruby
config = CodingAdventures::Core.default_multi_core_config
decoders = [
  CodingAdventures::Core::MockDecoder.new,
  CodingAdventures::Core::MockDecoder.new
]
cpu = CodingAdventures::Core::MultiCoreCPU.new(config, decoders)

# Load different programs at different addresses
cpu.load_program(0, program_a, 0)
cpu.load_program(1, program_b, 4096)

stats = cpu.run(10000)
stats.each_with_index { |s, i| puts "Core #{i} IPC: #{s.ipc}" }
```

### Preset Configurations

```ruby
# Simple teaching core (MIPS R2000-like, 5-stage, 4KB caches)
CodingAdventures::Core.simple_config

# ARM Cortex-A78-like (13-stage, 64KB caches, L2, FP unit)
CodingAdventures::Core.cortex_a78_like_config

# Default minimal config
CodingAdventures::Core.default_core_config
```

## Development

```bash
# Run tests (requires Ruby 3.3+)
ruby -I test -I lib -I ../cache/lib -I ../branch_predictor/lib \
  -I ../cpu_pipeline/lib -I ../hazard_detection/lib -I ../clock/lib \
  -e "require 'test_helper'; Dir.glob('test/test_*.rb').each { |f| require_relative f }"
```

## License

MIT
