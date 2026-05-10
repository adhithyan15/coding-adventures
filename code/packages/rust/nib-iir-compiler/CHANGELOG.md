# Changelog — `nib-iir-compiler`

## 0.1.0 — 2026-05-05

Initial release.  Compiles Nib source to `interpreter_ir::IIRModule`,
unlocking the LANG-runtime AOT (and JIT, eventually) pipeline for the
Nib language.

### Coverage

- `fn name(params...) -> ret_ty { body }` → `IIRFunction`
- `let name: ty = expr;` → `const + _move`
- `return expr;` → `ret`
- Integer literals (`5`, `0x1F`)
- Identifier references / parameters
- Binary arithmetic (`+`, `-`) → `call_builtin "+"` etc.
  (lowered to typed CIR by `aot-core::specialise`)
- Comparisons (`==`, `!=`, `<`, `<=`, `>`, `>=`) — same lowering
- `if expr { ... } else { ... }`

### Out of scope (deferred)

- Cross-function calls (V1 aarch64-backend has no relocation support yet)
- Wrap/saturating arithmetic, bitwise ops
- For loops over ranges
- BCD operations

### End-to-end demonstration

Six Nib programs compile through `nib-iir-compiler` →
`twig-aot::compile_module_macos_arm64_object` → `ld` → runnable
ARM64 Mach-O on Apple Silicon.  Each program's exit code matches its
intended return value:

| Source | Exit |
|---|---|
| `fn main() -> u4 { return 9; }` | 9 |
| `fn main() -> u4 { return 3 + 4; }` | 7 |
| `fn main() -> u4 { let x: u4 = 5; return x; }` | 5 |
| `fn main() -> u4 { if 1 == 1 { return 4; } else { return 9; } }` | 4 |
| `fn main() -> u4 { if 1 == 2 { return 4; } else { return 9; } }` | 9 |
| `fn main() -> u4 { if 3 < 5 { return 1; } else { return 0; } }` | 1 |

This validates the path to deprecating the older `compiler-ir::IrProgram`
chain: any language with a frontend → IIR shim now inherits the full
AOT (and forthcoming JIT) infrastructure.
