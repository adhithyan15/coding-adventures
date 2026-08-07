# wasm-simulator (C++)

A stack-based WebAssembly virtual machine in pure ISO C++17, header-only, in
namespace `ca::wasm`. A faithful port of the Rust `wasm-simulator` crate.

WASM is a **stack machine**: operands live on an implicit operand stack.
Supported opcodes (an i32 subset): `end`, `local.get`, `local.set`, `i32.const`,
`i32.add`, `i32.sub`. Arithmetic wraps modulo 2³².

## API

```cpp
#include "wasm_simulator.hpp"
namespace w = ca::wasm;

auto program = w::assemble_wasm({
    w::encode_i32_const(10), w::encode_i32_const(20),
    w::encode_i32_add(), w::encode_end(),
});
w::WasmSimulator sim(/*num_locals=*/0);
std::vector<w::WasmStepTrace> traces = sim.run(program, 1000);
// sim.stack[0] == 30
```

Each executed instruction yields a `WasmStepTrace` (stack before/after, a locals
snapshot, and a description). Where the Rust panics, this port throws
`std::runtime_error` / `std::out_of_range`. `WasmInstruction::operand` is a
`std::optional`.

## Portability

Pure ISO C++17 — standard library only. Compiles clean under GCC, Clang, and MSVC
with `-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../../c/iso-harness).

## Development

```bash
sh BUILD
```
