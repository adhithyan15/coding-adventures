# Changelog — iir-to-jvm-class-file

## 0.32.0 - 2026-07-20 — field_load/store CHECKCAST [Ljava/lang/Object; — fixes cons-cell VerifyError

Part of the fix restoring McCarthy-lisp list programs on the native-AOT / LLVM backends (`lang-aot` `lang_matrix`). See the umbrella commit for the full story: `null?` was never routed to a runtime call on the tagged native/LLVM path (breaking every cons-walk helper), `list-ref`/`assoc` unboxed a raw-int index/key (→ wrong element), a top-level `(null? …)` predicate result was unboxed instead of truthy-coerced, and cons-cell field access failed the JVM verifier. Verified end-to-end: native list-ref/assoc/length/reverse/append/null? all correct.
## [0.31.0] — 2026-07-11 (LANG-FULL E6d-2a: i64-width `box`/`unbox`)

`box`/`unbox` width-adapt for E6d-2 dynamic arithmetic (i64): `box` of a `long`-slot value emits `lload; l2i` before `Integer.valueOf`; `unbox` into a `long` slot emits `i2l; lstore` after `intValue`. i32-slot lisp box/unbox unchanged.

## [0.30.0] — 2026-07-10 (LANG-FULL E4-dyn — E4d-BA-arr: `java.lang.String[]` reference arrays)

BASIC string arrays (`DIM A$(n)`) lower to a JVM `java.lang.String[]` — the first
**reference-element** array on this backend (E5 numeric arrays were all primitive
`int[]`/`long[]`/`double[]`).

- `iir_type_to_jvm("array<str>")` now maps to `Some(JvmType::Ref)` (a supported
  reference element); a new `jvm_ref_array_element_class` returns
  `java/lang/String` for a `str` element.
- `alloc_array` emits `anewarray java/lang/String` (a `cp.add_class` reference
  array) instead of `newarray <atype>`; `array_get`/`array_set` use `aaload`/
  `aastore` (the reference element ops) instead of the typed `*aload`/`*astore`.
- A str value is a real `java.lang.String` reference, so no handle materialisation
  is needed; the validator accepts `str` on `array_get`/`array_set`.

Tests: `string_array_emits_reference_array_opcodes`; the pinned
`array_handle_maps_to_ref` now asserts `array<str>` → `Ref` (and an unsupported
ref element still → `None`).

## [0.29.0] — 2026-07-06 (LANG-FULL E4-dyn: BASIC string `INPUT A$`)

BASIC's string `INPUT A$` (E4-dyn) now lowers to real JVM bytecode: a whole line
is read from the host **as the string value itself**, a genuinely runtime string
the compiler cannot fold.

- **Validator** (`validate.rs`): the `str`-type gate now also accepts `str` on
  `call_builtin` and `mov` (previously only `str_const`/`str_concat`/`str_slice`/
  `call`/`ret`). `INPUT A$` emits a `str`-typed `call_builtin "input_str"` followed
  by a `str`-typed `mov` into the `$`-variable's slot — both were rejected before.
- **Lowering** (`lower.rs`): new `input_str` arm — `invokestatic
  env/BasicRuntime.readLine()Ljava/lang/String;` then `astore` the returned
  `String` reference into the `str`-typed dest slot (`iir_type_to_jvm("str") =
  Ref`). This is the string sibling of `input_i64`'s `readLong()J` + `lstore`; a
  `str` `mov` is a plain reference `aload`/`astore`.
- **Test**: `call_builtin_input_str_and_str_mov_accepted` proves both ops clear
  the whitelist and the `str`-type gate.

Run-proven end to end in `lang-aot`'s `lang_matrix` (real `javac`/`java`):
`10 INPUT A$ / 20 PRINT A$ / 30 END` with stdin `"OK"` prints `OK`.

## [0.28.0] — 2026-07-04 (LANG-FULL E4-dyn: `str` as a return value / call result)

A runtime string that arrives as a function **return value** or **call result**
— an ALGOL `string procedure`'s returned runtime string — now lowers on the JVM.

The `str` value model is already a `java.lang.String` reference, so a `str`
parameter, a `str`-returning method, and a `str` call result already lower
correctly. The only gap was the validator, which rejected a `str` type_hint on
any op other than `str_const`/`str_concat`/`str_slice`. A `call` (str return /
call result) and a `ret` (str-returning method) carry the `String`, so both are
now accepted. (Diagnosed with a probe: the failure was a validation rejection,
matching the WASM E4d-3b pattern — not a lowering gap.)

Unit test `validate_accepts_str_on_call_and_ret`. The `lang-aot` ALGOL
string-procedure matrix cell now runs on **all seven backends**.

## [0.27.0] — 2026-06-30 (BA-INPUT: JVM wide i64 model fixes for `input_i64`)

Two correctness fixes that together allow Dartmouth BASIC `INPUT X` programs to
run on the JVM without VerifyErrors:

**`emit_lconst_cp` (new helper)**: `emit_lconst` previously emitted a deliberate
invalid `ldc2_w 0xFFFF` placeholder for values outside the i16 range, on the
assumption that wide Long constants were unreachable in arithmetic programs.
`input_i64` puts BASIC programs into the wide i64 model, where `__basic_print_real`
uses constants such as `100000` (for the digit-extraction loop).  Added
`emit_lconst_cp(code, cp, value)` that interns a `CONSTANT_Long` entry via
`cp.add_long` and emits a proper `ldc2_w <idx>` — the Long counterpart of
`emit_dconst_cp` / `emit_iconst_cp`.

**`"const"` lowering**: Updated the `JvmType::Long` arm in `Operand::Int` handling
to call `emit_lconst_cp` instead of `emit_lconst`, so arbitrary integer literals
stored into Long slots get a valid CP entry.

**`"return"` lowering**: Same fix in the `Operand::Int` + `JvmType::Long` arm of
the `"return"` case, for functions that return a Long with an integer literal.

**`putchar` lowering** (prior session): When the calling program is in wide i64
model, the value being written via `putchar(I)V` lives in a Long slot. Updated
to emit `lload; l2i` instead of `iload` when `val_type == JvmType::Long`.

Combined with the `concretize_scalar_any_for_jvm` fix in `lang-aot` (adding
`input_i64` to the wide-model check), BASIC `INPUT X` now works end-to-end on
the JVM: `matrix_every_proven_cell_agrees` passes for both INPUT programs.

## [0.25.0] — 2026-06-29 (LANG-FULL BA-pow — `f64_pow` JVM lowering)

Added `"f64_pow"` arm: loads base and exponent onto the JVM operand stack with
`emit_typed_load`, emits `invokestatic java/lang/Math.pow:(DD)D`, and stores the
result with `emit_typed_store`.  Two-double-argument call — matches the existing
unary transcendental pattern but with a second source.

All notable changes to this crate are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.26.0] — 2026-06-29 (AL8-arctan — `f64_atan/f64_tan` via `java.lang.Math`)

Extended the f64 transcendental match arm to cover two more ops:
- `f64_atan` → `Math.atan`  (inverse tangent, range −π/2 to π/2)
- `f64_tan`  → `Math.tan`   (tangent)

Both emit `dload r; invokestatic java/lang/Math.<method>:(D)D; dstore rd`.
HotSpot intrinsifies `Math.atan` and `Math.tan` to native calls at JIT tier 4.

## [0.25.0] — 2026-06-28 (AL8-trig — `f64_sin/cos/ln/exp` via `java.lang.Math`)

Extended the `f64_sqrt` arm to cover all five f64 ops: `f64_sqrt` → `Math.sqrt`,
`f64_sin` → `Math.sin`, `f64_cos` → `Math.cos`, `f64_ln` → `Math.log` (natural log),
`f64_exp` → `Math.exp`.  All emit `dload r; invokestatic java/lang/Math.<method>:(D)D;
dstore rd`.  HotSpot intrinsifies all five; no JNI overhead.

## [0.24.0] — 2026-06-28 (AL8-sqrt — `f64_sqrt` lowers to `Math.sqrt`)

The JVM backend now lowers `f64_sqrt` by emitting `dload r; invokestatic
java/lang/Math.sqrt:(D)D; dstore rd`.  HotSpot treats `Math.sqrt` as an
intrinsic and lowers it directly to `sqrtsd` on x86_64 with no JNI overhead.
NaN propagates; negative input returns NaN (IEEE-754, matches VM).

## [0.23.0] — 2026-06-28 (literal string comparison — LANG-FULL E4)

The JVM backend now lowers `str_cmp` over managed `String` locals by invoking
`java/lang/String.compareTo(String)` and normalizing the result with
`java/lang/Integer.signum(int)`.

## [0.22.0] — 2026-06-28 (literal string slice — LANG-FULL E4)

The JVM backend now lowers `str_slice` over managed `String` locals by loading
the source, narrowing i64 start/end bounds with `l2i` when needed, and invoking
`java/lang/String.substring(II)Ljava/lang/String;`. The resulting string local
can feed the existing `str_index`, `str_len`, `str_eq`, and `print_str` paths.

## [0.21.0] — 2026-06-27 (literal string index — LANG-FULL E4)

The JVM backend now lowers the direct-literal `str_index` shape:

- `str_index` loads the direct `String` local and integer index, calls
  `java/lang/String.charAt(I)C`, and widens with `i2l` when the IIR destination
  is `i64`.
- Twig `(string-ref "ABC" 1)` now returns `66` on real `java` alongside the
  existing literal string length/equality/concat rows.
- Non-literal string values and byte-exact non-ASCII semantics remain follow-up
  representation work.

## [0.20.0] — 2026-06-27 (literal string metadata — LANG-FULL E4)

The JVM backend now lowers the literal `str_len`, `str_eq`, and `str_concat`
shapes:

- `str_len` loads the direct `String` local, calls
  `java/lang/String.length()I`, and widens with `i2l` when the IIR destination is
  `i64`.
- `str_eq` loads two direct `String` locals, calls
  `java/lang/String.equals(Ljava/lang/Object;)Z`, and widens with `i2l` when the
  IIR destination is `i64`.
- `str_concat` loads two direct `String` locals and calls
  `java/lang/String.concat(Ljava/lang/String;)Ljava/lang/String;`, producing a
  `String` local that the existing `str_len`/`str_eq`/`print_str` path can
  consume.
- This is enough for Twig `(string-length "HELLO")` and
  `(string=? "HELLO" "HELLO")` plus
  `(string-length (string-append "AB" "CDE"))` to run on real `java` while
  keeping the backend's representation a host `String`.
- Byte-oriented string operations (`str_index`) and non-literal string values
  remain rejected until the backend owns shared UTF-8 byte semantics.
- Tests assert both validator acceptance and the emitted
  `java/lang/String.length:()I`, `String.equals(Object):Z`, and
  `String.concat(String):String` method references.

## [0.19.0] — 2026-06-27 (string literal PRINT foothold — LANG-FULL E4 / BA4)

The JVM backend now lowers the first E4 string shape:

- `str_const` with an ASCII `Operand::Str` literal → `CONSTANT_String` loaded
  with `ldc`/`ldc_w` into a reference local.
- `print_str` → `getstatic java/lang/System.out` +
  `invokevirtual java/io/PrintStream.print(Ljava/lang/String;)V`.

This is deliberately narrower than full E4: byte-oriented string operations
(`str_len`, `str_index`, `str_concat`, `str_eq`) still fail closed until the JVM
representation owns the shared UTF-8 byte semantics. The validator now admits
only `str_const` + `print_str` and rejects the richer string algebra explicitly.

Verified by backend tests plus the `lang-aot` matrix row:
`10 PRINT "HELLO"` now runs on real `java` in the JVM column.

## [0.18.0] — 2026-06-23 (numeric conversions int ⇄ real — LANG-FULL E8 backend 4)

The three IIR numeric-conversion ops now lower to JVM bytecode, the fourth
backend (after VM/JIT, LLVM, WASM) to gain them and the prerequisite for
ALGOL's `entier` and integer↔real coercion:

| IIR op | JVM lowering |
|--------|--------------|
| `int_to_real` | `i2d` (0x87) / `l2d` (0x8A) — widen int→double, exact |
| `real_to_int_trunc` | `d2i` (0x8E) / `d2l` (0x8F) — truncate toward zero |
| `real_to_int_floor` | `invokestatic java/lang/Math.floor(D)D` then `d2i`/`d2l` — round toward −∞ |

The int vs long opcode form follows the operand's **value model** (the source's
own jtype for `int_to_real`, the dest slot's jtype for the narrowing ops), not
the `type_hint` — consistent with the dual-value-model rule the rest of the
backend obeys. `real_to_int_floor` has no single opcode, so it composes
`Math.floor` (round to −∞, still a double) with a bare narrowing (which now only
drops a `.0`).

**Trap divergence (documented — diverges from `lang-full-e8-numeric-conversions.md`
§7's uniform-trap recommendation; recorded in that spec's footnote ²):** the VM / LLVM / WASM backends
*trap* on NaN / ±∞ / out-of-i64-range inputs to `real_to_int_*`. The JVM's
`d2i`/`d2l` instead **saturate** (NaN→0, +∞→MAX, −∞→MIN) and never throw. For
every finite, in-range value — all the `entier`/coercion use case produces — the
two agree bit-for-bit, so the matrix cells (which exercise only such values)
match. Emitting a JVM range-check + `athrow` would require from-scratch
exception bytecode with no reusable precedent in this backend, so we take the
documented-divergence path.

Tests: emit-level coverage of the i32 (`i2d`/`d2i`) and i64 (`l2d`/`d2l`) value
models plus the `Math.floor` methodref, and `e8_conversions_round_trip_runs_on_real_java`
— an end-to-end run on real `java` of `floor(int_to_real(45) − 2.7) ⇒ 42`,
matching the LLVM/WASM/VM matrix-cell value.

## [0.17.1] — 2026-06-22 (fix: `global_load` into an i32 dest narrows with `l2i`)

The E6 `global_load` always read the 64-bit static field (`getstatic …:J`, a
long) and stored it with `emit_typed_store` — correct only for an `i64`/Long
dest. An `integer` ALGOL program concretised to **i32** has an `int` dest slot,
so `istore` of a long is a verifier type error (hidden in 0.17.0's e2e test by
`-Xverify:none`). The **E6 matrix proof** — which runs the real verifier — caught
it. Now `global_load` emits `l2i` before `istore` when the dest is narrower than
`long`, the mirror of the existing `i2l` widen on `global_store`. Regression
test `e6_global_load_into_i32_dest_narrows_with_l2i`.

## [0.17.0] — 2026-06-22 (typed module globals → static fields — LANG-FULL E6 layer 1)

`global_load` / `global_store` were a `LANG32b`-deferred `UnsupportedOp`
rejection. They now lower to JVM **static-field** access, so a function can
read/write a module-level global.

### Added
- **`global_load` / `global_store`** lowering:
  - `collect_global_fields` collects every distinct global name (read or written,
    first-seen order) → a `public static long G_N` field of the generated class.
    Field names are index-based (`G_0`, `G_1`, …) so an arbitrary source
    identifier can never form an invalid or colliding JVM field name. The fields
    are emitted in the class file's new `fields[]` table (`jvm-class-file` 0.2.0).
  - `global_load "g" -> %d` → `getstatic <this>.G_N:J ; lstore`.
  - `global_store "g", %v` → `lload (+ i2l if narrow) ; putstatic <this>.G_N:J`.
  - The name is an `Operand::Str` literal (never a register); a non-string /
    uncollected name is an `InvalidOperand` error.
  - Adds the `PUTSTATIC` (0xB3) opcode (GETSTATIC already existed for the BF tape).
- The class-file **serializer now emits the `fields[]` table** (`fields_count` +
  each `field_info`, with name/descriptor resolved to their CP Utf8 indices that
  `add_fieldref` already registers).

### Verified
- `tests/test_backend.rs`: the lowered class declares `static long G_0` and
  `bump` carries `getstatic`/`putstatic`; and **end-to-end on real `java`** a
  cross-function global program (`compute` seeds `g`; a separate `bump`
  reads/increments/writes it) prints **42**.

## [0.16.0] — 2026-06-21 (arrays → native JVM `int[]`/`long[]`/`double[]` — LANG-FULL E5 PR-3)

The four E5 array opcodes now lower to **real JVM primitive arrays**, so ALGOL
1-D arrays run on `java` (not just the VM/JIT):

| IIR op | JVM bytecode |
|--------|--------------|
| `alloc_array dest <- count` (`array<T>`) | `<count>; [l2i]; newarray T_<elem>; astore dest` |
| `array_get dest <- handle, idx` | `aload handle; <idx>; [l2i]; <T>aload; store dest` |
| `array_set handle, idx, val` | `aload handle; <idx>; [l2i]; <val>; <T>astore` |
| `array_len dest <- handle` | `aload handle; arraylength; [i2l]; store dest` |

- The element width comes from `T`: `int[]`/`long[]`/`float[]`/`double[]` with the
  matching `newarray` type code (`T_INT`/`T_LONG`/`T_FLOAT`/`T_DOUBLE`) and typed
  `*aload`/`*astore`. The **handle** is a reference local (`JvmType::Ref`,
  `aload`/`astore`, one slot) — `iir_type_to_jvm` now maps `array<T>` → `Ref`.
- **Bounds-checked for free**: every `*aload`/`*astore` does the JVM's native
  bounds check, so an out-of-range index throws `ArrayIndexOutOfBoundsException` —
  exactly E5's trap semantics, no explicit compare/branch emitted.
- No StackMapTable concern: the backend already targets Java 5 (version 49), which
  needs no verification frames even with the existing loop branches.
- Validation: `array<T>` type hints and the four ops already pass (they aren't
  `str`/`ref<`/`any`, and aren't blocklisted); added a regression test asserting it.
- 5 new unit tests (handle→Ref typing, element-opcode table, `int[]` emits
  `newarray`/`iastore`/`iaload`/`arraylength`, `double[]` emits `dastore`/`daload`,
  validation passes). Verified end to end: the `lang-aot` ALGOL sum-of-squares
  matrix `Prog` now runs on **real `java`** → exit 55.

Pairs with `lang-aot` 0.101.0, whose `concretize_scalar_any_for_jvm` narrows
`array<i64>`→`array<i32>` in lockstep with the scalar `i64`→`i32` narrowing so the
`newarray` element width and the `iaload`/`iastore` element opcodes agree.

## [0.15.0] — 2026-06-20 (f64 constants + comparisons — LANG-FULL E3; ALGOL reals run on the JVM)

The backend already lowered f64 *arithmetic* (`dadd`/`dmul`/…) and typed `f64`
locals as `Double` (two slots, `dstore`/`dload`), but two gaps made a real
*program* fail JVM verification (empty output), so ALGOL reals ran only on the
VM/JIT/LLVM/WASM. Both fixed:

### Fixed — non-0/1 `f64` constants pointed at constant-pool index `#0`

`emit_dconst` had a "placeholder for v1": `0.0d`/`1.0d` used the `dconst_0`/
`dconst_1` short forms, but any *other* double emitted `ldc2_w #0000` — the
unused phantom slot, not a real `CONSTANT_Double`. So a `real` literal like
`2.5` loaded garbage and the verifier rejected the class. `emit_dconst_cp` now
interns the value via a new `ConstantPoolBuilder::add_double` (which reserves
the two pool slots a `Double` occupies, mirroring `add_long`) and emits
`ldc2_w <real index>`.

### Fixed — `f64` comparisons fell through to the integer `if_icmp` path

The comparison dispatch handled only `Long` (`lcmp` + `ifXX`) and `int`
(`if_icmp*`). A `Double` operand fell into the `int` branch, which `iload`ed a
two-slot double as a single 32-bit int and used `if_icmpne` → the verifier
rejected it. A new `Double` branch emits `dcmpl`/`dcmpg` (→ int -1/0/1) then the
same unary `ifXX` the long path uses. `dcmpg` is used for `>`/`>=` (NaN → false)
and `dcmpl` for the rest, matching javac's convention. The boolean result is an
`int` (unchanged), stored with `istore`.

**Verified by RUNNING on real `java`**: the two ALGOL real programs
(`r := 2.5 * 2.0; if r = 5.0 …` → exit 42; `r := 7.0 / 2.0; if r < 4.0 …` →
exit 1) now execute on the JVM matrix column. Five new structural tests
(double pool entry, short forms, dcmp-not-if_icmp, dcmpg for `>`). Integer/long
programs are unaffected (the new branches key on `JvmType::Double`).

## [0.14.0] — 2026-06-16 (narrow-width mask on the LONG model — LANG-FULL O2)

### Fixed — a narrow-width op over `long` operands now masks correctly

The E2 width mask (`emit_jvm_width_mask`) assumed the **int** model: a scalar exit-code
program is concretized to `i32` (`lang_aot::concretize_scalar_any_for_jvm`), so a narrow
`u4`/`u8`/`u16` op runs on `int` and the mask is `iconst <m>; iand`. But a **printing**
program (Oct's `out`, Dartmouth BASIC's `PRINT`) keeps the `i64`/`long` model so its value
can reach `print_i64`. Oct's only integer type is `u8`, so a printing Oct program (`out(1,
200 + 100)`) emits a narrow-hinted `add`/`~` over **`long`** operands — and the bare `u8`
hint mapped it to `iadd`/`iand`, an unverifiable mix over longs. Result: every Oct printing
program with arithmetic returned empty on real `java` (`got ""`).

Now a narrow op whose operands ride the long model stays on the long model: `ladd`/`lxor`/…
for the op, and `i2l; land` for the mask (the masks are positive, so widening zero-extends).
`narrow_op_over_long` keys this off the actual operand types — used in both `build_type_map`
(so the dest gets a `Long` slot) and the op loop (so the opcode and mask agree). Concretized
int-model programs are unchanged (operands are `int` → the int path). Fixes Oct
`200u8 + 100u8 = 44` and `~0u8 = 255` on real `java`. New tests
`narrow_op_over_long_operands_stays_long`, `narrow_op_over_int_operands_stays_int`.

## [0.13.3] — 2026-06-16 (Oct `&&`/`||` run on the JVM — BA-JVM-1 follow-through)

### Fixed — a `mov` bridges int↔long when the dest slot width differs

With BA-JVM-1 (0.13.2) typing comparison dests `int`, Oct's short-circuit `&&`/
`||` exposed the next link in the same chain: it `mov`s an `int` (bool) comparison
result into a `long`-typed accumulator (Oct keeps the i64 value model — it
`out`-prints, so it skips the scalar concretize-to-i32 pass). The `mov` handler
stored using the SOURCE type, so it `istore`d an int into the `long` accumulator
slot, leaving the slot's second half uninitialized — a later `lload` of the
accumulator (the `jmp_if_false` guard) tripped `VerifyError: uninitialized
register pair 5/6`.

Fix: the `mov` handler now stores with the DEST slot's type, inserting `i2l`
(int→long) or `l2i` (long→int) when the source and dest widths differ. The
Int/Bool-constant `mov` cases widen with `i2l` into a long dest too. **Verified
on real `java`**: Oct's `&&` short-circuit (`1==2 && side()` → `9`, side NOT
called) and `||` (`1==1 || side()` → `7`) now run on the JVM; both added to the
matrix JVM column. With this, EVERY `lang_matrix.rs` program runs on all 7
backends. New test `mov_int_bool_into_long_accumulator_widens_with_i2l`; full
matrix + jvm consumers green.

## [0.13.2] — 2026-06-16 (LANG-FULL BA-JVM-1 — BASIC `IF`/`FOR` run on the JVM)

### Fixed — a comparison's dest slot is `int`, so an i64-operand guard verifies

Dartmouth BASIC's `IF`/`FOR` control-flow programs were excluded from the JVM
matrix column: real `java` rejected the class with `VerifyError: Accessing value
from uninitialized register pair`. (A print with no branch, and a loop with no
print — Nib's `for` — each worked; only the *combination* in BASIC failed.)

Root cause: `build_type_map` typed a comparison's dest slot from its `type_hint`,
which carries the **operand** width, not the result width. A comparison ALWAYS
produces a 0/1 **`int`** (it is stored with a bare `istore`), but a comparison
over `i64` operands had `type_hint = "i64"`, so the dest slot was typed `Long`.
The later `jmp_if_false` then read that slot with the long guard
(`lload; lconst_0; lcmp; ifeq`) — reading the uninitialized second half of a
"long" the comparison only `istore`d → the verifier's "uninitialized register
pair". Nib's loops are unaffected because scalar Nib is concretized to `i32`
(`lang_aot::concretize_scalar_any_for_jvm`); BASIC **prints**, so it keeps the
wide i64 value model and exposed the mismatch.

Fix: `build_type_map` now types any comparison op (`cmp_eq`/`ne`/`lt`/`le`/`gt`/
`ge`, via the new `is_comparison_op`) dest as `JvmType::Int`, regardless of the
operand-width hint. The slot is then `int`, the `istore` is consistent, and
`jmp_if_false` reads it with `iload; ifeq`. **Verified on real `java`**: the
BASIC `FOR` sum (`1..5 → 15`) and `IF` branch (`A>5 → 7`) now run on the JVM;
both are added to the `lang-aot` matrix JVM column. New regression test
`ba_jvm_1_i64_cmp_into_jmp_if_uses_int_guard`. No other backend or program
affected (full matrix + jvm consumers green).

## [0.13.1] — 2026-06-16 (LANG-FULL E2 — revert the long model back to the int model)

### Fixed — narrow types use the JVM `int` model, not the v0.13.0 `long` model

v0.13.0 moved narrow unsigned types (`u4`/`u8`/`u16`/`u32`) to a `long` register
model, reasoning (as for wasm) that a real frontend's `i64` operands would
otherwise meet an `int` op. **That is wrong on the JVM.** A scalar program
reaches this backend through `lang_aot::concretize_scalar_any_for_jvm`, which
narrows the module's `i64`→`i32` *before* lowering (the in-repo `jvm-simulator`
is a 32-bit machine and a scalar entry must `ireturn`). It leaves the
narrow-unsigned op alone. So the long model produced a module where the consts
and return were `int` but the narrow op was `long` — **unverifiable bytecode**:
`istore` int consts feeding an `lmul`, and `lreturn` from an `int`-returning
method. A real `java` rejected it, so the Nib `u8` integration proof returned
`None` on the JVM column (surfaced only when the Nib frontend actually emitted
narrow `type_hint`s — the v0.13.0 tests built self-consistent narrow modules
that bypass `concretize`).

This release reverts to the **int model**:

- `iir_type_to_jvm`: `u4`/`u8`/`u16`/`u32` → `JvmType::Int` (`u4` stays
  recognised). `i64`/`u64` remain `Long`.
- `type_to_jvm_descriptor`: those → `I`.
- `emit_jvm_width_mask`: `sipush/iconst/ldc <mask>; iand` (int), for `u4`/`u8`/
  `u16` (`u32` self-wraps via the 32-bit `int` op).
- Shifts drop the `l2i` count narrowing (the count is an `int` again).

Because `concretize` narrows the consts/return to `i32`, the whole scalar module
is now consistently `int`: `sipush 200; sipush 100; iadd; sipush 255; iand;
ireturn` → `44`. **Verified on real `java`**: a launcher invoking the lowered
`main()` prints `44` for `200u8 + 100u8`. Tests reverted to the int-mask shape;
new regression test `e2_concretized_u8_shape_is_all_int` builds the
post-`concretize` shape (`const i32; add u8; ret i32`) and asserts the bytecode
has the `iand` mask and **no** `ladd`/`lreturn`. Full matrix + jvm consumers
green.

(wasm genuinely keeps `i64` operands — no `concretize`-to-i32 there — so its i64
register model, v0.15.0, stands. The CIL backend is uniformly int32 and also
needs no long model. The JVM is the odd one out only because of the
32-bit-simulator concretization.)

## [0.13.0] — 2026-06-16 (LANG-FULL E2 integration — compute-wide + mask)

### Changed — narrow unsigned types ride the JVM `long` register model

The v0.12.0 E2 masking typed a narrow op at the JVM `int` width (`u8` → `iadd`)
and masked with `iand`. That is only valid when the **operands** are also `int`.
A real frontend's value model isn't: Nib (and the other LANG languages)
materialise every `const`/`let`/`ret` as `i64` (= JVM `long`) for module
uniformity, carrying the narrow width *only on the arithmetic op*. So a Nib `u8`
add emitted `iadd` over two `long` locals → **JVM bytecode verification error**
(type mismatch on the operand stack). The v0.12.0 structural tests never caught
it — they built self-consistent narrow-width modules (every operand `u8` too).

The fix makes narrow **unsigned** integers (`u4`/`u8`/`u16`/`u32`) use the
`long` register model, exactly like the vm-core/jit-core/LLVM/native/wasm
backends:

- `iir_type_to_jvm`: `u4`/`u8`/`u16`/`u32` → `JvmType::Long` (were `Int`). Signed
  narrow (`i8`/`i16`/`i32`) and `bool` keep `Int`. **`u4` is newly recognised**
  (before E2, Nib widened it to i64 first, so it never reached this backend).
- `type_to_jvm_descriptor`: those types → `J` (were `I`), so a method that
  takes/returns one has the right descriptor.
- Op selection (`instr_jtype` now `Long`) emits the long opcodes — `ladd`/`lsub`/
  `lmul`/`ldiv`/`lrem`/`land`/`lor`/`lxor`/`lshl`/`lshr`/`lneg`, long `not` as
  `ldc2_w -1; lxor`, and the load/store as `lload`/`lstore` — over the long
  operands.
- `emit_jvm_width_mask`: `ldc2_w <mask>; land` (was `…; iand`), pushing the mask
  from the constant pool as a `Long` so the wide values (`0xFFFF`, `0xFFFFFFFF`)
  work — and it now covers `u32` (within a 64-bit register a 32-bit op no longer
  self-wraps).
- Shifts: the JVM shift count is always an `int`, so a now-`long` narrow shift
  count is narrowed with `l2i` before `lshl`/`lshr` (a bare `iload` of a long
  slot would be a verify error).

So `200u8 + 100u8` wraps to `44` **with long operands** — the shape a frontend
actually emits. The full `lang-aot` matrix (real `java`) and all jvm consumers
stay green — the change is a no-op for every i64/u64 program. New structural
test `e2_u8_op_over_i64_operands_is_long` covers the regression; the v0.12.0 E2
tests were updated to the long-mask shape and `e2_u32_add_masks` added.

This is the second of the 3 stack-backend reworks (wasm ✅, jvm, then cil) the E2
Nib integration needs.

## [0.12.0] — 2026-06-14 (LANG-FULL E2 — register width & wrap, backend 4 of 6)

### Added — narrow-width arithmetic wraps mod-2ⁿ on the JVM

JVM `int` arithmetic (`iadd`/`imul`/…) wraps mod-2³², so `u32`/`i32` were
already correct.  But `u8`/`u16` left a full 32-bit result, so a lowered
`200u8 + 100u8` gave `300`, not `44`.

`emit_jvm_width_mask` now appends `iconst/sipush/ldc <mask>; iand` after a
narrow-width (`u4`→0xF, `u8`→0xFF, `u16`→0xFFFF) `add`/`sub`/`mul`/`div`/`mod` /
`neg` / `and`/`or`/`xor` / `not` / `shl`/`shr` result — mirroring vm-core's
`mask_result`, jit-core's `MASK_WIDTH`, the wasm `i32.and`, and the byte-tape
`baload`+mask precedent.  The mask uses a **positive constant + `iand`**, not
`i2b`/`i2s` (those sign-extend, giving a signed byte — wrong for the unsigned
narrow types the LANG-FULL frontends use).  `0xFFFF` exceeds `sipush` range, so
it loads from the constant pool via `emit_iconst_cp`.  `u32`/`i32` need no mask;
`i64`/`u64`/floats are unchanged.

Tests assert the lowering emits the `sipush 255; iand` byte mask for a `u8`
add / not / shl and omits it for `i64`/`u32`.  (The executed cross-backend JVM
proof lands in the E2 integration PR via lang-aot's real-`java` `run_jvm`.)

## [0.11.0] — 2026-06-12 (LANG-MATRIX LM-J Brainfuck — byte-tape ops on the JVM)

Adds the lowering Brainfuck needs to run on the JVM backend — the last code-gen
gap in Brainfuck's row after LLVM (LM-L) and WASM (LM-W). Verified by RUNNING
`++++++++[>++++++++<-]>+.` on real `java` in `lang-aot/tests/lang_matrix.rs`: it
prints `A`.

The backend already had the *raw* BF-frontend tape ops (`load_mem`/`store_mem` →
`baload`/`bastore` over a static `env/BFRuntime.__tape : [B`) and the
`putchar`/`getchar` host calls, but `lower_brainfuck_for_aot` rewrites the tape
into the *lowered* `alloc_bytes`/`load_byte`/`store_byte` form (the same ops the
LLVM/WASM/native backends consume) and widens the value model — which the JVM
backend didn't yet handle.

### Added

- **`alloc_bytes dest <- size`** → no bytecode. The JVM tape is the host class's
  pre-allocated static `byte[] __tape`, so there is nothing to allocate at
  runtime; `dest` (the BF tape base) is never materialised because the byte ops
  `getstatic` the tape directly.
- **`load_byte dest <- base, idx`** → `getstatic __tape`, load the index
  (`l2i`-narrowed if it is an `i64`), `baload`, `& 0xFF` (mask the sign-extended
  byte back to an unsigned cell), then `i2l`-widen if `dest` is `i64`. The base
  operand is the static tape, so it is ignored.
- **`store_byte base, idx, val`** → `getstatic __tape`, load index + value
  (`l2i`-narrowed if `i64`), `bastore` (stores `val & 0xFF`, giving BF's 8-bit
  cell wrap-around for free). Rejected if it carries a `dest`.

### Fixed (i64-widening ripple)

- **i64 branch conditions**: `jmp_if_true`/`jmp_if_false` hardcoded `iload` +
  `ifne`/`ifeq`, which assume a 32-bit condition. An `i64` guard (the widened
  Brainfuck loop guard — when the value model is *not* concretised back to i32)
  now reduces to an int with `lload; lconst_0; lcmp` before the branch; `iload`ing
  a long would read only one of its two local slots — a verify error. The width is
  taken from the condition register's declared `JvmType`.

Four new tests in `tests/test_backend.rs` cover the i32 + i64 tape lowerings, the
`store_byte`-with-dest rejection, and the i64-guard `lcmp` path.

## [0.10.0] — 2026-06-09 — large-`int` constants via the constant pool (McCarthy W5a / F6)

### Fixed

- **`int` literals beyond ±32767 now lower correctly.** `emit_iconst`'s
  out-of-`sipush`-range path emitted `ldc 0` — a reference to the *reserved*
  constant-pool slot 0 — which crashed real JVMs at class load
  (`constantTag.cpp ShouldNotReachHere`). Added `emit_iconst_cp`, which appends a
  `CONSTANT_Integer` entry (`ConstantPoolBuilder::add_integer`) and emits
  `ldc`/`ldc_w` (the `LDC_W` 0x13 opcode for a CP index > 255). Every
  *user-constant* call site (a `const`, a `mov`/`ret` immediate, a `call`
  argument) now routes through it; the old invalid path is `debug_assert`-guarded.
  Structural indices (field numbers, slot/arg counts) stay on `emit_iconst` — they
  are always small.

### Enabled

- **McCarthy symbols (F6) on the JVM.** A symbol interns to an id in a high
  reserved range (`SYMBOL_ID_BASE = 2²⁹`), exactly the large-`int` const this
  fixes — so `(EQ 'X 'X)` → T, `(EQ 'X 'Y)` → nil, `(QUOTE X)` → its id now run on
  a real JVM.

## [0.9.0] — 2026-06-09 — lisp predicates `pair?`/`not`/`equal?` (McCarthy W4)

### Added

- **`call_builtin` lowering for the McCarthy predicates** (F3–F5), the JVM
  counterparts of the wasm `ref.test`/`i32.eqz`/`i31.get_s`+`i32.eq`:
  - `pair?`  → `aload ; instanceof [Ljava/lang/Object; ; istore` — a cons is an
    `Object[]`, an atom an `Integer`, nil `null` (so `instanceof` is 0/1).
  - `not`    → `iload ; iconst_1 ; ixor ; istore` (logical not of a 0/1 bool).
  - `equal?` → unbox both `Integer`s (`checkcast` + `intValue`) then
    `if_icmpne`-synthesised 0/1 — `EQ` on atoms is integer equality.
  Added `pair?`/`not`/`equal?` to `CALL_BUILTIN_SUPPORTED_NAMES`, the `INSTANCEOF`
  (0xC1) opcode, and `builtin_dest`/`builtin_arg` operand helpers. With the
  already-lowered `jmp_if_false`/`is_null`, McCarthy `ATOM`/`EQ`/`COND` now run on
  a real JVM: `(ATOM 5)`→1, `(ATOM (CONS 1 2))`→0, `(EQ 5 5)`→1, `(COND …)`.

## [0.8.0] — 2026-06-09 — `box`/`unbox` + `ref<any>` (McCarthy W3b)

### Added

- **`box` / `unbox` lowering** — the managed uniform-reference value model's
  atom boxing, the JVM counterpart of the wasm backend's `i31ref`:
  - `box`  → `iload ; invokestatic java/lang/Integer.valueOf(I)Ljava/lang/Integer; ; astore`
  - `unbox`→ `aload ; checkcast java/lang/Integer ; invokevirtual Integer.intValue()I ; istore`
  These are the *same* backend-agnostic IIR ops the wasm path consumes — the
  structural representation pass emits them, each backend lowers them — so a
  McCarthy cons program (`(CAR (CONS 7 9))` → 7) now runs on a real JVM. Removed
  `box`/`unbox` from `UNSUPPORTED_OPS`.
- **`ref<any>` type** — maps to `JvmType::Ref` (descriptor `Ljava/lang/Object;`):
  a boxed lisp value is an `Integer` (atom) or `Object[]` (cons cell).

(cons cells were already `Object[]` allocations via `alloc`/`field_*` for
`ref<LispyPair>`; this release adds the atom boxing that lets integers live in
those cells and be read back out.)

## [0.7.0] — 2026-06-01 (G3 — `print_i64` host import → `env/BasicRuntime.println(J)V`)

### Added — `call_builtin "print_i64"` whitelisted and lowered

Companion to `iir-to-wasm` v0.8.0 (gap G2 of the
[multi-language backend plan][plan]).  The wasm backend lets BASIC's
`PRINT` reach real wasm bytecode by routing `call_builtin "print_i64"`
to the `env.__print_i64` host import.  This release does the same for
the JVM backend: routes it to `invokestatic env/BasicRuntime.println(J)V`,
so a BASIC program lowered through IIR-to-JVM produces a valid `.class`
that runs against a launcher providing one extra host class.

#### Validator changes (`src/validate.rs`)

* `CALL_BUILTIN_SUPPORTED_NAMES` widened from `["putchar", "getchar"]`
  to `["putchar", "getchar", "print_i64"]`.  Everything outside the
  whitelist still fails with `UnsupportedOp` — defence in depth unchanged.

#### Lowering changes (`src/lower.rs`)

* Added `const BASIC_RUNTIME_CLASS: &str = "env/BasicRuntime"`.  We
  deliberately pick a separate host class from `env/BFRuntime` because
  BASIC's I/O model (line/value, mostly numeric) differs from
  Brainfuck's (byte-stream), and the JVM launcher should be able to
  stub or provide them independently.
* New `"print_i64"` arm in the `call_builtin` match:
  ```
  srcs = [Var("print_i64"), Var(val: i64)]   dest = None
  →
  lload val_slot
  invokestatic env/BasicRuntime.println(J)V
  ```
  Uses `emit_lload` rather than `emit_iload` because i64 occupies a
  long slot; descriptor `(J)V` matches one long arg, void return.

#### Tests added (`tests/test_backend.rs`)

* `g3_validator_accepts_print_i64`
* `g3_validator_still_rejects_unknown_builtin`
* `g3_lowers_print_i64_to_invokestatic`
* `g3_constant_pool_has_basicruntime_println_methodref`
* `g3_print_i64_class_serializes_with_cafebabe_magic`

All five exercise the validator + lowerer + serializer path and assert
on the 0xCAFEBABE byte prefix to confirm a structurally valid `.class`.

[plan]: ../../../specs/MULTILANG-BACKEND-PLAN.md

## [0.6.0] — 2026-05-26 (Validator accepts `ref<any>` for `field_load`)

### Changed — `ref<any>` widens the supported reference types

Companion to Twig path-A increment 6c.  The Phase 2 heap-lowering
convention is `field_load dest, pair, idx [ref<any>]`.  JVM lowers
this to `aaload`, which returns `Object` — the same type cons-cell
fields are declared as in the `Object[2]` Phase 2 representation.

This release widens the JVM validator's UnsupportedType check: in
addition to `ref<LispyPair>` (the `Object[2]` cons cell), `ref<any>`
is now accepted (mapping to `Object`).  All other `ref<X>` continue
to be rejected.

No lowering changes — `aaload` already returns the right type for
both `ref<LispyPair>` (Object[]) and `ref<any>` (Object).

## [0.5.0] — 2026-05-22 (Brainfuck — `byte[]` tape + I/O via env/BFRuntime)

### Added — Brainfuck `load_mem` / `store_mem` / `call_builtin` lowering

Stage 2 of 4 for the BF→{wasm,jvm,clr,beam} story.  Mirrors the WASM PR
(iir-to-wasm 0.4.0) for the JVM target.  Lets BF's IIR — including
`load_mem`, `store_mem`, and `call_builtin "putchar"`/`"getchar"` —
flow through the same universal `iir-to-jvm-class-file` backend that
Twig, BASIC, Oct, and Nib already use.

#### Validator changes

- `load_mem` and `store_mem` removed from `UNSUPPORTED_OPS` (previously
  hard-rejected).  Both lower to JVM `baload` / `bastore` over a
  host-provided byte array — see `BF_RUNTIME_CLASS` below.
- `call_builtin` is now **conditionally** accepted: the builtin name
  carried in `srcs[0]` as `Operand::Var` must be in the new
  `CALL_BUILTIN_SUPPORTED_NAMES` whitelist.  Today's whitelist covers
  Brainfuck's two I/O builtins (`putchar`, `getchar`); extending it
  takes three steps documented in the constant's doc comment.
- Unknown builtin names still produce a clear `UnsupportedOp` error
  with the rejected name and the whitelist included.

#### Lowering changes

- New `BALOAD` / `BASTORE` opcode constants (0x33 / 0x54) for JVM byte
  array access.
- New `BF_RUNTIME_CLASS` constant `"env/BFRuntime"` — the host helper
  class providing the tape and I/O methods.  Picking a fixed host
  class keeps BF-compiled `.class` files self-contained: no `<clinit>`
  required on the BF side, no per-program tape size baked into the
  bytecode, and the host can dial the tape size without recompiling.
- New `emit_instr` arms:
  - `load_mem v ptr` → `getstatic env/BFRuntime.__tape : [B; iload ptr;
    baload; sipush 0x00FF; iand; istore v`.  The `sipush 0x00FF; iand`
    masks the sign-extension that `baload` performs, so the int result
    is properly `0..=255` (matching BF's unsigned u8 cell semantics).
  - `store_mem ptr v` → `getstatic env/BFRuntime.__tape : [B; iload ptr;
    iload v; bastore`.  `bastore` truncates the int value to a byte
    automatically, matching BF's u8 wraparound.
  - `call_builtin "putchar" v` → `iload v; invokestatic
    env/BFRuntime.putchar(I)V`.
  - `call_builtin "getchar" -> v` → `invokestatic
    env/BFRuntime.getchar()I; istore v`.

#### Host class contract

The host (Java runtime / launcher) must provide a class with binary name
`env/BFRuntime` containing:

| Symbol                  | JVM descriptor | Purpose                           |
|-------------------------|----------------|-----------------------------------|
| `public static byte[] __tape` | `[B`     | The BF tape (typically 30,000 B). |
| `public static void putchar(int)` | `(I)V` | Write one byte to stdout.         |
| `public static int getchar()`     | `()I`  | Read one byte from stdin; convention is 0 / -1 on EOF. |

This is the JVM analog of the WASM backend's `env` import namespace —
same pattern, different ABI.

### Tests

- 5 new validator unit tests covering the new acceptance:
  `load_mem_accepted_for_bf`, `store_mem_accepted_for_bf`,
  `call_builtin_putchar_accepted`, `call_builtin_getchar_accepted`,
  `call_builtin_unknown_name_rejected`.
- The existing `unsupported_ops_rejected` test updated: `load_mem`,
  `store_mem`, `call_builtin` removed from the unconditional-reject
  list with a comment pointing to the new tests.
- 43 lib + 86 integration tests pass.
- 4 new BF→JVM e2e tests in `brainfuck-iir-compiler/tests/jvm_e2e.rs`
  exercise the full chain from source to `.class` bytes.

### Compatibility

- Non-BF frontends (Twig, BASIC, Oct, Nib) are unchanged — they don't
  emit `load_mem` / `store_mem` or `call_builtin`, so the new code
  paths are only reached for BF.
- Modules without BF features get no `env/BFRuntime` constant-pool
  entries, preserving binary equivalence with pre-0.5.0 output for
  every non-BF caller.

## [0.4.1] — 2026-05-13

### Fixed (Multi-backend demo — fib(10)=55)

- **`"mov"` opcode support** — added handling for the `mov` IIR instruction
  (pre-lowered form of `call_builtin "_move"`).  The lowerer now emits the
  appropriate load + store sequence for Long / Int slots.
- **Long arithmetic for integer parameters** — parameters typed as `"i64"`
  (after the `fixup_control_flow_types` Pass 0 normalization) now use
  `lload`/`lstore` instead of `iload`/`istore`, preventing JVM verifier
  errors (`Bad local variable type`).
- **Long comparison** — `cmp_lt`/`cmp_gt`/`cmp_le`/`cmp_ge` now emit
  `lcmp` + conditional branch (not `if_icmp*`) when operands are `Long`.
  The `emit_long_compare` helper sequences `lcmp; ifXX 7; iconst_1; goto 4;
  iconst_0` to produce a boolean result.
- **`emit_lconst` fixed** — values 2–127 are now synthesised with
  `iconst_N; i2l` (or `bipush; i2l` / `sipush; i2l`) instead of an
  invalid `ldc2_w #0` placeholder that caused `VerifyError`.
- **Class file version 49** — downgraded from Java 8 (52) to Java 5 (49)
  to use the old type-inferencing verifier, removing the requirement for
  `StackMapTable` attributes in branching methods.

## [0.4.0] — 2026-05-12

### Added (LANG36 — JVM Closure Lowering)

This release promotes the JVM backend from "reject closures with ClosureOpcode"
to a full `long[]`-based dispatch-table implementation of first-class closures.

#### Closure representation

A JVM closure is a **`long[]` array** where `closure[0]` holds the function
dispatch index and `closure[1..]` holds the captured values (as `long`).
Integer captures (`i32`, `u32`, `bool`) are sign-extended to `long` via `i2l`;
`i64`/`u64` captures are stored directly.  Float captures (`f32`, `f64`) are
deferred to LANG38 and still produce a `ClosureOpcode` error.

#### `__callClosure` dispatch method

When a module contains any `alloc_closure` instruction, the lowering pass
generates a synthetic `static long __callClosure(long[] closure, long[] args)`
method.  It reads `closure[0]` as a dispatch index and uses a chain of
`lcmp` / `ifeq` branches — one branch per closure-eligible function — to
reconstruct the correct static call.  Dispatch indices are assigned
alphabetically (deterministic byte-identical output).

#### New JVM opcodes emitted

| Opcode     | Byte   | Description                                         |
|------------|--------|-----------------------------------------------------|
| `NEWARRAY` | `0xBC` | Allocate primitive array; operand `0x0B` = `T_LONG` |
| `LALOAD`   | `0x2F` | Load `long` from `long[]`                           |
| `LASTORE`  | `0x50` | Store `long` into `long[]`                          |
| `LCMP`     | `0x94` | Compare two longs (`-1`, `0`, or `1`)               |
| `L2I`      | `0x88` | Long → int narrowing conversion                     |
| `I2L`      | `0x85` | Int → long sign-extending conversion                |

#### `alloc_closure` lowering

```text
dest = alloc_closure(Str("fn_name"), Var(cap0)) : "closure"
→  iconst_2; newarray T_LONG
   dup; iconst_0; ldc2_w fn_idx; lastore   (closure[0] = dispatch_idx)
   dup; iconst_1; iload cap0_slot; i2l; lastore  (closure[1] = cap0)
   astore dest_slot
```

#### `call_closure` lowering

```text
dest = call_closure(Var(handle), Var(arg0)) : "any"
→  aload handle_slot
   iconst_1; newarray T_LONG
   dup; iconst_0; lload arg0_slot; lastore  (args[0] = arg0)
   invokestatic ClassName.__callClosure([J[J)J
   lstore dest_slot
```

#### Validator changes

- `alloc_closure` with non-float captures → accepted (no longer `ClosureOpcode`).
- `call_closure` → accepted.
- `alloc_closure` with `f32`/`f64` capture type hints → still emits `ClosureOpcode`
  (deferred to LANG38).

#### `serialize_jvm_class_file`

New public function `serialize_jvm_class_file(class_file: &JvmClassFile) -> Vec<u8>`
serializes a `JvmClassFile` to a valid `.class` byte stream (JVMS §4).
Used by the real-JVM round-trip test.

#### Tests

- `lang36_alloc_closure_accepted_by_jvm_validator`
- `lang36_call_closure_accepted_by_jvm_validator`
- `lang36_float_closure_still_rejected`
- `lang36_alloc_closure_emits_newarray`
- `lang36_alloc_closure_emits_lastore`
- `lang36_call_closure_emits_invokestatic_dispatch`
- `lang36_dispatch_method_generated`
- `lang36_dispatch_method_contains_lcmp`
- `lang36_real_jvm_closure_adder` — compiles a two-function module, serializes
  to a `.class` file, runs with `java -Xverify:none`, asserts output is `7`.
  Gated by `java_available()`.

---

## [0.3.0] — 2026-05-12

### Added (LANG35 — Closure Backend Integration)

#### Improved `ClosureOpcode` validator error

- `validate_for_jvm` now emits a dedicated `ClosureOpcode` error message
  (format: `"[fn_name] ClosureOpcode: alloc_closure/call_closure require the
  BEAM backend — JVM does not support heap-allocated closures"`) when it
  encounters `alloc_closure` or `call_closure`.
- Previously these fell through to the generic `UntypedInstruction` path;
  the closure check now runs first to give a more actionable error message.

#### Tests

- `lang35_alloc_closure_closure_opcode_error`: asserts `validate_for_jvm`
  returns an error containing "ClosureOpcode" for a module with `alloc_closure`.
- `lang35_call_closure_closure_opcode_error`: same for `call_closure`.
- `lang35_closure_opcode_error_not_untyped`: asserts the error does NOT
  contain "UntypedInstruction".

---

## [0.2.0] — 2026-05-11

### Added (LANG32 — Global Variables and I/O)

#### I/O support

- `io_out %v` → `getstatic java/lang/System.out` (Ljava/io/PrintStream;) +
  `lload <slot>` + `invokevirtual java/io/PrintStream.println(J)V`.
- Added `INVOKEVIRTUAL: u8 = 0xB6` and `GETSTATIC: u8 = 0xB2` bytecode
  constants.
- Added `add_fieldref` to `ConstantPoolBuilder`.

#### Global variables (LANG32b — deferred)

- `global_load` and `global_store` return `UnsupportedOp` with a clear
  LANG32b tracking note.  Full JVM static-field globals require extending
  `JvmClassFile` with a `fields: Vec<JvmFieldInfo>` table and adding
  `getstatic`/`putstatic` sequences; tracked in a follow-up PR.

#### Exhaustiveness fixes

- `Operand::Str` arms added to all `match` blocks in `lower.rs` (const,
  ret, call argument loops).

---

## [0.1.0] — 2026-05-11

### Added

- `validate::validate_for_jvm(module: &IIRModule) -> Vec<String>` — pre-flight
  validation pass that rejects modules containing JVM-incompatible instructions
  or types before any lowering starts. Catches:
  - Empty module (no functions)
  - Empty function (function with no instructions)
  - Untyped instructions (`type_hint == "any"` or `"polymorphic"`)
  - Unsupported types (`"str"`, `ref<…>`)
  - Unsupported opcodes (`call_builtin`, `io_in`, `io_out`, `cast`, memory ops,
    GC ops, `safepoint`)
  - Float type hints and float constants are **supported** (unlike the BEAM
    backend), since the JVM has native `fload`/`dload`/`fadd`/`dadd` opcodes.

- `lower::IIRJvmConfig` — lowering configuration: `class_name` String.
  Implements `Default` (uses `"IIRModule"`) and `new(class_name)`.

- `lower::IIRJvmError` — typed error variants:
  `ValidationFailed`, `UnsupportedOp`, `UnsupportedType`, `UndefinedLabel`,
  `UndefinedVariable`, `InvalidOperand`. Implements `Display` and `std::error::Error`.

- `lower::lower_iir_to_jvm(module: &IIRModule, config: &IIRJvmConfig) -> Result<JvmClassFile, IIRJvmError>` —
  two-pass lowering algorithm:
  - Pass 1 per function: assign JVM local variable slots to params (0..N-1)
    then walk dests and src Var operands in order for locals (N..).
  - Pass 2: emit raw JVM bytecode (Vec<u8>) per method using emit_* helpers.
  - Build `JvmClassFile` directly (Java 8, version 52.0).
  - Two-pass backpatching for forward label/jump references.

- Supported IIR opcodes:
  `const` (Int, Float, Bool), `add`, `sub`, `mul`, `div`, `mod`, `neg`,
  `and`, `or`, `xor`, `not`, `shl`, `shr`,
  `cmp_eq`, `cmp_ne`, `cmp_lt`, `cmp_le`, `cmp_gt`, `cmp_ge`,
  `label`, `jmp`, `jmp_if_true`, `jmp_if_false`,
  `ret`, `ret_void`, `call`, `load_reg`, `store_reg`, `type_assert`.

- Type mapping: `i8/i16/i32/u8/u16/u32/bool → int (I)`, `i64/u64 → long (J)`,
  `f32 → float (F)`, `f64 → double (D)`, `void → void (V)`.

- `codegen::IIRJvmCodeGenerator` — thin adapter that wires `validate_for_jvm`
  and `lower_iir_to_jvm` behind the `name()` / `validate()` / `generate()` API.

- 40+ integration tests in `tests/test_backend.rs` covering validation, lowering,
  instruction emission, register allocation, multi-function modules, float support,
  comparison synthesis, and bytecode non-emptiness checks.
