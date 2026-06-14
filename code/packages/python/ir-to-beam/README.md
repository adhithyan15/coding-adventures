# ir-to-beam

Lowers `compiler-ir` `IrProgram` instances into `BEAMModule`
records that `beam-bytecode-encoder` can serialize into real
`.beam` bytes.  This is BEAM01 Phase 3 of the
[BEAM01 spec](../../../specs/BEAM01-twig-on-real-erl.md).

## Where this fits

```
twig source
  ↓ twig package
typed AST
  ↓ twig.compile_program
IrProgram (interpreter-ir)
  ↓ ir-optimizer
IrProgram (optimised, compiler-ir)
  ↓ ir-to-beam               ← THIS PACKAGE
BEAMModule
  ↓ beam-bytecode-encoder
.beam bytes
  ↓ erl -noshell -s ...
program output
```

## v1 surface

The first iteration supports the IR ops needed to compile a
non-recursive numeric Twig program with one or more top-level
functions:

| `compiler-ir` op | BEAM lowering                                  |
|------------------|------------------------------------------------|
| `LABEL name`     | `label N` opcode (1-based, gensym'd integer)   |
| `LOAD_IMM v, n`  | `move {integer, n}, {x, v}`                    |
| `ADD v3, v1, v2` | `gc_bif2 fail, live, +/2, {x,v1}, {x,v2}, {x,v3}` |
| `SUB`/`MUL`/`DIV`| same `gc_bif2` pattern with `-`/`*`/`div`      |
| `CALL label`     | `call arity, label`                            |
| `RET`            | `return`                                       |

Plus the loader-required boilerplate per function:
- `func_info {atom, module}, {atom, name}, arity`
- a label after `func_info` to be the call target
- a closing `int_code_end` at the very end of the code stream

The lowering generates a synthesised `module_info/0` and
`module_info/1` that delegate to `erlang:get_module_info/1` and
`/2` — that's what `erlc` does, and the BEAM loader rejects
modules that don't have those exports.

## Out of scope (for v1)

- `BRANCH_Z`/`BRANCH_NZ`/`JUMP` — branch lowering needs live
  register tracking (the BEAM verifier checks operand liveness).
- `SYSCALL` (output) — `erlang:put_chars/1` works fine, but
  encoding the byte-as-binary requires `binary:list_to_binary/1`
  which means an extra import.  v2.
- `LOAD_BYTE`/`STORE_BYTE` etc. (memory ops) — Twig doesn't
  expose bytes-as-memory; the closure-and-cons heap lives on the
  Python side.

## Quick start

```python
from compiler_ir import IrInstruction, IrLabel, IrOp, IrProgram, IrRegister
from ir_to_beam import BEAMBackendConfig, lower_ir_to_beam
from beam_bytecode_encoder import encode_beam

prog = IrProgram(entry_label="main")
prog.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("main")], id=-1))
prog.add_instruction(IrInstruction(IrOp.LOAD_IMM, [IrRegister(0), 42]))
prog.add_instruction(IrInstruction(IrOp.RET))

mod = lower_ir_to_beam(
    prog,
    BEAMBackendConfig(module_name="answer"),
)
beam_bytes = encode_beam(mod)
# beam_bytes is now suitable for `erl -noshell -pa <dir> -s answer main`
```
