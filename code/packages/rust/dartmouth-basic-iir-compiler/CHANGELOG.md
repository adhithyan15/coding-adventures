# Changelog — `dartmouth-basic-iir-compiler`

## [0.37.0] — 2026-07-10 (LANG-FULL E4-dyn — E4d-BA-arr: BASIC string arrays)

`DIM A$(n)` string arrays now lower to the shared E5 aggregate substrate carrying
an E4-dyn runtime **string handle** per element (`array<str>`), rather than being
rejected as a BA4 follow-up. A program can `DIM A$(2)`, assign `A$(0)="O"` /
`A$(1)="K"`, and `PRINT A$(0)+A$(1)` (→ `OK`).

**Frontend (`src/lib.rs`):**
- New `string_arrays` set marks which DIMmed arrays hold `str` elements (they are
  also recorded in `arrays` for the shared row-major flat-index folding).
- `emit_dim` picks the element type per name: a `$`-suffixed array emits
  `alloc_array … array<str>`; every other array stays `array<f64>`. The aggregate
  handle register is sanitised to a `$`-free `__basic_strarr_<stem>` (mirroring the
  scalar `__basic_str_<stem>` convention) so a string array `A$` never collides with
  a numeric array `A`.
- `emit_let` array path: `A$(i) = <string expr>` lowers the RHS through the shared
  E4 string-expression path to a runtime `str` handle and stores it with a
  `str`-typed `array_set`. A numeric RHS is a clean `Unsupported`.
- `emit_basic_string_expr_to` recognises a subscripted string-array read `A$(i)` and
  emits a `str`-typed `array_get`, so element reads compose with PRINT, `+` concat,
  and string assignment.
- `emit_primary` rejects a string-array element used in a numeric expression.

**Tests:** 8 new unit tests (alloc_array `array<str>`; `array_set`/`array_get` `str`;
element-feeds-concat; numeric-context + numeric-RHS rejections; numeric/string array
coexistence).

**Deferred:** string `READ`/`DATA` (numeric `DATA` only today).

## [0.36.0] — 2026-07-06 (LANG-FULL E4-dyn — string `INPUT A$` reads a runtime string)

`INPUT A$` (a `$`-suffixed *string* variable) now reads a whole line from the
host **as the string value itself** — a genuinely runtime string the compiler
cannot fold, unlike every prior BA4 string cell where the literal was known at
compile time.  This is the BASIC sibling of the E4-dyn foothold's
branch-selected string and the ALGOL string-procedure result: the observable
output depends on stdin, not on a folded constant.

**Frontend (`src/lib.rs`):**
- `emit_input` now branches per variable: a numeric `INPUT X` keeps the existing
  `call_builtin "input_i64"` + coerce-to-scalar-type path (factored into
  `emit_input_numeric`); a string `INPUT A$` takes the new `emit_input_string`
  path.  It gets the name via `scalar_variable_name` (which still rejects a
  subscripted `A(I)`) and dispatches on `is_basic_string_name`.
- New `emit_input_string`: emits `call_builtin "input_str" -> t` at `type_hint
  "str"` (the `str` sibling of `input_i64`), then `mov __basic_str_<stem> = t`
  at `str`, so a later `PRINT A$` / `IF A$ = …` resolves the same string slot
  through the shared E4 `print_str` / `str_eq` ops.  No new IIR op.
- Module docs: the operations table and Strings section now describe string
  `INPUT`; it is no longer listed as a BA4/E4 follow-up.

**Tests:**
- `compiles_input_string`: `INPUT A$ / PRINT A$` emits `call_builtin "input_str"`
  (`str`-typed), `mov`s the temp into the string slot, keeps `$` out of every
  backend-facing register, and the subsequent `PRINT A$` reads that slot via
  `print_str`.

**Matrix proof** lives in `lang-aot` (0.179.0): `10 INPUT A$ / 20 PRINT A$ / 30
END` with stdin `"OK"` prints `OK` on the dynamic **VM/JIT** columns (a tagged
`Value::Str` read from the shared stdin buffer by a registered `input_str`
closure).  Wiring the four subprocess/WASM columns' host read-a-line primitive
is the next slice of this arc.

## [0.35.0] — 2026-07-02 (LANG-FULL BA-DIM-2D — multi-dimensional DIM arrays)

Dartmouth BASIC arrays can now be **multi-dimensional**: `DIM A(m,n)` (and
`A(m,n,p)`, …) declare one flat array, and `A(i,j)` reads/writes an element
through a row-major flat index.  No new IIR op and **no backend change** — the
existing E5 `alloc_array`/`array_set`/`array_get` ops carry it, exactly like the
ALGOL 60 multidim work.

**Frontend (`src/lib.rs`):**
- `arrays` changed from `HashSet<String>` to `HashMap<String, Vec<i64>>`,
  mapping each DIMmed array to its **row-major strides** (one per dimension).
  For `DIM A(M,N)` the sizes are `(M+1, N+1)` (0-based inclusive) and the
  strides are `[N+1, 1]`.  A 1-D `DIM A(N)` stores `[1]`, so `A(i)` folds to the
  bare subscript `i` — BA3 semantics unchanged, no extra IIR emitted.
- `emit_dim` reads all bounds (`dim_decl_bounds`), computes per-dimension sizes,
  the total element count (product), and the strides — all `checked_*` so an
  absurd size is a clean `Unsupported`, never a panic.
- New `emit_flat_index` folds the subscripts through the strides:
  `flat = Σ_d subscript[d] * stride[d]` (`const` + `mul` + `add`, with the
  innermost `stride == 1` term emitted as the bare subscript).  It enforces the
  subscript count against the array's dimensionality (a mismatch is
  `Unsupported`).
- The `LET` write, `READ`, and expression-read paths all use `emit_flat_index`
  via the new plural `array_subscript_indices` helper.

**Tests:** 5 new unit tests (73 total) — 2-D `DIM`/write/read, the stride
`mul`+`add`, a 3-D `DIM`, and the wrong-subscript-count error.  The 7-backend
proof is the `DIM A(1,2)` cell in `lang-aot` `lang_matrix.rs`.

Requires `coding-adventures-dartmouth-basic-parser` 0.3.0 (multi-subscript
grammar).

## [0.34.0] — 2026-06-29 (LANG-FULL BA-pow — general `^` exponentiation)

Extended the `^` operator to handle non-integer exponents.  Previously
`literal_integer_exponent` returned `Err` for float-valued or out-of-range
exponents (blocking `4 ^ 0.5`).  It now returns `Ok(None)`, letting the caller
fall through to the new general path: `emit_power` emits a two-operand
`f64_pow` IIR instruction (`coerce_value` to Real on both sides) for any case
where the literal-integer fast path does not apply.  The literal-integer
fast path (repeated f64 mul for small nonneg integer exponents) is unchanged.
## 0.33.0 — 2026-06-29 — BA-arctan: `ATN` and `TAN` built-ins (LANG-FULL)

Dartmouth BASIC's `ATN` (arc tangent) and `TAN` (tangent) built-in functions are now
lowered to the `f64_atan` and `f64_tan` IIR ops respectively:

| Function | Lowers to | All 7 backends |
|----------|-----------|----------------|
| `ATN(X)` | `f64_atan` IIR op | ✅ (libm `atan` / `env.__atan` / `Math.atan` / `System.Math.Atan` / `f64::atan`) |
| `TAN(X)` | `f64_tan`  IIR op | ✅ (libm `tan`  / `env.__tan`  / `Math.tan`  / `System.Math.Tan`  / `f64::tan` ) |

`ATN(0)` = 0.0 and `TAN(0)` = 0.0 exactly in IEEE-754 double; BA7's formatter prints
whole-valued reals as integers, so `PRINT ATN(0)` outputs `0` and `PRINT TAN(0)` outputs `0`.

Removed the `Unsupported` error for `ATN` and `TAN` from the BA-builtins error table.

## 0.32.0 — 2026-06-28 — BA-builtins: `SQR`, `INT`, `ABS`, `SGN` (LANG-FULL)

Dartmouth BASIC's built-in math functions `SQR`, `INT`, `ABS`, and `SGN` are
now lowered — all using existing E3/E8 IIR ops, so no new backend code was
needed.

| Function | Lowers to | All 7 backends |
|----------|-----------|----------------|
| `SQR(X)` | `f64_sqrt` IIR op | ✅ (hardware sqrt: WASM `f64.sqrt`, LLVM `@llvm.sqrt.f64`, JVM `Math.sqrt`, CLR `System.Math::Sqrt`, aarch64 `FSQRT`, x86_64 `SQRTSD`, VM/JIT `f64::sqrt`) |
| `INT(X)` | `real_to_int_floor` → `int_to_real` | ✅ (E8 floor + convert back to real) |
| `ABS(X)` | inline compare + branch (`cmp_lt`/`jmp_if_false`/`neg`) | ✅ (store-per-branch, no phi — same pattern as ALGOL `abs`) |
| `SGN(X)` | inline 3-way conditional | ✅ (returns −1.0/0.0/1.0 as float per BA7 value model) |

`SIN`, `COS`, `LOG`, `EXP`, `TAN`, `ATN`, and `RND` are rejected with a clear
`Unsupported` error until cross-backend math helper infrastructure lands.

Verified by RUNNING `PRINT SQR(49)` → `7`, `PRINT INT(3.7)` → `3`,
`PRINT ABS(-42)` → `42`, `PRINT SGN(-5)` → `-1` on all 7 backends via
`lang-aot/tests/lang_matrix.rs`.

## 0.31.0 — 2026-06-28 — BA4 lexical string ordering in IF branches

String `IF` now lowers the standard lexical ordering relops through the shared
E4 `str_cmp` op:

```basic
10 LET A$ = "ALPHA"
20 IF A$ < "BETA" THEN 40
30 END
40 IF "BETA" > A$ THEN 60
50 END
60 PRINT "OK"
```

The compiler compares the `str_cmp` result with zero using the existing typed
`cmp_lt` / `cmp_gt` / `cmp_le` / `cmp_ge` instructions, then reuses the normal
`jmp_if_true` line-control path. Equality and inequality continue to use
`str_eq` plus `jmp_if_true` / `jmp_if_false`.

## 0.30.0 — 2026-06-28 — BA4 variable-variable string concat in IF equality

String `IF` now has an explicit unit and matrix proof for a concatenation
expression whose operands are both scalar string variables and whose result
feeds the standard `=` relop:

```basic
10 LET A$ = "O"
20 LET B$ = "K"
30 IF A$ + B$ = "OK" THEN 60
40 PRINT "BAD"
50 END
60 PRINT "OK"
70 END
```

The compiler still lowers through E4 `str_concat` plus `str_eq`; the equality
path branches with `jmp_if_true`, proving the true-equality branch for a
variable-variable string expression without adding a new string-compare opcode.

## 0.29.0 — 2026-06-27 — BA4 variable-variable string concat in IF inequality

String `IF` now has an explicit unit and matrix proof for a concatenation
expression whose operands are both scalar string variables and whose result
feeds the standard `<>` relop:

```basic
10 LET A$ = "O"
20 LET B$ = "K"
30 IF A$ + B$ <> "NO" THEN 60
40 PRINT "BAD"
50 END
60 PRINT "OK"
70 END
```

The compiler still lowers through E4 `str_concat` plus `str_eq`; the inequality
path branches with `jmp_if_false`, so no new string-compare opcode or runtime
string representation is introduced.

## 0.28.0 — 2026-06-27 — BA4 variable-variable string PRINT concat proof

String `PRINT` now has an explicit unit and matrix proof for concatenating two
scalar string variables directly in the print expression:

```basic
10 LET A$ = "O"
20 LET B$ = "K"
30 PRINT A$ + B$
```

The compiler lowers the expression to E4 `str_concat` with
`__basic_str_A` and `__basic_str_B` operands, then feeds the temporary result
directly to `print_str`. `lang-aot` observes stdout `OK` on every LANG-FULL
backend.

## 0.27.0 — 2026-06-27 — BA4 chained string concatenation proof

String expressions now have an explicit unit and matrix proof for chained
left-associative concatenation with a variable operand:

```basic
10 LET A$ = "A"
20 LET B$ = A$ + "B" + "C"
30 PRINT B$
```

The compiler emits two E4 `str_concat` instructions, storing the final result
directly into `__basic_str_B`; `PRINT B$` observes `ABC` on every LANG-FULL
backend through `lang-aot`.

## 0.26.0 — 2026-06-27 — integer-literal exponentiation (LANG-FULL BA-^)

The compiler now lowers the backend-neutral subset of BASIC `^`: a
nonnegative integer-valued literal exponent from 0 through 64. The frontend
unrolls `base ^ n` to repeated `f64` `mul` instructions, so `6 ^ 2 + 6`
prints `42` on native/LLVM/WASM/JVM/CLR/VM/JIT without adding a runtime math
helper.

General exponentiation remains intentionally unsupported for now: variable,
nested, negative, fractional, and large exponents still report a clean
`Unsupported` error because those need a cross-backend math runtime.

## 0.25.0 — 2026-06-27 — BA4 comma-separated string PRINT sequencing

String `PRINT` now has an explicit proof that BA2 comma separators compose with
BA4/E4 string items:

```basic
10 LET A$ = "O"
20 LET B$ = "K"
30 PRINT A$, B$
```

The frontend emits `print_str(A$)`, then the existing comma separator
`putchar(' ')`, then `print_str(B$)`. The all-backend matrix observes `O K`,
proving string items do not detour through numeric formatting helpers.

## 0.24.0 — 2026-06-27 — BA4 multi-item string PRINT sequencing

String `PRINT` now has an explicit proof for multiple scalar string items in
one statement:

```basic
10 LET A$ = "O"
20 LET B$ = "K"
30 PRINT A$; B$
```

The frontend emits two ordered `print_str` calls and does not route the
string-only statement through numeric print helpers. The `;` separator keeps the
output adjacent, so the all-backend matrix observes `OK`.

## 0.23.0 — 2026-06-27 — BA4 copied string slots in IF equality

String `IF` comparisons now have an explicit proof for comparing two scalar
string slots after a copy:

```basic
10 LET A$ = "OK"
20 LET B$ = A$
30 IF B$ = A$ THEN 60
40 PRINT "BAD"
50 END
60 PRINT "OK"
```

The relation path reuses E4 `str_eq` over `__basic_str_B` and `__basic_str_A`,
then branches through the existing `jmp_if_true` line-control lowering.

## 0.22.0 — 2026-06-27 — BA4 variable-backed string concat assignment

String assignment now has an explicit proof for concat expressions whose left
operand is a scalar string variable:

```basic
10 LET A$ = "O"
20 LET B$ = A$ + "K"
30 PRINT B$
```

The compiler stores the `str_concat` result directly into `__basic_str_B`,
proving variable-backed expression assignment without widening into arrays,
input, or a general mutable string store.

## 0.21.0 — 2026-06-27 — BA4 string concat expressions in IF equality

`IF` string comparisons now have an explicit regression proof for consuming a
string expression result before `str_eq`:

```basic
10 LET A$ = "O"
20 IF A$ + "K" = "OK" THEN 50
30 PRINT "BAD"
40 END
50 PRINT "OK"
```

The existing BA4 relation path lowers `A$ + "K"` through E4 `str_concat`, feeds
that temporary into `str_eq`, and branches with the existing line-control
machinery.

## 0.20.0 — 2026-06-27 — BA4 string concat expressions in PRINT

`PRINT` now has an explicit regression proof for consuming a string expression
result directly:

```basic
10 LET A$ = "O"
20 PRINT A$ + "K"
30 END
```

The existing BA4 helper lowers the expression through E4 `str_concat` into a
temporary string slot, and `PRINT` consumes that slot with `print_str`. This
extends the proof surface beyond assignment-target concat without adding new
runtime string machinery.

## 0.19.0 — 2026-06-27 — BA4 literal-backed scalar string copy

Scalar string assignment now accepts a string variable RHS:

```basic
10 LET A$ = "OK"
20 LET B$ = A$
30 PRINT B$
```

The compiler lowers `B$ = A$` as E4 `str_concat B, A, ""`, materializing the
empty suffix as `str_const`. This proves immutable string-copy semantics over
the existing E4 opcode set without adding a dedicated copy opcode or claiming
general dynamic byte-string storage, string arrays, or string `INPUT`.

## 0.18.0 — 2026-06-27 — BA4 literal string concatenation

BASIC string assignment now accepts literal-backed `+` concatenation:

```basic
10 LET A$ = "O" + "K"
20 PRINT A$
```

The compiler lowers each literal to E4 `str_const`, emits `str_concat` directly
into the safe backend-facing `__basic_str_A` slot, and `PRINT A$` consumes that
slot through `print_str`. This proves the first BASIC string-expression shape
without claiming string-to-string copies or general dynamic byte-string storage.

## 0.17.0 — 2026-06-27 — BA4 string inequality control flow

BASIC string `IF` now supports the standard `<>` relop in the literal-backed
BA4 subset:

```basic
10 LET A$ = "N"
20 IF A$ <> "Y" THEN 40
30 PRINT "BAD"
40 PRINT "OK"
```

The compiler still lowers the comparison to the shared E4 `str_eq` op, then
uses `jmp_if_false` for the `THEN` target. This proves string inequality without
adding a new string-compare opcode or widening into dynamic string storage.

## 0.16.0 — 2026-06-27 — BA4 literal string reassignment

The BA4 scalar string slice now proves that the latest literal assignment wins:

```basic
10 LET A$ = "NO"
20 LET A$ = "OK"
30 PRINT A$
```

The compiler rematerialises each literal into the same safe backend-facing
`__basic_str_A` slot with E4 `str_const`, and `PRINT A$` consumes that slot with
`print_str`. This closes literal reassignment without claiming string-to-string
copies, string arrays, string `INPUT`, or dynamic byte-string storage.

## 0.15.0 — 2026-06-27 — BA4 scalar string variables

The first Dartmouth BASIC string-variable slice now lowers through E4:

- `LET A$ = "literal"` materialises the literal directly into a safe
  backend-facing string slot (`__basic_str_A`) instead of exposing `$` in IIR
  register names.
- `PRINT A$` emits `print_str` over that slot, reusing the all-backend E4 string
  output path.
- `IF A$ = "literal" THEN n` lowers to `str_eq` feeding the existing BASIC branch
  machinery. Richer string expressions, string arrays, and string `INPUT` remain
  follow-ups.

## 0.14.0 — 2026-06-27 — BA4 string literal `PRINT` on VM/JIT

The first E4 source-language proof is live:

- `PRINT "literal"` now lowers to shared string ops: `str_const` materialises
  the literal and `print_str` writes it without an implicit newline.
- The existing BASIC `PRINT` separator/newline model is preserved: `;` joins
  tightly, `,` emits a space before the next item, and the final newline still
  comes from `putchar(10)`.
- The VM/JIT capture harnesses register a `print_str` sink, and the lang matrix
  now includes a VM/JIT-only `PRINT "HELLO"` proof. Managed/static backend
  string lowering remains the next E4 work.

## 0.13.0 — 2026-06-27 — BA7 historical real formatting

BA7 real `PRINT` now implements the historical formatter tail instead of the
three-fractional-digit foothold:

- Real output rounds half-up to six significant digits and trims trailing
  fractional zeroes (`1.234567` => `1.23457`, `.250000` => `.25`).
- Magnitudes outside the fixed-decimal window use signed, at-least-two-digit
  `E` notation (`123456789` => `1.23457E+08`, `0.0001234567` =>
  `1.23457E-04`).
- The formatter is split into small synthetic helpers for zero padding,
  rounded fixed-decimal output, and exponent notation so direct native AArch64
  stays under its frame-size limit.
- Verified by frontend unit tests, a native `lang-aot --lang basic` smoke, and
  the all-backend `lang_matrix` BA7-2b cell on native / LLVM / WASM / JVM / CLR
  / VM / JIT.

## 0.12.0 — 2026-06-27 — BA7 real `DATA` and arrays

BA7 now carries Dartmouth BASIC's `f64` value model through aggregate storage,
not just scalars and `PRINT`.

- `DIM A(n)` allocates `array<f64>` elements. Subscripts remain the integer
  structural boundary: index expressions lower as real values, then use E8
  `real_to_int_trunc` before `array_get`/`array_set`.
- `LET A(i) = expr` stores an `f64` element, and `A(i)` in an expression returns
  an `f64` value.
- `DATA` literals are gathered as finite `f64` values and materialised once at
  the top of `main` as an `array<f64>` pool. `READ` fetches `f64` values into
  either scalar variables or array elements; the read pointer remains `i64`, and
  `RESTORE` still rewinds it to zero.
- The native direct backends now accept 8-byte `array<f64>` elements, matching
  the already-real-aware VM/JIT, LLVM, WASM, JVM, and CLR paths.
- Verified by frontend unit tests and the `lang-aot` matrix on native / LLVM /
  WASM / JVM / CLR / VM / JIT: `DATA 3.14, 0.25; READ A(0); READ B; PRINT A(0);
  PRINT B` => `3.14` and `.25`.

## 0.11.0 — 2026-06-27 — BA7 scalar real arithmetic + fixed-decimal `PRINT`

BA7 moves Dartmouth BASIC's scalar numeric model onto `f64` while keeping the
remaining integer boundaries explicit:

- Numeric literals, including integer-spelled literals like `42`, now lower as
  `Operand::Float` with an `f64` type hint instead of being silently truncated to
  or preserved as `i64`.
- Expression lowering carries `f64` through scalar arithmetic, `LET`, `IF`,
  `FOR`/`NEXT`, `DEF FN`, and `PRINT`. Scalar variables default to real slots, so
  a backend never sees the same BASIC scalar flip between integer and real
  storage.
- Integer-only boundaries in this scalar slice remained explicit: line numbers,
  `DIM` bounds, the then-`i64` DATA pool, array subscripts/elements, and GOSUB
  return stacks still used `i64`. Array subscripts and integer array elements used E8
  `real_to_int_trunc` when fed a scalar real expression; 0.12.0 moves `DATA` and
  array element storage to `f64`.
- `READ` and `INPUT` still consume integer sources today, then widen into scalar
  `f64` variables with E8 `int_to_real`.
- `PRINT` chooses a new synthetic `__basic_print_real(x: f64)` helper for numeric
  items. This BA7 helper implements whole-valued output by truncating with E8
  `real_to_int_trunc` and delegating to the BA2 digit printer; BA7-2a adds the
  fixed-decimal path for ordinary fractional values, including no-leading-zero
  magnitudes below 1 and negative fractional values. That helper intentionally
  kept a small fractional digit budget to stay within the direct AArch64
  backend's frame limit; full six-significant-digit rounding and `E` notation
  land in 0.13.0.
- Verified by frontend unit tests, backend validator/encoder smokes, and executed
  `lang-aot` matrix programs: `PRINT 42` and `PRINT 6.0 * 7.0` => `42`, plus
  `PRINT 3.14`, `PRINT 1.0 / 4.0`, and `PRINT 0.0 - 2.5` => `3.14`, `.25`, and
  `-2.5` on native / LLVM / WASM / JVM / CLR / VM / JIT.

## 0.10.0 — 2026-06-26 — `GOSUB` / `RETURN` (LANG-FULL BA1, enabler E7)

`GOSUB` and `RETURN` were `UnsupportedStatement` rejections; they now lower onto
the **E5 array** substrate + an **AL5 computed-`goto`** — no new IIR op, exactly
as designed in `code/specs/lang-full-e7-subroutine-return-stack.md`.

BASIC's `GOSUB`/`RETURN` is *unstructured*: the program is one flat list of
line-numbered statements in `main`, and the same `RETURN` resumes at the
**dynamically most-recent** `GOSUB`. Plain `call`/`ret` can't express that, so we
model it *inside* `main`:

- A pre-pass counts every `GOSUB` (so a `RETURN` appearing before some of the
  `GOSUB`s it returns to still emits the full dispatch chain) and, when the
  program uses `GOSUB`, materialises a return-address stack at the top of `main`:
  a fixed-capacity (64) `array<i64>` (`__basic_gosub_stack`) plus the
  `__basic_gosub_sp` pointer — mirroring the BA6 `DATA`-pool init.
- **`GOSUB n`** pushes its 0-based call-site id (`array_set` + sp bump), `jmp`s to
  `line_n`, and drops a `gosub_ret_<id>` resume label.
- **`RETURN`** pops the id (`array_get` after sp decrement) and computed-`goto`s
  to its `gosub_ret_<id>` via the AL5 switch chain (`cmp_eq` + `jmp_if_true` over
  every site). A bare `RETURN` (no `GOSUB` in the program) is a clean error;
  over-deep nesting traps via the bounds-checked `array_set` (the faithful
  "GOSUB too deep" runtime error).
- Every op (`alloc_array`/`array_set`/`array_get`, `const`/`add`/`sub`/`mov`,
  `cmp_eq`/`jmp`/`jmp_if_true`/`label`) already runs on every backend, so **BA1
  added ZERO backend ops** — proven by two executed `lang-aot` matrix programs:
  `GOSUB 100` twice with one shared `RETURN` ⇒ `919` (same `RETURN`, two sites),
  and a nested `GOSUB` ⇒ `876` (LIFO across depth > 1). Six new frontend unit
  tests.
- *Known gap (BA1-WASM):* the `RETURN` computed-`goto` produces an irreducible
  CFG that trips `iir-to-wasm`'s dispatch-loop lowering with a runtime
  `StackUnderflow` (the wasm *compiles* but traps), so the GOSUB matrix cells run
  on the other **six** backends (native/LLVM/JVM/CLR/VM/JIT) pending a focused
  iir-to-wasm fix.

## 0.9.0 — 2026-06-26 — multi-item `PRINT` on one line (LANG-FULL BA2)

`PRINT` could only print a single value per statement, because each item lowered
to `call_builtin "print_i64"` and `print_i64` appends a newline — so `PRINT A; B`
wrongly split `A` and `B` onto separate lines. BA2 moves BASIC to a
**character-level output model** that lets several items share a line:

- **Two synthetic helper functions** are appended to the module (only when a
  program actually prints a value): `__basic_print_uint(n)` renders an
  unsigned magnitude by **recursing** on `n / 10` (high-order digits first, so
  digits come out left-to-right with no reversal buffer) and emitting each digit
  with the universal `putchar` builtin; `__basic_print_int(n)` handles the sign
  (`putchar('-')` then the magnitude) and dispatches to it. Both use **only**
  ops every backend already runs — `const`, `cmp_*`, `div`/`mul`/`sub`/`add`,
  `call` (the ALGOL value-procedure ABI), `jmp`/`label`, and `putchar` (shared
  with Brainfuck) — so BA2 required **zero backend changes** and runs on all
  seven targets (NativeAot/LLVM/WASM/JVM/CLR/VM/JIT).
- **`emit_print`** now walks the `print_list` in source order, lowering each
  numeric item to a `call __basic_print_int`, applying separator spacing, and
  emitting a trailing `putchar(10)` newline unless the list ends on a separator.
- **Separators:** `;` joins items tightly (nothing between); `,` inserts a
  single space. A **trailing** separator (`PRINT X;` / `PRINT X,`) suppresses
  the final newline. Bare `PRINT` now emits a blank line (a lone newline)
  instead of being a no-op.
  - *Divergence (documented):* historical Dartmouth BASIC tabs `,` to the next
    14-column **print zone**; that needs a run-time output-column counter and is
    deferred to a later item. A single space is BA2's well-defined approximation.
  - *Limitation:* the sign path negates with `0 - n`, which overflows only at
    `i64::MIN` — a value no BA2 program can express; a saturating negate is a
    later refinement.
- **At this point in the history, string `PRINT` items still errored**; BA4 /
  enabler E4 removes that limitation in 0.14.0. **More relops** were already in
  place (the grammar and `extract_relop_op` cover all six: `= < > <= >= <>`);
  BA2 adds no relop work.
- Proven by two executed `lang-aot` matrix cells (`PRINT 0 - 12; 34` ⇒ `-1234`,
  `PRINT 5, 6` ⇒ `5 6`) that run on all 7 backends, plus six new frontend unit
  tests. The existing single-item BASIC matrix cells now route through `putchar`
  too and still pass unchanged.

## 0.8.0 — 2026-06-22 — `READ` / `DATA` / `RESTORE` (LANG-FULL BA6)

`READ`, `DATA`, and `RESTORE` were `UnsupportedStatement` rejections; they now
lower onto the **E5 array** substrate (no new IIR op, no enabler needed):

- A pre-pass gathers every `DATA` numeric literal across the whole program in
  line-number order into a pool. The pool is materialised **once at the top of
  `main`** as an `array<i64>` (`alloc_array` + one `array_set` per value) plus
  an `__basic_data_ptr` register seeded to 0. Because the program is a single
  `main` function (no `GOSUB`), the pointer lives in a register and persists
  across `READ`s — no module global needed.
- `READ var {, var}` fetches `array_get pool, __basic_data_ptr` into each target
  (a scalar `mov`, or an `array_set` for `READ A(I)`) and advances the pointer
  (`__basic_data_ptr := __basic_data_ptr + 1`). Reading past the pool traps via
  the bounds-checked `array_get` — the "out of DATA" runtime error.
- `RESTORE` rewinds the pointer to 0; `DATA` itself emits nothing at its line.
- **Limitations:** integer `DATA` only (a real/`f64` value is a clean
  `Unsupported` error — a follow-up, like integer-only `DIM` arrays); `READ`/
  `RESTORE` with no `DATA` in the program is a clean error.

Verified by **running**: a `lang_matrix.rs` cell (`DATA 21 / READ A / RESTORE /
READ B / PRINT A+B` ⇒ `42`, proving sequential consumption *and* the rewind)
runs on **all 7 backends**; plus JIT-run tests (`READ X` ⇒ 42; `READ A,B` +
`RESTORE` + `READ C` ⇒ 10,20,10) and four structural unit tests.

## 0.7.0 — 2026-06-21 — `DIM` arrays (LANG-FULL BA3, enabler E5)

### Added — one-dimensional integer arrays (`DIM A(n)` + subscripted `A(i)`)

`DIM` was previously an `UnsupportedStatement` and a subscripted variable `A(I)`
was a deferred `Unsupported` error. BASIC arrays now lower to the shared IIR
array ops (the *same* `alloc_array` / `array_set` / `array_get` ALGOL's E5 arrays
use), so they run on every backend E5 already supports:

```basic
10 DIM A(3)
20 LET A(1) = 40
30 LET A(2) = 2
40 PRINT A(1) + A(2)
50 END
```

⇒ `alloc_array A = 4` (see below) then `array_set A, 1, 40` / `array_set A, 2, 2`
and two `array_get`s feeding the `add`, printing `42`.

- **`DIM A(n)` → `alloc_array`** — Dartmouth BASIC arrays are **0-based and
  inclusive**: `DIM A(3)` declares `A(0)..A(3)`, so the element count is
  `n + 1` (here `4`). `DIM A(3), B(2)` declares both in one statement. The
  handle lives in the register named after the array.
- **`LET A(i) = e` → `array_set`** and **`A(i)` in an expression → `array_get`**.
  The subscript is used **directly** as the 0-based IIR index — no lower-bound
  subtraction (contrast ALGOL's `array A[lo:hi]`, which subtracts `lo`).
- **Undeclared use is a clean error**: subscripting a name that was never
  `DIM`med returns `Unsupported` rather than miscompiling against an undefined
  handle register.
- **Bounded + panic-free**: a `DIM` bound is validated against `MAX_DIM_BOUND`
  (2^24) *before* the `f64`→`i64` cast, so an oversized or non-finite literal
  (e.g. `DIM A(1E30)`, which a bare `as i64` cast would saturate to `i64::MAX`
  and then overflow on the `+ 1` element count) is a clean `Unsupported` error,
  never a debug-build panic or a release-build wrapped/garbage length. The
  element-count `+ 1` also uses `checked_add` as a second guard.
- **Verified by RUNNING** the straight-line array program across the matrix
  (`lang-aot` `tests/lang_matrix.rs`): the managed runtimes bounds-check natively
  (JVM `int[]`/`iastore`/`iaload`, CLR `int32[]`/`stelem`/`ldelem`), the static
  backends use the length-prefixed block + explicit bounds-trap (LLVM/WASM/
  NativeAot). 7 new unit tests cover the lowering shape + the error paths.

## 0.6.0 — 2026-06-20 — `DEF FN` user-defined functions (LANG-FULL BA5)

### Added — single-line user-defined functions (`DEF FNx(P) = expr`)

`DEF` was previously an `UnsupportedStatement`. A single-line BASIC function
now lowers to a **sibling `IIRFunction`** and `FNx(arg)` call sites lower to the
shared IIR `call` op — the same calling convention ALGOL's value procedures
(AL3) already run on every backend:

```basic
10 DEF FNS(X) = X * X
20 PRINT FNS(7)
30 END
```

⇒ `fn FNS(X: i64) -> i64 { ret X * X }` plus `call FNS, 7` in `main`, printing
`49`. **Verified by RUNNING** across native / LLVM / WASM / JVM / CLR / VM / JIT
(`lang-aot` `tests/lang_matrix.rs`).

Mechanics, mirroring `algol-iir-compiler`'s `compile_procedure`:

- A **pre-pass** registers every `DEF FNx` name before any statement is lowered,
  so a program may *call a function on an earlier line than its `DEF`* (BASIC
  permits forward use).
- Each `DEF` body is lowered in a swapped-in emission context (its own
  instruction stream / temp counter / source map), then assembled into a
  `FullyTyped` `IIRFunction` pushed onto the module **after** `main`.
- A `FNx(arg)` call evaluates its single argument and emits
  `call dest = [callee, arg]` with an `i64` return hint.

**Limits (follow-ups):** one numeric parameter only (per the 1964 grammar); the
body may reference **only its parameter** — global access from inside a function
needs the host global table the code-gen backends reject (enabler **E6**), so any
other variable reference is a clean `Unsupported` error rather than an
undefined-register miscompile. Built-in maths functions (`SIN`/`ABS`/…) stay
deferred until E3 (reals).

A companion fix in `lang-aot` 0.94.0 made the JVM scalar-concretization
**module-consistent** so the printing `main` and its non-printing callee `FNS`
share one value model (see that crate's changelog).

## 0.5.0 — 2026-06-13 — comparisons emit the operand width; control flow runs on the code-gen backends (LANG-FULL BA0)

### Fixed — `IF`/`FOR` comparisons emitted a `bool` type hint, breaking LLVM (and WASM)

`IF e1 relop e2` and `FOR`/`NEXT` emitted their `cmp_*` with `type_hint = "bool"`
(the result type). But a comparison's IIR `type_hint` is the **operand** width,
which the IIR-to-* backends use to size the machine compare: LLVM emitted
`icmp <op> i1` — a 1-bit compare that truncates the i64 operands (`7 > 5` became
`1 > 1` → false), so `IF A > 5` fell through and `FOR` mis-looped. The compiler now
emits the operand type `i64` (matching Nib / Oct / ALGOL); the boolean *result* is
implicit, exactly as those languages already do.

This is why BASIC control flow previously ran only on the VM/JIT. With the fix,
`lang-aot`'s `lang_matrix` battery RUNS a `FOR`/`NEXT` accumulator loop
(`FOR I = 1 TO 5: S = S + I` → prints 15) and an `IF A > 5 THEN 100` jump
(prints 7) across native / LLVM / WASM / CLR / VM / JIT.

### Removed — two stale `#[ignore]`s in `tests/backend_encode.rs`

`basic_control_flow_lowers_to_wasm_bytes` and `basic_for_loop_lowers_to_wasm_bytes`
were ignored on the premise that `iir-to-wasm` couldn't lower `cmp_gt`/`cmp_le`;
that gap has since been closed (the wasm lowering grew the full `cmp_*` table), so
both tests pass and are re-enabled.

### Known follow-up — BA-JVM-1

BASIC programs combining a **branch** (`IF`/`FOR`) with a `print_i64` call do not yet
run on the JVM (output is empty) — the `iir-to-jvm-class-file` StackMapTable
generation trips on the frame at the branch target when several `long` locals are
live across a host-method invoke. (A print with no branch — `10 PRINT 42` — and a
loop with no print — Nib's for-loops — both run on JVM; only the combination fails.)
The JVM cell is excluded for the two control-flow matrix programs pending that fix.

## 0.4.0 — 2026-05-30 (BASIC05 — source-location threading for debugger)

### Added — Real source positions in `IIRFunction.source_map`

BASIC's emitted IIR now carries real `(line, column)` per instruction
in `IIRFunction.source_map`, in lockstep with `instructions`.
Previously the field was either empty or all `SourceLoc::SYNTHETIC`.

This is the prerequisite for line-based breakpoints in the future
`basic-dap` debugger crate.  Without real positions, the debug
sidecar built by the DAP layer cannot resolve `setBreakpoints
{ file, lines: [N] }` requests to IIR instructions.

This mirrors the pattern landed for `oct-iir-compiler` 0.4.0
(OCT05 / PR #4583).  Same `node_loc()` + `Cell<SourceLoc>` +
statement-level `set_loc()` shape — the next step in the
horizontally-sequenced "every language gets every Twig-grade
tool" roadmap.

### Implementation

- New `node_loc(&GrammarASTNode) -> SourceLoc` helper extracts
  `(start_line, start_column)` from an AST node, falling back to
  `SYNTHETIC` when the parser couldn't attach positions.
- `Compiler` gained two fields: `source_map: Vec<SourceLoc>` (the
  per-function accumulator) and `current_loc: Cell<SourceLoc>`
  (the "currently compiling" position).  Manual `impl Default`
  replaces the `#[derive(Default)]` since `Cell<SourceLoc>` doesn't
  have a usable default (well, it does — but being explicit makes
  the SYNTHETIC start state obvious to readers).
- `Compiler::emit` now pushes `current_loc.get()` onto `source_map`
  for every instruction it appends, maintaining the lockstep
  invariant.
- `emit_line` calls `set_loc(node_loc(line))` on entry — all
  instructions emitted for that line (label + body) inherit the
  line's source position.
- `emit_statement` re-tags with the wrapped statement node's own
  position, which may be a tighter range than `emit_line` set.
- `emit_program` sets the initial loc to the program root so the
  synthesised end-of-program epilogue (`const 0; ret`) gets a
  sensible source line rather than `SYNTHETIC`.
- `compile_program` ends with the move-with-defensive-padding shape:
  `main.source_map = std::mem::take(&mut comp.source_map)` after
  ensuring `source_map.len() == instructions.len()`.

### Tests

- 2 new unit tests:
  - `source_map_lockstep_with_instructions`: every function's
    `source_map.len() == instructions.len()`.
  - `source_map_carries_real_line_numbers`: a 4-line BASIC program
    produces entries for every line — proving the per-line source
    positions get threaded through, not just SYNTHETIC.
- All existing lib tests still pass.

## 0.3.0 — 2026-05-29 (PL05-C — AOT backend acceptance proofs)

### Added — `tests/backend_compat.rs` exercises every IIR-to-* backend

BASIC's emitted IIR is now proven by automated tests to be accepted
by the validators of every AOT backend (wasm, jvm, clr, beam).  This
closes the "BASIC's IIR shape could regress without anyone noticing"
gap — the same shape Twig (`twig-ir-compiler/tests/backend_compat.rs`),
Nib (`nib-iir-compiler/tests/backend_compat.rs`), and Oct (PR #4580)
already had.

### Coverage (8 tests)

| Group | Test | Asserts |
|---|---|---|
| Minimal | `basic_minimal_end_accepted_by_every_backend` | `10 END` |
| Minimal | `basic_let_binding_accepted_by_every_backend` | `LET A = 42` |
| Arithmetic | `basic_typed_add_accepted_by_every_backend` | `C = A + B` |
| Arithmetic | `basic_typed_mul_accepted_by_every_backend` | `C = A * B` |
| Control flow | `basic_if_then_goto_accepted_by_every_backend` | `IF A > 5 THEN 100` |
| Control flow | `basic_for_next_loop_accepted_by_every_backend` | `FOR I = 1 TO 3 / NEXT I` |
| Control flow | `basic_goto_accepted_by_every_backend` | `GOTO 100` |
| Invariant | `basic_main_is_fully_typed` | main has `type_status == FullyTyped` |

All 8 pass on first run — BASIC's IIR is shape-compatible with every
backend with zero further changes.  This is the AOT counterpart to
the existing tests/jit_smoke.rs + tests/jit_real_backend.rs (which
prove the JIT path).

### Dependencies

Added `iir-to-wasm`, `iir-to-jvm-class-file`, `iir-to-cil-bytecode`,
`iir-to-beam` as **dev-dependencies**.  None of them ship to runtime
consumers of `dartmouth-basic-iir-compiler`.

### Tests

- 8 new backend_compat tests pass.
- 17 lib + 8 + 6 + 4 existing tests still pass.

## 0.2.0 — 2026-05-26 (PL05-B — real BasicCirJit backend)

### Added — `BasicCirJit`: a real `jit_core::backend::Backend`

Ships a real bytecode JIT for Dartmouth BASIC, modelled on Brainfuck's
`BrainfuckCirJit` pattern.  Translates the specialised CIR instruction
stream (`const_i64`, `add_i64`, `cmp_*_i64`, `jmp`, `jmp_if_false`,
`call_builtin "print_i64"`, `ret_void`) into a packed register-machine
bytecode and interprets it in a tight match-loop — bypassing
`vm-core`'s generic IIR dispatch entirely.

Same "classic JIT" shape used by the JVM Ignition tier, Smalltalk-80,
Lua, and V8 Ignition.  Not a native-code JIT (Cranelift / x86_64) —
swapping in a native backend later is the only change needed.

#### Bytecode opcodes

22 opcodes covering BASIC's full V1 vocabulary:
- Constants: `CONST_I64` (8-byte little-endian payload)
- Arithmetic: `ADD_I64` / `SUB_I64` / `MUL_I64` / `DIV_I64` / `NEG_I64`
- Comparisons: `CMP_EQ_I64` / `CMP_NE_I64` / `CMP_LT_I64` / `CMP_LE_I64`
  / `CMP_GT_I64` / `CMP_GE_I64`
- Control flow: `JMP` / `JMP_IF_FALSE` / `JMP_IF_TRUE` (i16 LE offsets)
- Builtins: `PRINT_I64` / `INPUT_I64`
- Returns: `RET_I64` / `RET_VOID`
- Plus `MOV` for register-to-register copy

Register file: 256 i64 registers, single-byte indices.

#### Shared I/O via Arc<Mutex<…>>

`BasicCirJit::new` takes `Arc<Mutex<Vec<i64>>>` (output),
`Arc<Mutex<VecDeque<i64>>>` (input), `Arc<Mutex<u64>>` (step counter),
and `Arc<Mutex<Option<String>>>` (error slot).  The same Arc handles
can be shared with `VMCore`'s `print_i64` / `input_i64` builtin
registrations, so interpreter-fallback and JIT-compiled paths see the
same logical streams.

#### `main.type_status = FullyTyped` override

BASIC's IIR uses `"void"` type hints on control-flow ops (`label`,
`jmp`, `ret`, `call_builtin "print_i64"`).  `"void"` is **not** in
`interpreter_ir::opcodes::CONCRETE_TYPES`, so `IIRFunction::new`'s
automatic `infer_type_status` returns `PartiallyTyped`.  Without an
explicit override, `jit-core`'s threshold-zero compile path (which
requires `FullyTyped`) would never fire.

The fix mirrors Brainfuck's compiler: after `IIRFunction::new`, set
`main.type_status = FunctionTypeStatus::FullyTyped`.  Every BASIC
instruction is in fact statically known (no `"any"` hints anywhere),
so the override is semantically correct.

#### Tests

- 5 unit tests in `jit_backend::tests` cover compile + run paths for
  CONST_I64 / RET_I64, PRINT_I64, ADD_I64, unknown-opcode rejection,
  and division-by-zero error reporting.
- 6 end-to-end integration tests in `tests/jit_real_backend.rs` run
  full BASIC programs through `JITCore` with `BasicCirJit` as the
  backend (instead of `NullBackend`).  Covers PRINT, LET +
  arithmetic, FOR loops, IF/GOTO branches, multiplication, and
  accumulating FOR with arithmetic in the body.

### Changed

- `jit-core` and `vm-core` promoted from dev-dependencies to main
  dependencies — `BasicCirJit` lives in this crate's `src/`, not its
  `tests/`.
- `jit_backend` module re-exported from `lib.rs`; `BasicCirJit`,
  `DEFAULT_OUTPUT_CAP`, and `DEFAULT_STEP_CAP` are part of the public
  API.

## 0.1.0 — 2026-05-20 (PL05 initial release)

Initial release.  Compiles Dartmouth BASIC source to
`interpreter_ir::IIRModule`, unlocking the LANG VM AOT chain
(twig-aot / lang-aot → x86_64-backend / aarch64-backend → object →
system linker → native executable) for BASIC programs.

Distinct from the existing `dartmouth-basic-ir-compiler` crate, which
targets the GE-225 simulator's custom `compiler_ir::IrProgram` shape
and is not pluggable into the LANG VM chain.

### V1 coverage (integer programs)

| Statement | Status |
|-----------|--------|
| `LET A = expr` | ✓ |
| `PRINT expr`   | ✓ (numeric in the initial release; string literal `PRINT` added in 0.14.0 for VM/JIT) |
| `INPUT X`      | ✓ |
| `IF cond THEN m` | ✓ |
| `GOTO m`       | ✓ |
| `FOR I = a TO b STEP s` / `NEXT I` | ✓ (positive STEP) |
| `END` / `STOP` | ✓ |
| `REM …`        | ✓ (no-op) |
| `GOSUB` / `RETURN` | **deferred** — V1 errors with `UnsupportedStatement` |
| `READ` / `DATA` / `RESTORE` | deferred — needs data pool |
| `DIM` / arrays | deferred — needs LANG76-based byte arrays |
| `DEF`          | deferred |

### Expression coverage

- Integer literals (floats truncate to i64; explicit float support
  deferred until backends grow SSE2).
- Variables (scalar `A..Z`, `A0..Z9` — array access `A(I)` deferred).
- Arithmetic: `+`, `-`, `*`, `/` with standard precedence.
- Unary minus.
- Exponentiation (`^`): deferred — needs a runtime helper.
- Built-in / user-defined functions (`SIN`, `FNA`, …): deferred.

### IIR shape

The whole program becomes a single function `main` returning `i64`.
Every BASIC line gets a label `line_<n>`; flow-control statements
jump between those labels.  FOR/NEXT loops use per-loop synthetic
labels `for_<id>_test` / `for_<id>_end`.

### Tests

11 unit tests cover each supported statement plus the deferred
`UnsupportedStatement` paths.  End-to-end smoke tests in
`lang-aot/tests/end_to_end_smoke.rs` compile BASIC programs all the
way to native executables on Windows + Linux and assert stdout:

- `10 PRINT 42 / 20 END` → stdout `"42\n"`.
- `10 FOR I = 1 TO 3 / 20 PRINT I / 30 NEXT I / 40 END` → stdout
  `"1\n2\n3\n"`.

Spec: `code/specs/PL05-dartmouth-basic-iir-compiler.md`.
