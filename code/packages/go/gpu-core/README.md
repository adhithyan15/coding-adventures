# gpu-core (Go)

A generic, pluggable GPU processing element simulator written in Go. This is a port of the Python `gpu-core` package.

## What is this?

This package simulates a single GPU core — the smallest independently programmable compute unit on a GPU. It's designed to be educational: every instruction's execution is fully traced, making the internals visible.

The core is **vendor-agnostic** through a pluggable instruction set architecture (ISA). The default `GenericISA` provides 16 opcodes covering arithmetic, memory, data movement, and control flow. Custom ISAs (e.g., NVIDIA PTX, AMD GCN) can be plugged in by implementing the `InstructionSet` interface.

## Architecture

```
+------------------------------------------+
|              GPU Core                     |
|                                          |
|  +---------+    +------------------+     |
|  | Program |---->   Fetch          |     |
|  | Memory  |    |   instruction    |     |
|  +---------+    |   at PC          |     |
|                 +--------+---------+     |
|                          |               |
|                 +--------v---------+     |
|  +-----------+  |   ISA.Execute()  |     |
|  | Register  |<-|   (pluggable!)   |---->| Trace
|  | File      |->|                  |     |
|  +-----------+  +--------+---------+     |
|                          |               |
|  +-----------+  +--------v---------+     |
|  |  Local   |<- |  Update PC       |     |
|  |  Memory  |   +------------------+     |
|  +-----------+                           |
+------------------------------------------+
```

## Package Structure

| File | Description |
|------|-------------|
| `protocols.go` | `InstructionSet` and `ProcessingElement` interfaces, `ExecuteResult` |
| `opcodes.go` | `Opcode` enum (16 opcodes via `iota`), `Instruction` struct |
| `helpers.go` | Convenient constructors: `Fadd()`, `Limm()`, `Halt()`, etc. |
| `registers.go` | `FPRegisterFile` — configurable floating-point register storage |
| `memory.go` | `LocalMemory` — byte-addressable scratchpad with FP load/store |
| `generic_isa.go` | `GenericISA` — the default educational ISA implementation |
| `core.go` | `GPUCore` — the main processing element with fetch-execute loop |
| `trace.go` | `GPUCoreTrace` — structured execution trace records |

## Dependencies

- `fp-arithmetic` (Go) — IEEE 754 floating-point arithmetic built from logic gates

## Usage

```go
package main

import (
    gpu "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

func main() {
    // Create a core with default settings (32 regs, FP32, 4KB memory)
    core := gpu.NewGPUCore()

    // Load a dot product program
    core.LoadProgram([]gpu.Instruction{
        gpu.Limm(0, 2.0),          // a = 2.0
        gpu.Limm(1, 3.0),          // b = 3.0
        gpu.Limm(2, 4.0),          // c = 4.0
        gpu.Limm(3, 5.0),          // d = 5.0
        gpu.Fmul(4, 0, 2),         // R4 = a*c = 8
        gpu.Ffma(5, 1, 3, 4),      // R5 = b*d + R4 = 23
        gpu.Halt(),
    })

    // Run and inspect traces
    traces, _ := core.Run(1000)
    for _, t := range traces {
        fmt.Println(t.Format())
    }
}
```

## The 16 Opcodes

| Category | Opcode | Description |
|----------|--------|-------------|
| Arithmetic | `FADD` | Rd = Rs1 + Rs2 |
| | `FSUB` | Rd = Rs1 - Rs2 |
| | `FMUL` | Rd = Rs1 * Rs2 |
| | `FFMA` | Rd = Rs1 * Rs2 + Rs3 |
| | `FNEG` | Rd = -Rs1 |
| | `FABS` | Rd = \|Rs1\| |
| Memory | `LOAD` | Rd = Mem[Rs1 + imm] |
| | `STORE` | Mem[Rs1 + imm] = Rs2 |
| Data Move | `MOV` | Rd = Rs1 |
| | `LIMM` | Rd = immediate |
| Control | `BEQ` | if Rs1 == Rs2: PC += offset |
| | `BLT` | if Rs1 < Rs2: PC += offset |
| | `BNE` | if Rs1 != Rs2: PC += offset |
| | `JMP` | PC = target (absolute) |
| | `NOP` | no operation |
| | `HALT` | stop execution |

## Testing

```bash
cd code/packages/go/gpu-core
go test ./... -v -cover
```

## How It Fits in the Stack

This is Layer 8 in the computing stack:

```
Layer 9: Warp/Thread scheduler (groups cores into warps)
Layer 8: GPU Core (this package) — single processing element
Layer 7: FP Arithmetic — IEEE 754 add/mul/fma from logic gates
Layer 6: Logic Gates — AND, OR, NOT, XOR from transistors
```
