# twig-vm

**LANG20 PRs 3 + 4** — runtime wiring + dispatcher between the Twig frontend and the LANG-runtime substrate.

This crate is the bridge between:
- the **Twig frontend** (`twig-lexer` → `twig-parser` → `twig-ir-compiler`) that produces an `IIRModule` from Twig source, and
- the **Lispy runtime** (`lispy-runtime`, LANG20 PR 2) that provides the value representation, builtins, and `LangBinding` impl.

It exposes [`TwigVM`](src/lib.rs) — a facade that compiles Twig source and dispatches the resulting IIR end to end.

## What this crate ships

| File | What |
|------|------|
| [`lib.rs`](src/lib.rs) | `TwigVM` facade: `compile()` + `compile_with_name()` + `resolve_builtin()` + `run()` |
| [`dispatch.rs`](src/dispatch.rs) | Tree-walking dispatcher (PR 4): `const`, `call_builtin`, `call`, `jmp`, `jmp_if_false`, `label`, `ret` |
| [`operand.rs`](src/operand.rs) | `operand_to_value` — IIR `Operand` → `LispyValue`, the per-language seam |

Plus build-time hardening (PR 3, retained):

- **Build-time grammar compilation** — `twig-lexer` and `twig-parser` compile their `.tokens` and `.grammar` files at build time via `grammar_tools::compiler` + `build.rs` scripts. The generated Rust source reconstructs the parsed grammar as a `OnceLock<TokenGrammar/ParserGrammar>` static. Zero runtime file I/O. Fully self-contained binaries. Miri-compatible without `-Zmiri-disable-isolation`.

## End-to-end demo

```rust
use twig_vm::TwigVM;

let vm = TwigVM::new();

// Arithmetic
let v = vm.run("(+ 2 3)").unwrap();
assert_eq!(v.as_int(), Some(5));

// Recursion
let v = vm.run("
    (define (fact n)
      (if (= n 0) 1 (* n (fact (- n 1)))))
    (fact 5)
").unwrap();
assert_eq!(v.as_int(), Some(120));

// Mutual recursion
let v = vm.run("
    (define (is_even n) (if (= n 0) #t (is_odd (- n 1))))
    (define (is_odd n) (if (= n 0) #f (is_even (- n 1))))
    (is_even 10)
").unwrap();
assert!(v == twig_vm::LispyValue::TRUE);
```

## Supported subset (PR 4)

The dispatcher covers the IIR opcodes emitted by `twig-ir-compiler` for programs without closures, top-level value defines, or quoted symbols:

| Twig form                                | Status   |
|------------------------------------------|----------|
| Arithmetic (`+`, `-`, `*`, `/`)          | ✅       |
| Comparison (`=`, `<`, `>`)               | ✅       |
| Cons family (`cons`, `car`, `cdr`)       | ✅       |
| Predicates (`null?`, `pair?`, `number?`) | ✅       |
| `if`, `let`, `begin`                     | ✅       |
| `(define (f args...) body)`              | ✅       |
| Recursion + mutual recursion             | ✅       |
| Bool / nil truthiness (Scheme semantics) | ✅       |
| `lambda` / closures                      | PR 5+    |
| `(define x value)` (top-level value)     | PR 5+    |
| `'foo` (quoted symbols)                  | PR 5+    |
| `send` / `load_property` / `store_property` | PR 6+    |
| JIT promotion + IC machinery             | PR 7+    |

Programs using unsupported features compile (the IR compiler emits valid IIR for them) but the dispatcher returns `RunError::UnsupportedOpcode` — explicit "not yet" rather than a silent miscompile.

## Pipeline

```text
Twig source
    │
    ▼  twig_lexer → twig_parser → twig_ir_compiler
IIRModule
    │
    ▼  twig_vm::dispatch (PR 4)                       ← THIS CRATE'S DISPATCHER
LispyValue results
    │
    └── via LispyBinding (operand_to_value, builtin   ← INTEGRATION SEAM
        resolution, frame lookups)
```

## Resource limits

- `MAX_DISPATCH_DEPTH = 256` — caps recursion depth so adversarial input can't blow the host Rust stack.
- `MAX_INSTRUCTIONS_PER_RUN = 2²⁰` — caps total instructions per top-level run as a backstop against infinite loops in hand-built malformed IIR.

Both are public constants (`twig_vm::MAX_DISPATCH_DEPTH`, `twig_vm::MAX_INSTRUCTIONS_PER_RUN`) so callers can verify the bounds; both have unit tests so a future change can't silently raise them.

## Tests

```bash
cargo test -p twig-vm
```

**60 unit + 2 doc tests** covering:
- Compilation success + error propagation
- Builtin resolution for every Lispy builtin
- `Operand → LispyValue` round-trip (Int, Bool, Float errors, Var, nil)
- Range checking (Lispy's tagged-int range is narrower than `i64`)
- Full dispatcher: arithmetic, comparison, cons family, `if`, `let`, `begin`, user-defined functions, factorial, fibonacci, mutual recursion
- Error paths: unsupported opcode, missing/invalid operands, unknown function/label/builtin, depth/instruction limits

```bash
MIRIFLAGS="-Zmiri-ignore-leaks" cargo +nightly miri test -p twig-vm
```

The full suite passes under Miri — the dispatcher's integration with `lispy-runtime`'s tagged-pointer code is exercised on every `call_builtin` (cons, car, cdr) and every recursive `call`.  CI runs this on every PR via `.github/workflows/lang-runtime-safety.yml`.

## Where this crate sits

```
LANG20 PR 1: lang-runtime-core           ← LangBinding trait
LANG20 PR 2: lispy-runtime               ← LispyBinding (concrete impl)
LANG20 PR 3: twig-vm  ← THIS CRATE        wires twig-frontend + lispy-runtime
LANG20 PR 4: twig-vm  ← THIS CRATE        adds the dispatcher
LANG20 PR 5: closures + globals + symbols    ← future
LANG20 PR 6: send/load/store + IC machinery  ← future
LANG20 PR 7: JIT promotion + deopt           ← future
```
