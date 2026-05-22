# Changelog — `oct-iir-compiler`

## 0.1.0 — 2026-05-20 (OCT02 phase 3)

Initial Rust port of the Oct IIR compiler.  Lowers a parsed +
type-checked Oct program to `interpreter_ir::IIRModule` ready for the
LANG VM AOT chain.

### What compiles (V1)

- Function declarations with parameters and return types.
- Cross-function calls + recursion (uses LANG43's cross-function reloc).
- Local variables (lowered to named IIR slots) and `mov` updates.
- Arithmetic `+` `-` → `add` / `sub`.
- Bitwise `&` `|` `^` → `and` / `or` / `xor`.
- Comparisons (`==` `!=` `<` `>` `<=` `>=`) → `cmp_*`.
- Logical `&&` `||` lowered as eager bitwise on 0/1 operands (the type
  checker already requires `bool` operands, so the truth values are
  preserved).
- Unary `!` / `~` → `not` (V1 doesn't distinguish bitwise NOT from
  logical NOT at the IIR level; on 0/1 operands the result is correct
  in both interpretations; full-width bitwise NOT for arbitrary `u8`
  is a V2 follow-up).
- `if`/`else`, `while`, `loop`, `break` via the canonical IIR loop
  scaffold (`label` / `jmp_if_false` / `jmp` / `label`).
- Integer / hex / binary literals → `const`.
- `true` / `false` → `const 1` / `const 0`.

### What's rejected

- Every 8008 hardware intrinsic (`in`, `out`, `adc`, `sbb`, `rlc`,
  `rrc`, `ral`, `rar`, `carry`, `parity`) → `OctError::Unsupported8008Intrinsic`.
- Type errors from the upstream type checker → `OctError::Type` with
  one message per diagnostic.
- Parser errors → `OctError::Parse`.

### Entry-point convention

The Oct language spec declares `fn main()` with void return.  The
LANG VM AOT chain expects `main` to return `i64` so the C runtime's
`exit()` truncation produces a sensible exit code.  This crate rewrites
Oct's void `main` to return `i64 0` so the chain works without any
backend changes.

### Tests

11 unit tests cover minimal main, arithmetic, if/else, while, loop +
break, cross-function calls, recursion, and every rejection path
(intrinsic, type error, parse error).
