# W03 - Dart WASM Stack

## Overview

This spec adds the WebAssembly package family to `code/packages/dart/` so Dart can:

- parse real `.wasm` binaries
- validate the WebAssembly 1.0 core model used by the repo's TypeScript stack
- instantiate modules with memory, tables, globals, and exports
- execute arbitrary WebAssembly 1.0 module functions, including imported calls, memory, globals, tables, and host hooks
- support an end-to-end Rust-to-WASM-to-Dart integration flow

The first concrete milestone was intentionally narrow and valuable:

1. write a `square(n: i32) -> i32` function in Rust
2. compile it to `wasm32-unknown-unknown`
3. load the emitted `.wasm` from Dart
4. call the exported `square` symbol
5. get the wrapped i32 result back

That milestone exercises the full path the repo cares about and establishes the
end-to-end fixture that broader runtime compatibility can build on.

## Survey

The repo already contains a clear `wasm-*` package family across several language roots.
The core runtime-oriented packages are:

- `wasm-assembler`
- `wasm-execution`
- `wasm-leb128`
- `wasm-module-encoder`
- `wasm-module-parser`
- `wasm-opcodes`
- `wasm-runtime`
- `wasm-simulator`
- `wasm-types`
- `wasm-validator`

Adjacent WASM-targeting packages also exist and should stay conceptually separate from
the core runtime stack:

- `brainfuck-wasm-compiler`
- `grammar-wasm-support`
- `ir-to-wasm-assembly`
- `ir-to-wasm-compiler`
- `ir-to-wasm-validator`
- `nib-wasm-compiler`

The Dart port in this phase focuses on the core `wasm-*` family first.

## Dart Package Set

The Dart implementation adds these packages under `code/packages/dart/`:

```text
wasm-leb128
  -> wasm-types
    -> wasm-opcodes
      -> wasm-module-parser
        -> wasm-validator
        -> wasm-execution
          -> wasm-runtime

wasm-module-encoder
  -> wasm-types
  -> wasm-leb128

wasm-assembler
  -> wasm-opcodes
  -> wasm-leb128
  -> wasm-types

wasm-simulator
  -> wasm-runtime
  -> wasm-module-parser
  -> wasm-opcodes
```

## Scope

### Fully operational in this phase

- `wasm-leb128` for signed and unsigned LEB128
- `wasm-types` for module/type data structures
- `wasm-opcodes` for the WebAssembly 1.0 opcode metadata set used by the runtime
- `wasm-module-parser` for core module parsing including custom sections
- `wasm-validator` for structural checks plus instruction validation across the supported WebAssembly 1.0 core instruction set
- `wasm-execution` for executing the supported WebAssembly 1.0 core instruction set
- `wasm-runtime` for parse/validate/instantiate/call
- `wasm-module-encoder` for emitting the core sections used by the package tests and fixtures
- `wasm-assembler` for assembling the supported instruction set covered by the package tests
- `wasm-simulator` as an educational wrapper that exposes an instruction trace

### Instruction Coverage

The Dart port now targets the same practical coverage as the repo's TypeScript stack, including the instructions needed by:

- hand-authored square modules
- `rustc --target wasm32-unknown-unknown` output for a minimal `no_std` square function

Required instruction coverage:

- `end`
- `return`
- `drop`
- `local.get`
- `local.set`
- `local.tee`
- `global.get`
- `global.set`
- `call`
- `i32.const`
- `i64.const`
- `f32.const`
- `f64.const`
- `i32.add`
- `i32.sub`
- `i32.mul`

Beyond the original square milestone, the Dart runtime also covers structured control flow,
table-based indirect calls, linear-memory load/store operations, globals, and a workspace-local
WASI stub for stdout/args/env/random host behavior.

## Rust Square Fixture

The Rust integration fixture uses a tiny `no_std` source file so the emitted `.wasm`
is realistic but still compact:

```rust
#![no_std]

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn square(n: i32) -> i32 {
    n * n
}
```

Compile command:

```text
rustc --target wasm32-unknown-unknown --crate-type cdylib -O -C panic=abort -C debuginfo=0 square.rs -o square.wasm
```

This output still includes memory and global exports like `memory`, `__data_end`, and
`__heap_base`, which is useful because it exercises more of instantiation than a
completely hand-minimized module.

## End-to-End Test

The first integration test should prove both the package layering and the runtime API:

1. compile the Rust fixture to `square.wasm`
2. parse it with `wasm-module-parser`
3. validate it with `wasm-validator`
4. instantiate it with `wasm-runtime`
5. call export `square`
6. assert:
   - `square(0) == 0`
   - `square(5) == 25`
   - `square(-3) == 9`
   - `square(2147483647)` wraps as i32

## Design Notes

- The parser stores custom sections rather than rejecting them. This matters because
  Rust-produced modules often include `name`, `producers`, and `target_features`.
- Validation is intentionally split from execution so the runtime API
  matches the repo's other language ports.
- The execution engine should treat Dart `int` values as arbitrary precision at the
  host language level, but all i32 arithmetic must wrap to 32 bits at WASM boundaries.
- `wasm-simulator` is educational rather than cycle-accurate. In this phase it provides
  a decoded instruction trace layered on top of the runtime, not a separate execution
  engine.

## Future Work

- extend coverage further toward post-MVP WebAssembly proposals after the core port stabilizes
- add broader WASI compatibility fixtures and regression tests
- keep the Dart package APIs aligned with the repo's TypeScript reference implementations
- add a Nib-to-WASM Dart pipeline once the core runtime is stable
