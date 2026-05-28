# Changelog — `oct-iir-compiler`

## 0.2.0 — 2026-05-28 (OCT03 — JIT via GenericCirJit)

### Added — Oct programs JIT-compile via `jit-core::GenericCirJit`

With `jit-core::GenericCirJit` landed in `jit-core` 0.3.0, Oct gets a
real JIT **without a per-language Backend impl**.  Oct functions
compile through `JITCore::execute_with_jit` → `GenericCirJit` →
packed bytecode.

This is the second language (after Brainfuck and Dartmouth BASIC) to
plug into the LANG VM's JIT chain.  Unlike Brainfuck and BASIC,
which still ship their own per-language Backend impls
(`BrainfuckCirJit` / `BasicCirJit`), Oct uses `GenericCirJit`
directly — no duplicated code.

### Changed — `IIRFunction::type_status = FullyTyped` override

`IIRFunction::new`'s automatic `infer_type_status` returns
`PartiallyTyped` because Oct's control-flow ops (`label`, `jmp`,
`jmp_if_false`, `ret_void`) carry `"void"` hints, and `"void"` is
NOT in `interpreter_ir::opcodes::CONCRETE_TYPES`.  Every Oct
instruction is in fact statically known (no `"any"` hints), so the
function is genuinely fully typed for the JIT's threshold-zero
compile path.  We now override `type_status = FullyTyped` after
construction, mirroring Brainfuck and BASIC.

Without this fix, `JITCore` would never call `compile()` on Oct's
functions, and `GenericCirJit` would never run.

### Tests

- 4 new end-to-end tests in `tests/jit_e2e.rs`:
  - `oct_jit_returns_constant_42`: `fn answer() -> u8 { return 42; }`
  - `oct_jit_arithmetic_and_return`: `let x: u8 = 30; let y: u8 = 12;
    return x + y;` → 42
  - `oct_jit_if_else`: `if x == 0 { x = 1; } else { x = 2; }` → 1
  - `oct_jit_while_loop`: `while n < 10 { n = n + 1; }` → 10
- All 11 existing lib tests continue to pass.

### Dependencies

- Added `vm-core` and `jit-core` as **dev-dependencies** (the JIT
  test harness lives in `tests/jit_e2e.rs`).  Oct's main library
  has no runtime JIT dependency — the JIT integration is purely a
  consumer-side concern (downstream `oct-vm` or similar would pull
  in `vm-core` + `jit-core` as needed).

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
