# RISC-V Simulator (Go Port)

**Layer 4b of the computing stack** — implements the RISC-V RV32I base integer instruction set simulator.

## Overview
RISC-V is an open-source instruction set architecture designed at UC Berkeley. It is built on the philosophy of a Reduced Instruction Set Computer — meaning it favors a small number of cleanly encoded and incredibly simple instructions.
This simulator interfaces with the `cpu-simulator` generic architecture.

### The MVP Instruction Set
This package currently supports the minimal instructions needed to compute `x = 1 + 2`:
- `addi`: Add Immediate (I-Type)
- `add`: Add (R-Type)
- `sub`: Subtract (R-Type)
- `ecall`: Environment Call (Used for halting)

### Register x0
RISC-V forces Register 0 (`x0`) to always be strictly `0`. Writes are silently ignored, allowing simple logic extensions (e.g. `addi x1, x0, 5` effectively handles loading the value `5` into `x1`).

## Usage
```go
import (
	"github.com/adhithyan15/coding-adventures/code/packages/go/riscv-simulator"
)

sim := riscvsimulator.NewRiscVSimulator(65536)

// Manually craft machine code bytes
program := riscvsimulator.Assemble([]uint32{
    riscvsimulator.EncodeAddi(1, 0, 1), // x1 = x0 + 1
    riscvsimulator.EncodeAddi(2, 0, 2), // x2 = x0 + 2
    riscvsimulator.EncodeAdd(3, 1, 2),  // x3 = x1 + x2
    riscvsimulator.EncodeEcall(),       // halt
})

// Run the full pipeline tracing
traces := sim.Run(program)
```
