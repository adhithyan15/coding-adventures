# Pipeline

**Layer 10 of the computing stack** — the orchestrator that ties all packages together.

## What this package does

Chains every layer of the computing stack into a single execution flow, supporting multiple target backends:

| Execution Path | Description |
|----------------|-------------|
| VM (interpreted) | Lexer → Parser → Bytecode Compiler → Virtual Machine |
| RISC-V (compiled) | Lexer → Parser → Assembler → RISC-V Simulator |
| ARM (compiled) | Lexer → Parser → Assembler → ARM Simulator |

Each stage captures a snapshot of its output so the full transformation can be visualized step by step.

## Where it fits

```
Logic Gates → Arithmetic → CPU → ARM → RISC-V → Assembler → Lexer → Parser → Compiler → VM → [Pipeline]
```

This package is the **top-level orchestrator** that imports and coordinates all other packages in the computing stack.

## Installation

```bash
uv add coding-adventures-pipeline
```

## Usage

```python
from pipeline import Pipeline

result = Pipeline.run("print(1 + 2)", target="vm")

# Inspect stage snapshots
for stage in result.stages:
    print(stage.name, stage.output)
```

## Spec

See [10-pipeline.md](../../../specs/10-pipeline.md) for the full specification.
