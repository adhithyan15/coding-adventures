# brainfuck-iir-compiler

**BF04** — Brainfuck → InterpreterIR compiler and `vm-core` wrapper.
**BF05** — JIT-chain dispatch (`vm-core` + `jit-core`) wired through
`BrainfuckVM::new(true, ...)`.

## What it does

This crate bridges the Brainfuck language and the LANG generic interpreter pipeline.
A Brainfuck program is compiled to a single-function
[`IIRModule`](../interpreter-ir) and executed by the generic
[`vm_core::VMCore`](../vm-core).

### Pipeline

```
Brainfuck source
       │
       ▼  brainfuck::parse_brainfuck()
GrammarASTNode
       │
       ▼  compile_to_iir() / compile_source()    ← this crate
IIRModule (one fn: "main", FULLY_TYPED)
       │
       ▼  BrainfuckVM::run() → vm-core
Vec<u8>  (stdout bytes)
```

## Usage

### One-shot execution

```rust
use brainfuck_iir_compiler::BrainfuckVM;

let vm = BrainfuckVM::new(false, 30_000, None).unwrap();

// "Hello, World!" in Brainfuck
let hello = "++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.";
let out = vm.run(hello, b"").unwrap();
println!("{}", String::from_utf8_lossy(&out));
```

### Compile-then-inspect

```rust
use brainfuck_iir_compiler::compile_source;

let module = compile_source("+++.", "demo").unwrap();
let fn_ = &module.functions[0];
println!("Instructions: {}", fn_.instructions.len());
// fn_.type_status == FunctionTypeStatus::FullyTyped
```

### Execute pre-compiled module with different inputs

```rust
use brainfuck_iir_compiler::BrainfuckVM;

let vm = BrainfuckVM::new(false, 30_000, None).unwrap();
let module = vm.compile(",.,.,." ).unwrap();

let out1 = vm.execute_module(&module, b"\x41\x42\x43").unwrap(); // ABC
let out2 = vm.execute_module(&module, b"\x61\x62\x63").unwrap(); // abc
```

## Command → IIR mapping

| BF command | IIR instructions emitted |
|---|---|
| `>` | `const k 1 u32` + `add ptr ptr k u32` |
| `<` | `const k 1 u32` + `sub ptr ptr k u32` |
| `+` | `load_mem v ptr u8` + `const k 1 u8` + `add v v k u8` + `store_mem ptr v u8` |
| `-` | `load_mem v ptr u8` + `const k 1 u8` + `sub v v k u8` + `store_mem ptr v u8` |
| `.` | `load_mem v ptr u8` + `call_builtin putchar v void` |
| `,` | `call_builtin getchar () i64` → test `v & ~0xFF` (EOF?) → store `0` (EOF) or the byte (`u8`) |
| `[`…`]` | structured loop (label + guard + body + back-edge) |

### `,` and end-of-input (B1-eof)

`getchar` returns a byte (0–255) or `-1` at end-of-input — but the runtimes widen that
`-1` differently (LLVM sign-extends to `0xFF…FF`; the native path zero-extends the `i32` to
`0x0000_0000_FFFF_FFFF`), so a signed `< 0` test is not portable. `,` instead reads at `i64`
and tests the bits **above** the low byte with the mask `~0xFF` (`-256`): those bits are `0`
for any valid byte and non-zero for `-1` under either extension. On EOF it stores `0`,
otherwise the byte — each branch doing its own `store_mem` so no register crosses the merge
(Brainfuck keeps all state in the tape; the IIR has no phi nodes). This is the "EOF leaves 0"
convention the canonical cat `,[.,]` relies on, so cat now halts on **every** backend
(`"Hi"` → `"Hi"`). A program that never reads past its input never reaches the EOF branch.

## Where it fits in the stack

```
LANG01  interpreter-ir     ← IIRModule format
LANG02  vm-core            ← executes IIRModule
LANG03  jit-core           ← JIT (hot functions → native bytes)
BF00    brainfuck-lexer    ← tokens
BF01    brainfuck-parser   ← GrammarASTNode
BF04    brainfuck-iir-compiler ← THIS CRATE (compiler + interp wrapper)
BF05    brainfuck-iir-compiler ← THIS CRATE (JIT-chain wiring + smoke)
```

## JIT (BF05)

`BrainfuckVM::new(jit: true, ...)` routes runs through
[`jit_core::core::JITCore::execute_with_jit`], the LANG VM's tiered JIT
engine.  Brainfuck's IIR is `FullyTyped` from birth, so the JIT chain
inherits the same eager-compile path that powers Dartmouth BASIC, Oct,
and Nib.

### The backend

A private `BrainfuckCirJit` in `src/jit_backend.rs` implements
`jit_core::backend::Backend`.  Its `compile()` translates BF's CIR
(post-specialise, post-`CIROptimizer`) into a **packed register-machine
bytecode** — 1-byte opcode tags, 1-byte register indices, `i16` LE
branch offsets, natural-width literals.  Its `run()` interprets that
bytecode in a tight `match`-loop, owning a fresh tape per call.  This
bypasses `vm-core`'s generic IIR dispatch entirely.

This is a JIT in the classic, historical sense — the same shape used
by the JVM (Ignition tier), Smalltalk-80, V8 Ignition, Lua, and many
other production JITs as their first tier.  It is **not** a
native-code JIT (no Cranelift, no hand-rolled machine code) — that's
separate work.  Swapping in a native backend that supports BF's CIR
is a one-line change in `src/vm.rs`.

### How to confirm the JIT is doing real work

```rust
use brainfuck_iir_compiler::BrainfuckVM;

let vm = BrainfuckVM::new(true, 30_000, None).unwrap();

// Run end-to-end through the JIT chain:
let out = vm.run("+++.", b"").unwrap();
assert_eq!(out, vec![3u8]);

// Confirm the JIT actually emitted bytecode (not silent fallback):
let bytecode_len = vm.jit_bytecode_len("+++.").unwrap();
assert!(bytecode_len.unwrap() >= 15);   // ~30-40 bytes for `+++.`
```

`tests/jit_smoke.rs` runs every program twice — once with `jit=false`
(pure interpreter), once with `jit=true` (CIR-bytecode JIT) — and
asserts byte-identical output.  If they ever diverge, the JIT is wrong.

## Cross-backend compilation status

Brainfuck flows through every universal IIR-to-* backend except BEAM:

| Target  | Status | Lowering                                                                  | Test |
|---------|--------|---------------------------------------------------------------------------|------|
| WASM    | ✅     | `i32.load8_u` / `i32.store8` + `env.putchar` / `env.getchar` host imports | `tests/wasm_e2e.rs` |
| JVM     | ✅     | `baload` / `bastore` + `invokestatic env/BFRuntime.{put,get}char`         | `tests/jvm_e2e.rs`  |
| CLR     | ✅     | `ldelem.u1` / `stelem.i1` + `call env.BFRuntime::{put,get}char`           | `tests/clr_e2e.rs`  |
| BEAM    | ❌     | *intentionally not supported — see below*                                 | —    |

### Why no BEAM target?

BEAM's substrate is **purely functional**.  Tuples, binaries, and process
dictionary entries are all immutable — every write produces a fresh copy
of the surrounding container.  Brainfuck's tape is the opposite shape:
mutable byte cells in random-access addressing, with one write per cell
per iteration of the program's main loop.

Compiled to vanilla BEAM bytecode, every `store_mem` would have to
allocate a fresh copy of the entire 30,000-byte tape with the one byte
changed.  That's O(N) per cell write × N cells × M loop iterations =
O(N²·M).  A `,[.,]` `cat` over a 30 KB input would copy a 30 KB tape on
every loop iteration — wall-clock seconds for a program that runs in
microseconds on every other target.

Alternative shapes do exist:

- **ETS** (Erlang Term Storage): mutable side-store, but every read/write
  is a function call with microsecond latency.  Defeats the point of a
  compiled target.
- **Process dictionary**: same shape; same problem.
- **NIF** (Native Implemented Function): write the tape ops in C, link
  the `.so` at load time.  This is C code wearing a BEAM costume —
  you're not "compiling BF to BEAM bytecode" anymore.

Each of those would technically work, but none honor the LANG VM
promise of "any frontend compiles to vanilla code on any backend."
Brainfuck's mutable byte cells are anti-BEAM, and the right call is a
**documented rejection** — readers reach this section instead of
silence, and future work that wants to revisit this has a clear starting
point.

## Running tests

```bash
cargo test -p brainfuck-iir-compiler
```

81 tests total (63 unit + 9 doc-tests + 9 JIT smoke tests).
