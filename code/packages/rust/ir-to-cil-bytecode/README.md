# ir-to-cil-bytecode

Translates a target-independent `IrProgram` into CLR CIL method bytecode.

The direct builder also provides `encode_ldc_i8` and `emit_ldc_i8` for typed
64-bit literals. They always emit opcode 0x21 and eight little-endian bytes,
even for small values. This builder capability does not widen legacy IR or
IIR lowering; their existing validation limits remain in effect.

## What is CIL?

Common Intermediate Language (CIL) is the stack-based bytecode format used
by the Common Language Runtime (CLR) — the virtual machine behind .NET,
Mono, and Xamarin. Unlike JVM bytecode which encodes types in opcodes,
CIL infers types from the evaluation stack:

```text
JVM:  iadd   ("i" = int32)
CIL:  add    (type inferred at JIT time from stack)
```

## Pipeline

```text
IrProgram
  → validate_for_clr()          — pre-flight constraint check
  → lower_ir_to_cil_bytecode()  — emit CIL body bytes
  → CILProgramArtifact          — structured multi-method artifact
      ↓ CLR simulator           — run directly
      ↓ (future) packager       — wrap in PE/COFF .exe/.dll
```

## Module structure

| Module | Contents |
|--------|----------|
| `builder` | `CILBytecodeBuilder` — two-pass assembler + encoding helpers |
| `backend` | `lower_ir_to_cil_bytecode`, `validate_for_clr`, artifact types |
| `codegen` | `CILCodeGenerator` — LANG20 `CodeGenerator` adapter |

## Usage

```rust
use compiler_ir::{IrInstruction, IrOp, IrOperand, IrProgram};
use ir_to_cil_bytecode::{lower_ir_to_cil_bytecode, validate_for_clr};

let mut prog = IrProgram::new("_start");
prog.add_instruction(IrInstruction::new(
    IrOp::LoadImm,
    vec![IrOperand::Register(1), IrOperand::Immediate(42)],
    1,
));
prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 2));

let errors = validate_for_clr(&prog);
assert!(errors.is_empty());

let artifact = lower_ir_to_cil_bytecode(&prog, None, None).unwrap();
println!("body bytes: {:?}", artifact.methods[0].body);
```

## Validation rules

| Rule | Constraint |
|------|------------|
| opcode support | Only 25 IR opcodes are supported (no MUL/DIV/OR/XOR yet) |
| imm_range | `LOAD_IMM` / `ADD_IMM` immediates must fit in `i32` |
| syscall_whitelist | SYSCALL numbers ∈ {1 (write), 2 (read), 10 (exit)} |
| static_data | Sum of data declarations ≤ 16 MiB |

## Runtime helpers

The lowered CIL code calls five runtime helper methods that must be provided
by the CLR host environment:

| Helper | Signature | Purpose |
|--------|-----------|---------|
| `MemLoadByte` | `(int32) → int32` | Load one byte from memory |
| `MemStoreByte` | `(int32, int32) → void` | Store one byte to memory |
| `LoadWord` | `(int32) → int32` | Load a 32-bit word |
| `StoreWord` | `(int32, int32) → void` | Store a 32-bit word |
| `Syscall` | `(int32, int32) → int32` | Invoke an OS syscall |

## How it fits in the stack

```
Frontend (Brainfuck, Nib, Oct, ...)
    ↓ IrProgram
[ir-to-cil-bytecode]  ← this crate
    ↓ CILProgramArtifact
[clr-simulator] → run on the .NET CLR simulator
```

### Explicit integer conversions (CLR03)

Add ConvI4/ConvI8 and emit_conv_i4/emit_conv_i8 builder APIs. Fixed-byte tests
verify opcodes 0x69/0x6a and branch offsets. No automatic conversions are inserted
into existing IR or IIR lowering.
