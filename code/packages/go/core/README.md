# Core (D05) -- Processor Core Integration Package

The `core` package is the integration point for the coding-adventures CPU simulator stack. It composes all D-series micro-architectural components into a complete, configurable processor core.

## What It Does

A processor core is a composition of independently designed sub-components:

```
Core
├── Pipeline (D04)         -- IF -> ID -> EX -> MEM -> WB
├── Branch Predictor (D02) -- speculative fetch direction
├── Hazard Detection (D03) -- data, control, structural hazards
├── Cache Hierarchy (D01)  -- L1I + L1D + optional L2
├── Register File          -- fast operand storage
├── Clock                  -- cycle-accurate timing
└── Memory Controller      -- access to main memory
```

The Core itself defines no new micro-architectural behavior. It wires the parts together, like a motherboard connects CPU, RAM, and peripherals. The same Core can run ARM, RISC-V, or any custom ISA by swapping the ISA decoder.

## How It Fits in the Stack

```
ISA Simulators (Layer 7)
├── ARM decoder
├── RISC-V decoder
└── Custom decoder
        │
        │ (ISA decoder injected here)
        ▼
Core (D05) ← THIS PACKAGE
├── Pipeline (D04)
├── Branch Predictor (D02)
├── Hazard Detection (D03)
├── Cache Hierarchy (D01)
├── Register File
└── Clock
```

## Usage

### Single Core

```go
package main

import "github.com/adhithyan15/coding-adventures/code/packages/go/core"

func main() {
    // Create a simple teaching core with a mock decoder.
    config := core.SimpleConfig()
    decoder := core.NewMockDecoder()
    c, _ := core.NewCore(config, decoder)

    // Load a program: R1 = 42, then halt.
    program := core.EncodeProgram(
        core.EncodeADDI(1, 0, 42),
        core.EncodeHALT(),
    )
    c.LoadProgram(program, 0)

    // Run until halt.
    stats := c.Run(10000)
    fmt.Printf("R1 = %d\n", c.ReadRegister(1))       // Output: R1 = 42
    fmt.Printf("IPC = %.3f\n", stats.IPC())
}
```

### Multi-Core

```go
config := core.DefaultMultiCoreConfig()
decoders := []core.ISADecoder{core.NewMockDecoder(), core.NewMockDecoder()}
mc, _ := core.NewMultiCoreCPU(config, decoders)

// Load different programs on different cores.
mc.LoadProgram(0, prog0, 0)
mc.LoadProgram(1, prog1, 4096)

stats := mc.Run(10000)
```

### Custom ISA Decoder

Implement the `ISADecoder` interface to plug in any instruction set:

```go
type ISADecoder interface {
    Decode(rawInstruction int, token *cpupipeline.PipelineToken) *cpupipeline.PipelineToken
    Execute(token *cpupipeline.PipelineToken, regFile *RegisterFile) *cpupipeline.PipelineToken
    InstructionSize() int
}
```

## Configuration Presets

| Preset | Pipeline | Predictor | L1I | L1D | L2 | Registers |
|--------|----------|-----------|-----|-----|----|-----------|
| Simple | 5-stage | Static (not taken) | 4KB 1-way | 4KB 1-way | None | 16x32 |
| CortexA78Like | 13-stage | 2-bit (4096) | 64KB 4-way | 64KB 4-way | 256KB 8-way | 31x64 |

## Mock Decoder Instruction Set

The built-in `MockDecoder` supports these instructions for testing:

| Opcode | Mnemonic | Encoding | Description |
|--------|----------|----------|-------------|
| 0x00 | NOP | - | No operation |
| 0x01 | ADD | Rd, Rs1, Rs2 | Rd = Rs1 + Rs2 |
| 0x02 | LOAD | Rd, [Rs1+imm] | Rd = Memory[Rs1 + imm] |
| 0x03 | STORE | [Rs1+imm], Rs2 | Memory[Rs1 + imm] = Rs2 |
| 0x04 | BRANCH | Rs1, Rs2, imm | If Rs1==Rs2, PC += imm*4 |
| 0x05 | HALT | - | Stop execution |
| 0x06 | ADDI | Rd, Rs1, imm | Rd = Rs1 + imm |
| 0x07 | SUB | Rd, Rs1, Rs2 | Rd = Rs1 - Rs2 |

## Dependencies

- `cpu-pipeline` (D04): Pipeline management and token flow
- `branch-predictor` (D02): Static and dynamic branch predictors
- `hazard-detection` (D03): Data, control, and structural hazard detection
- `cache` (D01): Configurable cache hierarchy
- `clock`: Cycle-accurate system clock
- `cpu-simulator`: Memory model

## Testing

```bash
go test ./... -v -cover
```

Coverage target: >80% (currently 91%+).
