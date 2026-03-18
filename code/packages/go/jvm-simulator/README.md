# JVM Simulator (Go Port)

**Layer 4-e of the computing stack** — simulates a Typed Stack-based Virtual Machine modeled natively across JVM bytecode behaviors.

## Overview
Unlike standard stack-based operations observed commonly in generic VM or WASM runtimes defining operations that execute against arbitrary contents pushed structurally against a Stack context stack (`ADD`), JVM specifically validates boundaries providing exclusively typed variants mapping tightly to data variants explicitly at Compile. (`IADD` evaluates purely integers, `LADD` purely longs).
Our simulation isolates Integer bounds (Two-s complement wrapping 32-bit width parameters) specifically to implement a minimal working implementation subset demonstrating mathematical properties in isolation natively decoupled from abstract generics.

## Usage
```go
import (
	"github.com/adhithyan15/coding-adventures/code/packages/go/jvm-simulator"
)

sim := jvmsimulator.NewJVMSimulator()

// Typed execution operations defining JVM specifications internally evaluating constants against memory references natively.
program := jvmsimulator.AssembleJvm([]jvmsimulator.Instr{
    {Opcode: jvmsimulator.OpIconst1}, // explicit Int-1 constant push onto Stack
    {Opcode: jvmsimulator.OpIconst2}, // explicit Int-2 constant push onto Stack
    {Opcode: jvmsimulator.OpIadd},    // specifically evaluating bounds-checked int derivations
    {Opcode: jvmsimulator.OpIstore0}, // load result across internal generic storage buffers explicitly tracking values structurally
    {Opcode: jvmsimulator.OpIload0},
    {Opcode: jvmsimulator.OpIreturn}, // Native variable returns decoupling halts.
})

sim.Load(program, nil, 16)
traces := sim.Run(100)
```
