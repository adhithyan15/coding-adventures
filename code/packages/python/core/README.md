# coding-adventures-core

Configurable processor core -- integrates pipeline, caches, branch predictor, hazard detection into a complete CPU core.

## Overview

This package is D05 in the coding-adventures CPU architecture stack. It composes all D-series micro-architectural components into a complete, configurable processor core:

| Component | Package | Role |
|-----------|---------|------|
| Cache Hierarchy | `coding-adventures-cache` (D01) | L1I, L1D, L2 caches |
| Branch Predictor | `coding-adventures-branch-predictor` (D02) | Speculative fetch direction |
| Hazard Detection | `coding-adventures-hazard-detection` (D03) | Data, control, structural hazards |
| Pipeline | `coding-adventures-cpu-pipeline` (D04) | Instruction flow through stages |
| **Core** | **This package (D05)** | **Wires everything together** |

The Core itself defines no new micro-architectural behavior. It wires the parts together, like a motherboard connects CPU, RAM, and peripherals. The same Core can run ARM, RISC-V, or any custom ISA -- the ISA decoder is injected from outside.

## Quick Start

```python
from core import Core, simple_config, MockDecoder, encode_program, encode_addi, encode_halt

config = simple_config()
decoder = MockDecoder()
c = Core(config, decoder)

program = encode_program(encode_addi(1, 0, 42), encode_halt())
c.load_program(program, 0)
stats = c.run(100)

print(f"R1 = {c.read_register(1)}")  # R1 = 42
print(f"IPC: {stats.ipc():.3f}")
```

## Preset Configurations

| Config | Inspired By | Pipeline | Predictor | L1 Cache | L2 Cache | Registers |
|--------|------------|----------|-----------|----------|----------|-----------|
| `simple_config()` | MIPS R2000 | 5-stage | Static (not taken) | 4KB DM | None | 16x32 |
| `cortex_a78_like_config()` | ARM Cortex-A78 | 13-stage | 2-bit (4096) | 64KB 4-way | 256KB 8-way | 31x64 |

## Multi-Core

```python
from core import MultiCoreCPU, default_multi_core_config, MockDecoder, encode_program, encode_addi, encode_halt

config = default_multi_core_config()  # 2 cores
decoders = [MockDecoder(), MockDecoder()]
cpu = MultiCoreCPU(config, decoders)

prog = encode_program(encode_addi(1, 0, 42), encode_halt())
cpu.load_program(0, prog, 0)       # Core 0 at address 0
cpu.load_program(1, prog, 4096)    # Core 1 at address 4096

stats = cpu.run(200)
print(f"Core 0 R1 = {cpu.cores[0].read_register(1)}")
print(f"Core 1 R1 = {cpu.cores[1].read_register(1)}")
```

## Custom ISA Decoder

Implement the `ISADecoder` protocol to plug in any instruction set:

```python
from core import ISADecoder
from cpu_pipeline import PipelineToken
from core import RegisterFile

class MyDecoder:
    def decode(self, raw_instruction: int, token: PipelineToken) -> PipelineToken:
        # Fill in opcode, registers, control signals
        ...

    def execute(self, token: PipelineToken, reg_file: RegisterFile) -> PipelineToken:
        # Compute ALU result, resolve branches
        ...

    def instruction_size(self) -> int:
        return 4
```

## Architecture

```
                    ISA Decoder (injected)
                         |
                         v
  IF --> ID --> EX --> MEM --> WB    (Pipeline stages)
   |      |     |      |      |
   |      |     |      |      +--- RegisterFile (writeback)
   |      |     |      +---------- CacheHierarchy (L1D read/write)
   |      |     +----------------- BranchPredictor + BTB (update)
   |      +------------------------ ISADecoder.decode()
   +------------------------------- CacheHierarchy (L1I fetch)
                                    MemoryController (backing store)
```

## Development

```bash
uv venv && uv pip install -e ".[dev]"
uv pip install -e "../cpu-pipeline[dev]" -e "../cache[dev]" -e "../branch-predictor[dev]" -e "../hazard-detection[dev]" -e "../clock[dev]"
python -m pytest tests/ -v
```
