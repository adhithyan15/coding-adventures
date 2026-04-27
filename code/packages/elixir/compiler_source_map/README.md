# compiler_source_map (Elixir)

Elixir port of the source-map chain for the AOT compiler pipeline.

## What is this?

This package provides the source-map sidecar that flows through every stage of the compiler
pipeline, recording how source positions map all the way to machine-code bytes.

## How it fits in the stack

```
Source text
    ↓ Segment 1: SourceToAst  (source position → AST node ID)
AST
    ↓ Segment 2: AstToIr  (AST node ID → IR instruction IDs)
IR
    ↓ Segment 3: IrToIr  (IR ID → optimised IR IDs, one segment per pass)
Optimised IR
    ↓ Segment 4: IrToMachineCode  (IR ID → machine code byte offset)
Machine code
```

The chain supports **composite queries** in both directions:
- `SourceMapChain.source_to_mc/2` — given a source position, find the machine-code bytes.
- `SourceMapChain.mc_to_source/2` — given a machine-code offset, find the source position.

## Usage

```elixir
alias CodingAdventures.CompilerSourceMap.{
  SourcePosition, SourceMapChain, SourceToAst, AstToIr, IrToMachineCode
}

chain = SourceMapChain.new()

# Segment 1: parser records source → AST
pos = %SourcePosition{file: "hello.bf", line: 1, column: 1, length: 1}
chain = %{chain | source_to_ast: SourceToAst.add(chain.source_to_ast, pos, 0)}

# Segment 2: compiler records AST → IR
chain = %{chain | ast_to_ir: AstToIr.add(chain.ast_to_ir, 0, [7, 8, 9, 10])}

# Segment 4: backend records IR → machine code
mc = IrToMachineCode.add(%IrToMachineCode{}, 7, 0x14, 4)
chain = %{chain | ir_to_machine_code: mc}

# Forward: source → MC
results = SourceMapChain.source_to_mc(chain, pos)
# => [{7, 0x14, 4}]

# Reverse: MC → source
source_pos = SourceMapChain.mc_to_source(chain, 0x14)
# => %SourcePosition{file: "hello.bf", ...}
```

## Modules

| Module | Purpose |
|--------|---------|
| `SourcePosition` | A span of source characters (file, line, column, length) |
| `SourceToAst` | Segment 1: source positions → AST node IDs |
| `AstToIr` | Segment 2: AST node IDs → IR instruction IDs |
| `IrToIr` | Segment 3: IR IDs → optimised IR IDs (one per optimiser pass) |
| `IrToMachineCode` | Segment 4: IR IDs → machine code byte offsets |
| `SourceMapChain` | All segments + composite forward/reverse queries |

## Running tests

```bash
mix deps.get && mix test --cover
```
