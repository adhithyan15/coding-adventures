# iir-to-llvm

IIR → textual LLVM IR backend.  Emits a `.ll` source string for an LLVM
target triple, without depending on `llvm-sys` or a native LLVM install.

**Status: v0.1.0 — skeleton (LLVM01).**  This release emits a valid empty
module (a `; ModuleID` comment + a `target triple` directive) but does
**not** lower IIR instructions.  Instruction lowering arrives in v0.2.0+
(LLVM02–04).

## Where it fits

| Backend                       | Target                                |
|-------------------------------|---------------------------------------|
| `iir-to-wasm`                 | WebAssembly 1.0 bytecode              |
| `iir-to-jvm-class-file`       | JVM class file                        |
| `iir-to-cil-bytecode`         | CLR CIL bytecode                      |
| `iir-to-beam`                 | Erlang BEAM bytecode                  |
| **`iir-to-llvm` (this crate)**| LLVM textual IR (`.ll`)               |

The first four target *managed* runtimes (each runtime owns register
allocation, GC, etc.).  This crate is the first **AOT-native** IIR backend
that doesn't hand-roll its own machine encoder — instead it hands a `.ll`
string to LLVM (`opt` + `llc`) and lets LLVM produce native code for any
CPU LLVM ships a backend for.

The hand-rolled `aarch64-backend` / `x86_64-backend` crates remain the
right call when we want full encoding control (e.g. for the AOT debugger
story).  This crate is the right call when we want world-class O2
optimization for free.

## Why textual `.ll`, not `llvm-sys`?

- **Zero build-time dep.**  CI doesn't need LLVM installed; emit a string.
- **Debuggability.**  The output is the human-readable form.
- **Forward-compat.**  A `llvm-sys` emitter can be added later as a sibling
  without breaking callers.

The cost is that `.ll` is slower to ingest than bitcode.  For a hobby
codebase compiling small modules, that's irrelevant.

## Quick start

```rust
use interpreter_ir::IIRModule;
use iir_to_llvm::{validate_for_llvm, lower_iir_to_llvm, IIRLlvmConfig};

let module = IIRModule {
    name: "demo".into(),
    functions: vec![],
    entry_point: None,
    language: "demo".into(),
    exports: vec![],
    imports: vec![],
};

assert!(validate_for_llvm(&module).is_empty());

let ll = lower_iir_to_llvm(&module, &IIRLlvmConfig::default())
    .expect("lowering should succeed");
println!("{ll}");
// ; ModuleID = 'iir_module'
// target triple = "x86_64-unknown-linux-gnu"
```

## Configuration

`IIRLlvmConfig` has two knobs:

- `module_name` — emitted in the `; ModuleID = '<name>'` comment.
- `target_triple` — emitted in `target triple = "<triple>"`.

The default triple is a **fixed** string (`"x86_64-unknown-linux-gnu"`)
rather than a host-derived value.  This keeps test output byte-identical
across CI runners.  Override via `.with_target("riscv32-unknown-elf")` when
you actually intend to run `llc` for a non-default architecture.

## Roadmap

| Version | Scope                                                       |
|---------|-------------------------------------------------------------|
| v0.1.0  | Crate skeleton: empty module header. *(this release)*       |
| v0.2.0  | Function signatures + `ret`/`ret_void`/`const`/`mov`.       |
| v0.3.0  | Typed arithmetic (`add`/`sub`/`mul`/...) + cmp + branches.  |
| v0.4.0  | `call` + `call_builtin print_i64` extern declarations.      |
| v0.5.0  | Tagged-word lisp `cons`/`car`/`cdr` → `call @__twig_lispy_*` (McCarthy W12b-1). |
| v0.6.0  | `COND` via stack-slot (`alloca`) SSA-merge + `jmp_if` void-cond + empty-block `br` (McCarthy W12b-3). |
| v0.7.0  | Lisp symbols — `symbol` type → `i64` tagged immediate (McCarthy W13a). |
| v0.8.0  | Lisp lambda (F7) — declare `lispy_to_exit_code` runtime switch; **LLVM McCarthy-complete F1–F7** (McCarthy W13b). |
| v0.9.0  | Byte-tape ops `alloc_bytes`→`@calloc`, `load_byte`/`store_byte` (zext/trunc at the byte boundary), `putchar`/`getchar` libc builtins, + slot-dest SSA rename. **Brainfuck runs on LLVM** (LANG-MATRIX LM-L Brainfuck). |
| v0.10.0 | Reassigned **parameters** are promoted to i64 stack slots (initialised from the incoming argument, narrow args zext'd) — a parameter accumulated across a loop back-edge is no longer silently dropped (LANG-FULL — LLVM first-class). |
| v0.11.0 | **Narrow unsigned arithmetic wraps mod-2ⁿ** (LANG-FULL E2). A `u4`/`u8`/`u16`/`u32` op computes at i64 then `and i64 …, <mask>` (see below). Adds `u4` to the supported types. |
| v0.12.0 | **Bitwise `not`** — synthesised as `xor x, -1` (LLVM has no `not`); a narrow width masks the result (`~0u8 = 255`). Unblocks Nib N3-`~` / Oct O2-`~`. |
| (later) | GC, debug info via `!dbg`. |

### Byte-tape memory (v0.9.0)

Brainfuck builds an implicit byte tape; `lower_brainfuck_for_aot` (in `lang-aot`)
rewrites it into the same `alloc_bytes` / `load_byte` / `store_byte` ops the
native x86_64 backend already uses (LANG76). This crate's lowering:

| IIR op | LLVM emitted | Notes |
|--------|--------------|-------|
| `alloc_bytes d <- n` | `%d = call ptr @calloc(i64 n, i64 1)` | zero-filled tape |
| `load_byte d <- base, i` | `getelementptr i8` + `load i8` + `zext i8…i64` | cell → word |
| `store_byte base, i, v` | `getelementptr i8` + `trunc i64…i8` + `store i8` | word → cell (8-bit wrap) |
| `call_builtin putchar v` | `trunc i64…i32` + `call i32 @putchar(i32)` | libc; Brainfuck `.` |
| `call_builtin getchar -> d` | `call i32 @getchar()` + `sext i32…i64` | libc; Brainfuck `,` |

Byte width lives **only at the tape boundary** (the `zext`/`trunc`); every register
in between is a uniform `i64`, which is what lets the i64-only stack-slot model
consume Brainfuck's reassigned `ptr`/`v` without a width mismatch.

### Narrow-width register arithmetic (v0.11.0, LANG-FULL E2)

The same "uniform i64 in registers" model is exactly why narrow **unsigned**
arithmetic must wrap with a *value mask*, not a narrow-typed op. A `u8` add
whose operands are `i64` SSA values cannot be `add i8 %a, %b` — that is invalid
IR `clang` rejects. So `add`/`sub`/`mul`/`div`/`mod` and `and`/`or`/`xor` on a
`u4`/`u8`/`u16`/`u32` `type_hint` compute at i64 and mask the result back into
the width:

```llvm
  %__nw1 = add i64 200, 100     ; compute wide
  %v     = and i64 %__nw1, 255  ; 300 & 0xFF = 44   (u8 wrap)
```

| type_hint | mask         |  | type_hint | mask         |
|-----------|--------------|--|-----------|--------------|
| `u4`      | `0xF`        |  | `u16`     | `0xFFFF`     |
| `u8`      | `0xFF`       |  | `u32`     | `0xFFFFFFFF` |

`u64`/`i64` (full word), signed narrow widths, and floats get no mask. This
mirrors the VM/JIT/wasm/JVM/CLR backends (each masks the narrow result by
`type_hint`) and generalises the byte-tape's 8-bit `store_byte` wrap to register
arithmetic. Verified by RUNNING the emitted `.ll` through real `clang`:
`200u8 + 100u8` exits `44`.

See [`code/specs/iir-to-llvm.md`](../../../specs/iir-to-llvm.md) for the
full spec and [`code/specs/MULTILANG-BACKEND-PLAN.md`](../../../specs/MULTILANG-BACKEND-PLAN.md)
§LLVM for how this fits the broader plan.

## Tests

```sh
cargo test -p iir-to-llvm
```

7 tests at v0.1.0 covering validator stub, output shape, config defaults,
and error display.
