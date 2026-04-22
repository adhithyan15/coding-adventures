# ir-to-riscv-compiler

Go backend for the AOT compiler pipeline. It lowers `compiler-ir` programs to
RV32I machine-code bytes that can run on the existing `riscv-simulator`.

## Supported IR

The first slice supports the current v1 IR used by the Brainfuck frontend:

| Category | IR opcodes |
|----------|------------|
| Constants | `LOAD_IMM`, `LOAD_ADDR` |
| Memory | `LOAD_BYTE`, `STORE_BYTE`, `LOAD_WORD`, `STORE_WORD` |
| Arithmetic | `ADD`, `ADD_IMM`, `SUB`, `AND`, `AND_IMM` |
| Comparison | `CMP_EQ`, `CMP_NE`, `CMP_LT`, `CMP_GT` |
| Control flow | `LABEL`, `JUMP`, `BRANCH_Z`, `BRANCH_NZ`, `CALL`, `RET` |
| System/meta | `SYSCALL`, `HALT`, `NOP`, `COMMENT` |

## Usage

```go
backend := irtoriscvcompiler.NewIrToRiscVCompiler()
result, err := backend.Compile(program)
if err != nil {
    panic(err)
}

sim := riscvsimulator.NewRiscVSimulator(65536)
sim.Run(result.Bytes)
```

`MachineCodeResult.Bytes` contains the assembled text section followed by any
declared IR data bytes. `DataOffsets` and `LabelOffsets` are absolute byte
offsets from the beginning of the loaded program.
