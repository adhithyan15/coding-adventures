# compiler-source-map (Python)

Source map chain sidecar for the AOT compiler pipeline.

## What It Does

This package provides the `SourceMapChain` — a data structure that flows
through every stage of the AOT compiler pipeline. Each stage appends its own
segment, enabling full bidirectional traceability:

- **Forward**: "Which machine code bytes correspond to this source position?"
- **Reverse**: "Which source position produced this machine code byte?"

## Pipeline Segments

```
Segment 1: SourceToAst      — source text position  → AST node ID
Segment 2: AstToIr          — AST node ID            → IR instruction IDs
Segment 3: IrToIr           — IR instruction ID      → optimised IR IDs
                               (one segment per optimiser pass)
Segment 4: IrToMachineCode  — IR instruction ID      → machine code offset
```

## Usage

```python
from compiler_source_map import (
    SourceMapChain, SourcePosition, IrToIr, IrToMachineCode
)

# Create a fresh chain
chain = SourceMapChain.new()

# Segment 1: frontend records source positions
chain.source_to_ast.add(SourcePosition("hello.bf", 1, 1, 1), ast_node_id=0)

# Segment 2: frontend records which IR instructions each AST node produced
chain.ast_to_ir.add(ast_node_id=0, ir_ids=[2, 3, 4, 5])

# Segment 3: optimizer pass (identity — each IR ID maps to itself)
identity = IrToIr(pass_name="identity")
for i in range(6):
    identity.add_mapping(i, [i])
chain.add_optimizer_pass(identity)

# Segment 4: backend fills machine code positions
mc = IrToMachineCode()
mc.add(ir_id=2, mc_offset=0x0C, mc_length=8)
chain.ir_to_machine_code = mc

# Forward lookup: source position → machine code
entries = chain.source_to_mc(SourcePosition("hello.bf", 1, 1, 1))

# Reverse lookup: machine code offset → source position
pos = chain.mc_to_source(0x0C)
```

## How It Fits in the Stack

```
brainfuck source  →  [brainfuck-ir-compiler]  →  IR + SourceMapChain
                                                        ↓
                      [compiler-ir-optimizer]  →  IR + chain (extended)
                                                        ↓
                      [codegen-riscv]          →  binary + chain (complete)
```

## Dependencies

None (no runtime dependencies).

## Tests

```bash
cd code/packages/python/compiler-source-map
mise exec -- uv run pytest tests/ --cov=src --cov-report=term-missing -v
```
