# LANG-FULL E5 — Arrays / linear aggregates (design spec)

**Status:** design pass (this document) — implementation gated on sign-off.
**Enabler:** E5 in [`LANG-FULL-IMPLEMENTATION.md`](LANG-FULL-IMPLEMENTATION.md).
**Unlocks:** ALGOL 60 arrays (AL2), Dartmouth BASIC `DIM` (BA3), Twig lists/vectors
(TW3), and any future language with indexable aggregates.

---

## 1. Goal

A **representation-agnostic, bounds-checked array primitive** in the shared
`interpreter_ir::IIRModule`, lowered to *every* backend (vm-core, jit-core,
iir-to-llvm, iir-to-wasm, iir-to-jvm-class-file, iir-to-cil-bytecode,
x86_64-backend, aarch64-backend), and **verified by RUNNING** a real program on
each.

### 1.1 The dual-mode requirement (the design driver)

The toolchain must serve **both** families of target language, so the IIR array
model must lower naturally to **both** memory models:

- **Static-allocation / unmanaged** (C, Pascal, ALGOL, the native + native-AOT
  backends): the array is a block of raw memory the program owns; bounds are the
  program's responsibility; there is no GC.
- **Garbage-collected / managed** (the JVM, the CLR, WasmGC): the array is a
  first-class managed object that *carries its own length* and is *bounds-checked
  and reclaimed by the runtime*.

The IIR therefore must **not** bake in a representation. It expresses array
*operations* abstractly; each backend chooses the representation that is natural
(and safe) for its target. This is the same principle E3 followed (one `f64`
abstraction, each backend picks `double`/`float64`/SSE) and matches the project's
"generic reusable substrate, not point solutions" goal.

---

## 2. IIR surface

Four new ops. The element type rides the `type_hint` as `array<T>` (e.g.
`array<i64>`, `array<f64>`, later `array<u8>`); the *element* type `T` is one of
the existing scalar hints.

| Op | Form | Result | Semantics |
|---|---|---|---|
| `alloc_array` | `dest <- count` | `array<T>` | Allocate a `count`-element array of `T`, **zero/default-initialised**. `dest` is the array value (a managed reference *or* a fat handle — backend's choice). `count` is an `i64` operand. |
| `array_len` | `dest <- arr` | `i64` | The element count of `arr`. |
| `array_get` | `dest <- arr, idx` | `T` | **Bounds-checked** load: if `idx < 0 \|\| idx >= len(arr)` → **trap**; else `dest = arr[idx]`. |
| `array_set` | `arr, idx, val` | — | **Bounds-checked** store: trap on out-of-range; else `arr[idx] = val`. |

### 2.1 Type model

- New `type_hint` shape `array<T>` (string form, like the existing `ref<…>`).
  Helpers `make_array_type("i64") == "array<i64>"`, `is_array_type`,
  `array_elem_type` mirror the existing `ref<…>` helpers in
  `interpreter-ir/src/opcodes.rs`.
- No new `Operand` variant — an array value flows as a `Var` (a register holding
  a managed reference or a fat handle), exactly like `ref<LispyPair>` cons cells.
- **Element type in v1:** `i64` and `f64` (the two 8-byte scalar worlds, covering
  ALGOL integer/real arrays and BASIC numeric arrays). Narrow-element packing
  (`array<u8>`, …) and reference-element arrays (`array<ref<…>>`) are explicit
  follow-up slices, not v1.

### 2.2 Bounds-check + trap convention

`array_get`/`array_set` are **bounds-checked by definition** (the user chose
checked-from-the-start). The check is `0 <= idx < array_len(arr)`; on violation
the program **traps** (aborts), reusing each backend's existing hard-trap path:

| Backend | Trap mechanism (already exists) |
|---|---|
| native x86_64 | `ud2` (`0F 0B`) — see `type_assert` lowering |
| native aarch64 | `udf #0xDEAD` |
| LLVM | a `trap`/`unreachable` block reached by a guard branch (or `@llvm.trap`) |
| WASM | the managed `array.get`/`array.set` **trap natively**; a linear-memory fallback uses an explicit `if` + `unreachable` |
| JVM | `aaload`/`laload` **throw `ArrayIndexOutOfBoundsException` natively** (process exits non-zero) |
| CLR | `ldelem`/`stelem` **bounds-check natively** (`IndexOutOfRangeException`) |
| vm-core / jit-core | a `VMError::Custom("array index out of bounds")` (clean error, non-zero) |

> **Design note:** on the *managed* backends the bounds check is **free** — the
> runtime instruction already does it. We must *not* add a redundant manual check
> there (it would only differ on a NaN-equivalent edge). On the *static* backends
> we emit an explicit `cmp idx, len` + conditional branch to the trap. So the same
> IIR `array_get` lowers to "one bounds-checked instruction" on managed targets and
> "compare-branch-trap then load" on unmanaged ones — exactly the dual-mode split.

---

## 3. Per-backend representation

| Backend | Family | Representation | `alloc_array` | `array_get`/`set` | `array_len` |
|---|---|---|---|---|---|
| **vm-core** | (interp) | `Vec<Value>` behind a handle in `memory` | push a zero/`0.0`-filled `Vec`, return its handle | index the `Vec`, range-check in Rust | `.len()` |
| **jit-core** | (interp) | same as vm-core (CIR mirrors it) | — | — | — |
| **JVM** | GC | `long[]` / `double[]` (`newarray`) — or `Object[]` for ref elems | `newarray T_LONG`/`T_DOUBLE` | `laload`/`lastore` (native bounds check) | `arraylength` |
| **CLR** | GC | `int64[]` / `float64[]` (`newarr`) | `newarr [mscorlib]System.Int64` | `ldelem.i8`/`stelem.i8` (native check) | `ldlen` |
| **WASM** | GC | a WasmGC `(array (mut i64))` type — or linear memory + length header if GC disabled | `array.new_default` | `array.get`/`array.set` (native trap) | `array.len` |
| **LLVM** | static | length-prefixed `@calloc` block: `[i64 len][elems…]` (reuses the byte-tape allocator) | `@calloc(8*count + 8, 1)`; store `len` at offset 0 | guard `icmp ult idx,len` → `br` to a `trap` block; else `getelementptr` + load/store | load the header word |
| **x86_64** | static | length-prefixed `__twig_alloc_bytes` block | `call __twig_alloc_bytes(8*count+8)`; store len | `cmp`/`jae trap` (`ud2`); else `mov [base+8+idx*8]` | load header |
| **aarch64** | static | length-prefixed `__twig_alloc_bytes` block | same shape | `cmp`/`b.hs trap` (`udf`); else `ldr/str [base+8+idx*8]` | load header |

**Unmanaged header layout** (LLVM / x86_64 / aarch64): the allocation is
`8 + 8*count` bytes; word 0 is the element count (so `array_len` is a single load
and the bounds check has the length to hand); the elements start at byte offset 8.
This reuses the *exact* `alloc_bytes`/flat-memory machinery these backends already
have (`@calloc`, `__twig_alloc_bytes`) — E5 adds the header + stride + guard, not a
new allocator.

**Managed backends** (JVM/CLR/WASM): a real managed array; the length and bounds
check come for free; GC reclaims it. No header word — `array_len` is the native
length op.

---

## 4. Frontend drivers

The array grammar is **already parsed** (no grammar work):

- **ALGOL 60 (AL2)** — `algol-parser` already produces `array_decl` /
  `array_segment` / `subscripts` nodes for `integer array A[1:10]` and `A[i]`.
  `algol-iir-compiler` lowers a declaration to `alloc_array` (size = `hi-lo+1`,
  with a `lo` offset folded into the index so ALGOL's 1-based / arbitrary lower
  bounds map onto 0-based `array_get`/`set`), and a subscript to
  `array_get`/`array_set`. Multi-dimensional `A[i,j]` → row-major
  `idx = (i-lo_i)*extent_j + (j-lo_j)` computed with existing arithmetic ops (no
  new IIR).
- **Dartmouth BASIC (BA3)** — `DIM A(10)` → `alloc_array`; `A(i)` → `array_get`/
  `array_set`. (BASIC arrays are 0..N inclusive; size = `N+1`.)

The frontends emit `array<i64>` (and `array<f64>` once real arrays are wired).

---

## 5. Verification (the matrix proof)

Each implemented backend gets an **executed** `Prog` in
`lang-aot/tests/lang_matrix.rs`, observable via the integer exit code (the same
comparison-fold trick E3 used — no array *printing* needed). E.g. an ALGOL
program:

```algol
begin integer array A[1:3]; integer result;
  A[1] := 10; A[2] := 20; A[3] := 12;
  result := A[1] + A[3]            comment 22;
end
```

⇒ exit `22`, asserted across every backend the toolchain is present for. A
**bounds-trap** proof (a program that indexes out of range and is expected to
abort non-zero) confirms the check fires on each backend.

`run_native` runs the host arch, so the `NativeAot` cell exercises aarch64
locally and x86_64 on CI (as for E3).

---

## 6. PR breakdown (incremental — one concern per PR)

E5 is large, so it ships as a sequence, each a `feat(lang-full): …` PR babysat to
merge before the next:

0. **E5-spec** (this document) — committed specs-first, for design sign-off.
1. **E5-ir + vm** — ✅ **done** (interpreter-ir 0.7.0, vm-core 0.7.0). The 4 ops +
   `array<T>` type helpers in `interpreter-ir`; bounds-checked execution in
   `vm-core` (per-allocation `Vec<Value>` heap, default-init, OOB → `VMError`).
   7 unit tests incl. out-of-bounds + negative-index traps and no-alias. (jit-core
   array execution deferred to a later slice — it has no byte-tape either; the
   matrix Jit column joins with the frontend PR.)
2. **E5-algol-frontend** — ✅ **done** (`algol-iir-compiler` 0.5.0).
   `integer`/`real array A[lo:hi]` decls lower to `alloc_array` (run-time span);
   `A[i]` reads/writes lower to `array_get`/`array_set` with the 0-based index
   `i - lower`. 9 unit tests + a `lang-aot` matrix `Prog` (sum-of-squares) running
   on **VM + JIT** (exit 55). 1-D, integer/real elements; multidim + array params
   are follow-up.
3. **E5-managed-backends** — JVM, CLR, WASM array lowering (native managed
   arrays + native bounds checks); extend the matrix Prog's backend list. (May be
   one PR per backend if they diverge.)
4. **E5-static-backends** — LLVM, then native x86_64 + aarch64 (length-prefixed
   flat allocation + explicit guard + trap); extend the matrix Prog to all 7.
   Native encodings byte-verified vs the system assembler (as for E3-native).
5. **E5-bounds-trap-proof** — a matrix program that traps on OOB, asserted to
   abort on every backend.
6. **(follow-ups, not v1)** real (`array<f64>`) arrays end-to-end; BASIC `DIM`
   (BA3); multi-dimensional arrays; narrow-element packing (`array<u8>`);
   reference-element arrays (`array<ref<…>>`, needs E6); ALGOL `own` arrays.

Ordering rationale: get the IIR + reference interpreter right first (1–2), then
the *managed* backends (3) where bounds-checking is free and the representation is
trivial, then the *static* backends (4) where the header + guard + trap is the
real work. This front-loads the cheap, high-confidence wins and de-risks the
dual-mode design early (managed and static both proven before the harder native
encoding).

---

## 7. Open questions / decisions for review

1. **Element types in v1** — proposal: `i64` + `f64` only (8-byte words). Narrow
   (`u8`/`u16`/…) packing and reference-element arrays are follow-ups. *OK?*
2. **Unmanaged header** — proposal: a single length word prefix (`[len][elems…]`).
   Alternative: a separate `(base, len)` fat pointer kept in two registers (no
   header, but doubles the register pressure and complicates passing arrays to
   functions). The header is simpler and matches the GC backends' "length travels
   with the object" shape. *Header preferred — confirm?*
3. **Trap vs. recoverable error** — proposal: OOB is a hard **trap/abort**
   (matches the managed runtimes' native behaviour and the existing `type_assert`
   trap). A catchable exception model is out of scope. *OK?*
4. **First frontend driver** — proposal: ALGOL `integer array` (AL2), 1-D, since
   its grammar + the AL3/AL5 lowering scaffolding already exist. BASIC `DIM` (BA3)
   follows. *OK, or prefer BASIC first?*
5. **GC interaction** — on the managed backends arrays are ordinary managed
   objects (no special rooting needed for this slice, since arrays don't yet hold
   references — `array<i64>`/`array<f64>` are primitive-element). When
   reference-element arrays land (post-E6), GC rooting is revisited. *Noted.*

---

*This spec is the E5 contract; each ☐ in the PR breakdown becomes a
`feat(lang-full): …` PR, and divergences are called out in the implementing PR's
commit message and folded back here.*
