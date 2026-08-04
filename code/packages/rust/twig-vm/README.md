# twig-vm

**LANG20 PRs 3 – 8** — runtime wiring + dispatcher between the Twig frontend and the LANG-runtime substrate.  Runs the full Twig surface language end to end (closures, top-level value defines, quoted symbols, recursion, etc.) plus the method-dispatch opcodes (`send` / `load_property` / `store_property`) with persistent inline-cache slots **plus a V8-Ignition-style profiler** that records per-function call counts and per-instruction type observations — the data feed for the future JIT promotion threshold and AOT-PGO codegen.

This crate is the bridge between:
- the **Twig frontend** (`twig-lexer` → `twig-parser` → `twig-ir-compiler`) that produces an `IIRModule` from Twig source, and
- the **Lispy runtime** (`dynval-runtime`, LANG20 PR 2) that provides the value representation, builtins, and `LangBinding` impl.

It exposes [`TwigVM`](src/lib.rs) — a facade that compiles Twig source and dispatches the resulting IIR end to end.

## What this crate ships

| File | What |
|------|------|
| [`lib.rs`](src/lib.rs) | `TwigVM` facade: `compile()` + `compile_with_name()` + `resolve_builtin()` + `run()` |
| [`dispatch.rs`](src/dispatch.rs) | Tree-walking dispatcher (PR 4) + closures/globals/symbols (PR 5): `const`, `call_builtin`, `call`, `jmp`, `jmp_if_false`, `label`, `ret`; PR 5 adds inline handling of `apply_closure`, `global_set`, `global_get` and string-literal `const` operands |
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

// Closures + higher-order
let v = vm.run("
    (define (make-adder x) (lambda (y) (+ x y)))
    ((make-adder 10) 5)
").unwrap();
assert_eq!(v.as_int(), Some(15));

// Top-level value defines + quoted symbols
let v = vm.run("(define answer 42) answer").unwrap();
assert_eq!(v.as_int(), Some(42));
```

## Supported subset (PRs 4 + 5)

The dispatcher covers the IIR opcodes emitted by `twig-ir-compiler` for the full Twig surface language minus method dispatch:

| Twig form                                   | Status   |
|---------------------------------------------|----------|
| Arithmetic (`+`, `-`, `*`, `/`)             | ✅       |
| Comparison (`=`, `<`, `>`)                  | ✅       |
| Cons family (`cons`, `car`, `cdr`)          | ✅       |
| Predicates (`null?`, `pair?`, `number?`)    | ✅       |
| `if`, `let`, `begin`                        | ✅       |
| `(define (f args...) body)`                 | ✅       |
| Recursion + mutual recursion                | ✅       |
| Bool / nil truthiness (Scheme semantics)    | ✅       |
| `lambda` / closures                         | ✅ (PR 5) |
| `(define x value)` (top-level value)        | ✅ (PR 5) |
| `'foo` (quoted symbols)                     | ✅ (PR 5) |
| Higher-order (passing fns + builtins)       | ✅ (PR 5) |
| `send` / `load_property` / `store_property` | ✅ (PR 6) — wired through `LangBinding`; Lispy returns NoSuchMethod / NoSuchProperty (correct for Lispy); future Ruby/JS bindings get dispatch for free |
| Persistent IC slots (`IIRInstr::ic_slot`)   | ✅ (PR 7) — `ICTable` indexed by `(function_name, slot)`; same IC instance shared across activations |
| vm-core profiler (call counts + slot states) | ✅ (PR 8) — `ProfileTable` records per-function call counts + per-instruction `SlotState` observations |
| JIT promotion + deopt                       | LANG22 PR 11+ |

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

- `MAX_DISPATCH_DEPTH = 256` — caps recursion depth so adversarial input can't blow the host Rust stack.  Closures count toward this limit (`apply_closure` recurses through the same `dispatch` path as direct `call`).
- `MAX_INSTRUCTIONS_PER_RUN = 2²⁰` — caps total instructions per top-level run as a backstop against infinite loops in hand-built malformed IIR.
- `MAX_REGISTERS_PER_FRAME = 2¹⁶` — caps per-frame `HashMap` allocation so a hand-built module with `register_count = usize::MAX` can't abort the process at allocation time.
- `MAX_IC_SLOTS_PER_FUNCTION = 2¹⁶` — caps the per-function IC vector growth so `IIRInstr::ic_slot = Some(u32::MAX - 1)` can't OOM the process.
- `MAX_IC_FUNCTIONS = 2¹⁶` — caps `ICTable`'s distinct-function-name key set so a module with millions of unique function names can't unboundedly grow IC storage.
- `MAX_PROFILED_FUNCTIONS = 2¹⁶` — caps `ProfileTable`'s distinct-function-name key set (call_counts + per-instruction slots) for the same reason.
- `MAX_PROFILED_INSTRUCTION_SLOTS = 2²⁰` — caps `ProfileTable.instruction_slots.len()` directly so a long-lived `ProfileTable` reused across many `run_with_profile` calls (the future per-VM state pattern) can't grow without bound.

All seven are public constants (`twig_vm::MAX_*`) so callers can verify the bounds; all have unit tests so a future change can't silently raise them.

## Tests

```bash
cargo test -p twig-vm
```

**117 unit + 2 doc tests** covering:
- Compilation success + error propagation
- Builtin resolution for every Lispy builtin (plus PR 5: `make_symbol`, `make_closure`, `make_builtin_closure`)
- `Operand → LispyValue` round-trip (Int, Bool, Float errors, Var-as-symbol, nil)
- Range checking (Lispy's tagged-int range is narrower than `i64`)
- Full dispatcher: arithmetic, comparison, cons family, `if`, `let`, `begin`, user-defined functions, factorial, fibonacci, mutual recursion
- **PR 5**: anonymous lambdas, lambda captures (single + multiple values), nested lambdas, curried-add pattern, higher-order via user-fn or builtin closure, top-level value defines (read, function-uses-global, overwrite), quoted symbols, `Globals` struct round-trip
- **PR 6**: `send` / `load_property` / `store_property` opcodes through `LangBinding` — receiver/object + symbol-id selector/key extraction, IC allocation, NoSuchMethod / NoSuchProperty paths for Lispy, arity validation, non-symbol selector type errors
- **PR 7**: persistent IC table mechanics (allocate, get, slot-count, dense-per-function, separate-functions-don't-share, repeated-access-returns-same-instance), dispatcher routing through ic_slot, hot-site invariant (IC persists across two calls to same function), backward compat (ic_slot=None falls through to stack IC)
- **PR 8**: ProfileTable mechanics (note_call increments, note_observation advances Mono→Poly→Mega state machine, MAX_PROFILED_FUNCTIONS cap), dispatcher records call counts (main = 1, fact recursion = 6) + per-instruction observations (int observations on `(+ 1 2)` instructions), control-flow opcodes don't get observations, profile persists across multiple `run_with_profile` calls
- Error paths: unsupported opcode, missing/invalid operands, unknown function/label/builtin, depth/instruction/register limits, undefined globals, apply on non-closure

```bash
MIRIFLAGS="-Zmiri-ignore-leaks" cargo +nightly miri test -p twig-vm
```

The full suite passes under Miri — the dispatcher's integration with `dynval-runtime`'s tagged-pointer code is exercised on every `call_builtin` (cons, car, cdr) and every recursive `call`.  CI runs this on every PR via `.github/workflows/lang-runtime-safety.yml`.

## Where this crate sits

```
LANG20 PR 1: lang-runtime-core           ← LangBinding trait
LANG20 PR 2: dynval-runtime               ← LispyBinding (concrete impl)
LANG20 PR 3: twig-vm  ← THIS CRATE        wires twig-frontend + dynval-runtime
LANG20 PR 4: twig-vm  ← THIS CRATE        adds the dispatcher (call/jmp/ret)
LANG20 PR 5: twig-vm  ← THIS CRATE        adds closures + globals + symbols
LANG20 PR 6: twig-vm  ← THIS CRATE        adds send/load/store opcodes via LangBinding
LANG20 PR 7: twig-vm  ← THIS CRATE        adds persistent IC table indexed by ic_slot
LANG20 PR 8: twig-vm  ← THIS CRATE        adds ProfileTable (call counts + SlotState observations)
LANG22 PR 11+: .ldp serialiser, JIT promotion, AOT-PGO ← reads ProfileTable
```

See [`LANG22-typing-spectrum-aot-jit.md`](../../specs/LANG22-typing-spectrum-aot-jit.md) for the unified AOT/JIT/PGO compilation story that builds on top of this stack.
