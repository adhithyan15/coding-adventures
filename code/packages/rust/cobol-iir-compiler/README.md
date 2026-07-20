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

## This slice (v0.20 — the core plus control flow, reference modification `IDENT(start:len)`, `STRING … DELIMITED BY SIZE INTO`, `EVALUATE` (numeric/alphanumeric, multi-value/`THRU`), `COMPUTE` incl. `**` and nested `/`, `ON SIZE ERROR`, signed, alphanumeric, compound (AND/OR/NOT) + symbolic conditions, level-88 with ranges + `SET … TO TRUE`)

COBOL's WORKING-STORAGE is a **PICTURE-typed** data model. Each elementary item
becomes one IIR register: a **numeric** item (`PIC 9…`) is an `i64` holding its
value *scaled* by its fractional-digit count (`PIC 9(2)V9` holding 12.3 is the
integer `123`); an **alphanumeric** item (`PIC X`/`A`) is a `str`.

| COBOL | IIR |
| --- | --- |
| `VALUE <lit>` / `MOVE <lit> TO item` | the register's `const`/`str_const` — the literal formatted into the item's picture at compile time |
| `MOVE item TO item` | numeric→numeric rescales the implied point; character→character reshapes to the receiver's size (`str_slice` to truncate, `str_concat` to space-pad) |
| `ADD`/`SUBTRACT`/`MULTIPLY`/`DIVIDE … [GIVING r] [ROUNDED] [ON SIZE ERROR …]` | `add`/`sub`/`mul`/`div` on the `i64` slots, the result reduced to the receiver's field; a size error runs the handler and leaves the receiver unchanged |
| `COMPUTE r [ROUNDED] = expr [ON SIZE ERROR …]` | the precedence cascade (`+ - * /`, unary minus, `**` with a constant exponent, parentheses) evaluated bottom-up over scaled `i64`, each step overflow-guarded |
| `DISPLAY op…` | each operand's image emitted, then `putchar('\n')` — a literal prints its source text, a numeric item via the fixed-width digit helper (signed items via a trailing-overpunch helper), an alphanumeric via `print_str` |
| `IDENT(start:len)` / `IDENT(start:)` (reference modification) | a 1-based substring of an alphanumeric item — constant integer indices lower to a constant-index `str_slice` over the byte range `[start-1, start-1+len)` (omitted length runs to the item's end). Supported in `DISPLAY` and alphanumeric-comparison (`IF`/`EVALUATE`) operands; bounds validated at compile time |
| `IF cond then… [ELSE else…]` | conditions combine simple conditions with `AND`/`OR`/`NOT` (and parentheses; `NOT` tightest, then `AND`, then `OR`) — `AND`/`OR` fold the `0`/`1` leaf booleans with bitwise `and`/`or`, and a `NOT` inverts one with `xor` against `1`. A simple condition is a relation (numeric: align operands + `cmp_*`; alphanumeric: space-pad + `str_cmp`) or a level-88 condition-name (`cmp_eq` on its slot). Relations use word (`GREATER THAN`, …) or symbolic (`> < = >= <= <>`) operators; a relop `NOT` and the baseline negation `>=`/`<=`/`<>` carry compose by XOR and invert the relation directly. `jmp_if_false` over the then-branch |
| `88 cond-name VALUE lit… [lo THRU hi]` | registers a boolean condition-name over the preceding item; `IF cond-name` / `PERFORM … UNTIL cond-name` hold when the variable equals any listed value or falls in any inclusive range — lowered as an OR-fold of `cmp_eq` / `and(cmp_ge, cmp_le)` (numeric) |
| `SET cond-name TO TRUE` | stores the condition-name's first `VALUE` (a range's low bound) into its conditional variable — a `const` store into the slot (numeric) |
| `EVALUATE subj WHEN v… [lo THRU hi]… WHEN OTHER … END-EVALUATE` | a `jmp_if_false` branch cascade (a chain of `IF`s): each `WHEN` OR-folds its value-list (a single value → `cmp_eq`, a `THRU` range → `and(cmp_ge, cmp_le)`) into one boolean; the first match runs and jumps to the end (no fall-through); `WHEN OTHER` runs unconditionally once reached. A **numeric** subject compares with scaled `cmp_*`; an **alphanumeric** subject compares with `str_cmp` (space-padded) |
| `STRING s… DELIMITED BY SIZE INTO t` | the sending fields concatenated with a `str_concat` chain (each source a `(reg, compile-time len)` pair — an item's slot or a `str_const` literal), then overlaid onto the receiver: a result at least as wide is truncated (`str_slice(concat, 0, width)`), a shorter one preserves the receiver's old tail (`str_concat(concat, str_slice(t, len, width))`) — COBOL's no-space-fill rule |
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
rounding and zero-divisor handling. A division **nested** inside a larger
expression reproduces the oracle's fixed scale-12 intermediate (`COMPUTE_DIV_SCALE`,
re-exported so the two stay in lockstep): `a / b` becomes
`(a · 10^(b.scale+12)) / (b · 10^a.scale)` truncated toward zero, a scale-12 value
the surrounding operators then combine exactly. (Because the scale-12 math is `i64`
here but `i128` in the oracle, a numerator/denominator that could exceed 18 digits
is a clean later rung; and a nested division under `ON SIZE ERROR` — whose zero
divisor would need routing to the handler — is deferred.) **Exponentiation** (`**`) with a compile-time
non-negative integer exponent `e` unrolls into a chain of `e − 1` `mul`s of the
base — the oracle computes `base**e` by multiplying `1` by `base` `e` times, so
the mul-chain's magnitude (`base_scaled^e`) and scale (`e · base.scale`) match it
exactly; `x ** 0` is the constant `1` (the base is never read). `**` folds
right-associatively (`A ** B ** C = A ** (B ** C)`) and binds tighter than `* /`.

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

Character handling is fixed-length string work: a character item's slot always
holds exactly its declared width, so an item-to-item `MOVE` and an alphanumeric
comparison both reduce to a single compile-time-sized `str_slice`/`str_concat`
(reshape) or a space-pad plus `str_cmp` (compare), with `SPACE`/`ZERO`
figuratives expanded to the partner operand's length.

### Deliberately a later rung

Each of these is a clean `CompileError::Unsupported` (never wrong output): group
items, cross-category item `MOVE` (`numeric↔alphanumeric`, which needs runtime
int↔string conversion), a numeric-vs-alphanumeric comparison, a `COMPUTE` nested
division whose scale-12 intermediate could exceed the 18-digit `i64` model (or one
paired with `ON SIZE ERROR`), a `COMPUTE` `**` whose exponent is a variable, a
parenthesised expression, negative, fractional, or past the oracle's `MAX_POW_EXP`
(or whose conservative digit bound could exceed the 18-digit model), a
level-88 condition-name over an alphanumeric variable, a **reference
modification** with a computed (data-name) start/length, of a numeric item, or in
a numeric/arithmetic/`MOVE`-source context (an out-of-range *constant* reference
modification is likewise rejected at compile time, never lowered to a runtime
trap), and a `STRING` with a real (identifier/literal) delimiter, `WITH POINTER`,
`ON`/`NOT ON OVERFLOW`, a numeric item or figurative as a sending field, or a
non-alphanumeric receiver.

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
