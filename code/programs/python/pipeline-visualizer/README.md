# Pipeline Visualizer

Runs source code through the full computing stack pipeline (lexer, parser, compiler, VM/simulator) and produces a self-contained HTML file that visualizes every stage.

## Usage

```bash
python generate_report.py
```

This generates `pipeline-report.html` in the current directory.

## Targets

The visualizer supports different compilation targets:

- **vm** — stack-based virtual machine
- **riscv** — RISC-V assembly
- **arm** — ARM assembly

## Specs

- [specs/10-pipeline.md](../../../specs/10-pipeline.md) — Pipeline design
- [specs/11-html-visualizer.md](../../../specs/11-html-visualizer.md) — HTML visualizer design
