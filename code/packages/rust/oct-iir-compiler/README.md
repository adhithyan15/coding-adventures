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

| Compiles                          | Doesn't                                   |
|-----------------------------------|-------------------------------------------|
| Arithmetic, bitwise, comparison   | `in` + arithmetic/rotation intrinsics     |
| short-circuit `&&` / `||`         | Strings (no STRING token in Oct grammar)  |
| `if` / `while` / `loop` / `break` | Body-level `static` (static-lifetime local) |
| User fns (i64 + void returns)     | Floating-point (8008 doesn't have FP)     |
| Local variables                   |                                           |
| **Top-level `static` globals**    |                                           |
| `out(port, value)` → stdout       |                                           |

**`static` module globals** (LANG-FULL O3): a top-level `static counter: u8 = 40;`
lowers to a module global (`global_load`/`global_store`) — shared across every function
and surviving across calls — not a per-function register. The initialiser runs once at
the top of `main`. A void function (`fn bump() { counter = counter + 1; }`) called as a
statement mutates it; reading it back yields the accumulated value on all 7 backends.
(Body-level `static` — a static-lifetime *local* — is not yet lowered; a local `let` may
not shadow a static.)

**u8 width & wrap** (LANG-FULL O2): Oct's only integer type is `u8` (the 8008 byte),
and arithmetic wraps modulo 256, so arithmetic/bitwise ops and unary `~` carry the
`u8` type_hint — every backend masks the result. `out(1, 200 + 100)` prints `44`
(not 300) and `out(1, ~0)` prints `255` (not -1). Comparisons stay `i64` (their 0/1
`bool` result is never masked). Because Oct has a single integer width, every integer
op is u8 by construction — there is no per-expression type to track.

`out(port, value)` (LANG-FULL O-OUT) lowers to `call_builtin "print_i64"` — all 24
8008 output ports collapse to stdout — giving Oct its first observable output (its
`main` is void, so the exit code is always 0).  The remaining intrinsics (`in`,
`adc`, `sbb`, the rotations, `carry`, `parity`) still fail with a clean
`OctError::Unsupported8008Intrinsic` pointing at the dedicated Intel-8008 simulator
backend.

See [code/specs/OCT02-oct-rust-frontend.md](../../../specs/OCT02-oct-rust-frontend.md).
