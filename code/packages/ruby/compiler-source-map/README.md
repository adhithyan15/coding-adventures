# coding_adventures_compiler_source_map

Source map chain sidecar for the AOT compiler pipeline.

## What it does

This gem provides the multi-segment source map chain that flows alongside the compiler through every transformation stage. It makes the compiler pipeline **transparent and debuggable** at every level, not just at the final machine-code output.

## The four segments

```
Source text     → SourceToAst      → AST node IDs
AST node IDs    → AstToIr          → IR instruction IDs
IR inst. IDs    → IrToIr (×N)      → Optimised IR instruction IDs
IR inst. IDs    → IrToMachineCode  → Machine code byte offsets
```

Each segment is filled by a different pipeline stage:

| Segment | Produced by |
|---------|------------|
| `SourceToAst` | Language frontend (brainfuck-ir-compiler) |
| `AstToIr` | Language frontend (brainfuck-ir-compiler) |
| `IrToIr` | Optimiser passes (one segment per pass) |
| `IrToMachineCode` | Code generator backend |

## Composite queries

Once all segments are filled, the `SourceMapChain` supports end-to-end lookups:

```ruby
chain.source_to_mc(pos)   # SourcePosition → [IrToMachineCodeEntry, ...]
chain.mc_to_source(offset) # Integer → SourcePosition or nil
```

## Usage

```ruby
require "coding_adventures_compiler_source_map"

SM = CodingAdventures::CompilerSourceMap

chain = SM::SourceMapChain.new

# Segment 1: parser records source positions
pos = SM::SourcePosition.new(file: "hello.bf", line: 1, column: 1, length: 1)
chain.source_to_ast.add(pos, 0)   # "+" at 1:1 → AST node 0

# Segment 2: frontend records AST → IR
chain.ast_to_ir.add(0, [5, 6, 7, 8])  # AST node 0 → IR instructions 5–8

# Segment 4: backend records IR → machine code
mc = SM::IrToMachineCode.new
mc.add(5, 0, 4)   # IR#5 → 4 bytes at offset 0
chain.ir_to_machine_code = mc

# Composite query
results = chain.source_to_mc(pos)
results[0].mc_offset  #=> 0

# Reverse query
source = chain.mc_to_source(0)
source.file   #=> "hello.bf"
source.line   #=> 1
```
