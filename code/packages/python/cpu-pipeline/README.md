# cpu-pipeline

**CPU Instruction Pipeline (IF -> ID -> EX -> MEM -> WB)** -- the assembly line at the heart of every CPU.

## What is a CPU Pipeline?

A CPU pipeline overlaps instruction execution so that multiple instructions are in-flight simultaneously. Instead of completing one instruction fully before starting the next (taking 5 cycles per instruction), a pipelined CPU starts a new instruction every cycle:

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

## What This Package Does

This package manages the **flow** of instructions through pipeline stages. It does NOT interpret instructions -- that is the ISA decoder's job. The pipeline moves "tokens" (representing instructions) through stages, handling:

- **Normal advancement**: tokens move one stage per clock cycle
- **Stalls**: freeze earlier stages and insert a "bubble" (NOP)
- **Flushes**: replace speculative instructions with bubbles
- **Statistics**: track IPC, stall cycles, flush cycles

The actual work of each stage is performed by callback functions injected from the CPU core, making the pipeline ISA-independent.

## How It Fits in the Stack

```
Layer 4 (D04): CPU Pipeline  <-- this package
    depends on: nothing (callbacks injected)
    used by: CPU Simulator (programs layer)

Related packages:
    - cache: provides instruction/data cache (injected as fetch/memory callbacks)
    - hazard-detection: provides hazard checking (injected as hazard callback)
    - branch-predictor: provides next-PC prediction (injected as predict callback)
```

## Quick Start

```python
from cpu_pipeline import Pipeline, classic_5_stage

# Define callbacks for your ISA
def fetch(pc: int) -> int:
    return instruction_memory[pc // 4]

def decode(raw: int, tok):
    tok.opcode = "ADD"
    tok.rd = (raw >> 7) & 0x1F
    # ... fill in fields
    return tok

def execute(tok):
    tok.alu_result = tok.rs1 + tok.rs2
    return tok

def memory(tok):
    if tok.mem_read:
        tok.mem_data = data_memory[tok.alu_result]
    return tok

def writeback(tok):
    if tok.reg_write:
        registers[tok.rd] = tok.write_data

# Create and run
config = classic_5_stage()
pipeline = Pipeline(config, fetch, decode, execute, memory, writeback)
stats = pipeline.run(max_cycles=1000)
print(f"IPC: {stats.ipc():.3f}, CPI: {stats.cpi():.3f}")
```

## Configurable Depth

The pipeline depth is configurable. Use `classic_5_stage()` for the textbook pipeline or `deep_13_stage()` for a modern high-performance pipeline:

| Depth | Clock Speed | Misprediction Penalty | Example          |
|-------|------------|----------------------|------------------|
| 5     | 1.0 GHz    | 2 cycles             | Teaching         |
| 13    | 2.2 GHz    | 10 cycles            | ARM Cortex-A78   |

## Development

```bash
uv venv
uv pip install -e ".[dev]"
pytest tests/ -v
ruff check src/ tests/
```

## Package Info

- **PyPI name**: `coding-adventures-cpu-pipeline`
- **Import name**: `cpu_pipeline`
- **Python**: >= 3.12
- **License**: MIT
