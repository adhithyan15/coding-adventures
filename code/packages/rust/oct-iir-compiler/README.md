# `oct-iir-compiler`

Oct frontend for the LANG VM AOT chain (OCT02 phase 3).  Lowers parsed
+ type-checked Oct programs to `interpreter_ir::IIRModule`.

## Usage

```rust
use oct_iir_compiler::compile_source;

let m = compile_source("fn main() { let x: u8 = 42; }", "oct_demo")
    .expect("compile");
assert_eq!(m.entry_point.as_deref(), Some("main"));
assert_eq!(m.functions[0].return_type, "i64");  // rewritten for AOT
```

## Where it fits

```
Oct source
   │
   ▼ oct-lexer + oct-parser     (phase 1)
AST
   │
   ▼ oct-type-checker           (phase 2)
verified AST
   │
   ▼ oct-iir-compiler           ← THIS CRATE
IIRModule
   │
   ▼ lang-aot                   (phase 4)
native executable
```

## V1 scope

| Compiles                         | Doesn't                                   |
|----------------------------------|-------------------------------------------|
| Arithmetic, bitwise, comparison  | 8008 intrinsics (`in`/`out`/`carry`/…)    |
| `if` / `while` / `loop` / `break`| Strings (no STRING token in Oct grammar)  |
| User fns, recursion              | Static globals (silently ignored for V1)  |
| Local variables                  | Floating-point (8008 doesn't have FP)     |

Every 8008 intrinsic fails with a clean `OctError::Unsupported8008Intrinsic`
pointing at the dedicated Intel-8008 simulator backend.

See [code/specs/OCT02-oct-rust-frontend.md](../../../specs/OCT02-oct-rust-frontend.md).
