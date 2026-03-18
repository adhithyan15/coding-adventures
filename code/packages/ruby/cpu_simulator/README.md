# CPU Simulator

A generic CPU simulator providing registers, memory, program counter, and the
fetch-decode-execute pipeline. Part of the **coding-adventures** computing stack.

## What it does

This gem provides the core CPU abstraction — the "brain" of a computer — without
being tied to any specific instruction set. You plug in a decoder and executor
for the ISA you want (ARM, RISC-V, WASM, Intel 4004) and the CPU drives the
pipeline.

## How it fits in the stack

```
Layer 8: CPU Simulator  <-- this gem
Layer 7: ISA Simulators (ARM, RISC-V, WASM, 4004, JVM, CLR)
```

The ISA simulators depend on this gem for registers, memory, and the pipeline
framework.

## Usage

```ruby
require "coding_adventures_cpu_simulator"

# Create a CPU with your own decoder and executor
cpu = CodingAdventures::CpuSimulator::CPU.new(
  decoder: my_decoder,
  executor: my_executor,
  num_registers: 32,
  bit_width: 32
)

# Load a program and run it
cpu.load_program(machine_code_bytes)
traces = cpu.run

# Inspect the results
traces.each { |t| puts t.format_pipeline }
```
