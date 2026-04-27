# @coding-adventures/compiler-source-map

Source map chain for the AOT compiler pipeline. TypeScript port of `code/packages/go/compiler-source-map`.

## What is the source map chain?

The source map chain is a sidecar data structure that flows through every stage of the compiler pipeline. It connects source text positions to machine code byte offsets, and back again.

```
Source position (hello.bf:1:3)
    ↓  Segment 1: SourceToAst
AST node ID (42)
    ↓  Segment 2: AstToIr
IR instruction IDs [7, 8, 9, 10]
    ↓  Segment 3: IrToIr (per optimizer pass)
Optimized IR instruction IDs [100]
    ↓  Segment 4: IrToMachineCode
Machine code offset + length (0x14, 8 bytes)
```

## Usage

```typescript
import {
  SourceMapChain,
  IrToMachineCode,
  IrToIr
} from "@coding-adventures/compiler-source-map";

// Create a chain
const chain = new SourceMapChain();

// Frontend fills Segments 1 and 2
chain.sourceToAst.add({ file: "hello.bf", line: 1, column: 1, length: 1 }, 0);
chain.astToIr.add(0, [0, 1, 2, 3]);

// Optional: optimizer fills Segment 3
const pass = new IrToIr("contraction");
pass.addMapping(0, [100]);
pass.addMapping(1, [100]);
chain.addOptimizerPass(pass);

// Backend fills Segment 4
chain.irToMachineCode = new IrToMachineCode();
chain.irToMachineCode.add(100, 0, 8);

// Composite forward query: source → machine code
const entries = chain.sourceToMC({ file: "hello.bf", line: 1, column: 1, length: 1 });

// Composite reverse query: machine code → source
const pos = chain.mcToSource(4); // byte 4 in .text → source position
```

## Stack position

Layer 5 — Compiler Infrastructure. Sidecar to the IR package; produced by brainfuck-ir-compiler and consumed by the backend.
