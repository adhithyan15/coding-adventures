# wasm-simulator (C)

A stack-based WebAssembly virtual machine in pure ISO C17. A faithful port of the
Rust `wasm-simulator` crate.

Unlike a register machine (RISC-V, ARM), WASM is a **stack machine**: operands
live on an implicit operand stack. `i32.const 10 / i32.const 20 / i32.add`
pushes 10 and 20, then `add` pops both and pushes 30. Bytecode is
variable-length (one opcode byte, optional operand bytes).

Supported opcodes (an i32 subset): `end`, `local.get`, `local.set`, `i32.const`,
`i32.add`, `i32.sub`. Arithmetic wraps modulo 2³².

## API

```c
#include "wasm_simulator.h"

WasmProgram p; wasm_program_init(&p);
wasm_emit_i32_const(&p, 10); wasm_emit_i32_const(&p, 20);
wasm_emit_i32_add(&p); wasm_emit_end(&p);

WasmSimulator *sim = wasm_sim_new(/*num_locals=*/0);
WasmStepTrace *traces; size_t count;
size_t plen; const uint8_t *bytes = wasm_program_bytes(&p, &plen);
wasm_sim_run(sim, bytes, plen, 1000, &traces, &count);   /* count traces */
/* wasm_sim_stack(sim, &n)[0] == 30 */
wasm_traces_free(traces, count); wasm_sim_free(sim); wasm_program_free(&p);
```

Each executed instruction yields a `WasmStepTrace` (stack before/after, a locals
snapshot, and a description). Where the Rust panics (unknown opcode, truncated
code, stack underflow, stepping a halted VM, out-of-range local), this port
returns a `WasmStatus`. Growable buffers guard against `size_t` overflow.

## Portability

Pure ISO C17 — no extensions. Compiles clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../iso-harness).

## Development

```bash
sh BUILD
```
