# 10 — Pipeline Orchestrator

## Overview

The pipeline package is the glue that connects all other packages into a single execution flow. It takes source code and runs it through the full computing stack, capturing the output of each stage for inspection and visualization.

This is not a "layer" in the computing stack — it is the conductor that orchestrates all layers.

## What it does

```
Source code → Lexer → Parser → Compiler → [VM or Assembler → Simulator]
```

The pipeline supports two execution paths:
- **Path A (Interpreted):** Source → Lexer → Parser → Bytecode Compiler → Virtual Machine
- **Path B (Compiled to RISC-V):** Source → Lexer → Parser → RISC-V Compiler → Assembler → RISC-V Simulator
- **Path C (Compiled to ARM):** Source → Lexer → Parser → ARM Compiler → Assembler → ARM Simulator

## Concepts

### Stage Snapshots

Each pipeline stage captures its input and output as a snapshot. This allows the HTML visualizer (and tests) to inspect what happened at every stage:

```python
@dataclass
class StageSnapshot:
    name: str              # "lexer", "parser", "compiler", "vm", etc.
    input: object          # What this stage received
    output: object         # What this stage produced
    duration_ms: float     # How long this stage took
```

### Pipeline Result

A complete pipeline run produces a result containing all snapshots:

```python
@dataclass
class PipelineResult:
    source: str
    target: str                          # "vm", "riscv", "arm"
    stages: list[StageSnapshot]
    tokens: list[Token]                  # Convenience: lexer output
    ast: Program                         # Convenience: parser output
    bytecode: CodeObject | None          # Path A only
    assembly: str | None                 # Path B/C only
    execution_trace: list[TraceStep]     # VM or simulator trace
    final_variables: dict[str, object]   # Final variable state
    output: list[str]                    # Printed output
    error: str | None                    # Any error that occurred
```

## Public API

```python
class Pipeline:
    @staticmethod
    def run(source: str, target: str = "vm") -> PipelineResult: ...
        # Run source code through the full pipeline

    @staticmethod
    def run_stage(source: str, stage: str) -> StageSnapshot: ...
        # Run only up to a specific stage ("lexer", "parser", "compiler")

    @staticmethod
    def available_targets() -> list[str]: ...
        # Returns ["vm", "riscv", "arm"]
```

## Data Flow

```
Input:  Source code (str) + target ("vm" | "riscv" | "arm")
Output: PipelineResult with snapshots of every stage
```

## Dependencies

This package depends on ALL other packages:
- lexer
- parser (lang_parser)
- bytecode-compiler
- virtual-machine
- assembler
- arm-simulator
- riscv-simulator

## Test Strategy

- Run `x = 1 + 2` through VM path: verify final_variables["x"] == 3
- Run `x = 1 + 2` through RISC-V path: verify register x3 == 3
- Verify all stage snapshots are populated
- Verify error handling: invalid source produces error in result, not an exception
- Verify run_stage stops at the requested stage

## JSON Export

The pipeline can export its result as JSON conforming to the **PipelineReport** contract defined in `11-html-visualizer.md`. This enables:
- The HTML renderer to generate static HTML visualizations
- Any language implementation (Ruby, TypeScript) to produce the same JSON
- External tools to consume pipeline data

```python
class Pipeline:
    @staticmethod
    def run_to_json(source: str, target: str = "vm") -> str: ...
        # Run pipeline and return JSON string conforming to PipelineReport schema

    @staticmethod
    def run_to_json_file(source: str, target: str, output_path: str) -> None: ...
        # Run pipeline and write JSON to file
```

## Future Extensions

- **Profiling:** Detailed timing of each stage
- **Diff mode:** Compare VM execution vs hardware execution side by side
- **Batch mode:** Run multiple programs, collect statistics
