# brainfuck-iir-compiler

**BF04** — Rust port of the Brainfuck → InterpreterIR compiler and vm-core wrapper.

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
BF04    brainfuck-iir-compiler ← THIS CRATE
BF05    brainfuck-jit-wasm ← JIT via wasm-backend (future)
```

## JIT (BF05)

`BrainfuckVM::new(jit: true, ...)` is accepted but returns an error when
`run()` is called in BF04.  The JIT path (specialise via `jit-core`,
compile to WASM via `wasm-backend`) is implemented in BF05.

## Running tests

```bash
cargo test -p brainfuck-iir-compiler
```

60 tests total (52 unit + 8 doc-tests).
