# nib-iir-compiler

Compile Nib source to `interpreter_ir::IIRModule` so it can flow through the
LANG-runtime AOT and JIT pipelines.

## Why

Nib historically went through `compiler-ir::IrProgram`, the older more
assembly-flavoured IR shared with brainfuck-wasm and the Intel-4004 toolchain.
The new pipeline (twig-vm, twig-aot, jit-core, aot-core) is built on
`interpreter_ir::IIRModule`, which carries enough type information that
`aot-core::specialise` can lower primitive operators to typed CIR ops the
native backend handles directly.

By compiling Nib straight to IIR, every Nib program inherits:
- Native ARM64 Mach-O via `twig-aot` + `ld`
- (Future) in-process JIT via `jit-core` + `aarch64-backend` + a JIT loader

## Quick example

```rust
use nib_iir_compiler::compile_source;
use twig_aot::compile_module_macos_arm64_object;

let m = compile_source("fn main() -> u4 { return 3 + 4; }", "demo")?;
let obj = compile_module_macos_arm64_object(&m)?;
// Feed `obj` to ld → executable Mach-O → exits 7.
```

## Status

Covers literals, let/return, identifiers, binary arithmetic (`+` `-` `*` `/`),
bitwise (`&` `|` `^` `~`, N3), short-circuit logical `&&`/`||` (N4), logical
not (`!`, N9), comparisons, `if`/`else`, `while`, `for` (exclusive `lo .. hi` range, N2), module-scoped
`const`s (N5), module-scoped mutable `static`s (N8), const/static expression
folding (N10), and cross-function calls. `*`/`/` lower to
`mul`/`div` (N1); `&`/`|`/`^` to `and`/`or`/`xor` and unary `~` to the `not` op
(N3, narrow-masked so `~0u8 = 255`, `~15u4 = 0`); `&&`/`||` short-circuit
via `jmp_if_false` branches (N4); a `const` reference folds to its literal (N5).
`static` initializers seed shared IIR module globals at `main` entry, reads use
`global_load`, and assignments use `global_store` (N8).
Logical `!` lowers through the same truthiness branch contract that conditions
already consume, producing a 0/1 scalar result (N9).
Top-level const/static initializers fold deterministic integer/boolean
expressions, including references to previously declared consts, before runtime
code is emitted (N10).
Narrow `u4`/`u8` arithmetic wraps mod-2ⁿ (N6, via the E2 backend masks), and the
explicit-overflow operators **`+%` (wrapping)** and **`+?` (saturating)** are
supported (N7): `+%` is the narrow-typed `add` (`15u4 +% 1 = 0`), `+?` is a wide
add + a `min(sum, MAX)` clamp branch (`15u4 +? 1 = 15`, `200u8 +? 100 = 255`).
All run on every backend. BCD storage semantics and 4004 RAM mapping for statics
are deferred — see CHANGELOG and `code/specs/LANG-FULL-IMPLEMENTATION.md`.
