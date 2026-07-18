# cobol-iir-compiler

Lower a parsed **COBOL-60** program to the shared **IIR** (`interpreter_ir::IIRModule`)
so COBOL runs on every execution backend the LANG VM AOT chain targets —
Native-AOT, LLVM, WASM, JVM, CLR, and the generic register VM / JIT.

This is the COBOL sibling of [`flow-matic-iir-compiler`](../flow-matic-iir-compiler),
and step 4 of [PL09](../../specs/PL09-codegen.md). The tree-walking interpreter
[`cobol-runtime`](../cobol-runtime) is the **semantic oracle**: the compiled
program's output must be byte-identical to what the interpreter `DISPLAY`s.

## Where it sits in the stack

```
COBOL-60 source (carded, 80-column)
   │  cobol-lexer + cobol-parser   (cobol.grammar → CST)
   ▼
GrammarASTNode  (rule_name == "program")
   │  cobol-iir-compiler::compile_source     ← this crate
   ▼
interpreter_ir::IIRModule   (one `main`, returns i64 exit code)
   │  lang-aot  (Language::Cobol60)
   ▼
NativeAOT · LLVM · WASM · JVM · CLR · VM · JIT
```

## This slice (v0.6 — the core plus control flow, `COMPUTE`, `ON SIZE ERROR`, signed)

COBOL's WORKING-STORAGE is a **PICTURE-typed** data model. Each elementary item
becomes one IIR register: a **numeric** item (`PIC 9…`) is an `i64` holding its
value *scaled* by its fractional-digit count (`PIC 9(2)V9` holding 12.3 is the
integer `123`); an **alphanumeric** item (`PIC X`/`A`) is a `str`.

| COBOL | IIR |
| --- | --- |
| `VALUE <lit>` / `MOVE <lit> TO item` | the register's `const`/`str_const` — the literal formatted into the item's picture at compile time |
| `ADD`/`SUBTRACT`/`MULTIPLY`/`DIVIDE … [GIVING r] [ROUNDED] [ON SIZE ERROR …]` | `add`/`sub`/`mul`/`div` on the `i64` slots, the result reduced to the receiver's field; a size error runs the handler and leaves the receiver unchanged |
| `COMPUTE r [ROUNDED] = expr [ON SIZE ERROR …]` | the precedence cascade evaluated bottom-up over scaled `i64`, each step overflow-guarded |
| `DISPLAY op…` | each operand's image emitted, then `putchar('\n')` — a literal prints its source text, a numeric item via the fixed-width digit helper (signed items via a trailing-overpunch helper), an alphanumeric via `print_str` |
| `IF cond then… [ELSE else…]` | `cmp_*` on the aligned operands → `jmp_if_false` over the then-branch; `NOT` inverts the relation |
| `GO TO para` | `jmp para_<name>` |
| `PERFORM para [THRU q] [n TIMES \| UNTIL c \| VARYING v FROM a BY b UNTIL c]` | the paragraph range **inlined** at the call site (out-of-line-but-returns semantics), with loop control emitted around it |
| `STOP RUN` | `ret 0` |

### Why it is exact

A numeric item does not display as a plain integer: `PIC 9(5)` holding 42 shows
`00042`, and `PIC 9(2)V9` holding 123.456 shows `234` (truncated, implied point).
For a literal stored into a field, the value is known at compile time, so the
compiler calls the very same picture/value functions the oracle uses —
`cobol-runtime`'s `move_into_numeric` / `move_into_char`, re-exported for exactly
this reuse — and stores the resulting scaled value. A numeric *literal* in a
`DISPLAY`, by contrast, shows its **source text** (`DISPLAY 42` → `42`).

Runtime arithmetic is native `i64` over the scaled slots.
`ADD`/`SUBTRACT`/`MULTIPLY`/`DIVIDE` all honour the implied point (`PIC …V…`):
additive verbs align operands to a common working scale then accumulate;
`MULTIPLY` products carry scale `sa + sb`; `DIVIDE` scales the dividend up before
the truncating division to land the receiver's decimals (plus a guard digit for
rounding). Every store rounds **half away from zero** with `ROUNDED` (else
truncates) and keeps the low-order `int_digits + dec_digits` digits (COBOL's
silent-overflow-truncation rule). `ON SIZE ERROR` turns that silent truncation
into a caught condition (handler runs, receiver unchanged) when the integer part
overflows or a divisor is zero.

`COMPUTE` evaluates the grammar's precedence cascade (`+ - * /`, unary minus,
parentheses) bottom-up in the same scaled-`i64` model; every node carries a
compile-time `(scale, integer-digit)` bound and every combining step is
overflow-guarded, so an intermediate that could exceed 18 digits is a clean error
rather than a silent wrap. A **top-level** division reuses the `DIVIDE` verb's
rounding and zero-divisor handling.

**Signed numerics (`PIC S9…`)** keep their sign in the `i64` slot — so arithmetic
and `IF` comparisons are signed — and `DISPLAY` shows the sign as a trailing
**overpunch** on the units digit (`{A-I}` positive, `{J-R}` negative, `'{'`/`'}'`
for zero). An unsigned receiver stores only magnitude, so a signed→unsigned `MOVE`
drops the sign. Numeric **item-to-item `MOVE`** reshapes the source value into the
receiver's picture. Control flow: **`GO TO`** jumps to a paragraph label;
**`PERFORM`** (paragraph / `THRU` / `TIMES` / `UNTIL` / `VARYING`) inlines the
paragraph range at the call site, which reproduces COBOL's return semantics
exactly (a `STOP RUN` inside returns, a `GO TO` inside jumps away), bounded by
depth and instruction-count caps against a recursive `PERFORM`.

### Deliberately a later rung

Each of these is a clean `CompileError::Unsupported` (never wrong output): group
items, alphanumeric item `MOVE` and alphanumeric comparison, `COMPUTE` division
nested inside a larger expression, and `COMPUTE` exponentiation (`**`).

## Usage

```rust
use cobol_iir_compiler::compile_source;

let src = "\
000000 IDENTIFICATION DIVISION.
000000 PROGRAM-ID. HELLO.
000000 PROCEDURE DIVISION.
000000 MAIN.
000000     DISPLAY \"HELLO, WORLD\".
000000     STOP RUN.";
let module = compile_source(src, "hello").unwrap();
assert!(module.validate().is_empty());
// → run `module` on any backend; it prints "HELLO, WORLD\n".
```

## Testing

* `cargo test -p cobol-iir-compiler` — unit tests (compile shape + honest-failure
  errors), `tests/backend_compat.rs` (every AOT backend validator accepts the IIR),
  and `tests/jit_e2e.rs` (compiled-and-run output is byte-identical to the
  `cobol-runtime` oracle).
* `lang-aot/tests/lang_matrix.rs` carries the COBOL rows proven across the backend
  columns.
