# ARM Simulator (Go Port)

**Layer 4-b of the computing stack** — implements the foundational ARMv7 32-bit architecture subset structure.

## Overview
Whereas RISC-V represents an idealized instruction set avoiding historical cruft, ARM (historically meaning Acorn RISC Machine) models commercial real-world complexities. This module attaches onto `cpu-simulator` precisely to demonstrate these fundamental decoding boundaries.

### Distinct Differences
1. **16 Registers Instead of 32**: Memory addresses are conserved since there are less available registers.
2. **PC is Accessible (R15)**: Program execution flow modifications can literally occur by overriding R15.
3. **No Automatic Constraints**: ARM registers lack `x0` enforcing constraints (the zero-hardwired pseudo-register).
4. **Conditional Executions**: The MSB nibble determines if instructions execute contextually based on recent Flags without dedicating specific looping instructions.

## Usage
```go
import (
	"github.com/adhithyan15/coding-adventures/code/packages/go/arm-simulator"
)

sim := armsimulator.NewARMSimulator(65536)

program := armsimulator.Assemble([]uint32{
    armsimulator.EncodeMovImm(0, 1), // R0 = 1
    armsimulator.EncodeMovImm(1, 2), // R1 = 2
    armsimulator.EncodeAdd(2, 0, 1), // R2 = R0 + R1
    armsimulator.EncodeHlt(),        // halt
})

// Run simulation
traces := sim.Run(program)
```
