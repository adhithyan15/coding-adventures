# intel4004-backend

A `jit_core.BackendProtocol` implementation that targets the Intel 4004
(the world's first commercial microprocessor, 1971).

This is a **native code-generation backend** for the LANG pipeline:
`jit-core` (LANG03) and `aot-core` (LANG04) hand it a list of
`CIRInstr` typed-SSA instructions, it returns a 4004 binary, and
`intel4004-simulator` executes it.

## How backends fit in the LANG pipeline

```
   IIRModule  (any frontend's bytecode in InterpreterIR)
       │
       ▼
   jit-core / aot-core  (specialise + optimise to CIRInstr)
       │
       ▼
   ┌───────────────────────────────────────────────────┐
   │  BackendProtocol implementation (this package)    │
   │                                                   │
   │  compile(cir) → bytes  (target machine code)      │
   │  run(binary, args) → result                       │
   └───────────────────────────────────────────────────┘
       │
       ▼
   intel4004-simulator   (or a real device, an emulator, …)
```

Every native target gets its own backend package, named
`<arch>-backend`.  Future siblings will follow the same shape:

- `intel8008-backend`
- `intel8080-backend`
- `mos6502-backend`
- `z80-backend`
- `riscv32-backend`
- `x86_64-backend`
- `arm64-backend`
- `wasm32-backend`
- …

This separation means a Lisp / Scheme / ML / JavaScript runtime picks
its target by importing the right backend package — no monolithic
"backends" package that pulls in every codegen toolchain, no
language-specific machinery in the way.

## Quick start

```python
from interpreter_ir import IIRModule
from jit_core import JITCore
from vm_core import VMCore
from intel4004_backend import Intel4004Backend

vm = VMCore()
jit = JITCore(vm, backend=Intel4004Backend())

module: IIRModule = ...      # produced by your language frontend
result = jit.execute_with_jit(module, fn="main")
```

The frontend's choice of language has no influence here — the same
`Intel4004Backend()` works for Tetrad, Lisp, BASIC, or any other
LANG-pipeline frontend whose CIR emission stays inside the 4004's
operand set.

## What it can compile

The 4004 codegen supports a focused subset of CIR ops:

| Category    | Supported                                            |
|-------------|------------------------------------------------------|
| Arithmetic  | `add`, `sub`, `add` with immediate                    |
| Comparison  | `cmp_eq`, `cmp_ne`, `cmp_lt`, `cmp_le`, `cmp_gt`, `cmp_ge` |
| Memory      | `param`, `store_var`, `load_var` (≤8 vars)             |
| Control     | `jmp`, `jz`, `jnz`, `label`, `ret`                     |

Anything else (`mul`, `div`, `mod`, bitwise ops, I/O, function calls,
type guards, generic runtime calls) returns `None` from `compile()`.
`jit-core` interprets that as "deopt" and runs the function on the
interpreter instead.

## Architecture notes

- **CIR re-projection**: `Intel4004Backend.compile` re-projects
  `CIRInstr` (jit-core's typed-SSA shape) into a small `IRInstr` form
  the codegen consumes directly.  That shape is a holdover from when
  the codegen lived inside `tetrad-jit`; a future PR will rewrite the
  codegen to consume `CIRInstr` directly and remove the re-projection
  helper.
- **Two-pass assembler**: the codegen emits abstract instructions
  (tuples), then a two-pass assembler resolves label addresses and
  encodes to bytes.
- **One ROM page**: 4004 binaries are bounded at 256 bytes.  The
  codegen's variable allocator recycles register pairs to fit
  realistic functions.

## Layer position

```
LANG03 jit-core  ─┐
LANG04 aot-core  ─┴──▶  intel4004-backend  ──▶  intel4004-simulator
                              │                       │
                       BackendProtocol           Intel4004State
                       implementation            (ALU + ROM/RAM)
```
