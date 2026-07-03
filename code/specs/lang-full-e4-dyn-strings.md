# LANG-FULL E4-dyn — Runtime (dynamic) strings

**Status:** design (spec-first, pending sign-off)
**Depends on:** E4 (immutable literal strings, landed) · E5 (length-prefixed
heap arrays, landed on all 7 backends)
**Unblocks:** ALGOL string procedures (AL) · BASIC string `INPUT` + string
arrays (BA4) · the captured/reassigned/parameter Twig string paths (TW4-dyn)

---

## 0. One-paragraph summary

E4 shipped an **immutable, length-counted string** value model whose ops
(`str_const`/`str_len`/`str_index`/`str_concat`/`str_slice`/`str_eq`/`str_cmp`/
`print_str`) run on all 7 backends — but on the **static** backends
(LLVM/WASM/x86_64/aarch64) only for **literal / compile-time-foldable** operands:
`str_concat("AB","CDE")` is folded to the constant `"ABCDE"` at compile time,
and the LLVM validator rejects a `str_const` whose operand is not an
`Operand::Str`. The managed backends (JVM `java.lang.String`, CLR
`System.String`) and the VM/JIT (tagged value) already carry **runtime**
strings. E4-dyn closes the gap by giving the static backends a **runtime
heap-string representation** — the exact length-prefixed block E5 already uses
for arrays — so a string built or chosen at run time (a concat of two variables,
a slice of an input, a value selected by a branch) is a first-class value on
every backend. No new IIR op is introduced: the change is that the existing
`str_*` ops accept **runtime register** operands, not only literals.

---

## 1. Goal & non-goals

### 1.1 Goal

A string whose bytes are **not known at compile time** — produced by
`str_concat`/`str_slice` over runtime operands, read from `input`, or chosen by
control flow — is a fully-supported E4 value on **all 7 backends**, observable by
running (stdout via `print_str`, or a derived `str_len`/`str_index`/`str_eq`
result as an exit code).

Strings stay **immutable** (E4's invariant): every op still *produces a new
string*; nothing mutates in place. "Dynamic" here means *runtime-constructed*,
not *mutable*.

### 1.2 Non-goals (explicit follow-ups)

- **Mutable strings / string builders.** Out of scope; the immutable model
  stands.
- **Unicode-aware indexing.** `str_index`/`str_len` remain **byte**-indexed
  (the E4 contract). UTF-8 grapheme handling is a separate concern.
- **Garbage collection of runtime strings.** Runtime strings allocate through
  the same `__twig_alloc_bytes` / TWIG-GC path E5 arrays already use; E4-dyn
  adds no new allocator or ownership model. (Leaks/repeated concat in a hot loop
  are the same shape E5 arrays already have and are handled by TWIG-GC where it
  is wired.)
- **Closure-captured strings across function boundaries beyond value passing.**
  Passing a runtime string *by value* into/out of a function is in scope (it is
  what ALGOL string procedures need); full closure capture is E6.

---

## 2. Representation — the E5 heap block, reused

A runtime string is the **same length-prefixed heap block** E5 arrays use, with
`i8` elements:

```
        +--------+--------+--------+ ... +--------+
handle→ | i64 length (# bytes)     | b0 | b1 | ... | b(len-1) |
        +--------+--------+--------+ ... +--------+
        [ 8 bytes            ]      [ len bytes            ]
```

- The **handle** is the block base pointer (static backends) / object reference
  (managed) / tagged value (VM/JIT) — identical to how an E5 `array<i8>` handle
  flows.
- **Length** lives at offset 0 (`i64`), bytes at offset 8 — byte-for-byte the E5
  layout, so `str_len` is an `array_len`-shaped load and `str_index` is an
  `array_get`-shaped (bounds-checked) `i8` load.

| backend | literal string (E4, landed) | **runtime string (E4-dyn)** |
|---------|------------------------------|------------------------------|
| **VM / JIT** | tagged value | tagged value — already dynamic ✅ |
| **JVM** | `ldc` `String` | `java.lang.String` from `char[]`/`String.concat` — already dynamic ✅ |
| **CLR** | `ldstr` `String` | `System.String` from `String.Concat`/`Substring` — already dynamic ✅ |
| **LLVM** | private `{i64,[N×i8]}` global | **`__twig_alloc_bytes`-backed `[i64 len][i8…]` heap block** (E5 model) |
| **WASM** | data segment + side-table | **linear-memory `[i64 len][i8…]` block** (E5 model) |
| **x86_64** | folded literal | **`__twig_alloc_bytes` heap block** (E5 model) |
| **aarch64** | folded literal | **`__twig_alloc_bytes` heap block** (E5 model) |

The four static backends are the whole job; the other three already run runtime
strings.

---

## 3. Runtime helpers (C ABI, `twig_runtime.c`)

The static backends already link `__twig_alloc_bytes`, `__twig_print_string`,
and `__twig_str_eq`. E4-dyn adds the heap **builders** the folded-literal path
never needed. Each takes handle(s) to length-prefixed blocks and returns a
freshly allocated block (via `__twig_alloc_bytes`), preserving immutability:

| helper | signature | semantics |
|--------|-----------|-----------|
| `__twig_str_concat` | `(int64_t a, int64_t b) -> int64_t` | allocate `8 + len(a)+len(b)`; write length; `memcpy` a then b; return handle |
| `__twig_str_slice` | `(int64_t s, int64_t start, int64_t end) -> int64_t` | **bounds-check** `0 ≤ start ≤ end ≤ len` (else trap, per §2.2 of E4); allocate + copy `[start,end)` |
| `__twig_str_len` | `(int64_t s) -> int64_t` | load length at offset 0 (may be inlined as a raw load instead) |
| `__twig_str_index` | `(int64_t s, int64_t i) -> int64_t` | **bounds-check** `0 ≤ i < len` (else trap); return the `i8` byte zero-extended |
| `__twig_str_cmp` | `(int64_t a, int64_t b) -> int64_t` | lexicographic byte compare → `-1/0/1` |

`__twig_str_eq` already exists and already reads the length header + `memcmp`s —
it works unchanged for runtime blocks. `str_len`/`str_index` may lower to inline
loads (matching E5 `array_len`/`array_get`) rather than calls; the helper column
is the fallback / managed-parity reference.

WASM has no C runtime, so it grows the equivalent as emitted linear-memory code
(the same way E5 `array_*` is inlined into WASM), reusing `env.__print_str`.

---

## 4. IIR surface — unchanged

**No new op.** E4-dyn is a *capability* change, not a surface change:

- The validators that today require `str_const`/`str_concat` operands to be
  literals are relaxed so a `str`-typed **register** operand is legal. `str_const`
  with an `Operand::Str` stays the literal fast-path; the new path is a `str`
  register flowing into `str_concat`/`str_slice`/`str_index`/`str_len`/`str_eq`/
  `str_cmp`/`print_str`.
- A `str` value's `type_hint` stays `"str"`. On the static backends the slot
  holds the block handle (`i64`); the AOT specialiser already collapses
  `array<T>`→`any`, and `str` rides the same 8-byte handle slot.

The compile-time literal folding stays as an **optimisation** (a `str_concat` of
two literals still folds), so nothing regresses; the runtime path is what's new.

---

## 5. PR breakdown (small, dependency-ordered)

> Convention: each ☐ is one `feat(lang-full): …` PR, security-reviewed, matrix-proven, babysat to merge.

1. **E4d-1 — runtime helpers.** ✅ **Landed** (`twig-aot` 0.26.0). Added
   `__twig_str_concat`/`__twig_str_slice`/`__twig_str_index`/`__twig_str_len`/
   `__twig_str_cmp` to `twig_runtime.c`, all on the E5 length-prefixed heap block
   via `__twig_alloc_bytes`; `slice`/`index` `abort()`-trap out-of-range per the
   E4 bounds contract; every producer allocates a fresh block (immutability).
   Unit-tested by a `cc`-compiled C driver (`tests/e4d_str_helpers.rs`,
   Unix-gated) covering valid paths + the three trap cases. No backend/IIR change.
   (The VM/JIT already execute runtime `str_concat` over registers — e.g. BASIC
   variable concat — so those semantics were already proven; this PR is the
   static-backend substrate.) *(blocks 2–5)*
2. **E4d-2 — LLVM runtime strings.** Relax the `iir-to-llvm` validator; lower
   non-literal `str_concat`/`str_slice`/`str_index`/`str_len`/`str_cmp`/`print_str`
   to the heap-block helpers + guarded loads. Matrix cell adds `Llvm`. *(needs 1)*
3. **E4d-3 — WASM runtime strings.** Same for `iir-to-wasm`, inlined over linear
   memory (mirror E5 `array_*`). Matrix cell adds `Wasm`. *(needs 1)*
4. **E4d-4 — native runtime strings.** `aarch64-backend` + `x86_64-backend` lower
   the ops to `__twig_alloc_bytes` blocks + `bl/call` helpers + `udf/ud2` bounds
   traps (mirror E5 arrays). Run-verify aarch64 locally + x86_64 on CI. Matrix
   cell reaches **all 7 backends**. *(needs 1)*
5. **Frontend payoffs** *(each needs 1–4 for the backends it targets)*:
   - **E4d-AL — ALGOL string procedures.** Lift the `string procedures`
     `Unsupported` (algol-iir-compiler:886): a `string procedure` returns a
     runtime `str`; a runtime string variable (`s := t & u`-style concat) works.
     Matrix cell: a string procedure whose result is printed.
   - **E4d-BA-input — BASIC string `INPUT`.** `INPUT A$` reads a runtime string
     from the host input queue; `PRINT A$` echoes it. Matrix cell.
   - **E4d-BA-arr — BASIC string arrays.** `DIM A$(n)` + `A$(i)` over runtime
     string elements (reuses E5 array-of-handles + E4-dyn strings).

Managed backends (JVM/CLR) already run runtime strings, so each frontend cell can
tag `Jvm`/`Clr` from the start and add the static backends as E4d-2…4 land.

---

## 6. Verification (matrix proof)

Each PR adds/extends a `lang_matrix.rs` cell proving a **non-foldable** runtime
string, guarded by `proven_columns_do_not_silently_skip` +
`matrix_every_proven_cell_agrees`. Representative cells:

- **Runtime concat length** — build `s` from two variables, exit `str_len(s)`.
  Guarantees the operand is a register, not a foldable literal.
- **Runtime slice + index** — `str_index(str_slice(s, a, b), k)` over runtime
  `s`/`a`/`b`, exit the byte.
- **Runtime equality across a branch** — a string chosen by an `if` compared with
  `str_eq`, exit 42/0.
- **Bounds trap** — a runtime out-of-range `str_index`/`str_slice` traps on every
  backend (mirrors the landed E4 literal-bounds proof).
- **Frontend** — ALGOL `string procedure`, BASIC `INPUT A$`, printed to stdout.

A cell is "runtime" only if the string cannot be constant-folded (operands are
variables / inputs / branch-selected), so the static backends genuinely exercise
the heap path rather than the literal fast-path.

---

## 7. Why this ordering

The static-backend **representation** (a heap block + helpers) is the one hard,
shared dependency; every frontend payoff is blocked on it but trivial after it.
Landing helpers first (E4d-1) locks the byte layout and bounds semantics against
the already-dynamic VM/JIT, so the four backend PRs (E4d-2…4) each become a
mechanical "lower to the agreed helpers" change verifiable in isolation — the
same shape that made E5 arrays land cleanly one backend per PR. The frontend
features that motivated the whole arc (ALGOL string procedures, BASIC string
`INPUT`/arrays) then fall out as thin frontend PRs on a proven substrate.
