# BF06 — Brainfuck JIT for I/O programs (WASI fd_write / fd_read)

## Overview

BF05 wired `BrainfuckVM(jit=True)` so that Brainfuck programs without
I/O JIT to WebAssembly via the in-house WASM stack.  Programs that use
`.` or `,` (i.e. that emit `call_builtin "putchar"` / `"getchar"` in
their IIR) silently deopted to the interpreter because
`cir-to-compiler-ir` didn't know how to lower `call_builtin` and
`WASMBackend.compile()` returned `None`.

This spec closes that gap.  After BF06, the canonical Hello World
Brainfuck program JITs end-to-end:

```python
vm = BrainfuckVM(jit=True)
vm.run(HELLO_WORLD)          # prints "Hello World!\n" via WASI fd_write
assert vm.is_jit_compiled    # True
```

## How it works (and how surprisingly little new code is needed)

When BF05 landed I expected BF06 to require new IIR ops and substantial
plumbing.  In fact, **`ir-to-wasm-compiler` already supports WASI**.  The
existing `IrOp.SYSCALL` opcode emits the right WASI sequence via
`_emit_wasi_write` / `_emit_wasi_read` (see
`ir-to-wasm-compiler/compiler.py:913-973`):

- `SYSCALL imm=1, reg`  →  WASI `fd_write(1, iovec, 1, nwritten)` with
  the byte from `reg` written to scratch memory.
- `SYSCALL imm=2, reg`  →  WASI `fd_read(0, iovec, 1, nread)` with the
  byte placed back into `reg`.

`brainfuck-ir-compiler` already uses these for the static-AOT path.  And
`wasm-runtime` already has a `WasiHost` with `fd_write` / `fd_read`
implementations that route to `stdout` / `stdin` callbacks.

So BF06 is just three small pieces of glue:

1. **`cir-to-compiler-ir` lowering** for `call_builtin "putchar"` and
   `"getchar"` to the existing `IrOp.SYSCALL` pattern.
2. **`wasm-backend` accepts a `WasiHost`** (or the bits to construct
   one) so the WASM runtime can route stdout/stdin to caller-supplied
   buffers.
3. **`BrainfuckVM(jit=True)`** constructs a per-run `WasiHost` with the
   same `output` bytearray and `input_bytes` it already manages for the
   interpreter path.

## The `cir-to-compiler-ir` lowering

```
CIR:  call_builtin dest=None  srcs=["putchar", "v"]   type="void"
IR :  SYSCALL imm=1, val_reg                  ; val_reg = self._var("v")

CIR:  call_builtin dest="v"   srcs=["getchar"]        type="u8"
IR :  SYSCALL imm=2, dest_reg                 ; dest_reg = self._var("v")
```

Other `call_builtin` names are still rejected with `CIRLoweringError`
(deopt-friendly) until specific lowerings are added — this keeps the
surface tight and predictable.

The lowering goes in `cir_to_compiler_ir/lowering.py` next to the
existing `load_mem` / `store_mem` cases (BF05).  It checks
`instr.srcs[0]` (a literal string — the builtin name) to dispatch.

## `wasm-backend` host wiring

`WASMBackend.run()` currently constructs a default `WasmRuntime()`.
BF06 lets the caller supply a `WasiHost`:

```python
class WASMBackend:
    def __init__(self, *, entry_label: str = "_start",
                 host: "WasiHost | None" = None) -> None:
        self.entry_label = entry_label
        self._host = host

    def run(self, binary, args):
        ...
        results = WasmRuntime(host=self._host).load_and_run(...)
```

When `host=None`, behaviour is unchanged — Tetrad's existing tests
keep passing.

## `BrainfuckVM` integration

The wrapper already maintains per-run `output: bytearray` and
`input_buffer: list[int]`.  BF06 routes those through WASI:

```python
output_chunks: list[str] = []
def stdout_cb(text: str) -> None:
    # WASI emits Latin-1 strings — round-trip back to bytes after.
    output_chunks.append(text)

def stdin_cb(n: int) -> bytes:
    chunk = bytes(input_buffer[:n])
    del input_buffer[:n]
    return chunk

host = WasiHost(stdout=stdout_cb, stdin=stdin_cb)
backend = WASMBackend(host=host)
jit = JITCore(vm, backend, threshold_fully_typed=0)
...
# After run:
output += "".join(output_chunks).encode("latin-1")
```

This keeps the interpreter path's per-run behaviour identical for the
JIT path: same `tape_size`, same `max_steps` (well — `max_steps`
doesn't apply to the JIT'd binary; that's a known V1 trade-off, see
"Out of scope" below).

## Tests

- `cir-to-compiler-ir`: new tests for `call_builtin "putchar"` /
  `"getchar"` lowering.
- `wasm-backend`: run with a `host=` parameter actually routes
  stdout/stdin.
- `brainfuck-iir-compiler`:
  - `BrainfuckVM(jit=True).run("+++.")` JITs and emits `b"\x03"`.
  - Hello World JITs and matches the interpreter byte-for-byte.
  - `,.` echo with `input_bytes=b"X"` JITs and emits `b"X"`.
  - Updated TW04-era assertions (`is_jit_compiled is True` for I/O
    programs, was `False`).

## Out of scope

- **`max_steps` enforcement inside the JIT'd binary.**  The interpreter
  enforces fuel by counting `label` crossings; the WASM binary doesn't
  expose that hook.  Programs that JIT and run forever can only be
  stopped by host-level interruption.  Fix: if `max_steps` is set,
  fall back to the interpreter (deopt) — or BF07 adds fuel injection
  in the WASM compiler.  V1 simply documents the limitation.
- **Other `call_builtin` names** beyond `putchar` / `getchar`.  Those
  remain `CIRLoweringError` for now and so deopt to the interpreter.
- **Multiple-byte writes per fd_write call.**  WASI supports it but
  Brainfuck's IR emits one SYSCALL per `.`, so each call writes a
  single byte.  A peephole pass that batches consecutive `.` operations
  is a future optimisation.
