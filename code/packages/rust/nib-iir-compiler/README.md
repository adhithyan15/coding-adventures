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
bitwise (`&` `|` `^`, LANG-FULL N3), comparisons, `if`/`else`, `while`, `for`
(exclusive `lo .. hi` range, LANG-FULL N2), and cross-function calls. `*`/`/`
lower to the shared IIR `mul`/`div` (N1); `&`/`|`/`^` to `and`/`or`/`xor` (N3).
All of these run on every backend. Wrap/sat arithmetic, unary `~` (needs the
integer-wrap enabler E2), `&&`/`||`, `const`/`static`, and BCD are deferred — see
CHANGELOG and `code/specs/LANG-FULL-IMPLEMENTATION.md`.
