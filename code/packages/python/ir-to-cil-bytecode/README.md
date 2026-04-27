# IR to CIL Bytecode

`coding-adventures-ir-to-cil-bytecode` lowers `compiler_ir` programs into CIL
method-body artifacts. It intentionally stops below PE/CLI metadata emission:
call targets and runtime helper methods are resolved through an injectable token
provider so a later assembly writer can provide real metadata tokens.

The package is the CLR equivalent of the JVM backend's bytecode-lowering layer,
but split out as a composable stage.

```python
from compiler_ir import IrImmediate, IrInstruction, IrLabel, IrOp, IrProgram, IrRegister
from ir_to_cil_bytecode import CILBackendConfig, lower_ir_to_cil_bytecode

program = IrProgram(entry_label="_start")
program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")]))
program.add_instruction(
    IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(7)])
)
program.add_instruction(IrInstruction(IrOp.RET))

artifact = lower_ir_to_cil_bytecode(program, CILBackendConfig())
entry_method = artifact.entry_method
```

The returned `CILProgramArtifact` contains callable method artifacts, static data
offsets, helper requirements, and the token provider used during lowering.
