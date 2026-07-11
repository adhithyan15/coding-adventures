# Changelog — iir-to-cil-bytecode

## 0.40.1 - 2026-07-11 (DVAL01-2: rename IIR builtin names lispy_* -> dyn_*)

DVAL01-2: the CIL structural lowering's references to the `lispy_cons` IIR name are renamed to `dyn_cons` (heap builtins are handled structurally; the name only appears in a comment + a lowering test). Pure rename.

## [0.40.0] — 2026-07-11 (LANG-FULL E6d-2a: i64-width `box`/`unbox`)

`box`/`unbox` width-adapt for E6d-2 dynamic arithmetic (i64): `box` of an `int64` local emits `conv.i4` before `box System.Int32`; `unbox` into an `int64` local emits `conv.i8` after `unbox.any System.Int32`. int32-slot lisp box/unbox unchanged.

## [0.39.0] — 2026-07-10 (LANG-FULL E4-dyn — E4d-BA-arr: `System.String[]` reference arrays)

BASIC string arrays (`DIM A$(n)`) lower to a CIL `System.String[]` in the textual
`il_text` emitter (the real-CoreCLR / `ilasm` matrix path).

- **`cil_array_elem`** gains a `"string"` arm: `("string[]", "ldelem.ref",
  "stelem.ref", "[System.Runtime]System.String")` — the reference element ops, the
  same `ldelem.ref`/`stelem.ref` pair the McCarthy `object[]` cons cells already use.
- `alloc_array`/`array_get`/`array_set` consume that tuple generically, so
  `newarr System.String` + `stelem.ref`/`ldelem.ref` fall out with no other change.
- A str value is a real `System.String` reference (a `str` local is `string`), so —
  unlike the static backends — no handle materialisation is needed.

Tests: `string_array_emits_ref_ops`, plus a `str` case in `array_element_opcode_table`.

## [0.38.0] — 2026-07-06 (LANG-FULL E4-dyn: BASIC string `INPUT A$`)

BASIC's string `INPUT A$` (E4-dyn) now lowers on the CLR: the whole stdin line is
read **as the string value itself**, not parsed as a number.

- **Textual `il_text` emitter**: new `input_str` `call_builtin` arm — `call string
  [System.Console]System.Console::ReadLine()`, then store into the `str`-typed
  dest. Unlike numeric `input_i64` there is **no** `Int32.Parse`: `ReadLine`
  already returns a `System.String`, which is exactly `cil_local_type("str")`.
  Like `input_i64` this assumes input is present (the V1 permissive contract).
- **Test**: `input_str_reads_line_via_console_readline` asserts the emitted `.il`
  calls `Console.ReadLine()` and never `Int32.Parse`.

Proven in `lang-aot`'s `lang_matrix` (`10 INPUT A$ / 20 PRINT A$ / 30 END`, stdin
`"OK"` → `OK`) on the CLR column via real `ilasm`/`dotnet` in CI.

## [0.37.0] — 2026-06-30 (BA-INPUT: `input_i64` → `Console.ReadLine` + `Int32.Parse`)

Added `"input_i64"` arm to the `call_builtin` handler in `il_text.rs`:

```
call string [System.Console]System.Console::ReadLine()
call int32 [System.Runtime]System.Int32::Parse(string)
store_var <dest>
```

The CLR backend's scalar concretization narrows BASIC integers to `int32`, so
`Int32.Parse` matches the `print_i64` overload (`Console.WriteLine(int32)`) and
needs no widening. Returns `0` on EOF or parse failure via CLR exception propagation
(same permissive-V1 contract as the other backends). Enables `10 INPUT X\n20 PRINT X`
to run end-to-end on the CLR backend in `matrix_every_proven_cell_agrees`.

## [0.36.0] — 2026-06-30 (LANG-FULL — CIL text backend: `Operand::Bool` and `neg`)

Two missing cases in `il_text.rs` that caused CLR failures for ALGOL boolean
programs on the text-based CIL backend:

**`Operand::Bool` in `const` handler** (`il_text.rs`):
The `const` IIR op's source-operand match arm only handled `Operand::Int(n)`.
ALGOL emits `Operand::Bool(true)` / `Operand::Bool(false)` for boolean
declarations (`boolean b; b := true`).  The text backend returned an error for
these; the binary backend (`lower.rs` line 695) already handled them.

Fix: added `Some(Operand::Bool(b)) => if *b { 1 } else { 0 }` case to the
match arm, giving booleans the same treatment as in the binary backend.

**`"neg"` arithmetic-negation op** (`il_text.rs`):
The `neg` IIR op (emitted by BASIC `ABS(n)` as `neg` + select-positive) was
only handled by the binary backend (`lower.rs`).  The text backend had no
arm for it, causing a "unknown op" error.

Fix: added `"neg"` arm after the `"not"` arm that loads the source variable,
emits CIL `neg`, applies `emit_narrow_width_mask`, and stores the result.

## [0.35.0] — 2026-06-29 (LANG-FULL BA-pow — `f64_pow` CLR lowering)

Added `"f64_pow"` arm in `il_text`: loads base and exponent via `load_var`,
emits `call float64 [System.Runtime]System.Math::Pow(float64, float64)`, and
stores the result via `store_var`.  Two-argument static call, matching the
existing unary Math calls (Sqrt, Sin, etc.) but with a second argument.
## [0.34.0] — 2026-06-29 — `f64_atan/f64_tan` via `System.Math` (LANG-FULL AL8-arctan)

Extended the f64 transcendental match arm to cover two more ops:
- `f64_atan` → `System.Math::Atan` (inverse tangent)
- `f64_tan`  → `System.Math::Tan`  (tangent)

Both emit `load_var src; call float64 [System.Runtime]System.Math::<Method>(float64); store_var dest`.
CoreCLR JIT intrinsifies both to native libm calls.

## [0.33.0] — 2026-06-28 — `f64_sin/cos/ln/exp` via `System.Math` (LANG-FULL AL8-trig)

Extended the `f64_sqrt` arm to cover all five f64 ops: `f64_sqrt` → `Sqrt`,
`f64_sin` → `Sin`, `f64_cos` → `Cos`, `f64_ln` → `Log` (natural log),
`f64_exp` → `Exp`.  All emit:
`load_var src; call float64 [System.Runtime]System.Math::<Method>(float64); store_var dest`.

## [0.32.0] — 2026-06-28 — `f64_sqrt` lowers to `System.Math::Sqrt` (LANG-FULL AL8-sqrt)

The textual `.il` emitter now handles `f64_sqrt`: `load_var src; call float64
[System.Runtime]System.Math::Sqrt(float64); store_var dest`.  The .NET JIT
intrinsifies `Math.Sqrt` to a hardware `sqrtsd`/`fsqrt` with no P/Invoke cost.

## [0.31.0] — 2026-06-28 — textual CLR literal string comparison (LANG-FULL E4)

The textual `.il` path now lowers `str_cmp` over managed string locals by
calling `System.String::CompareOrdinal(string,string)` and
`System.Math::Sign(int32)`.

## [0.30.0] — 2026-06-28 — textual CLR literal string slice (LANG-FULL E4)

The textual `.il` path now lowers `str_slice` by loading the source string,
start, and end, computing `end - start`, and calling
`System.String::Substring(int32, int32)`. The result is stored as a managed
string local that can feed the existing `str_index`, `str_len`, and `str_eq`
paths.

## [0.29.0] — 2026-06-27 — textual CLR literal string index (LANG-FULL E4)

The textual `.il` path now lowers direct-literal `str_index`:

- `str_index` loads the `string` local and integer index, then calls
  `System.String::get_Chars(int32)`.
- Twig `(string-ref "ABC" 1)` now returns `66` on real CoreCLR alongside the
  existing literal string length/equality/concat rows.
- Non-literal string values and byte-exact non-ASCII semantics remain follow-up
  representation work.

## [0.28.0] — 2026-06-27 — textual CLR literal string metadata (LANG-FULL E4)

The textual `.il` path now lowers `str_len`, `str_eq`, and `str_concat` for
direct string literals:

- `str_len` loads the `string` local and calls
  `System.String::get_Length()`, storing the resulting integer.
- `str_eq` loads two `string` locals and calls
  `System.String::Equals(string, string)`, storing the resulting integer.
- `str_concat` loads two `string` locals and calls
  `System.String::Concat(string, string)`, storing the resulting string.
- This proves Twig `(string-length "HELLO")` and
  `(string=? "HELLO" "HELLO")` plus
  `(string-length (string-append "AB" "CDE"))` on real CoreCLR alongside the
  earlier BASIC `PRINT "HELLO"` literal-output row.
- Richer byte-oriented string operations (`str_index`) and non-literal string
  values still fail closed until the CLR representation owns shared UTF-8 byte
  semantics.
- Emitter and validator tests cover the new accepted shape.

## [0.27.0] — 2026-06-27 — textual CLR string literal PRINT foothold (LANG-FULL E4 / BA4)

The real-CoreCLR textual `.il` path now lowers the first E4 string shape:

- `str_const` with an ASCII `Operand::Str` literal → `ldstr "..."` into a
  `string` local.
- `print_str` → `call void [System.Console]System.Console::Write(string)`.
- The generated launcher treats `print_str` as side-effecting output, so it
  discards the entry result instead of double-printing `0`.

This is intentionally narrower than full E4: the byte-oriented string algebra
(`str_len`, `str_index`, `str_concat`, `str_eq`) remains rejected until the CLR
representation owns the shared UTF-8 byte semantics. The validator now documents
that contract: `str_const` + `print_str` pass, richer string ops fail closed.

Verified with emitter/validator tests plus the `lang-aot` BASIC matrix row:
`10 PRINT "HELLO"` runs on real `ilasm` + `dotnet` in the CLR column.

## [0.26.0] — 2026-06-23 — numeric conversions int ⇄ real (LANG-FULL E8 backend 5)

The textual `il_text` path (the cross-backend matrix's `ilasm` route) now lowers
the three IIR numeric-conversion ops — the fifth backend (after VM/JIT, LLVM,
WASM, JVM) to gain them and the prerequisite for ALGOL's `entier` and integer↔real
coercion:

| IIR op | CIL lowering |
|--------|--------------|
| `int_to_real` | `conv.r8` (or `conv.r4` for `f32`) — widen int→float, exact for any int width |
| `real_to_int_trunc` | `conv.ovf.i4` — truncate toward zero |
| `real_to_int_floor` | `call float64 [System.Runtime]System.Math::Floor(float64)` then `conv.ovf.i4` — round toward −∞ |

**The CLR matches the VM's fail-closed trap contract — for free.** The
overflow-checking `conv.ovf.i4` truncates toward zero *and* throws
`OverflowException` on NaN / ±∞ / out-of-`int32`-range, which is exactly the
VM/LLVM/WASM trap semantics (spec §7's recommendation) in a single opcode with no
exception-table plumbing. This is strictly better than the JVM backend, whose
`d2i`/`d2l` saturate and required a documented divergence. `conv.r8` needs no
overflow check — widening an integer to a double is always exact.

This backend's scalar integer model is uniformly **32-bit** (`cil_local_type`
collapses `i64`/`i32`/… → `int32`, as the existing scalar-`i64` and E5 `int32[]`
paths already do), so `real_to_int_*` always narrows to `conv.ovf.i4`.

Tests: emit-level coverage of `conv.r8` / `conv.ovf.i4` over both the `i64` and
`i32` IR widths plus the `Math::Floor` methodref, and
`e8_conversions_round_trip_runs_on_real_clr` — an end-to-end run on real
`ilasm` + `dotnet` of `floor(int_to_real(45) − 2.7) ⇒ 42`, matching the
LLVM/WASM/VM/JVM matrix-cell value.

## [0.25.0] — 2026-06-22 — void functions & void calls in the textual emitter (LANG-FULL O3)

The textual `il_text` path (the one the cross-backend matrix assembles with `ilasm`)
gained support for **void functions** — a latent gap surfaced by Oct's O3 `static`-global
proof, whose `bump()` is the first void *user* function to reach the CLR column (every
prior matrix program returned a value, and `main`'s `ret_void` is rewritten to `ret i32`).

- **`ret_void`** now lowers to a bare `ret` (was an `UnsupportedOp` rejection).
- **`cil_ret_type`** maps a `void` return type to the CIL `void` signature (it used to
  fall through `cil_local_type` to `int32` — wrong for a value-less method).
- **`call` to a void method** (IIR `dest == None`) emits `call void …` and performs **no**
  trailing `store` (previously the arm hard-required a `dest` and always stored a result,
  so a dest-less void call panicked). Value-returning calls are unchanged.
- Proven by **running**: the Oct `static counter` program (`bump()` mutates a shared
  global twice) assembles with `ilasm` and runs under `dotnet` → `42`, alongside the other
  six backends in `lang_matrix.rs`. Unit test: `void_function_and_void_call_lower`.

## [0.24.0] — 2026-06-22 — typed module globals → static fields (LANG-FULL E6 layer 1)

`global_load` / `global_store` were a `LANG32b`-deferred `UnsupportedOp`
rejection. They now lower (in the textual `il_text` path the matrix assembles) to
CLR **static-field** access, so a function can read/write a module-level global.

### Added
- **`global_load` / `global_store`** lowering in `il_text`:
  - `collect_global_fields` collects every distinct global name (first-seen
    order) → a `public static int64 G_N` field of the generated class. Field
    names are index-based (`G_0`, `G_1`, …) so an arbitrary source identifier can
    never form an invalid or colliding CIL field name. CLR zero-initialises
    static fields (the never-written-global-reads-0 convention).
  - `global_load "g" -> %d` → `ldsfld int64 <asm>Program::G_N` (+ `conv.i4` if the
    dest is a 32-bit local — the field is always 64-bit, like the JVM `J` /
    native 8-byte slot).
  - `global_store "g", %v` → `ld<v>` (+ `conv.i8` if `v` is 32-bit) → `stsfld
    int64 <asm>Program::G_N`.
  - The name is an `Operand::Str` literal (never a register); a non-string /
    uncollected name is an `InvalidOperand` error.

### Verified
- `tests/test_backend.rs`: the emitted `.il` declares `.field public static
  int64 G_0` and carries `ldsfld`/`stsfld`; and **end-to-end on real `ilasm` +
  `dotnet`** a cross-function global program (`compute` seeds `g`; a separate
  `bump` reads/increments/writes it) prints **42**.

## [0.23.0] — 2026-06-21 — arrays → native CIL `int32[]`/`float64[]` (LANG-FULL E5 PR-3b)

The four E5 array opcodes now lower to **real single-dimensional CIL arrays** in
the textual `.il` emitter, so ALGOL 1-D arrays run on real CoreCLR (`ilasm` +
`dotnet`):

| IIR op | CIL (`.il`) |
|--------|-------------|
| `alloc_array dest <- count` (`array<T>`) | `ld<count>; newarr <Elem>; st<dest>` |
| `array_get dest <- handle, idx` | `ld<handle>; ld<idx>; ldelem.<t>; st<dest>` |
| `array_set handle, idx, val` | `ld<handle>; ld<idx>; ld<val>; stelem.<t>` |
| `array_len dest <- handle` | `ld<handle>; ldlen; conv.i4; st<dest>` |

- Element type → CIL: `int32[]` (`ldelem.i4`/`stelem.i4`, `newarr System.Int32`),
  `float64[]` (`.r8`, `System.Double`), `float32[]` (`.r4`, `System.Single`). The
  handle is a reference local (`cil_local_type("array<T>")` → the array CIL type,
  the same machinery the Brainfuck byte-tape `unsigned int8[]` already uses).
- **`i64` collapses to `int32[]`** — `cil_local_type` already maps `i64`→`int32`
  (CIL stack ints are 32-bit on this slice, exactly as scalar `i64` programs
  lower), so unlike the JVM backend **no `array<i64>`→`array<i32>` concretization
  is needed**: the handle, index, and value all land on `int32`/`ldelem.i4`.
- **Bounds-checked for free**: CoreCLR's native `ldelem`/`stelem` bounds check
  throws `System.IndexOutOfRangeException` on an out-of-range index — exactly E5's
  trap, no explicit guard emitted.
- New helper `cil_array_elem` (element → array type + ldelem/stelem/newarr); 5 new
  unit tests (handle typing, element-opcode table, `int32[]` and `float64[]` op
  emission). Verified end to end: the ALGOL sum-of-squares matrix `Prog` now
  assembles with real `ilasm` and runs on real `dotnet` → exit 55.

Scope: the **textual `.il`** emitter (the real-CoreCLR path the matrix runs on).
The binary `CILProgramArtifact` emitter (the in-repo `clr-simulator` path) still
returns `UnsupportedOp` for the array ops — a follow-up.

## [0.22.0] — 2026-06-20 — `f64` (ALGOL `real`) in the textual `.il` emitter (LANG-FULL E3)

The CLR backend was uniformly `int32` and the validator rejected every float
const ("float constants are not supported in CLR v1"). The **textual `.il`
emitter** (the real-CLR / `ilasm` matrix path) now lowers `f64`:

- **`float64` locals** — `cil_local_type` maps `f64` → `float64` (and `f32` →
  `float32`), so a `real` register is declared `float64` in `.locals`. CIL's
  `add`/`sub`/`mul`/`div` and `ceq`/`cgt`/`clt` are stack-type-overloaded, so
  arithmetic and comparison need **no opcode change** for doubles.
- **`ldc.r8` constants** — a float const lowers to `ldc.r8 (b0 b1 … b7)` using
  the exact little-endian IEEE-754 bytes, so a `real` literal round-trips
  bit-for-bit (a decimal would be re-parsed by `ilasm`). `f32` uses `ldc.r4`.
- **Comparison result is `int32`** — a `cmp_*` over `float64` operands carries
  the `f64` *operand* width in its `type_hint`, but `ceq`/`cgt`/`clt` push a 0/1
  `int32`; the register typer now forces a comparison dest to `int32` so it isn't
  declared `float64` (which would `stloc` an int into a float local).
- **Validator** — a float const with a `f32`/`f64` `type_hint` is accepted; one
  with a non-float hint is still rejected (it would silently truncate).

**Verified by RUNNING on real `ilasm` + `dotnet`**: the two ALGOL 60 `real`
programs (`r := 2.5 * 2.0; if r = 5.0 …` → exit 42; `r := 7.0 / 2.0; if r < 4.0 …`
→ exit 1) run on the CLR matrix column. Integer programs are unaffected. (The
structured **bytecode** `lower` emitter still keeps its own f64 guard — the
real-CLR path is the textual one; the bytecode path's f64 is a later follow-up.)

## [0.21.0] — 2026-06-16 — unary `not` op in the textual `.il` emitter (LANG-FULL N3)

The bytecode `lower` path already lowered the unary IIR `not` op, but the textual
`il_text` `.il` emitter had **no arm for it** — only binary `and`/`or`/`xor` and the
lispy `call_builtin "not"` (a boolean negate, `ldc.i4.1; xor`). So a real bitwise-NOT
program (Nib `~0u8`) failed to assemble on CoreCLR through `compile_source_to_cil_text`,
which is the matrix's CLR path. (The v0.20.0 note's `~0u8=255` claim therefore held only
for the bytecode emitter — the textual path was never exercised for `not` until now.)

### Added

- **Unary `not` arm in `il_text`** — emits the CIL `not` opcode (one's complement)
  followed by the existing E2 narrow mask (`ldc.i4 <mask>; and`), so `~0u8 = 255`
  (`-1 & 0xFF`) and `~15u4 = 0` run on real `dotnet`. This is the unary IIR `not` op
  (one source operand), distinct from the `call_builtin "not"` boolean negate. New test
  `unary_not_emits_cil_not_then_masks`. With this, Nib `~` runs on all 7 backends.

## [0.20.1] — 2026-06-16 — E2 verified for the i64 frontend value model (no rework needed)

### Added — regression test for a narrow op over i64 operands

The E2 Nib integration surfaced that the wasm and jvm backends typed the masking
op at the narrow width (i32/int) and so **trapped** when a narrow op met an
`i64` operand — which is exactly what a real frontend emits (Nib materialises
every `const`/`let` as `i64` and carries the narrow width only on the op). They
were reworked to an i64/long register model.

The **CIL backend needs no such rework** because it is **uniformly int32**:
`cil_local_type` maps every scalar — including `i64` — to `int32`, and `const`
emits `ldc.i4` (`i32::try_from`). So a frontend's `i64` consts collapse to
`int32`, the arithmetic is `int32`, and the existing `ldc.i4 <mask>; and` mask
is int32-consistent. A Nib `u8` add of `200 + 100` lowers to
`ldc.i4 200; ldc.i4 100; add; ldc.i4 0xFF; and` → `44`, valid IL on real dotnet.

This release adds a regression test, `e2_u8_op_over_i64_operands_stays_int32`,
that builds the exact Nib shape (i64-hinted consts feeding a `u8` add) and
asserts the emitted IL has **no** `int64`/`ldc.i8` and still carries the byte
mask — locking in that a future change to the value model can't silently break
narrow-width frontends on the CLR. No production code change.

This completes the 3 stack-backend reworks the E2 Nib integration needed:
**wasm** (i64 register model), **jvm** (long register model), and **cil**
(verified int32-uniform — no change).

## [0.20.0] — 2026-06-14 — narrow-width arithmetic wraps mod-2ⁿ (LANG-FULL E2, backend 5/6)

### Added — `u4`/`u8`/`u16` results are masked back into their width

LANG-FULL **E2 — register width & wrap**. A CIL arithmetic/bitwise op runs on a
full 32-bit `int32` stack slot, so a narrow unsigned value silently overflows
its declared width: `200u8 + 100u8` lands as `300` on the stack, but the `u8`
contract requires it to wrap to `300 & 0xFF = 44`. We restore the contract by
AND-masking the result down to the width **after** the op:

```text
  add               ; 200 + 100 = 300  (int32)
  ldc.i4 0xFF       ; push the u8 mask
  and               ; 300 & 0xFF = 44  ✓
```

| type_hint   | mask     | example                          |
|-------------|----------|----------------------------------|
| `u4`        | `0xF`    | `15u4 + 1u4` → `16 & 0xF = 0`     |
| `u8`        | `0xFF`   | `200u8 + 100u8` → `44`; `~0u8` → `255` |
| `u16`       | `0xFFFF` | `~0u16` → `65535`                |
| `u32`,`i32` | —        | the 32-bit op already wraps mod-2³² |
| `i64`,…     | —        | wider/signed: left unchanged     |

The mask fires after `add`/`sub`/`mul`/`div`/`mod`, `neg`, `and`/`or`/`xor`/
`shl`/`shr`, and `not` in **both** emitters — the `lower.rs` bytecode builder
(`emit_narrow_width_mask` → `emit_ldc_i4(mask); emit_and()`) and the textual
`il_text.rs` path (`ldc.i4 0x..; and`) — so narrow-width programs wrap
identically whether assembled from raw bytes or from `.il` via `ilasm`. A
positive mask + `and` is used (never `conv.u1`/`conv.i1`, which would
sign-extend a signed narrow value) to keep the unsigned widths unsigned —
matching the JVM `iand`, wasm `i32.and`, VM, and JIT backends.

This is **backend 5/6** of the E2 enabler (after vm-core, jit-core, iir-to-wasm,
iir-to-jvm-class-file). The narrow `type_hint`s are wired into the Nib/Oct
frontends in the E2 integration PR (6/6), which adds the executed cross-backend
matrix proof. New unit tests: `e2_u8_add_masks_with_ldc_0xff_and`,
`e2_u16_and_u4_masks_match_width`, `e2_wide_widths_emit_no_mask`,
`e2_not_masks_to_width` (bytecode path); `e2_narrow_width_add_masks_result`,
`e2_narrow_width_masks_match_hint`, `e2_wide_widths_are_not_masked` (textual path).

## [0.19.0] — 2026-06-13 — bitwise ops on the textual `.il` path (LANG-FULL N3)

### Fixed — `and` / `or` / `xor` (and `shl` / `shr`) now emit on the textual path

The bytecode emitter (`lower.rs`) already lowered the bitwise/shift ops, but the
**textual `.il` emitter** (`il_text.rs`) — the path the LANG matrix exercises on
real CoreCLR (textual `.il` → `ilasm` → `dotnet`) — only handled
`add`/`sub`/`mul`/`div`/`mod`. An IIR `and`/`or`/`xor` reached the unhandled-op
path, so a program using them failed to assemble. They map to the identically
named CIL opcodes (`and`/`or`/`xor`/`shl`/`shr`); `il_text.rs` now emits them.

Surfaced by LANG-FULL N3 (Nib `& | ^`): the executed cross-backend matrix test
caught the CLR gap that the bytecode-path unit tests didn't. New test
`bitwise_ops_emit_cil_opcodes`.

## [0.18.0] — 2026-06-12 — byte-tape ops on real CoreCLR (LANG-MATRIX LM-C Brainfuck)

Adds the lowering Brainfuck needs to run on the CLR backend's **textual `.il` path**
(`emit_il`, assembled by real `ilasm` and run on real `dotnet`) — the **last code-gen
cell** of the LANG-PLATFORM-MATRIX (after native/LLVM/WASM/JVM). Verified by RUNNING
`++++++++[>++++++++<-]>+.` on real CoreCLR in `lang-aot/tests/lang_matrix.rs`: it
prints `A`.

`lower_brainfuck_for_aot` rewrites Brainfuck's tape into `alloc_bytes`/`load_byte`/
`store_byte` and `concretize_scalar_any_for_cil` retypes the value model to `int32`
(Brainfuck doesn't call `print_i64`, so it isn't kept at i64 — which is exactly what we
want: the tape ops and `brfalse` conditions are all `int32`).

### Added (in `il_text.rs::emit_il`)

- **`alloc_bytes dest <- size`** → `ld<size>; newarr [System.Runtime]System.Byte;
  st<dest>` — a zero-filled `unsigned int8[]` tape. `FnRegs::build` types an
  `alloc_bytes` dest as `unsigned int8[]` (not the scalar `int32` its concretised hint
  would give), so the `.locals` declaration matches the array access.
- **`load_byte dest <- base, idx`** → `ld<base>; ld<idx>; ldelem.u1; st<dest>`.
  `ldelem.u1` loads an *unsigned* byte (a cell value 200 reads as 200, not −56),
  zero-extended to `int32`.
- **`store_byte base, idx, val`** → `ld<base>; ld<idx>; ld<val>; stelem.i1`. `stelem.i1`
  truncates to a byte — Brainfuck's 8-bit cell wrap-around for free. Rejects a `dest`.
- **`call_builtin putchar`** → `ld<v>; ldc.i4 0xFF; and; conv.u2; call void
  [System.Console]System.Console::Write(char)` — writes the cell as a *character* (so
  `.` of 65 emits `A`, not `65`). Dest-less, handled before the dest lookup like
  `print_i64`.
- **`call_builtin getchar`** → `call int32 [System.Console]System.Console::Read()` →
  dest (EOF `-1` truncates to `0xFF` at a later `store_byte`).

### Changed

- The `Run()` launcher's `prints` detection now also matches `putchar` (not just
  `print_i64`), so a Brainfuck program **discards** `MccarthyEntry`'s `int32` result
  (`pop`) instead of `Console.WriteLine`-ing it — otherwise the program would print both
  its own output and its meaningless exit value (a double-print).

CIL `brfalse`/`brtrue` test any integer width against zero, so the (int32) loop guard
needs no special handling — unlike the JVM (`lcmp`) and wasm (`i64.eqz`) which had an
i64 branch-condition ripple. Three new tests in `il_text.rs` cover the tape ops +
putchar (with the launcher discard), `getchar`, and the `store_byte`-with-dest rejection.

## [0.17.0] — 2026-06-12 — `print_i64` → `Console.WriteLine` + I/O launcher (LANG-MATRIX LM-C BASIC)

The textual `.il` emitter gains the **`print_i64`** I/O primitive that Dartmouth
BASIC's `PRINT` lowers to — previously `UnsupportedOp`. It has **no dest** (it's a side
effect), so it's handled before the dest lookup: the value is loaded and handed to
`call void [System.Console]System.Console::WriteLine(int32)` (the CLR analogue of the
wasm `env.__print_i64` import / JVM `env.BasicRuntime.println(J)V`).

The `Run()` launcher is now **I/O-aware**: an expression program still
`Console.WriteLine`s the entry method's `int` result, but a program that calls
`print_i64` has already written its own output as a side effect, so the launcher merely
runs the entry and **discards** its (unused) `int32` return with `pop` — no double-print.

Verified by RUNNING on real `ilasm` + `dotnet` (via `lang-aot`'s `lang_matrix` CLR
column): BASIC `10 PRINT 42` → `Console` `42` (exactly once). New unit test asserts the
single `Console.WriteLine` and the launcher `pop`. No change to expression-language
output (the prior CIL suites + conformance stay green).

## [0.16.0] — 2026-06-12 — integer arithmetic + comparison opcodes (LANG-MATRIX LM-C)

The textual `.il` emitter (`il_text.rs`) grows two op families it previously rejected
with `UnsupportedOp` — McCarthy only ever emitted a constant return, so arithmetic and
comparison had never been needed until the LANG-MATRIX campaign ran the expression
languages (Nib, Oct, ALGOL 60) on the real CLR:

* **Binary integer arithmetic** `add` / `sub` / `mul` / `div` / `mod` → the CIL opcodes
  `add` / `sub` / `mul` / `div` / `rem` (note `mod` → `rem`, signed remainder). Both
  operands are loaded and the single opcode emitted; CoreCLR's `div`/`rem` raise on
  divide-by-zero, matching the other backends' trap behaviour.
* **Integer comparisons** `cmp_eq` / `cmp_ne` / `cmp_lt` / `cmp_le` / `cmp_gt` / `cmp_ge`
  → a `0`/`1` `int32`. CIL has only `ceq` / `clt` / `cgt`; the other three relations are
  the logical negation of one (`<primitive>; ldc.i4.0; ceq`). The result feeds either a
  `st<dest>` or directly a `brfalse`/`brtrue`.

Verified by RUNNING on real `ilasm` + `dotnet` (via `lang-aot`'s `lang_matrix` CLR
column): Nib `double(21)`→42, Oct `if x == 1`→0, ALGOL `17 mod 5`→2. New unit tests
assert the emitted opcodes for every arithmetic + comparison op. No change to existing
behaviour (49 + 86 prior tests still green).

## [0.15.0] — 2026-06-11 — textual `.il`: lambda / LABEL / recursion (CLR-real C5)

`emit_il` becomes a **multi-function** emitter — the last McCarthy F-feature:

- Every IIR function is now its own static `.method` (the entry → `MccarthyEntry`,
  each hoisted lambda/label keeps its name `lambda_<n>`/`label_<n>`), with a CIL
  signature derived from its IIR params/return type (a lambda returns `object`).
- `call <dest> = <fn>(args…)` → a by-name `call <ret> <Class>::<m>(<argtys>)`;
  `ilasm` resolves the token, so **self-recursive `LABEL`** is just a method calling
  itself. A `call` to an unknown function is rejected (`UndefinedLabel`).
- **Parameters** live in `ldarg`/`starg` slots (locals stay `ldloc`/`stloc`) — a new
  `FnRegs` register model assigns argument slots to params and local slots to dests.
- `is_null` → `ldnull; ceq`.
- **`field_*` on an `object`-typed array operand** (a lambda parameter, vs a
  freshly-`alloc`-ed `object[]`) now emits a `castclass object[]` before
  `ldelem.ref`/`stelem.ref` — real CoreCLR's importer requires an array on the
  stack, a constraint the lenient in-repo simulator never enforced (exactly the
  class of bug this chapter exists to catch).

**Security:** function names join labels as the IIR-supplied *strings* that reach
the `.il` text (`.method`/`call`), so they go through the same fail-closed
`[A-Za-z0-9_$]` identifier whitelist (`checked_cil_ident`, which `checked_label`
now delegates to). New unit test `malicious_function_name_is_rejected_not_injected`.

The resolved entry-point name (`entry_point` or the `"main"` fallback) is computed
**once** via `entry_name` and used for every entry comparison (the existence check,
the `MccarthyEntry` rename, `is_entry`, the `call` callee) so the launcher's
hardcoded `call …::MccarthyEntry()` can never dangle when `entry_point` is `None`.

Verified by RUNNING on real CoreCLR (`lang-aot/tests/clr_real_lambda.rs`):
`((LAMBDA (X) X) 5)`→5, `((LAMBDA (X) (CAR X)) (CONS 7 9))`→7,
`((LAMBDA (X Y) (EQ X Y)) 3 3)`→1, a COND-body lambda→100, and a recursive
`LABEL` descending CARs→7. New unit tests `lambda_emits_second_method_param_ldarg_and_call`,
`recursive_label_calls_itself_by_name`, `call_to_unknown_function_is_rejected`,
`none_entry_point_falls_back_to_main_and_names_mccarthy_entry`.

## [0.14.0] — 2026-06-11 — textual `.il`: symbols (CLR-real C4)

**No new emit ops** — symbols reuse the existing value model. The shared
`intern_symbols_structural` pass lowers each `(QUOTE S)` to a *tagged integer id*
(`A` → `0x20000000`, `B` → `0x20000001`, …); on the CLR that id is just a boxed
`System.Int32` atom, the exact scalar/predicate shape C1–C3 already emit. So
`(EQ (QUOTE A) (QUOTE A))` is two equal `ldc.i4 536870912` consts, boxed, then
`equal?`-unboxed + `ceq`; `(ATOM (QUOTE A))` is `not (pair? boxed-int)`. New unit
test `symbol_eq_emits_tagged_id_consts_unboxed_and_compared` pins the value model;
the real-CoreCLR proof is `lang-aot/tests/clr_real_symbols.rs`.

## [0.13.0] — 2026-06-11 — textual `.il`: predicates + COND (CLR-real C3)

`emit_il` grows the McCarthy predicate primitives and `COND` control flow:

- `call_builtin "pair?"` → `isinst object[]; ldnull; ceq; ldc.i4.0; ceq` (a clean
  0/1 bool: is the boxed value a cons cell?). Note the **textual** form is
  `isinst object[]` — `ilasm` rejects an explicit `[System.Runtime]System.Object[]`
  assembly scope in that position (syntax error), unlike the `newarr` element type.
- `call_builtin "not"` → `ldc.i4.1; xor` (boolean negation of a 0/1 value).
- `call_builtin "equal?"` → `unbox.any [System.Runtime]System.Int32` on both
  operands + `ceq` (atom identity reduces to integer equality; symbols are interned
  to ints upstream).
- `COND` lowering: `label` → a `<name>:` anchor, `jmp` → `br <name>`,
  `jmp_if_false` → `ldloc cond; brfalse <name>` (and `jmp_if_true` → `brtrue`).
- `const` of a **reference** type (`ref<…>`) is the McCarthy nil — emit `ldnull`
  (the canonical null `object[]`), never `ldc.i4 0`, which would be an ill-typed
  store into an object-typed local. A non-zero reference constant is rejected.

**Security:** branch-target / label names are the one IIR-supplied *string* (not a
numeric slot) that reaches the `.il` text, so they are validated by a new
`checked_label` helper — only `[A-Za-z0-9_$]` passes, anything else (newlines,
braces, `.`-directives, `//` comments) is rejected as `InvalidOperand`. This closes
a latent CIL-injection vector before the source-derived names of C4/C5 land. Unit
test `malicious_label_name_is_rejected_not_injected`.

Verified by RUNNING on real CoreCLR (`lang-aot/tests/clr_real_predicates.rs`):
`(ATOM 7)`→1, `(ATOM (CONS 1 2))`→0, `(EQ 7 7)`→1, `(EQ 7 8)`→0,
`(COND ((ATOM 7) 11) …)`→11, `(COND ((ATOM (CONS 1 2)) 11) ((EQ 5 5) 22))`→22. New
unit tests: `atom_emits_isinst_xor_predicate_chain`, `eq_emits_double_unbox_then_ceq`,
`cond_emits_branches_labels_and_nil_fallthrough`, `const_of_reference_type_rejects_non_nil`.

## [0.12.0] — 2026-06-11 — textual `.il`: cons / car / cdr (CLR-real C2)

`emit_il` grows the cons value model: `alloc` → `ldc.i4.2; newarr
[System.Runtime]System.Object` (a McCarthy cons cell is a 2-element reference
array), `box`/`unbox.any [System.Runtime]System.Int32` for integer atoms,
`field_store` → `stelem.ref`, `field_load` → `ldelem.ref`. Locals are now typed
per producing instruction — a cons cell local is `object[]`, a boxed atom `object`,
a raw int `int32` (`cil_local_type`) — so `ilasm` verifies the program. Verified by
RUNNING on real CoreCLR (`lang-aot/tests/clr_real_cons.rs`): `(CAR (CONS 7 9))`→7,
`(CDR …)`→9, nested→2. New unit test `cons_car_emits_object_array_box_and_unbox`.

## [0.11.0] — 2026-06-11 — textual `.il` emitter for the real-CoreCLR path (CLR-real C1)

New `il_text` module + `emit_il(module, config) -> String`: emits **textual CIL**
(`.il`) — the real-runtime counterpart to the binary `lower_iir_to_cil` (which feeds
the in-repo `clr-simulator`). The `.il` is assembled by real `ilasm` into a loadable
PE that runs on real `dotnet`, exactly as the LLVM backend emits textual `.ll` for
real `clang`. Metadata ownership (PE headers + the `#~`/`#Strings`/`#Blob` streams +
token resolution) is delegated to `ilasm` (no hand-rolled ECMA-335).

C1 covers scalar McCarthy: the entry function's `const`/`mov`/`ret` →
`ldc.i4`/`ldloc`/`stloc`/`ret`, wrapped in a `MccarthyEntry()` method plus a printing
`.entrypoint` launcher. Every other op returns `UnsupportedOp`, so later slices grow
the op match (cons, predicates, COND, symbols, lambda). New unit tests
`scalar_emits_well_formed_il`, `unsupported_op_is_rejected_not_emitted`.

## [0.10.0] — 2026-06-10 — McCarthy lambda: accept `call`/`ref<any>` (W8b, F7)

The validator now accepts the `call` op with a `ref<any>` type — a lisp function
call returns the callee's uniform-reference result. The `call` lowering already
computed the `MethodDef` token + pushed args (boxed by the structural pass); this
one-line allowlist addition lets McCarthy `(LAMBDA …)` applications validate and
emit. `((LAMBDA (X) (CAR X)) (CONS 7 9))` → 7 on the `clr-simulator`.

## [0.9.0] — 2026-06-10 — McCarthy predicates: ATOM / EQ / COND (W7, F3–F5)

Lower the structural pass's `pair?`/`not`/`equal?` `call_builtin`s on the CLR:
`pair?` → `isinst object[]; ldnull; ceq; ldc.i4.0; ceq`; `not` → `x ^ 1`;
`equal?` → `unbox.any int32; unbox.any int32; ceq`. `COND` reuses the existing
`jmp_if_true`/`jmp_if_false`. Whitelisted the three names in the validator.
`(ATOM 7)`→1, `(ATOM (CONS 1 2))`→0, `(EQ 7 7)`→1, `(COND …)` all run on the
`clr-simulator`. These are the CLR twins of the JVM `instanceof`/`ixor`/`if_icmpeq`.

## [0.8.0] — 2026-06-10 — McCarthy cons: `box`/`unbox` lowering (W6b)

Lower the shared structural pass's `box`/`unbox` ops: `box` → `ldloc ; box [int32] ; stloc`,
`unbox` → `ldloc ; unbox.any [int32] ; stloc`. With the already-supported
`alloc`/`field_*` (`newarr`/`stelem.ref`/`ldelem.ref` over `System.Object[]`),
McCarthy **cons** runs: `(CAR (CONS 7 9))` → 7 on the in-repo `clr-simulator`.
Removed `box`/`unbox` from `UNSUPPORTED_OPS`; the validator already accepted
`ref<any>` and now lists `box`/`unbox` as heap ops.

All notable changes to this crate are documented here.

## [0.7.0] — 2026-06-01 (G4 — `print_i64` host call → `env.BasicRuntime::PrintI64(int64)`)

### Added — `call_builtin "print_i64"` whitelisted and lowered

Completes the cross-backend trio for BASIC's `PRINT`:

| Backend                  | Builtin     | Target                                            |
|--------------------------|-------------|---------------------------------------------------|
| iir-to-wasm v0.8.0       | `print_i64` | `env.__print_i64` host import                     |
| iir-to-jvm-class-file v0.7.0 | `print_i64` | `invokestatic env/BasicRuntime.println(J)V`   |
| **iir-to-cil-bytecode v0.7.0 (this)** | `print_i64` | `call void env.BasicRuntime::PrintI64(int64)` |

After this release, BASIC's PRINT lowers to real bytecode on all three
non-BEAM backends — gap G4 (final gap in the `print_i64` group) of the
[multi-language backend plan][plan].

#### Validator changes (`src/validate.rs`)

* `CALL_BUILTIN_SUPPORTED_NAMES` widened from `["putchar", "getchar"]`
  to `["putchar", "getchar", "print_i64"]`.  Defence in depth unchanged:
  every other name still fails with `UnsupportedOp`.

#### Lowering changes (`src/lower.rs`)

* New sentinel metadata token
  `BASIC_PRINT_I64_TOKEN: u32 = 0x0A00_0005` (MemberRef row 5, next
  after `BF_GETCHAR_TOKEN` @ row 4).  At link / sim time the token
  resolves to `env.BasicRuntime::PrintI64(int64)`.
* New `"print_i64"` arm in the `call_builtin` match:
  ```
  srcs = [Var("print_i64"), Var(val: i64)]   dest = None
  →
  ldloc val_slot          ; via emit_load (picks width from reg_info)
  call <BASIC_PRINT_I64_TOKEN>
  ```

Why a dedicated host class (vs. reusing `env.BFRuntime`): BASIC's I/O
is line/value oriented; Brainfuck's is byte-stream oriented.  Separate
host classes let a CLR runtime / launcher stub or provide either one
independently.

#### Tests added (`tests/test_backend.rs`)

* `g4_validator_accepts_print_i64`
* `g4_validator_still_rejects_unknown_builtin`
* `g4_lowers_print_i64_to_call_with_basic_token`

[plan]: ../../../specs/MULTILANG-BACKEND-PLAN.md

## [0.6.0] — 2026-05-26 (Validator accepts `ref<any>` + `mov` for ref types)

### Changed — `ref<any>` widens the supported reference types; `mov` for refs

Companion to Twig path-A increment 6c.  The Phase 2 heap-lowering
convention is `field_load dest, pair, idx [ref<any>]`.  CLR lowers
this to `ldelem.ref`, which returns `System.Object` — the same type
cons-cell fields are declared as in the `System.Object[2]` Phase 2
representation.

Two validator changes:

1. `ref<any>` is now accepted alongside `ref<LispyPair>`.
2. `mov` is added to the list of supported ops for reference types
   (matches the `emit_move` path that twig-ir-compiler uses to flow
   a `ref<any>` value through a register-to-register copy).

All other `ref<X>` types continue to be rejected.  No lowering
changes — `ldelem.ref` and `stloc` already work for both types.

## [0.5.0] — 2026-05-22 (Brainfuck — `byte[]` tape + I/O via env.BFRuntime)

### Added — Brainfuck `load_mem` / `store_mem` / `call_builtin` lowering

Stage 3 of 4 for the BF→{wasm,jvm,clr,beam} story.  Mirrors PR #3921
(iir-to-wasm 0.4.0) and PR #3928 (iir-to-jvm-class-file 0.5.0) for the
CLR target.  Lets BF's IIR — including `load_mem`, `store_mem`, and
`call_builtin "putchar"`/`"getchar"` — flow through the same universal
`iir-to-cil-bytecode` backend that Twig, BASIC, Oct, and Nib already
use.

#### Validator changes

- `load_mem` and `store_mem` removed from `UNSUPPORTED_OPS` (previously
  hard-rejected).  Both lower to CIL `ldelem.u1` / `stelem.i1` over a
  host-provided byte array.
- `call_builtin` is now **conditionally** accepted via a new
  `CALL_BUILTIN_SUPPORTED_NAMES` whitelist (currently
  `["putchar", "getchar"]`).  Unknown builtin names still produce a
  clear `UnsupportedOp` error that includes the rejected name and the
  whitelist.

#### Lowering changes

- Three new reserved metadata tokens referencing the simulated
  `env.BFRuntime` host class:
  - `BF_TAPE_TOKEN   = 0x0400_0001` — FieldRef row 1, the static `byte[] __tape`.
  - `BF_PUTCHAR_TOKEN = 0x0A00_0003` — MemberRef row 3 (Console.WriteLine is row 2),
    `void env.BFRuntime::putchar(int32)`.
  - `BF_GETCHAR_TOKEN = 0x0A00_0004` — MemberRef row 4,
    `int32 env.BFRuntime::getchar()`.
- New `emit_instr` arms:
  - `load_mem v ptr` → `ldsfld BF_TAPE_TOKEN; ldloc ptr; ldelem.u1;
    stloc dest`.  `ldelem.u1` zero-extends the byte to int32 — matching
    BF's u8 cell semantics without the sign-extension surgery the JVM
    target requires after `baload`.
  - `store_mem ptr v` → `ldsfld BF_TAPE_TOKEN; ldloc ptr; ldloc v;
    stelem.i1`.  `stelem.i1` truncates the int32 to a byte, matching
    BF's u8 wraparound.
  - `call_builtin "putchar" v` → `ldloc v; call BF_PUTCHAR_TOKEN`.
  - `call_builtin "getchar" -> v` → `call BF_GETCHAR_TOKEN; stloc v`.
- Defense in depth: hand-crafted IIR that slips an unknown builtin
  past the validator still hits a `UnsupportedOp` in `lower.rs`.

#### Host class contract

The CLR runtime / launcher (or PE packager) must provide `env.BFRuntime`:

| Symbol                                       | Metadata token reserved | Notes                          |
|----------------------------------------------|--------------------------|--------------------------------|
| `public static byte[] __tape`                | FieldRef row 1           | typically 30 KB BF tape         |
| `public static void putchar(int32)`          | MemberRef row 3          | write one byte to stdout        |
| `public static int32 getchar()`              | MemberRef row 4          | read one byte; -1 / 0 on EOF     |

This is the CLR analog of the WASM backend's `env` import namespace
and the JVM backend's `env/BFRuntime` class — same model, different ABI.

### Tests

- 5 new validator unit tests (`load_mem_accepted_for_bf`,
  `store_mem_accepted_for_bf`, `call_builtin_putchar_accepted`,
  `call_builtin_getchar_accepted`,
  `call_builtin_unknown_name_rejected`).
- 33 lib + 83 integration tests pass.
- 4 new BF→CLR e2e tests in
  `brainfuck-iir-compiler/tests/clr_e2e.rs` assert exact byte
  sequences for `call BF_PUTCHAR_TOKEN`, `ldsfld BF_TAPE_TOKEN`,
  and `call BF_GETCHAR_TOKEN`.

### Compatibility

- Non-BF frontends (Twig, BASIC, Oct, Nib) unchanged.  Modules without
  BF features get no `env.BFRuntime` token references, preserving CIL
  byte-equivalence with pre-0.5.0 output.

## [0.4.1] — 2026-05-13

### Fixed (Multi-backend demo — fib(10)=55)

- **`"mov"` opcode support** — added handling for the `mov` IIR instruction
  (pre-lowered form of `call_builtin "_move"`).  The CLR lowerer now emits
  the source operand load followed by a `stloc`/`starg` store, mirroring what
  was already done for other copy-value instructions.

## [0.4.0] — 2026-05-12

### Added (LANG37 — CLR Closure Lowering)

#### `int32[]`-based closure representation

The CLR backend now supports first-class closures (LANG34 `alloc_closure` /
`call_closure` opcodes) using an `int32[]` dispatch-table approach.

A closure is represented as an `int32[]` array:
- `closure[0]` — function dispatch index (alphabetical among closure targets)
- `closure[1..n]` — captured values (all stored as `int32`)

#### `alloc_closure` lowering

`alloc_closure(Str("fn_name"), Var(cap0), …) : "closure"` lowers to:

```cil
ldc.i4 {n+1}              ; array size = 1 (idx) + n (captures)
newarr [System.Int32]     ; int32[] closure_arr = new int32[n+1]
dup
ldc.i4.0
ldc.i4 {dispatch_idx}
stelem.i4                 ; closure_arr[0] = dispatch_idx
dup
ldc.i4.1
ldloc cap0_slot
stelem.i4                 ; closure_arr[1] = cap0
…
stloc dest_slot           ; dest = closure_arr
```

#### `call_closure` lowering

`call_closure(Var(handle), Var(arg0), …) : "any"` lowers to:

```cil
ldloc handle_slot         ; push closure handle (int32[])
ldc.i4 {n_args}           ; args array size
newarr [System.Int32]     ; int32[] args_arr = new int32[n_args]
dup
ldc.i4.0
ldloc arg0_slot
stelem.i4                 ; args_arr[0] = arg0
…
call int32 ClassName::__callClosure(int32[], int32[])
stloc dest_slot           ; dest = result
```

#### Synthetic `__callClosure` dispatch method

When any `alloc_closure` instruction is present in the module,
`lower_iir_to_cil` appends a synthetic `__callClosure(int32[], int32[]) →
int32` static method.  It reads `closure[0]` and dispatches to the correct
user function via a chain of `ldc.i4 N; beq case_N` branches.

Token: `0x0600_0001 + module.functions.len()` (the next slot after all user
functions in the MethodDef table).

#### New token

- `INT32_ARRAY_TYPE_TOKEN = 0x0100_0002` added to `ir-to-cil-bytecode`
  alongside the existing `OBJECT_ARRAY_TYPE_TOKEN = 0x0100_0001`.  Used with
  `newarr` to allocate `int32[]` closure and argument arrays.

#### Validator changes

`validate_iir_for_clr` now:
- **Accepts** `alloc_closure` with `i32`/`bool` captures (LANG37 early-accept).
- **Accepts** `call_closure` unconditionally (type_hint `"any"` is fine here).
- **Rejects** `alloc_closure` with `i64`/`u64`/`f32`/`f64` captures with a
  `ClosureOpcode` error: `"only i32/bool captures are supported by the CLR
  backend in v1 — use integer types or upgrade to LANG38"`.

#### Tests

- `lang37_alloc_closure_i32_cap_accepted_by_clr_validator`: i32 capture passes.
- `lang37_call_closure_accepted_by_clr_validator`: call_closure passes.
- `lang37_i64_capture_still_rejected`: i64 capture → ClosureOpcode.
- `lang37_float_capture_still_rejected`: f32 capture → ClosureOpcode.
- `lang37_alloc_closure_emits_newarr`: alloc_closure emits `newarr` (0x8D).
- `lang37_alloc_closure_emits_stelem_i4`: alloc_closure emits `stelem.i4` (0x9E).
- `lang37_call_closure_emits_call_dispatch`: call_closure emits `call` (0x28).
- `lang37_dispatch_method_generated`: artifact contains `__callClosure` method.
- `lang37_dispatch_method_contains_ldelem_i4`: dispatch body has `ldelem.i4` (0x94).

#### Deferred

- i64/f32/f64 closure captures — LANG38.
- WASM closure lowering — LANG38.
- Real .NET round-trip test — LANG39.

---

## [0.3.0] — 2026-05-12

### Added (LANG35 — Closure Backend Integration)

#### Improved `ClosureOpcode` validator error

- `validate_iir_for_clr` now emits a dedicated `ClosureOpcode` error message
  (format: `"[fn_name] ClosureOpcode: alloc_closure/call_closure require the
  BEAM backend — CLR does not support heap-allocated closures"`) when it
  encounters `alloc_closure` or `call_closure`.
- Previously these fell through to the generic `UntypedInstruction` path;
  the closure check now runs first to give a more actionable error message.

#### Tests

- `lang35_alloc_closure_closure_opcode_error`: asserts `validate_iir_for_clr`
  returns an error containing "ClosureOpcode" for a module with `alloc_closure`.
- `lang35_call_closure_closure_opcode_error`: same for `call_closure`.
- `lang35_closure_opcode_error_not_untyped`: asserts the error does NOT
  contain "UntypedInstruction".

---

## [0.2.0] — 2026-05-11

### Added (LANG32 — Global Variables and I/O)

#### I/O support

- `io_out %v` → `ldloc <slot>; call System.Console.WriteLine(int64)`.
  Uses token `CONSOLE_WRITELINE_I64_TOKEN = 0x0A00_0002` (pre-defined
  member reference to `Console.WriteLine(long)`).

#### Global variables (LANG32b — deferred)

- `global_load` and `global_store` return `UnsupportedOp` with a clear
  LANG32b tracking note.  Full CLR static-field globals require extending
  `CILProgramArtifact` with a fields table and adding `ldsfld`/`stsfld`
  sequences; tracked in a follow-up PR.

#### Exhaustiveness fixes

- `Operand::Str` arms added to all `match` blocks in `lower.rs` (const,
  call argument loop).

---

## [0.1.0] — 2026-05-11

### Added

- Initial release.
- `validate_iir_for_clr(module: &IIRModule) -> Vec<String>` — pre-flight
  validator that checks for empty modules/functions, untyped instructions
  (`"any"` / `"polymorphic"` type hints), unsupported types (`"str"`,
  `"ref<…>"`), float constants (unsupported in CLR v1), and unsupported
  opcodes.
- `IIRClrConfig` — backend configuration struct (assembly name).
- `IIRClrError` — rich error enum with function-scoped context for all
  failure modes: `ValidationFailed`, `UnsupportedOp`, `UnsupportedType`,
  `UndefinedLabel`, `UndefinedVariable`, `InvalidOperand`, `AssemblyError`.
- `lower_iir_to_cil(module: &IIRModule, config: &IIRClrConfig) -> Result<CILProgramArtifact, IIRClrError>`
  — two-pass register allocator + CIL emitter that lowers every IIRFunction
  to a `CILMethodArtifact` (assembled CIL body bytes).
- `IIRClrCodeGenerator` — `codegen_core::CodeGenerator<IIRModule, CILProgramArtifact>`
  adapter so the backend participates in the shared code-generator protocol.
- Opcode coverage: `const`, `add`, `sub`, `mul`, `div`, `mod`, `neg`,
  `and`, `or`, `xor`, `not`, `shl`, `shr`, `cmp_eq`, `cmp_ne`, `cmp_lt`,
  `cmp_le`, `cmp_gt`, `cmp_ge`, `label`, `jmp`, `jmp_if_true`,
  `jmp_if_false`, `ret`, `ret_void`, `call`, `load_reg`, `store_reg`,
  `type_assert`.
- 47 integration tests in `tests/test_backend.rs`.
