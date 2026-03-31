# coding-adventures-cpu-simulator (Lua)

CPU simulator building blocks — Memory, SparseMemory, and RegisterFile.
Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) project.

## What is a CPU Simulator?

A CPU simulator models the fundamental components of a processor:

- **Registers** — fast, small storage inside the CPU (0.3 ns access)
- **Memory** — large, slower storage connected to the CPU (100 ns access)

The classic fetch-decode-execute cycle repeats forever:

```
1. FETCH   — Read instruction from memory at the Program Counter
2. DECODE  — Parse opcode, source/destination registers, immediate values
3. EXECUTE — Run the ALU (arithmetic/logic unit) operation
4. STORE   — Write result to register or memory
5. ADVANCE — Move PC to the next instruction
```

## Package Contents

| Type | Description |
|------|-------------|
| `Memory` | Fixed-size byte-addressable RAM |
| `SparseMemory` | Sparse address space (efficient for large, mostly-empty spaces) |
| `RegisterFile` | Fast CPU register storage with bit-width masking |

## Usage

```lua
local cpu = require("coding_adventures.cpu_simulator")
local Memory       = cpu.Memory
local RegisterFile = cpu.RegisterFile

-- Create 64KB of memory and 16 32-bit registers
local mem = Memory.new(65536)
local rf  = RegisterFile.new(16, 32)

-- Load a simple program
mem:load_bytes(0, { 0x01, 0x00, 0x00, 0x00 })  -- instruction at address 0

-- Read a word back
local word = mem:read_word(0)
print(string.format("0x%08X", word))  -- 0x00000001

-- Use registers
rf:write(0, 42)
rf:write(1, 58)
print(rf:read(0) + rf:read(1))  -- 100
```

## Layer Position

```
Logic Gates → Arithmetic → CPU Simulator (here) → ARM → Assembler → ...
```

This package provides the infrastructure used by ISA simulators (ARM, RISC-V)
and the CPU pipeline package.
