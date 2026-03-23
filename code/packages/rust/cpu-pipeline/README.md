# cpu-pipeline

A configurable N-stage CPU instruction pipeline simulator in Rust.

## What It Does

This crate manages the **flow** of instructions through pipeline stages. It does NOT interpret instructions -- that is the ISA decoder's job. The pipeline moves "tokens" (representing instructions) through stages, handling:

- **Normal advancement**: tokens move one stage per clock cycle
- **Stalls**: freeze earlier stages and insert a "bubble" (NOP)
- **Flushes**: replace speculative instructions with bubbles
- **Statistics**: track IPC, stall cycles, flush cycles

## The Classic 5-Stage Pipeline

```text
Stage 1: IF  (Instruction Fetch)  -- read instruction from memory at PC
Stage 2: ID  (Instruction Decode) -- decode opcode, read registers
Stage 3: EX  (Execute)            -- ALU operation, branch resolution
Stage 4: MEM (Memory Access)      -- load/store data from/to memory
Stage 5: WB  (Write Back)         -- write result to register file
```

## How It Fits in the Stack

This is layer D04 in the coding-adventures CPU stack:

```text
D01: Logic Gates (AND, OR, NOT, ...)
D02: Arithmetic (adder, ALU)
D03: Cache, Branch Predictor, Hazard Detection
D04: CPU Pipeline  <-- this crate
```

The pipeline uses **dependency injection** via callback functions. It accepts fetch, decode, execute, memory, and writeback callbacks so it can work with any ISA without importing specific implementations.

## Usage

```rust
use cpu_pipeline::{Pipeline, PipelineConfig, PipelineToken};

// Create a classic 5-stage pipeline with your callbacks
let config = PipelineConfig::classic_5_stage();
let mut pipeline = Pipeline::new(
    config,
    Box::new(|pc| { /* fetch instruction at pc */ 0 }),
    Box::new(|raw, tok| { /* decode raw into tok */ tok }),
    Box::new(|tok| { /* execute */ tok }),
    Box::new(|tok| { /* memory access */ tok }),
    Box::new(|tok| { /* writeback */ }),
).unwrap();

// Run for 100 cycles or until HALT
let stats = pipeline.run(100);
println!("IPC: {:.3}", stats.ipc());
```

## Configurable Depth

The pipeline depth is configurable. Use `PipelineConfig::classic_5_stage()` for the textbook 5-stage pipeline, or `PipelineConfig::deep_13_stage()` for a modern 13-stage configuration inspired by ARM Cortex-A78.

## Testing

```bash
cargo test -p cpu-pipeline
```
