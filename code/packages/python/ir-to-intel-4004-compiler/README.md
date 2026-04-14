# coding-adventures-ir-to-intel-4004-compiler

Compiles a generic `IrProgram` into Intel 4004 assembly text.

The Intel 4004-specific IR feasibility checks live in the sibling
`coding-adventures-intel-4004-ir-validator` package. This package is the code
generation facade that validates first and then emits assembly.

## Installation

```bash
pip install coding-adventures-ir-to-intel-4004-compiler
```

## Usage

```python
from compiler_ir import IrImmediate, IrInstruction, IrLabel, IrOp, IrProgram, IrRegister
from ir_to_intel_4004_compiler import IrToIntel4004Compiler

program = IrProgram(entry_label="_start")
program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
program.add_instruction(
    IrInstruction(IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(5)], id=0)
)
program.add_instruction(IrInstruction(IrOp.HALT, [], id=1))

compiler = IrToIntel4004Compiler()
asm = compiler.compile(program)
print(asm)
```
