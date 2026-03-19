# Virtual Machine (Go Port)

**Layer 5 of the computing stack** — A dynamic, stack-based bytecode interpreter.

## Overview
Where earlier ISA Simulators map precisely to constrained, 32-bit hardware behaviors (ARM, RISC-V), a virtual machine provides a universal instruction set capable of executing natively inferred logic loops without explicit primitive allocations required under typed layers like JVM constraints. Operations manipulate a central Context array natively utilizing dynamic resolutions mirroring architectures seen within Python (`CPython`) or Ruby (`YARV`).

## Execution Example
```go
import "github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine"

vm := virtualmachine.NewVirtualMachine()

code := virtualmachine.AssembleCode(
    []virtualmachine.Instruction{
        {Opcode: virtualmachine.OpLoadConst, Operand: 0},
        {Opcode: virtualmachine.OpLoadConst, Operand: 1},
        {Opcode: virtualmachine.OpAdd},
        {Opcode: virtualmachine.OpStoreName, Operand: 0},
        {Opcode: virtualmachine.OpHalt},
    },
    []interface{}{10, 20}, // Constants Pool
    []string{"x"},         // Names Pool
)

vm.Execute(code)
// Expected Outcome -> vm.Variables["x"] === 30!
```
