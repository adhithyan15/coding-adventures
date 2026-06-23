# LANG-FULL E8 — Numeric conversions (`integer` ↔ `real`)

**Status:** design spec, pending user sign-off (no implementation yet).
**Depends on:** E3 (reals / `f64`, COMPLETE — every backend executes f64).
**Unblocks:** ALGOL **AL8 `entier`**, int→real **coercion** in mixed arithmetic,
BASIC **BA7** (floats) and **`INT()`**, and any future numeric standard
function that has to cross the integer/real boundary.

---

## 1. Why this is needed

E3 gave every backend (VM/JIT + LLVM/WASM/JVM/CLR/native) the ability to
*compute* with `f64`, but there is **no way to convert between `i64` and
`f64`**. Today the ALGOL frontend makes mixing the two a clean type error, and
there are no conversion IIR ops at all (`grep` for `f2i`/`i2f`/`fptosi` in
`interpreter-ir` + `vm-core` returns nothing).

Several queued LANG-FULL items are blocked purely on this one missing
primitive:

| Item | Needs | Conversion |
| --- | --- | --- |
| ALGOL **AL8 `entier(E)`** | largest integer ≤ `E` (a `real`) | `real → integer`, **floor** (toward −∞) |
| ALGOL int→real **coercion** | `1 + 2.5` should be `3.5`, not an error | `integer → real` |
| BASIC **`INT(x)`** | truncate/floor a number | `real → integer` |
| BASIC **BA7** (floats) | a `real`-valued variable assigned an integer literal | `integer → real` |

Rather than special-case each, E8 adds **one small family of conversion ops**
to the shared IIR — the same "build the generic primitive once, every backend
and every frontend reuses it" approach E5 (arrays) and E6 (globals) took.

## 2. The ops

Three new IIR ops. Each takes one operand and produces one result; the
`type_hint` records the *result* type (matching the existing convention where a
backend sizes the operation from the hint).

| Op | Operand | Result | Semantics |
| --- | --- | --- | --- |
| `int_to_real` | `i64` | `f64` | exact for \|x\| < 2⁵³; round-to-nearest-even beyond (IEEE-754 `i64→f64`) |
| `real_to_int_trunc` | `f64` | `i64` | round **toward zero** (C / most `INT()`): `2.7 → 2`, `-2.7 → -2` |
| `real_to_int_floor` | `f64` | `i64` | round **toward −∞** (ALGOL `entier`): `2.7 → 2`, `-2.7 → -3` |

Both `real_to_int_*` ops are only defined for finite operands whose floored /
truncated value fits in `i64`. **Out-of-range / NaN / ±∞ is a trap**, not a
silent wrap — consistent with how E5 array bounds and E4-spec out-of-range
behave (fail-closed). The frontends that emit these ops are responsible for any
language-level clamping they want *before* the conversion; the op itself never
produces a garbage integer.

Why both `trunc` and `floor`? They differ for negative non-integers
(`entier(-2.5) = -3` but a C cast gives `-2`), and different languages want
different ones (ALGOL `entier` = floor; many BASIC `INT()` = truncate). Adding
both now is cheap (one extra opcode) and avoids a frontend having to synthesise
floor from truncate with a branch.

## 3. Per-backend lowering

All three ops have a direct hardware/runtime equivalent on every target — this
is why E8 is tractable as a normal multi-PR arc rather than an architectural
fork.

| Backend | `int_to_real` | `real_to_int_trunc` | `real_to_int_floor` |
| --- | --- | --- | --- |
| **vm-core** | `v as f64` | `f.trunc() as i64` (range-checked) | `f.floor() as i64` (range-checked) |
| **JIT** | cold-interprets on VM (free, like E5/E6) | — | — |
| **LLVM** | `sitofp i64 → double` | `fptosi double → i64` | `llvm.floor.f64` then `fptosi` |
| **WASM** | `f64.convert_i64_s` | `i64.trunc_sat_f64_s` ¹ | `f64.floor` then `i64.trunc_sat_f64_s` |
| **JVM** | `i2d` ² | `d2l` (truncates toward zero by spec) | `Math.floor(D)D` then `d2l` |
| **CLR** | `conv.r8` | `conv.i8` (truncates toward zero) | `[mscorlib]Math::Floor(float64)` then `conv.i8` |
| **native x86_64** | `cvtsi2sd` | `cvttsd2si` (truncating convert) | `roundsd $1,…` (SSE4.1, round-down) then `cvttsd2si` |
| **native aarch64** | `scvtf` | `fcvtzs` (round-toward-zero) | `frintm` (round-toward-−∞) then `fcvtzs` |

¹ `trunc_sat` saturates rather than traps on overflow at the wasm level; the
range check the frontend/VM applies still gives the uniform trap semantics, and
we assert the operand is in range before lowering for the non-saturating story.
Open question O-2 below.
² JVM `i2d` is always exact (an `int` fits a `double`); when the operand is a
concretised `long`, use `l2d`.

**No backend needs a new capability** — every one already emits float
arithmetic (E3); these are just the convert opcodes that sit next to the
arithmetic ones. The `native floor` path is the only mildly involved case
(`roundsd` needs SSE4.1 / `frintm` on aarch64), mirroring how E3 added the
SSE2/NEON arithmetic.

## 4. Frontend uses (after the op lands)

- **ALGOL `entier(E)`** (AL8 next slice): resolve the name like `abs`/`sign`
  (built-in, overridable), require a `real` operand, emit `real_to_int_floor`,
  result type `integer`. One-liner once the op exists.
- **ALGOL int→real coercion**: where `emit_binary` currently errors on a mixed
  `integer`/`real` pair, insert an `int_to_real` on the integer side and compute
  in `f64`. (Scope/precedence to be decided in its own PR — coercion has
  language-design questions the conversion op itself does not.)
- **BASIC `INT()` / BA7**: BA7's value-model change is separate and larger, but
  it will lower its int-literal-into-real-slot and `INT()` cases onto these ops
  instead of inventing its own.

## 5. PR plan (each small, run-verified — mirrors E5/E6)

0. **This spec** (sign-off).
1. **`interpreter-ir` + `vm-core` + JIT** — define the three ops; implement in
   the VM with range-checked `as` casts; JIT inherits via cold-interpret. Unit
   tests: `int_to_real(3) = 3.0`, `real_to_int_trunc(-2.7) = -2`,
   `real_to_int_floor(-2.7) = -3`, and the out-of-range trap.
2. **LLVM** — `sitofp`/`fptosi`/`llvm.floor`. RUN-verified on clang.
3. **WASM** — `f64.convert`/`trunc_sat`/`f64.floor`. RUN-verified on the
   wasm-runtime harness.
4. **JVM** — `i2d`/`d2l`/`Math.floor`. RUN-verified on java.
5. **CLR** — `conv.r8`/`conv.i8`/`Math::Floor`. RUN-verified on ilasm+dotnet.
6. **native** — aarch64 (`scvtf`/`fcvtzs`/`frintm`) + x86_64
   (`cvtsi2sd`/`cvttsd2si`/`roundsd`). RUN-verified, plus an **x86-simulator**
   cell so the x86_64 path runs locally.
7. **ALGOL `entier`** — the frontend slice + a `lang_matrix.rs` cell
   (`entier(2.7)` ⇒ 2, `entier(0.0 - 2.5)` ⇒ −3 via arithmetic) on all 7
   backends. Closes the first E8 consumer.

Each layer is its own PR; the matrix proof in step 7 is the end-to-end
RUN-verification that the whole stack agrees, exactly as E5/E6 closed.

## 6. Non-goals (this spec)

- **No rounding-to-nearest op** (`round`) yet — add it when a consumer needs it.
- **No `real`-typed BASIC** (BA7) — that is a separate, larger value-model
  change; E8 only gives it the conversion primitive it will build on.
- **No general numeric tower** (rationals, bignums, complex) — out of scope.

## 7. Open questions (for sign-off)

1. **Op naming.** `real_to_int_floor` / `_trunc` are explicit but verbose.
   Alternative: a single `real_to_int` op with a rounding-mode immediate operand
   (`trunc`/`floor`/`round`), like the comparison ops carry their predicate.
   The immediate-mode form is more extensible (round/ceil later) but adds an
   operand the validators must check. **Recommendation: the immediate-mode
   single op** — fewer opcodes, room to grow. Spec above shows the verbose form
   for clarity; the implementation can collapse it.
2. **Overflow on `real → int`.** Spec says *trap* (fail-closed, consistent with
   E5 bounds). WASM's `trunc_sat` saturates natively; do we (a) assert-in-range
   before lowering everywhere for uniform trap semantics, or (b) adopt
   saturation as the cross-backend contract (cheaper on wasm, matches some
   languages' `INT()` clamping)? **Recommendation: trap** — it is the
   no-silent-garbage default the rest of LANG-FULL uses; a language that wants
   clamping does it in the frontend.
3. **int→real coercion scope.** Should E8 ship the coercion frontend change, or
   only the op (leaving coercion to its own PR with its precedence/EXPR-position
   design questions)? **Recommendation: op only in E8**; coercion is a separate
   frontend PR so its language-design questions don't block the primitive.
