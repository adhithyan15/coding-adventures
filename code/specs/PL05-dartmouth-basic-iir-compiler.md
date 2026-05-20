# PL05 — `dartmouth-basic-iir-compiler` crate

**Status:** Draft — 2026-05-20
**Depends on:** PL01, PL02, LANG43, LANG44, LANG75
**Related:** LANG76 (only if arrays are in V1 scope)

## Motivation

The existing `dartmouth-basic-ir-compiler` (in Rust and Python) lowers
Dartmouth BASIC to a custom `IrProgram` / `IrInstruction` shape
designed for the **GE-225 simulator backend** — register file with
fixed widths, syscall numbers, etc.  That IR doesn't plug into the
LANG VM AOT chain because the AOT chain expects
`interpreter_ir::IIRModule`.

`lang-aot` (PR #3673) currently returns a clean
`UnsupportedLanguage` error for `.bas` / `.basic` files, with
guidance pointing at this spec.

PL05 specs a **new** crate (`dartmouth-basic-iir-compiler`) that
parses BASIC and emits `IIRModule` directly.  The existing
`dartmouth-basic-ir-compiler` keeps its GE-225 mission; the new crate
targets the LANG VM AOT chain.

## Non-goals (V1)

- **Strings**.  BASIC's classic `PRINT "HELLO"` and `INPUT A$` are
  deferred to V2 (needs LANG77 strings).
- **Arrays** (`DIM A(100)`).  Deferred to V2 (needs LANG76).
- **Floating-point**.  BASIC's numeric type is real; V1 emits
  integer-only IIR.  Programs using division that doesn't divide
  evenly silently truncate.
- **`DEF FN`** (user-defined functions).  Deferred — BASIC's DEF FN
  is a single-expression macro, easy enough to inline later.
- **Data files** (`OPEN` / `INPUT#` / `PRINT#`).
- **Multi-line `IF…THEN…ELSE` blocks**.  V1 supports only the
  single-line classic `IF cond THEN line#`.

## V1 supported BASIC

Statements that compile:

```basic
10 LET A = 5
20 LET B = A * 3 + 2
30 IF B > 10 THEN 60
40 PRINT B
50 GOTO 80
60 PRINT A
70 GOTO 80
80 END
```

Also: `FOR I = 1 TO 10 STEP 2 / NEXT I`, `GOSUB line# / RETURN`,
`INPUT A`, `REM <comment>`.

The aim is "every program in the original Dartmouth BASIC manual that
doesn't use strings or arrays should compile."

## Crate layout

```
code/packages/rust/dartmouth-basic-iir-compiler/
├── BUILD
├── Cargo.toml
├── CHANGELOG.md
├── README.md
└── src/
    ├── lib.rs          — pub fn compile_source(src, name) -> Result<IIRModule, …>
    ├── lower.rs        — AST → IIR lowering
    └── errors.rs
```

Depends on:
- `coding-adventures-dartmouth-basic-parser` (existing — produces AST)
- `interpreter-ir` (IIRModule target)
- No dependency on `dartmouth-basic-ir-compiler` — they're siblings.

## Lowering scheme

The whole BASIC program becomes a **single function** named `main`
returning `i64`.  Line numbers become IIR labels; flow-control
statements jump between labels.

| BASIC | IIR sequence |
|---|---|
| `<n> LET X = expr` | `label "line_<n>"`; `<eval expr → tmp>`; `mov_i64 X = tmp` |
| `<n> IF cond THEN <m>` | `label "line_<n>"`; `<eval cond → c>`; `jmp_if_true c, "line_<m>"` |
| `<n> GOTO <m>` | `label "line_<n>"`; `jmp "line_<m>"` |
| `<n> GOSUB <m>` | `label "line_<n>"`; `call basic_sub_<m> -> _`; (after) `<implicit fall-through>` |
| `<n> RETURN` | `label "line_<n>"`; `ret_void` |
| `<n> FOR I = a TO b STEP s` | `label "line_<n>"`; `mov_i64 I = a`; `mov_i64 _for_<n>_limit = b`; `mov_i64 _for_<n>_step = s`; `label "for_<n>_test"`; `cmp_le_i64 I, limit -> c`; `jmp_if_false c, "for_<n>_end"` |
| `<n> NEXT I` | `label "line_<n>"`; `add_i64 I, step -> I`; `jmp "for_<n>_test"`; `label "for_<n>_end"` |
| `<n> PRINT expr` | `label "line_<n>"`; `<eval expr → v>`; `call_builtin "print_i64", v` |
| `<n> INPUT X` | `label "line_<n>"`; `call_builtin "input_i64" -> X` |
| `<n> END` | `label "line_<n>"`; `const_i64 0 -> r`; `ret r` |
| `<n> REM …` | `label "line_<n>"`; no-op |

### Variables

BASIC supports single-letter (`A..Z`) and letter-digit (`A0..Z9`)
variables.  The IIR compiler maintains a `HashMap<String, String>`
mapping BASIC variable names to IIR virtual register names; each
unique BASIC variable gets a slot allocated on first use.

### GOSUB / RETURN

The classic semantics are call-with-return-address-stack.  PL05 V1
implements this by emitting each `GOSUB <m>` target as a synthetic
IIR function `basic_sub_<m>` that runs from `line_<m>` to the next
`RETURN` (whose `label` is also bound in that function).  Cross-fn
calls already work in the backend (PR #3331).

This is over-strict — real BASIC allows multiple entry points and
shared bodies — but it covers every example in the Dartmouth manual
without any heroics.

## Public API

```rust
use interpreter_ir::module::IIRModule;

pub enum DartmouthBasicCompileError { /* lex / parse / lower errors */ }

pub fn compile_source(source: &str, module_name: &str)
    -> Result<IIRModule, DartmouthBasicCompileError>;
```

…matches the signature `lang-aot::compile_source_to_iir` already
expects.

## lang-aot integration

After this crate lands, `lang-aot/src/lib.rs`:

1. Adds `dartmouth-basic-iir-compiler` to its deps.
2. Replaces the `UnsupportedLanguage` arm for
   `Language::DartmouthBasic` with a call to the new
   `compile_source` function.
3. Adds an end-to-end smoke test that compiles a 10-line BASIC
   program and asserts the exit code.

## Tests

### Per-crate

- `let_then_print_then_end`: `10 LET A = 42 / 20 PRINT A / 30 END`
  → IIR contains `const_i64 42`, `call_builtin "print_i64", _`, `ret`.
- `for_next_counts_correctly`: emit and execute via a mocked
  interpreter or inspect IIR.
- `gosub_return_emits_two_functions`.
- `if_then_goto_lowers_to_jmp_if_true`.

### End-to-end (via lang-aot)

```basic
10 LET A = 30
20 LET B = 12
30 LET C = A + B
40 END
```

Should compile and exit with code 42 (the `END` lowering returns
constant 0; we change it to "return the last LET'd variable" as a
test idiom — or set the exit code via a side channel).

A more honest test is to use `PRINT`:

```basic
10 PRINT 42
20 END
```

…and assert stdout contains "42\n".

## Risk register

| Risk | Mitigation |
|---|---|
| BASIC's line-number addressing isn't a clean tree — programs jump backwards arbitrarily.  Pre-scanning every line number to a label is essential | First pass over the AST registers a label for every line; second pass emits code with labels already bound. |
| Some BASIC dialects have `LET` optional (`A = 5` instead of `LET A = 5`) | Parser handles this already; lower it the same way. |
| User reuses a variable name across types (`A = 5 ; A = 5.5`) | V1 is integer-only, so `5.5` either parses as `5` or errors — pick one in the lexer and document. |
| `STEP 0` or `STEP -1` edge cases for FOR loops | V1 documents the loop continues until `I > limit` (or `< limit` if step is negative).  Two-test loop semantics like the BASIC manual. |
| `END` in the middle of the program vs at the end | Both work; `END` is just `ret 0` and falling off the end of `main` does the same. |
