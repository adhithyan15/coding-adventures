# compiler-source-map

Source-mapping sidecar for the AOT compiler pipeline.

## Overview

`compiler-source-map` provides the source map chain that flows through every stage of the AOT compiler pipeline. It allows any compiler error or debugging event to be traced back to the original source position.

## Why a chain?

A flat table (machine-code offset → source position) works for the final consumer — a debugger or error reporter. But the chain also helps when debugging the compiler itself:

- "Which AST node produced this IR instruction?" → look at `AstToIr`
- "Why did the optimiser delete instruction #42?" → look at the `IrToIr` segment for that pass
- "The machine code seems wrong — what IR produced it?" → look at `IrToMachineCode` in reverse

## Segments

```
Segment 1: SourceToAst      — source text position  → AST node ID
Segment 2: AstToIr          — AST node ID           → IR instruction IDs
Segment 3: IrToIr           — IR instruction ID     → optimised IR instruction IDs
                              (one segment per optimiser pass)
Segment 4: IrToMachineCode  — IR instruction ID     → machine code byte offset + length
```

## Usage

```rust
use compiler_source_map::{SourceMapChain, SourcePosition, IrToMachineCode};

let mut chain = SourceMapChain::new();

// Frontend fills segments 1 and 2
let pos = SourcePosition { file: "hello.bf".to_string(), line: 1, column: 1, length: 1 };
chain.source_to_ast.add(pos.clone(), 0);      // "+" at col 1 → AST node 0
chain.ast_to_ir.add(0, vec![7, 8, 9, 10]);   // AST node 0 → IR IDs 7-10

// Backend fills segment 4
let mut mc = IrToMachineCode::new();
mc.add(7, 0, 4);   // IR 7 → bytes [0, 4)
mc.add(8, 4, 4);   // IR 8 → bytes [4, 8)
mc.add(9, 8, 4);
mc.add(10, 12, 4);
chain.ir_to_machine_code = Some(mc);

// Forward: source position → machine code offsets
let results = chain.source_to_mc(&pos).unwrap();
assert_eq!(results[0].mc_offset, 0);

// Reverse: machine code offset → source position
let found = chain.mc_to_source(0).unwrap();
assert_eq!(found.column, 1);
```

## Where it fits

```
Brainfuck source
      |
 [brainfuck-ir-compiler] — fills SourceToAst + AstToIr
      |
 [optimizer] — appends IrToIr segments per pass
      |
 [codegen-riscv] — fills IrToMachineCode
      |
 SourceMapChain (complete chain for debuggers/error reporters)
```
