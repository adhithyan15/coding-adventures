# compiler-source-map

A **source-mapping sidecar** that flows through every stage of the AOT
compiler pipeline, from original source text to final machine code bytes.

## What It Does

When you compile `hello.bf` to a RISC-V executable, the source map chain
tracks the provenance of every machine code byte back to the exact character
in the source file that produced it. This enables:

- **Debuggers** to show source lines at breakpoints
- **Profilers** to attribute CPU time to source positions
- **Error reporters** to show where a runtime fault originated
- **Compiler developers** to trace bugs through every pipeline stage

## Architecture

The source map is not one flat table — it's a **chain of segment maps**,
each connecting two adjacent representations:

```
Source Position  →  AST Node ID  →  IR Instr ID  →  Opt IR ID  →  MC Offset
  (SourceToAst)    (AstToIr)       (IrToIr × N)    (IrToMachineCode)
```

Each segment is bidirectional, supporting both forward lookups (source →
machine code) and reverse lookups (machine code → source).

## Usage

```go
import sm "github.com/adhithyan15/coding-adventures/code/packages/go/compiler-source-map"

// Create a new chain
chain := sm.NewSourceMapChain()

// Frontend fills segments 1 and 2
chain.SourceToAst.Add(sm.SourcePosition{
    File: "hello.bf", Line: 1, Column: 1, Length: 1,
}, astNodeID)
chain.AstToIr.Add(astNodeID, []int{irID1, irID2, irID3})

// Optimizer appends segment 3
identity := sm.NewIrToIr("identity")
identity.AddMapping(irID1, []int{irID1})
chain.AddOptimizerPass(identity)

// Backend fills segment 4
mc := &sm.IrToMachineCode{}
mc.Add(irID1, 0x00, 4)
chain.IrToMachineCode = mc

// Composite queries
entries := chain.SourceToMC(sm.SourcePosition{File: "hello.bf", Line: 1, Column: 1})
pos := chain.MCToSource(0x14)
```

## Segment Types

| Segment | From | To | Cardinality |
|---------|------|----|-------------|
| `SourceToAst` | Source position | AST node ID | 1:1 |
| `AstToIr` | AST node ID | IR instruction IDs | 1:many |
| `IrToIr` | IR instruction ID | Optimised IR IDs | 1:many (per pass) |
| `IrToMachineCode` | IR instruction ID | MC offset + length | 1:1 |

## Part Of

This package is part of the AOT native compiler pipeline (spec BF03).
See the [spec](../../../specs/BF03-aot-native-compiler.md) for the full
architecture.
