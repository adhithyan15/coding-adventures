# Hazard Detection

Pipeline hazard detection and resolution for a pipelined CPU. This package detects the three types of hazards that can occur when multiple instructions are in flight simultaneously:

1. **Data hazards (RAW)** - An instruction reads a register that a previous instruction hasn't finished writing yet. Resolved by forwarding (bypassing) values from later pipeline stages, or stalling when forwarding isn't possible (load-use hazard).

2. **Control hazards** - A branch was mispredicted, so the pipeline fetched wrong instructions. Resolved by flushing the pipeline and restarting from the correct address.

3. **Structural hazards** - Two instructions need the same hardware resource (ALU, FP unit, memory port) at the same time. Resolved by stalling one instruction.

## How It Fits in the Stack

This package sits between the pipeline and the instruction decoder. It receives `PipelineSlot` descriptors (ISA-independent) from each pipeline stage and returns a `HazardResult` telling the pipeline what to do (forward, stall, flush, or proceed normally).

```
┌────────────┐    ┌────────────────┐    ┌──────────┐
│  Pipeline   │───>│ Hazard Unit    │───>│ Pipeline  │
│  Stages     │    │ (this package) │    │ Control   │
└────────────┘    └────────────────┘    └──────────┘
```

The package is standalone - it works with any pipeline implementation and any ISA.

## Usage

```python
from hazard_detection import HazardUnit, PipelineSlot, HazardAction

# Create the hazard unit (configurable hardware resources)
unit = HazardUnit(num_alus=1, num_fp_units=1, split_caches=True)

# Each cycle, describe what's in each pipeline stage
if_stage = PipelineSlot(valid=True, pc=0x100C)
id_stage = PipelineSlot(valid=True, pc=0x1008, source_regs=(1, 2), uses_alu=True)
ex_stage = PipelineSlot(valid=True, pc=0x1004, dest_reg=1, dest_value=42, uses_alu=True)
mem_stage = PipelineSlot(valid=True, pc=0x1000, dest_reg=3, uses_alu=True)

# Check for hazards
result = unit.check(if_stage, id_stage, ex_stage, mem_stage)

if result.action == HazardAction.FORWARD_FROM_EX:
    print(f"Forward value {result.forwarded_value} from EX stage")
elif result.action == HazardAction.STALL:
    print(f"Stall for {result.stall_cycles} cycle(s): {result.reason}")
elif result.action == HazardAction.FLUSH:
    print(f"Flush {result.flush_count} stages: {result.reason}")

# Performance stats
print(f"Total stalls: {unit.stall_count}")
print(f"Total flushes: {unit.flush_count}")
print(f"Total forwards: {unit.forward_count}")
```

## Individual Detectors

You can also use the detectors independently:

```python
from hazard_detection import DataHazardDetector, ControlHazardDetector, StructuralHazardDetector

# Data hazard detection only
data_detector = DataHazardDetector()
result = data_detector.detect(id_stage, ex_stage, mem_stage)

# Control hazard detection only
control_detector = ControlHazardDetector()
result = control_detector.detect(ex_stage)

# Structural hazard detection only
structural_detector = StructuralHazardDetector(num_alus=2, split_caches=True)
result = structural_detector.detect(id_stage, ex_stage)
```

## Priority System

When multiple hazards occur simultaneously, the highest-priority action wins:

| Priority | Action | Meaning |
|----------|--------|---------|
| 4 (highest) | FLUSH | Branch misprediction - wrong instructions in pipeline |
| 3 | STALL | Data not ready - must wait |
| 2 | FORWARD_FROM_EX | Forward value from EX stage |
| 1 | FORWARD_FROM_MEM | Forward value from MEM stage |
| 0 (lowest) | NONE | All clear, proceed normally |

## Installation

```bash
pip install coding-adventures-hazard-detection
```

## Development

```bash
uv venv && uv pip install -e ".[dev]"
python -m pytest tests/ -v --cov=hazard_detection --cov-report=term-missing
```
