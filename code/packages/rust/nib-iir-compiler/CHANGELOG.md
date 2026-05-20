# Changelog — `nib-iir-compiler`

## 0.2.0 — 2026-05-20 (NIB04 — print + cross-function calls)

Adds proper lowering for `call_expr` nodes in the Nib AST.  Before
this release the IIR compiler silently treated `foo(args)` as a bare
variable reference to `foo`, dropping every argument and emitting a
broken `ret Var("foo")` — which compiled to "return the function's
own address" at the AOT layer.  Two of NIB04's three V1 steps land
here:

**1. `print(x)` lowers to `call_builtin "print_i64", x`.**  The
runtime helper `__twig_print_i64` already exists from LANG75; no new
runtime work needed.  V1 `print` takes exactly one i64-shaped
argument; zero or two arguments produces a clean `Unsupported` error.

**2. Cross-function calls.**  `f(a, b, c)` lowers to a proper
`call f, a, b, c -> dest` IIR instruction.  The x86_64 + aarch64
backends already implement cross-function relocations (LANG43 PR
#3331); this PR just wires up the frontend.

**3. Zero-argument calls.**  `f()` works too — emits `call f -> dest`
with `srcs.len() == 1` (just the callee).

**Step 3 of NIB04 (while loops) is deferred** to a follow-up PR
because it requires grammar changes (no `while` rule today) plus
regenerating `nib-parser/src/_grammar.rs`.

### Tests added (4)

- `compiles_print_call` — IIR shape check for `print(42)`.
- `compiles_cross_function_call` — `double(21)` from main produces
  `call double, 21 -> dest`.
- `compiles_zero_arg_call` — `forty_two()` produces `call forty_two`
  with no extra srcs.
- `rejects_print_with_wrong_arity` — `print()` with 0 args fails
  with `Unsupported` instead of silently producing garbage IIR.

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
