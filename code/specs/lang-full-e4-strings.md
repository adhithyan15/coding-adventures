# LANG-FULL E4 — Strings (design spec)

**Status:** design pass (this document) — implementation gated on sign-off.
**Enabler:** E4 in [`LANG-FULL-IMPLEMENTATION.md`](LANG-FULL-IMPLEMENTATION.md).
**Unlocks:** Dartmouth BASIC strings + string `PRINT` (BA4), ALGOL 60 strings +
`print`/`output` I/O (AL4), Twig strings on the code-gen backends (TW4), and any
future language with text values.

---

## 1. Goal

A **representation-agnostic string primitive** in the shared
`interpreter_ir::IIRModule`, lowered to *every* backend (vm-core, jit-core,
iir-to-llvm, iir-to-wasm, iir-to-jvm-class-file, iir-to-cil-bytecode,
x86_64-backend, aarch64-backend), and **verified by RUNNING** a real program on
each that produces observable text output.

### 1.1 What a "string" is in v1 (the scope decision)

A v1 string is an **immutable, length-counted sequence of bytes** (UTF-8 /
ASCII). Concretely:

- **Bytes, not codepoints.** Indexing returns the *byte* at a position; length is
  the *byte* count. No Unicode-codepoint, grapheme, or normalisation semantics —
  those are an explicit follow-up. This matches what the source languages of this
  era actually had (ALGOL/BASIC strings are byte/character sequences) and keeps
  every backend's representation a flat byte buffer.
- **Immutable.** String operations (`str_concat`, …) produce *new* strings;
  there is no in-place mutation. Immutability removes aliasing hazards, lets the
  managed backends use their native interned `String` types directly, and lets
  the static backends keep literals in read-only memory.
- **A string is morally an `array<u8>` + identity.** It reuses the E5 array
  substrate (length-prefixed flat buffer on the static backends; a managed array
  / native `String` on the managed backends). E4 is therefore **not** a brand-new
  allocator — it is the E5 byte-aggregate plus a literal pool, a few text ops, and
  a print primitive.

### 1.2 The dual-mode requirement (the design driver)

As with E3 (`f64`) and E5 (arrays), the IIR must **not** bake in a representation,
because the toolchain serves both target families:

- **Static-allocation / unmanaged** (C, ALGOL, BASIC, the native + native-AOT
  backends, LLVM, wasm-linear): a string is a **length-prefixed byte buffer** —
  literals live in read-only data; runtime-built strings live in heap memory the
  program owns. This is exactly E5's unmanaged array header (`[i64 len][bytes…]`).
- **Garbage-collected / managed** (the JVM, the CLR, WasmGC): a string is a
  **native managed object** — `java/lang/String`, `System.String`, or a WasmGC
  `(array i8)` — that carries its own length and is reclaimed by the runtime.

The IIR expresses string *operations* abstractly; each backend picks the
representation that is natural and safe for its target.

---

## 2. IIR surface

Six new ops. A string value rides the `type_hint` `str`; it flows as a `Var`
(a managed reference or a fat handle), exactly like an `array<T>` value.

| Op | Form | Result | Semantics |
|---|---|---|---|
| `str_const` | `dest <- "literal"` | `str` | Materialise a compile-time string constant. The bytes ride as an `Operand::Str` **value** (see §2.1). `dest` is the string value. |
| `str_len` | `dest <- s` | `i64` | The **byte** length of `s`. |
| `str_concat` | `dest <- a, b` | `str` | A new string = bytes of `a` followed by bytes of `b`. Neither input is mutated. |
| `str_index` | `dest <- s, idx` | `i64` | **Bounds-checked** byte load: if `idx < 0 \|\| idx >= str_len(s)` → **trap**; else `dest` = the unsigned byte value `s[idx]` (0–255). |
| `str_eq` | `dest <- a, b` | `i64` (bool) | `1` if `a` and `b` have identical bytes, else `0`. |
| `print_str` | `s` | — | Write the bytes of `s` to stdout (no implicit newline). The text-I/O primitive — the string sibling of `call_builtin "print_i64"`. |

`str_cmp` (lexicographic `-1/0/1`) and `str_substr` are **follow-ups**, not v1 —
`str_eq` covers the first runnable proofs (`PRINT`, equality), and lexical
ordering can layer on `str_index` + `str_len` later.

### 2.1 Type model

- New `type_hint` `str` (a scalar-form hint string, like `f64` or `ref<…>`).
  Helpers `is_str_type` live beside the existing `ref<…>`/`array<…>` helpers in
  `interpreter-ir/src/opcodes.rs`.
- **No new `Operand` variant.** A string *literal* reuses the existing
  `Operand::Str(String)` — but as a **value** carried by `str_const`, distinct
  from its existing use as a compile-time *name* in `global_load`/`global_store`.
  The two are disambiguated **by opcode** (exactly as the spec note on
  `Operand::Str` already anticipates: "used by the global ops and by any future
  string-value opcode"). A string *value* in flight is a `Var`.
- **Encoding:** the literal bytes are the source bytes after escape processing
  (`\n`, `\t`, `\"`, `\\`, `\0`). The IIR stores raw bytes; it does not interpret
  them as codepoints.

### 2.2 Bounds-check + trap convention

`str_index` is **bounds-checked by definition** (mirroring E5 `array_get`): the
check is `0 <= idx < str_len(s)`; on violation the program **traps**, reusing each
backend's existing hard-trap path (the same table as E5 — `ud2`/`udf`/`unreachable`
/ native exception / `VMError`). On the managed backends the native
`charAt`/indexer bounds check is reused (free); the static backends emit an
explicit `cmp idx,len` + branch-to-trap.

---

## 3. Per-backend representation

| Backend | Family | Representation | `str_const` | `str_len` | `str_index` | `str_concat` | `print_str` |
|---|---|---|---|---|---|---|---|
| **vm-core** | (interp) | `Value::Str(Vec<u8>)` (new value variant) or a handle into `memory` | intern the literal bytes | `.len()` | range-checked byte index | allocate a new buffer | write bytes to the host stdout sink |
| **jit-core** | (interp) | same as vm-core (CIR mirrors it) | — | — | — | — | — |
| **JVM** | GC | `java/lang/String` (or `byte[]`) | `ldc "…"` (constant pool `String`) | `invokevirtual String.length()` (or `arraylength`) | `String.charAt`/`bytes[i]` (native check) | `StringBuilder`/`String.concat` | `BasicRuntime.printStr(String)` → `System.out.print` |
| **CLR** | GC | `System.String` | `ldstr "…"` | `callvirt String::get_Length` | `String::get_Chars` (native check) | `String::Concat` | `BasicRuntime::PrintStr` → `Console.Write` |
| **WASM** | GC | WasmGC `(array i8)` — or linear-memory buffer + length header if GC disabled | data segment / `array.new_data` | `array.len` | `array.get_u` (native trap) | `array.new` + copy | host import `env.__print_str(ptr,len)` |
| **LLVM** | static | length-prefixed buffer `[i64 len][bytes…]`; literals in a `private constant` global | a `@.str.N` global + a header word | load header word | guard `icmp ult` → `br trap`; else `getelementptr`+`load i8` (zero-extended) | `@malloc(len_a+len_b+8)` + two `memcpy`s | `@__print_str(i8* base+8, i64 len)` C-runtime |
| **x86_64** | static | length-prefixed `__twig_alloc_bytes` buffer; literals in `.rodata` | emit the literal into rodata, materialise its address | load header | `cmp`/`jae trap`; else `movzx [base+8+idx]` | alloc + `rep movsb` ×2 | `call __print_str` |
| **aarch64** | static | length-prefixed buffer; literals in `__TEXT,__const` | `adrp`/`add` the literal address | load header | `cmp`/`b.hs trap`; else `ldrb [base+8+idx]` | alloc + copy | `bl __print_str` |

**Unmanaged header layout** (LLVM / x86_64 / aarch64): identical to E5's array
header — word 0 is the byte count, bytes start at offset 8. String literals are
emitted once into read-only data with that header; `str_concat` allocates a fresh
`8 + len_a + len_b` block via the existing `alloc_bytes`/`__twig_alloc_bytes`
machinery. **No new allocator** — E4 reuses E5's.

**Managed backends** (JVM/CLR/WASM): native `String` / managed `(array i8)`; the
length and bounds check come for free; GC reclaims. `str_const` is the native
constant-load (`ldc`/`ldstr`/data segment).

**The print runtime** (`print_str`): the static backends share one
`__print_str(const char* base_plus_8, long len)` C runtime (the string sibling of
the existing `__print_i64`), compiled into the LLVM/native matrix harness exactly
as `PRINT_RUNTIME_C` is today. WASM imports `env.__print_str(ptr,len)` (the sibling
of the existing `env.__print_i64`). The JVM/CLR call a `BasicRuntime.printStr`.
This is the one genuinely new piece of host surface E4 adds beyond E5.

---

## 4. Frontend drivers

- **Dartmouth BASIC (BA4)** — `PRINT "HELLO"` already *parses* (the grammar has a
  `STRING` `print_item`; the frontend currently errors "string literals in PRINT
  (need LANG77)"). Lower a `STRING` print-item to `str_const` + `print_str`;
  string variables (`A$`) and `PRINT A$` to a `str`-typed slot. String compare in
  `IF A$ = "Y"` lowers to `str_eq`.
- **ALGOL 60 (AL4)** — `string` is already a `type` keyword; `algol-parser`
  produces string literals. Lower `string` declarations to `str` slots and the
  (to-be-added) `print`/`output` intrinsic to `print_str`.
- **Twig (TW4)** — Twig string literals + `print` lower to `str_const` +
  `print_str`; `++`/`string-append` to `str_concat`, `string=?` to `str_eq`. (The
  dynamic-`any` Twig path still needs broader E6; the *typed* string slice here is
  the statically-typed subset that clears the code-gen validators, mirroring how
  E5/E6 carved a typed slice out of Twig.)

The frontends emit `str` values and the six ops; no backend learns anything
language-specific.

---

## 5. Verification (the matrix proof)

Unlike the integer/array proofs (observable via exit code), strings are observable
via **stdout** — the same channel the Dartmouth BASIC `PRINT 42` and Oct `out`
cells already use in `lang-aot/tests/lang_matrix.rs` (`Expect::Stdout`). The first
proof:

```basic
10 PRINT "HELLO"
```

⇒ stdout `HELLO` on every backend the toolchain is present for. A second proof
exercises `str_concat` + `str_len` observably, e.g. a program that prints the
length of `"AB" ++ "CDE"` ⇒ `5`, and a third exercises `str_eq` driving a branch
(`IF A$ = "Y" THEN PRINT 1 ELSE PRINT 0`). A **bounds-trap** proof (`str_index`
out of range) confirms the check fires (aborts non-zero) on each backend.

`run_native` runs the host arch, so `NativeAot` exercises aarch64 locally and
x86_64 on CI (as for E3/E5). The `x86-simulator` harness can additionally run the
x86_64 string output locally.

---

## 6. PR breakdown (incremental — one concern per PR)

E4 is large, so it ships as a sequence, each a `feat(lang-full): …` PR babysat to
merge before the next:

0. **E4-spec** (this document) — committed specs-first, for design sign-off.
1. **E4-ir + vm** — define the six ops + the `str` type helper in `interpreter-ir`;
   implement them in `vm-core` (+ jit-core): a string value model, the literal
   pool, `str_len`/`str_index` (bounds-checked)/`str_concat`/`str_eq`, and
   `print_str` to the host sink. Unit tests incl. an out-of-bounds trap. *No
   matrix Prog yet (needs a frontend), but a direct IIR unit test proves it runs.*
2. **E4-basic-frontend** — `dartmouth-basic-iir-compiler` lowers `PRINT "…"` to
   `str_const` + `print_str`; matrix `Prog` (`PRINT "HELLO"` ⇒ stdout `HELLO`)
   runs on VM + JIT.
3. **E4-managed-backends** — JVM, CLR, WASM string lowering (native `String` /
   managed `(array i8)` + the `printStr` runtime); extend the matrix Prog's
   backend list. (May be one PR per backend if they diverge.)
4. **E4-static-backends** — LLVM, then native x86_64 + aarch64 (length-prefixed
   rodata literals + heap `str_concat` + the shared `__print_str` C runtime +
   explicit `str_index` guard); extend the matrix Prog to all 7. Native encodings
   byte-verified vs the system assembler (as for E3/E5-native).
5. **E4-ops-proofs** — matrix programs for `str_concat`+`str_len` (⇒ `5`) and
   `str_eq` driving a branch, plus the `str_index` out-of-bounds **trap** proof,
   across every backend.
6. **(follow-ups, not v1)** `str_cmp` (lexical ordering) + `str_substr`; string
   *variables* and reassignment in each frontend; ALGOL `string` arrays; Unicode
   codepoint/grapheme semantics; the dynamic-`any` Twig string path (needs broader
   E6); string interpolation.

Ordering rationale mirrors E5: get the IIR + reference interpreter right (1), prove
it end-to-end through the simplest frontend (2), then the *managed* backends (3)
where `String` is native and bounds-checking is free, then the *static* backends
(4) where the rodata literal + header + heap concat + guard is the real work, then
the richer ops + trap proof (5). Front-loads the cheap high-confidence wins.

---

## 7. Open questions / decisions for review

1. **Bytes vs. codepoints in v1** — proposal: **bytes** (UTF-8/ASCII; `str_index`
   returns a byte 0–255, `str_len` is the byte count). Unicode codepoint/grapheme
   semantics are a follow-up. *OK?*
2. **Immutability** — proposal: strings are **immutable**; `str_concat` allocates a
   new string. This lets the managed backends use interned `String` directly and
   keeps literals in read-only memory. A mutable `StringBuilder`-style buffer is a
   separate later primitive. *OK?*
3. **`Operand::Str` reuse for literals** — proposal: carry a string *literal* in
   the existing `Operand::Str`, disambiguated from its name use by opcode
   (`str_const` value vs `global_load` name). Alternative: a new `Operand::Bytes`
   variant. Reuse is simpler and the operand already documents this intent.
   *Reuse preferred — confirm?*
4. **The `print_str` host runtime** — proposal: add a `__print_str(base,len)` C
   runtime (static backends), an `env.__print_str` import (WASM), and a
   `BasicRuntime.printStr` (JVM/CLR) — the string siblings of the existing
   `__print_i64` surface. This is the one new host primitive. *OK?*
5. **First frontend driver** — proposal: Dartmouth BASIC `PRINT "…"` (BA4), since
   its grammar already produces the `STRING` print-item and the frontend already
   has a clean error site to replace. ALGOL `string`/`output` (AL4) and Twig (TW4)
   follow. *OK, or prefer ALGOL first?*
6. **Trap vs. recoverable error on OOB `str_index`** — proposal: a hard **trap**
   (matches E5 arrays + the managed runtimes' native behaviour). *OK?*
7. **Concat growth on the static backends** — proposal: `str_concat` always
   allocates a fresh `8 + len_a + len_b` block (no small-string optimisation, no
   in-place growth, since strings are immutable). *OK?*

---

*This spec is the E4 contract; each ☐ in the PR breakdown becomes a
`feat(lang-full): …` PR, and divergences are called out in the implementing PR's
commit message and folded back here. It deliberately mirrors
[`lang-full-e5-arrays.md`](lang-full-e5-arrays.md): strings are the byte-aggregate
sibling of arrays, reusing the same dual-mode allocator, header, trap, and matrix
methodology.*
