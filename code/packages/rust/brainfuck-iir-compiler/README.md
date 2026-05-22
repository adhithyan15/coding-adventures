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
| `,` | `call_builtin getchar () u8` → `v` + `store_mem ptr v u8` |
| `[`…`]` | structured loop (label + guard + body + back-edge) |

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

**The careful caveat**: today the wrapper supplies a private
`InterpOnlyBackend` whose `compile()` always returns `None`, pinning
every function on the interpreter tier.  Why?  Because Brainfuck's
`load_mem` / `store_mem` opcodes are wired through *custom* `vm-core`
opcode handlers, and no existing JIT backend (NullBackend, EchoBackend,
future WASM/x86_64) knows how to lower them.  Trying to compile with a
backend that silently can't handle those would replace `main` with a
stub that returns `Null`, producing no output.  Refusing to compile is
correct.

**Why ship the wiring anyway?**  Because when a future backend learns
Brainfuck's tape memory model, swapping the backend in `src/vm.rs` is
the only change needed for tier promotion to kick in.  Until then, the
JIT chain runs Brainfuck programs identically to the pure interpreter —
verified by `tests/jit_smoke.rs`, which runs every program twice and
asserts byte-identical output.

```rust
use brainfuck_iir_compiler::BrainfuckVM;

let vm = BrainfuckVM::new(true, 30_000, None).unwrap();  // JIT enabled
let out = vm.run("+++.", b"").unwrap();
assert_eq!(out, vec![3u8]);
```

## Running tests

```bash
cargo test -p brainfuck-iir-compiler
```

69 tests total (55 unit + 8 doc-tests + 6 JIT smoke tests).
