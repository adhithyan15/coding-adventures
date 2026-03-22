# CPU Pipeline

A configurable N-stage CPU instruction pipeline simulator, part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) project.

## What It Does

This gem simulates a CPU instruction pipeline -- the central execution engine of a processor core. Instead of completing one instruction fully before starting the next, a pipelined CPU overlaps instruction execution:

```
Single-cycle (no pipeline):
Instr 1: [IF][ID][EX][MEM][WB]
Instr 2:                       [IF][ID][EX][MEM][WB]
Throughput: 1 instruction every 5 cycles

Pipelined:
Instr 1: [IF][ID][EX][MEM][WB]
Instr 2:     [IF][ID][EX][MEM][WB]
Instr 3:         [IF][ID][EX][MEM][WB]
Throughput: 1 instruction every 1 cycle (after filling)
```

The pipeline manages the **flow** of instructions through stages. It does NOT interpret instructions -- that is the ISA decoder's job. The pipeline moves "tokens" through stages, handling:

- **Normal advancement**: tokens move one stage per clock cycle
- **Stalls**: freeze earlier stages and insert a "bubble" (NOP)
- **Flushes**: replace speculative instructions with bubbles
- **Forwarding**: shortcut data from later stages to earlier ones
- **Statistics**: track IPC, stall cycles, flush cycles

## Where It Fits

This is layer D04 in the coding-adventures CPU stack:

```
D01: Logic Gates (AND, OR, NOT, ...)
D02: Sequential Logic (flip-flops, registers)
D03: Clock (timing, phases)
D04: CPU Pipeline (this package) <-- you are here
D05: Hazard Detection
D06: Branch Predictor
D07: Cache
```

The pipeline is ISA-independent -- actual instruction semantics are provided via callback procs (fetch, decode, execute, memory, writeback).

## Usage

```ruby
require "coding_adventures_cpu_pipeline"

# Create a classic 5-stage pipeline configuration.
config = CodingAdventures::CpuPipeline.classic_5_stage

# Define callbacks for each stage.
pipeline = CodingAdventures::CpuPipeline::Pipeline.new(
  config: config,
  fetch_fn:     ->(pc) { instruction_memory[pc / 4] || 0 },
  decode_fn:    ->(raw, tok) { decode_instruction(raw, tok) },
  execute_fn:   ->(tok) { execute_alu(tok) },
  memory_fn:    ->(tok) { access_memory(tok) },
  writeback_fn: ->(tok) { write_register(tok) }
)

# Optional: add hazard detection and branch prediction.
pipeline.set_hazard_fn(->(stages) { detect_hazards(stages) })
pipeline.set_predict_fn(->(pc) { predict_next_pc(pc) })

# Run for up to 1000 cycles or until HALT.
stats = pipeline.run(1000)
puts stats  # => PipelineStats{cycles=..., completed=..., IPC=..., ...}
```

## Pipeline Configurations

Two preset configurations are provided:

- `CodingAdventures::CpuPipeline.classic_5_stage` -- The textbook MIPS R2000 pipeline (IF, ID, EX, MEM, WB)
- `CodingAdventures::CpuPipeline.deep_13_stage` -- A 13-stage pipeline inspired by ARM Cortex-A78

Custom configurations are supported by providing your own array of `PipelineStage` objects.

## Development

```bash
bundle install
bundle exec rake test
```

## License

MIT
