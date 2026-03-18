# Intel 4004 Simulator (Go Port)

**Layer 4-d of the computing stack** — simulates the 1971 first commercial microprocessor architectures.

## Overview
The Intel 4004 was an incredibly constrained 4-bit data path processor. We diverge significantly from the generic 32-bit `cpu-simulator` layout since the Accumulator model doesn't interact generically with memory blocks using conventional means.

Because it was designed initially for Calculators, it effectively simulates adding numbers sequentially through the bounds of an explicitly 4-bit width threshold, rolling over naturally utilizing the `Carry` indicator. This is heavily opposed to the Stack semantics of WASM or the robust instruction constraints of RISC-V.

## Usage
```go
import (
	"github.com/adhithyan15/coding-adventures/code/packages/go/intel4004-simulator"
)

sim := intel4004simulator.NewIntel4004Simulator(4096)

// Intel 4004 explicitly handles "1 + 2" using raw Accumulator swaps
program := []byte{
    intel4004simulator.EncodeLdm(1), // Accumulator = 1
    intel4004simulator.EncodeXch(0), // Swap Accumulator out -> Store into Register 0
    intel4004simulator.EncodeLdm(2), // Accumulator = 2
    intel4004simulator.EncodeAdd(0), // Add Register 0 with Accumulator target
    intel4004simulator.EncodeXch(1), // Swap Accumulator out -> Store into Register 1
    intel4004simulator.EncodeHlt(),  // Halts
}

traces := sim.Run(program, 1000)
```
