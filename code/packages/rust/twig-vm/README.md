# twig-vm

**LANG20 PR 3** — runtime wiring between the Twig frontend and the LANG-runtime substrate.

This crate is the bridge between:
- the **Twig frontend** (`twig-lexer` → `twig-parser` → `twig-ir-compiler`) that produces an `IIRModule` from Twig source, and
- the **Lispy runtime** (`lispy-runtime`, PR 2) that provides the value representation, builtins, and `LangBinding` impl.

It exposes [`TwigVM`](src/lib.rs) — a thin facade that holds the runtime state needed to compile and (eventually) execute Twig programs.

## What this PR ships

| File | What |
|------|------|
| [`lib.rs`](src/lib.rs) | `TwigVM` facade with `compile()` + `compile_with_name()` + `resolve_builtin()` |
| [`operand.rs`](src/operand.rs) | `operand_to_value` — converts IIR `Operand` → `LispyValue`, the per-language seam |
| [`evaluate.rs`](src/evaluate.rs) | 1-instruction evaluator (`evaluate_call_builtin`) — proves the substrate composes end-to-end without yet needing vm-core |

Plus, as part of unblocking Miri on this crate (which exercises the Twig pipeline transitively):

- **Build-time grammar compilation** — `twig-lexer` and `twig-parser` now compile their `.tokens` and `.grammar` files at build time via a new `grammar_tools::codegen` module + `build.rs` scripts. The generated Rust source reconstructs the parsed grammar as a `OnceLock<TokenGrammar/ParserGrammar>` static. **Zero runtime file I/O. Fully self-contained binaries. Miri-compatible without `-Zmiri-disable-isolation`.**

## What this PR does NOT ship (PR 4+)

- **Real execution** — `vm-core` (PR 4) wires up the dispatch loop against `LangBinding`. Until then, `TwigVM::run` is intentionally absent; the 1-instruction evaluator covers the integration check.
- **Closures, control flow, locals** — those need the full interpreter (PR 4).
- **`send` / `load_property` / `store_property` IIR opcodes** — Lispy doesn't use those (LANG20 PR 5).

## End-to-end demo: `(+ 2 3)` → `5`

The test suite proves the full PR 1+2+3 stack composes:

```rust
let vm = TwigVM::new();
let module = vm.compile("(+ 2 3)").unwrap();          // twig-ir-compiler

// Find the call_builtin "+" instruction in the compiled main.
let instr = find_call_builtin(&module, "+").unwrap();

// Resolve "+" through LispyBinding (PR 2) and dispatch.
// Frame is built from the const instructions in main.
let frame = build_const_frame(&module);
let result = evaluate_call_builtin(instr, &|n| frame.get(n).copied()).unwrap();
assert_eq!(result.as_int(), Some(5));
```

When PR 4 lands, `TwigVM::run` will replace this manual dance with a proper dispatch loop. The boundary between "what the frontend produces" and "what the binding consumes" doesn't move; only the dispatcher gets fleshed out.

## Pipeline

```text
Twig source
    │
    ▼  twig_lexer → twig_parser → twig_ir_compiler   ← already shipped (PR 1+2 didn't change this)
IIRModule
    │
    ▼  TwigVM::compile()                             ← THIS CRATE'S COMPILE PATH
    │
    ▼  vm-core (PR 4)
execution
    │
    ▼  LispyBinding (operand → LispyValue, builtin   ← THIS CRATE'S RUNTIME WIRING
    │   resolution via resolve_builtin, etc.)
LispyValue results
```

## Tests

```bash
cargo test -p twig-vm
```

35 unit + 2 doc tests covering:
- Compilation success + error propagation
- Builtin resolution for every Lispy builtin
- `Operand → LispyValue` round-trip (Int, Bool, Float (errors), Var, nil)
- Range checking (Lispy's tagged-int range is narrower than `i64`)
- Single-instruction evaluation for `+`, `-`, `*`, `/`, `=`, `<`, `>`, `cons`, `car`, `cdr`, `null?`, `pair?`
- Error paths: wrong opcode, missing builtin name, unknown builtin, operand conversion failure, builtin runtime error
- End-to-end: Twig source → IIR → evaluate

## Where this crate sits

```
LANG20 PR 1: lang-runtime-core        ← LangBinding trait
LANG20 PR 2: lispy-runtime            ← LispyBinding (concrete impl)
LANG20 PR 3: twig-vm  ← THIS CRATE     wires twig-frontend + lispy-runtime
LANG20 PR 4: vm-core wiring           ← future: full dispatch loop
LANG20 PR 5: send/load/store opcodes  ← future: dynamic dispatch
```
