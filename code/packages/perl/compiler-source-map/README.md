# CodingAdventures::CompilerSourceMap

Source map chain sidecar for the AOT compiler pipeline. This is a Perl port of the Go package at `code/packages/go/compiler-source-map/`.

## What is a source map chain?

A source map connects machine code bytes back to the source characters that produced them. This package uses a **chain** rather than a flat table so that every pipeline stage is debuggable individually:

```
Source (hello.bf)
     ↓  SourceToAst     — source position → AST node ID
   AST
     ↓  AstToIr         — AST node ID → IR instruction IDs
   IR
     ↓  IrToIr (×N)     — original IR ID → optimised IR IDs (one per pass)
   IR (optimised)
     ↓  IrToMachineCode — IR ID → machine code byte offset + length
Machine code
```

The chain lets you ask:
- "Which AST node produced IR instruction #42?" → look at `AstToIr`
- "Why did the optimiser delete instruction #7?" → look at `IrToIr`
- "What source character caused this machine code?" → compose all segments

## Modules

| Module | Description |
|--------|-------------|
| `SourcePosition` | A span of characters in a source file (`file:line:col len=N`) |
| `SourceToAst` | Segment 1: source positions → AST node IDs |
| `AstToIr` | Segment 2: AST node IDs → IR instruction IDs (one-to-many) |
| `IrToIr` | Segment 3: original IR IDs → optimised IR IDs (one per pass) |
| `IrToMachineCode` | Segment 4: IR IDs → machine code byte offset + length |
| `SourceMapChain` | Full pipeline sidecar with composite forward/reverse queries |

## Usage

```perl
use CodingAdventures::CompilerSourceMap::SourceMapChain;
use CodingAdventures::CompilerSourceMap::SourcePosition;
use CodingAdventures::CompilerSourceMap::IrToMachineCode;

my $chain = CodingAdventures::CompilerSourceMap::SourceMapChain->new_chain;

# Frontend fills segments 1 and 2
my $pos = CodingAdventures::CompilerSourceMap::SourcePosition->new(
    file => 'hello.bf', line => 1, column => 1, length => 1,
);
$chain->{source_to_ast}->add($pos, $ast_node_id);
$chain->{ast_to_ir}->add($ast_node_id, \@ir_ids);

# Backend fills segment 4
my $i2mc = CodingAdventures::CompilerSourceMap::IrToMachineCode->new;
$i2mc->add($ir_id, $mc_offset, $mc_length);
$chain->{ir_to_machine_code} = $i2mc;

# Composite queries
my $mc_entries = $chain->source_to_mc($pos);        # forward
my $source_pos = $chain->mc_to_source($mc_offset);  # reverse
```

## Dependencies

No runtime dependencies. Test dependency: `Test2::V0`.
