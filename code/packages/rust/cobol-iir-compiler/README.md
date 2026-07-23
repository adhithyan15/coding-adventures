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

## This slice (v0.21 — the core plus control flow, reference modification `IDENT(start:len)`, `STRING … DELIMITED BY SIZE INTO`, `UNSTRING … DELIMITED BY … INTO`, `EVALUATE` (numeric/alphanumeric, multi-value/`THRU`), `COMPUTE` incl. `**` and nested `/`, `ON SIZE ERROR`, signed, alphanumeric, compound (AND/OR/NOT) + symbolic conditions, level-88 with ranges + `SET … TO TRUE`)

COBOL's WORKING-STORAGE is a **PICTURE-typed** data model. Each elementary item
becomes one IIR register: a **numeric** item (`PIC 9…`) is an `i64` holding its
value *scaled* by its fractional-digit count (`PIC 9(2)V9` holding 12.3 is the
integer `123`); an **alphanumeric** item (`PIC X`/`A`) is a `str`.

| COBOL | IIR |
| --- | --- |
| `VALUE <lit>` / `MOVE <lit> TO item` | the register's `const`/`str_const` — the literal formatted into the item's picture at compile time |
| `MOVE item TO item` | numeric→numeric rescales the implied point; character→character reshapes to the receiver's size (`str_slice` to truncate, `str_concat` to space-pad); **numeric→alphanumeric** (unsigned **or** signed, integer **or** scaled `PIC [S]9(i)V9(d)`) builds the `(i+d)`-digit zero-padded image at run time (each digit `(slot / 10^k) % 10` sliced out of a constant `"0123456789"` table — integer part then fractional part, **no decimal point**, since the scaled slot already holds `value·10^d`) and feeds it through the same char reshape (left-justify, space-pad, or truncate) — so `9(2)V9=4.2 → X(3)` is `"042"`, `→ X(5)` is `"042  "`, `→ X(2)` is `"04"`. A **signed** source (`PIC S9…`) additionally folds the operational sign into a **trailing overpunch** on the units digit: `neg = slot < 0`, `units = \|slot\| % 10`, and the last character is re-sliced from a single combined table `"{ABCDEFGHI}JKLMNOPQR"` at `units + neg*10` (positive row `{A…I`, negative row `}J…R`) — byte-identical to the runtime's `overpunch_trailing`/`__cob_print_signed` — so `S9(3)=+123 → "12C"`, `= -123 → "12L"`, `S9V9=-4.2 → "4K"` (a signed *positive* source overpunches too, differing from an unsigned `"123"`); **alphanumeric→unsigned numeric** (integer **or** scaled `PIC 9(i)V9(d)`) folds the `m` source bytes left-to-right into an `i64` (`d = str_index(src,k) - '0'`; `value = value*10 + d`); that fold **is the receiver's scaled slot magnitude directly**, so the store hands `store_scaled` the receiver's own scale `d` (a no-op rescale) and keeps the low-order `(i+d)` digits (`mag mod 10^(i+d)`), the implied point `d` from the right — right-justified, zero-padded or high-order-truncated (`MOVE "042" TO 9(2)V9` is `042` reads `4.2`, `MOVE "12345" TO 9(2)V9` is `345` reads `34.5`; **not** the arithmetic decimal-align rule — `value` is not multiplied by `10^d`; `d=0` reproduces the integer-receiver path). An alphanumeric→**signed** numeric MOVE, a `SIGN` clause with `SEPARATE`/`LEADING`, a group on either side, and a source wider than 18 chars are later rungs |
| `ADD`/`SUBTRACT`/`MULTIPLY`/`DIVIDE … [GIVING r] [ROUNDED] [ON SIZE ERROR …]` | `add`/`sub`/`mul`/`div` on the `i64` slots, the result reduced to the receiver's field; a size error runs the handler and leaves the receiver unchanged |
| `COMPUTE r [ROUNDED] = expr [ON SIZE ERROR …]` | the precedence cascade (`+ - * /`, unary minus, `**` with a constant exponent, parentheses) evaluated bottom-up over scaled `i64`, each step overflow-guarded |
| `DISPLAY op…` | each operand's image emitted, then `putchar('\n')` — a literal prints its source text, a numeric item via the fixed-width digit helper (signed items via a trailing-overpunch helper), an alphanumeric via `print_str` |
| `IDENT(start:len)` / `IDENT(start:)` (reference modification) | a 1-based substring of an alphanumeric item over the byte range `[start-1, start-1+len)` (omitted length runs to the item's end), in `DISPLAY` and alphanumeric-comparison (`IF`/`EVALUATE`) operands. **Constant (literal) indices** lower to a constant-index `str_slice`, bounds validated at compile time. **Computed (data-name) indices** — `WS(J:K)`, `WS(J:)`, `WS(2:K)` — read each index into an `i64` register and build `start0 = start-1` / `end = start0+len` (or the item width) with `sub`/`add`, feeding a run-time `str_slice`. The index item must be an unsigned integer. An out-of-range computed refmod **traps at run time** under the same predicate (`start0 < 0 \|\| end < start0 \|\| end > width`) the oracle applies, so both engines error identically |
| `IF cond then… [ELSE else…]` | conditions combine simple conditions with `AND`/`OR`/`NOT` (and parentheses; `NOT` tightest, then `AND`, then `OR`) — `AND`/`OR` fold the `0`/`1` leaf booleans with bitwise `and`/`or`, and a `NOT` inverts one with `xor` against `1`. A simple condition is a relation (numeric: align operands + `cmp_*`; alphanumeric: space-pad + `str_cmp`; **mixed numeric ↔ alphanumeric** (integer **or** scaled `PIC [S]9(i)V9(d)`, **unsigned or signed**): build the numeric side's `(i+d)`-digit zero-padded image (int part then frac part, no point) with the same run-time digit helper the numeric→alphanumeric `MOVE` uses, then space-pad + `str_cmp` like any alphanumeric relation — so `NUM = "042"` is true, `NUM = "42"` is false, and `9(2)V9=4.2 = "042"` is true. A **signed** operand additionally folds the operational sign into a **trailing overpunch** on the units digit (via the same `emit_signed_num_alpha_image` the signed→alphanumeric `MOVE` uses), so `S9(3)=-123` equals `"12L"`, `= +123` equals `"12C"`, `S9V9=-4.2` equals `"4K"`; ordering follows the byte comparison of those images) or a level-88 condition-name (`cmp_eq` on its slot). Relations use word (`GREATER THAN`, …) or symbolic (`> < = >= <= <>`) operators; a relop `NOT` and the baseline negation `>=`/`<=`/`<>` carry compose by XOR and invert the relation directly. `jmp_if_false` over the then-branch |
| `88 cond-name VALUE lit… [lo THRU hi]` | registers a boolean condition-name over the preceding item; `IF cond-name` / `PERFORM … UNTIL cond-name` hold when the variable equals any listed value or falls in any inclusive range — lowered as an OR-fold of `cmp_eq` / `and(cmp_ge, cmp_le)` (numeric) |
| `SET cond-name TO TRUE` | stores the condition-name's first `VALUE` (a range's low bound) into its conditional variable — a `const` store into the slot (numeric) |
| `EVALUATE subj WHEN v… [lo THRU hi]… WHEN OTHER … END-EVALUATE` | a `jmp_if_false` branch cascade (a chain of `IF`s): each `WHEN` OR-folds its value-list (a single value → `cmp_eq`, a `THRU` range → `and(cmp_ge, cmp_le)`) into one boolean; the first match runs and jumps to the end (no fall-through); `WHEN OTHER` runs unconditionally once reached. Each subject-vs-`WHEN`-value comparison reuses the **same relation dispatch** an `IF subject <relop> value` uses (`emit_operand_relation`), so `EVALUATE` inherits `IF`'s full category handling: numeric (scaled `cmp_*`), alphanumeric (`str_cmp`, space-padded), and **mixed numeric↔alphanumeric** (unsigned / **signed** overpunch / **scaled** digit image), plus figuratives and the `ZERO`-numeric routing (`WHEN ZERO` stays a numeric comparison). Deferrals are inherited too: a numeric-literal `WHEN` against an alphanumeric subject (and a group operand) is a clean reject on both engines |
| `STRING s… DELIMITED BY SIZE INTO t` | the sending fields concatenated with a `str_concat` chain (each source a `(reg, compile-time len)` pair — an item's slot or a `str_const` literal), then overlaid onto the receiver: a result at least as wide is truncated (`str_slice(concat, 0, width)`), a shorter one preserves the receiver's old tail (`str_concat(concat, str_slice(t, len, width))`) — COBOL's no-space-fill rule |
| `UNSTRING s DELIMITED BY d INTO r1 [r2 …]` | a run-time **scan loop** (the delimiter position is data-dependent): `len = str_len(s)`, a cursor `p`, and a single delimiter byte `d` (a `const` for a 1-char literal, or `str_index(item, 0)` for a `PIC X(1)` item). Each receiver, guarded by `if p <= len`, scans `s[j]` with `str_index`/`cmp_eq`/`cmp_ge` for the next delimiter (or end-of-source) `q`, cuts `piece = str_slice(s, p, q)`, reshapes it into the receiver as `str_slice(piece, 0, min(len, W)) ++ spaces(W - take)` (the alphanumeric MOVE), and advances `p = q + 1`; an exhausted source leaves the remaining receivers unchanged |
| `INSPECT s TALLYING c FOR ALL d` | a run-time **count loop** (the delimiter position is data-dependent): `len = str_len(s)`, a cursor `j`, a count accumulator `cnt` (init 0), and a single delimiter byte `d` (a `const` for a 1-char literal, or `str_index(item, 0)` for a `PIC X(1)` item). While `j < len` (`cmp_ge`) it reads `s[j]` with `str_index`, `cmp_eq`s it to `d`, and bumps `cnt` on a match; then the count is folded into the counter with the same numeric-store path `ADD` uses — INSPECT **adds** to the counter, it does not clear it first. `LEADING`/`CHARACTERS` tallies, `BEFORE`/`AFTER` phrases, and several counters/`FOR` phrases are later rungs |
| `INSPECT s REPLACING ALL x BY y` | a per-position **rebuild** of the source, **unrolled** over its compile-time-known width `W` (the map is length-preserving since both `x` and `y` are single characters): at each `j`, `str_index` reads `s[j]`, `cmp_eq` tests it against the search byte `x`, and a branch splices either the replacement `y` (a 1-char string — a `str_const` literal or a `PIC X(1)` item register) or the original `str_slice(s, j, j+1)` onto a `str_concat` accumulator; the `W`-wide result is copied into the source register. `REPLACING CHARACTERS`/`LEADING`/`FIRST`, `BEFORE`/`AFTER` regions, several replace items, and a multi-character/figurative/wider search or replacement are later rungs |
| `INSPECT s TALLYING c FOR ALL d REPLACING ALL x BY y` | the **combined** form (one `INSPECT`, both phrases) — composes the two lowerings above on the same source register in ISO order: the **count loop FIRST** (tallying the ORIGINAL bytes into `c`), then the **replace rebuild** (overwriting `s`). Counting before replacing is what makes a shared `d == x` correct: the count sees every occurrence before any is substituted. A combined statement whose `TALLYING` or `REPLACING` half is itself a deferred sub-form stays a later rung |
| `INSPECT s CONVERTING f TO t` | a per-position **rebuild** through a translation table, **unrolled** over the compile-time-known width `W` (length-preserving, since each character maps to exactly one). The two equal-length string literals `f`/`t` bake a compile-time table: each `f[k]` a `const` compare byte, each `t[k]` a 1-char `str_const`. At each `j`, `str_index` reads `s[j]` once and a **first-match-wins** chain tests it against each `f[k]` (`cmp_eq`), splicing the earliest matching `t[k]` — else the original `str_slice(s, j, j+1)` — onto a `str_concat` accumulator; the `W`-wide result is copied into the source register. First-match-wins mirrors the oracle's map, so a duplicated `f` character keeps its leftmost `t` partner. Unequal-length/non-ASCII literals, a data-name/figurative/reference-modified `f`/`t`, and a `BEFORE`/`AFTER` region are later rungs; a `CONVERTING` combined with `TALLYING`/`REPLACING` does not parse |
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
figuratives expanded to the partner operand's length. Two figuratives compared
against each other (`IF ZERO = SPACE`) have no length to borrow, so each resolves
to a single fill character (width 1) — `ZERO` → `"0"`, `SPACE` → `" "` — matching
the oracle, so `ZERO = ZERO` is true and `ZERO > SPACE`.

### Deliberately a later rung

Each of these is a clean `CompileError::Unsupported` (never wrong output): group
items, a **group item** in a mixed numeric↔alphanumeric comparison and a
**numeric-literal-vs-alphanumeric** pairing (a different pairing kept out of
scope — a numeric *item* ↔ alphanumeric comparison, **unsigned or signed**,
integer or scaled, is now supported: a signed operand compares by its magnitude
image with the operational sign folded into a trailing overpunch on the units
digit, the same image the signed→alphanumeric `MOVE` builds, so `S9(3)=-123`
equals `"12L"` and `= +123` equals `"12C"`), a `COMPUTE` nested
division whose scale-12 intermediate could exceed the 18-digit `i64` model (or one
paired with `ON SIZE ERROR`), a `COMPUTE` `**` whose exponent is a variable, a
parenthesised expression, negative, fractional, or past the oracle's `MAX_POW_EXP`
(or whose conservative digit bound could exceed the 18-digit model), a
level-88 condition-name over an alphanumeric variable, a **reference
modification** with a signed/fractional/non-numeric index item, of a numeric item,
or in a numeric/arithmetic/`MOVE`-source context (an out-of-range *constant*
reference modification is likewise rejected at compile time, never lowered to a
runtime trap; a *computed* one traps at run time, matching the oracle), a `STRING`
with a real (identifier/literal) delimiter, `WITH POINTER`,
`ON`/`NOT ON OVERFLOW`, a numeric item or figurative as a sending field, or a
non-alphanumeric receiver, and an `UNSTRING` with a multi-character / `ALL` / `OR`
delimiter, `WITH POINTER`, `ON`/`NOT ON OVERFLOW`, a numeric/figurative/reference-
modified delimiter, or a numeric/group source or receiver.

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
