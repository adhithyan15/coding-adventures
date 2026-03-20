# cpu-simulator

CPU simulator -- registers, memory, and the fetch-decode-execute pipeline.

## What is this?

This crate provides the core framework for simulating a CPU. It models the fundamental components that every processor shares:

- **Register File** -- a small, fast set of numbered storage slots inside the CPU
- **Memory** -- a large, slow byte-addressable array (simulated RAM)
- **Pipeline** -- the fetch-decode-execute cycle that drives all computation
- **CPU** -- the orchestrator that ties registers, memory, and the pipeline together

## Architecture-independent design

The CPU does not know what instruction set it is running. Instead, it relies on two traits:

- `InstructionDecoder` -- translates raw 32-bit instruction words into structured decode results
- `InstructionExecutor` -- performs the decoded operation, modifying registers and memory

To simulate a specific architecture (RISC-V, ARM, WASM, Intel 4004), implement these two traits and pass them to `CPU::new`.

## How it fits in the stack

This is Layer 3 of the computing stack:

```
Layer 1: logic-gates     (AND, OR, NOT, XOR, NAND, NOR, XNOR)
Layer 2: arithmetic      (adders, ALU)
Layer 3: cpu-simulator   <-- you are here
Layer 4: ISA simulators  (RISC-V, ARM, etc.)
```

## Usage

```rust
use cpu_simulator::{CPU, Memory, RegisterFile, format_pipeline};

// Implement InstructionDecoder and InstructionExecutor for your ISA,
// then create a CPU:
//
//   let cpu = CPU::new(
//       Box::new(my_decoder),
//       Box::new(my_executor),
//       32,     // 32 registers
//       32,     // 32-bit registers
//       65536,  // 64 KB of memory
//   );
//   cpu.load_program(&machine_code, 0);
//   let traces = cpu.run(10000);
//   for trace in &traces {
//       println!("{}", format_pipeline(trace));
//   }
```

## Dependencies

- `arithmetic` -- adder circuits and ALU (Layer 2)

## Running tests

```bash
cargo test -p cpu-simulator -- --nocapture
```
