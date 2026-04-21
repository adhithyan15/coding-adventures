# tetrad-compiler

Stage 4 of the [Tetrad](../../specs/TET00-tetrad-language.md) pipeline. Walks the typed AST from `tetrad-type-checker` and emits a register-based `CodeObject` consumed by the VM and JIT.

## Pipeline position

```
source → [lexer] → [parser] → [type-checker] → [compiler] → [vm] → [jit]
                                                     ↑ you are here
```

## Two-path compilation

The compiler consults the type map for every binary operation and function call:

```
Typed (both operands u8):
  ADD r0          ← 2 bytes, no feedback slot
  feedback_slot_count unchanged

Untyped (at least one Unknown):
  ADD r0, slot=3  ← 3 bytes, slot allocated
  feedback_slot_count++
```

A `FULLY_TYPED` function emits zero feedback slots — the VM skips vector allocation entirely and the JIT compiles it before the first call.

## Public API

```python
from tetrad_compiler import compile_program, compile_checked, CompilerError
from tetrad_compiler.bytecode import CodeObject, Instruction, Op

# Full pipeline: lex + parse + type-check + compile
code = compile_program("fn add(a: u8, b: u8) -> u8 { return a + b; }")
print(code.functions[0].immediate_jit_eligible)  # True
print(code.functions[0].feedback_slot_count)     # 0

# Compile from pre-built TypeCheckResult (avoids re-running the checker)
from tetrad_type_checker import check_source
result = check_source("fn add(a: u8, b: u8) -> u8 { return a + b; }")
code = compile_checked(result)
```

## Spec

See [`code/specs/TET03-tetrad-bytecode.md`](../../specs/TET03-tetrad-bytecode.md).
