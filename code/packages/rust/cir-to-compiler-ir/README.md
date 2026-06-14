# cir-to-compiler-ir

**LANG21** — Bridge that converts a `Vec<CIRInstr>` (specialised typed CIR from
`jit-core`) into an `IrProgram` (target-independent AOT IR from `compiler-ir`),
completing the Tetrad end-to-end compilation pipeline.

## Where it fits

```
Tetrad source
  → tetrad-lexer / parser / type-checker / compiler  → CodeObject
  → tetrad-runtime.compile_to_iir()                  → IIRModule
  → jit-core.specialise(fn, min_observations=0)      → Vec<CIRInstr>  (forced AOT)
  → CIROptimizer.run()                               → Vec<CIRInstr>
  → lower_cir_to_ir_program()       ← THIS CRATE (LANG21)
  → IrProgram
  → WASMCodeGenerator / JVMCodeGenerator / …          ← LANG20 adapters
  → WasmModule / JVMClassArtifact / …
  → wasm-runtime / GraalVM / GE-225 simulator / CIL simulator
```

## Public API

```rust
use jit_core::{CIRInstr, CIROperand};
use cir_to_compiler_ir::{
    CIRLoweringError,
    validate_cir_for_lowering,
    lower_cir_to_ir_program,
};

let instrs = vec![
    CIRInstr::new("const_i32", Some("x".into()), vec![CIROperand::Int(40)], "i32"),
    CIRInstr::new("const_i32", Some("y".into()), vec![CIROperand::Int(2)],  "i32"),
    CIRInstr::new("add_i32",   Some("z".into()),
                  vec![CIROperand::Var("x".into()), CIROperand::Var("y".into())], "i32"),
    CIRInstr::new("ret_void",  None, vec![], "void"),
];

// Optional pre-flight check
let errors = validate_cir_for_lowering(&instrs);
assert!(errors.is_empty());

// Lower to IrProgram
let prog = lower_cir_to_ir_program(&instrs, "_start").unwrap();
assert_eq!(prog.entry_label, "_start");
```

## Supported CIR ops

| Category | Ops |
|---|---|
| Constants | `const_i8/i16/i32/i64/u8/u16/u32/u64/bool` |
| Arithmetic | `add_*`, `sub_*`, `and_*`, `neg_*` |
| Comparisons | `cmp_eq_*`, `cmp_ne_*`, `cmp_lt_*`, `cmp_gt_*`, `cmp_le_*` (synthesised), `cmp_ge_*` (synthesised) |
| Control flow | `label`, `jmp`, `jmp_if_true/false`, `br_true/false_bool`, `ret_void`, `ret_*`, `call` |
| Meta | `type_assert`, `tetrad.move`, `call_builtin` |
| Memory | `load_mem`, `store_mem` |

## Unsupported ops (v1 limitation)

The v1 `IrOp` set in `compiler-ir` does not include `MUL`, `DIV`, `OR`, `XOR`,
`NOT`, or any floating-point opcodes. Attempting to lower these returns a
`CIRLoweringError`. Adding them is planned for a future LANG.

## Synthesised ops

Because v1 `IrOp` lacks `NOT`:

- **`cmp_le(a, b)`** → `1 − CmpGt(a, b)` (3 instructions)
- **`cmp_ge(a, b)`** → `1 − CmpLt(a, b)` (3 instructions)
- **`neg(x)`** → `LoadImm(0)` + `Sub(0, x)` (2 instructions)

## Dependencies

| Crate | Role |
|---|---|
| `jit-core` | Provides `CIRInstr`, `CIROperand` |
| `compiler-ir` | Provides `IrProgram`, `IrInstruction`, `IrOp`, `IrOperand`, `IdGenerator` |
