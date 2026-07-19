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
| **LLVM** | private `{i64,[N×i8]}` global | **`[i64 len][i8…]` block; handle = block address, `inttoptr`+`load` in `print_str`** ✅ (E4d-2) |
| **WASM** | data segment + side-table | **linear-memory `[i32 len][i8…]` block; handle = i32 offset, `i32.load` in `print_str`** ✅ (E4d-3) |
| **x86_64** | folded literal | **`alloc_bytes` `[i64 len][i8…]` block; slot holds address, `field_load` header in `print_str`** ✅ (E4d-4) |
| **aarch64** | folded literal | **`alloc_bytes` `[i64 len][i8…]` block; slot holds address, `field_load` header in `print_str`** ✅ (E4d-4) |

All four static backends now run runtime strings; the E4-dyn foothold is proven
on **all seven backends**.

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
1a. **E4d-foothold — first runtime-string matrix cell.** ✅ **Landed**
   (`lang-aot` 0.171.0). A BASIC branch-selected string (`INPUT N` picks
   `"HI"`/`"LO"`, so the value is non-foldable) proven on the **four
   already-dynamic backends** `Vm`/`Jit`/`Jvm`/`Clr`. **This is the shared cell
   the backend PRs below extend** — it resolves the ordering constraint that a
   static-backend runtime-string lowering can't be matrix-proven until a frontend
   emits a non-foldable string. (Discovered during E4d-2 scoping: iir-to-llvm
   string support is pure compile-time literal folding, and no frontend emitted a
   runtime string to the static columns, so those columns had nothing to prove
   against. The foothold supplies it.)
2. **E4d-2 — LLVM runtime-string representation + `print_str`.** ✅ **Landed**
   (`iir-to-llvm` 0.30.0 / `lang-aot` 0.172.0). Key finding: `iir-to-llvm` is
   **alloca-backed**, and a str var was never promoted to a slot — so a
   cross-block string emitted invalid IR. Fix: `collect_slot_vars` promotes a
   `str` var assigned in **>1 basic block** to an `i64`-**handle** slot (a
   literal's `{i64 len,[N×i8]}` global address is a valid handle, stored via
   `ptrtoint`); `print_str` of such a slot reads the length from the block header
   at run time (`inttoptr` + `load`). Single-assignment strings keep the folded
   fast path. The **foothold cell now runs on `Llvm`** (real `clang`), verified
   locally + by the two guard tests; 2 unit tests assert the emitted IR.
2b. **E4d-2b — LLVM runtime `str_len`/`str_concat`/`str_slice`/`str_index`/
   `str_cmp`** over slot (runtime) operands → the E4d-1 `__twig_str_*` helpers +
   guarded loads (mirror E5 `array_*`). Not needed by the foothold (it only
   *observes* a runtime string via `print_str`); needed by the frontend payoffs
   that *build* runtime strings (ALGOL string procedures' `s := t & u`). *(needs 2)*
3. **E4d-3 — WASM runtime strings.** ✅ **Landed** (`iir-to-wasm` 0.29.0 /
   `lang-aot` 0.173.0). Mirrors E4d-2 over linear memory: `collect_runtime_str_vars`
   promotes a `str` var assigned in **>1 basic block** to an i32 **handle** = the
   offset of a length-prefixed block `[i32 len (LE)][bytes]` laid down in the
   string data segment (deduped by text). `str_const` of a promoted var stores its
   block offset; `print_str` reads the length back with `i32.load` and calls
   `env.__print_str(handle + 4, len)` — the WASM sibling of LLVM's `inttoptr` +
   `load` + `getelementptr … i64 8`. Single-assignment strings keep the folded
   fast path. The **foothold cell now runs on `Wasm`** (in-process `wasm-runtime` +
   `env.__print_str` host), verified by the two guard tests; 3 unit tests assert
   the emitted wasm. Deferred (like E4d-2b): runtime `str_len`/`str_concat`/
   `str_slice`/`str_index`/`str_cmp` over promoted operands (E4d-3b — not needed by
   the foothold, which only *observes* a runtime string via `print_str`). *(needs 1, 1a)*
   **E4d-3b progress:** runtime `str_len`/`print_str`/`str_concat` (header read /
   bump-alloc + `memory.copy`) and runtime `str_eq` (in-module `$__str_eq` helper)
   landed with the E4d-AL / BA payoffs below; **runtime `str_cmp`** landed
   `iir-to-wasm` 0.39.0 (sibling `$__str_cmp` helper: min-length prefix scan +
   length tiebreak → `-1`/`0`/`1`, byte-identical to the folded
   `bytes.cmp` fold, sign-extended to `i64`). Remaining E4d-3b: runtime
   `str_slice` / `str_index` over promoted operands.
4. **E4d-4 — native runtime strings.** ✅ **Landed** (`twig-aot` 0.27.0 /
   `lang-aot` 0.174.0). Key finding: the native path *already* had everything —
   `lower_string_literals_for_aot` builds each `str_const`'s `[i64 len][bytes]`
   buffer via `alloc_bytes`/`field_store`/`store_byte` and stores its **address**
   in the var's stack slot (`mov dest = buf`), and `print_str`/`str_len` already
   read the length header at run time (`field_load src[0]`) for runtime string
   *parameters*. The only bug was that `str_const` registered every dest in the
   compile-time `strings` map, so a branch-selected local wrongly took the
   static-length (last-writer-wins) path. Fix: `collect_runtime_str_vars_for_aot`
   (same >1-basic-block rule as E4d-2/E4d-3) and skip the `strings` registration
   for promoted vars → `print_str` reads the runtime length. No backend code
   changed, so **one change covers both aarch64 and x86_64**. Foothold cell adds
   `NativeAot` → **all 7 backends** (aarch64 run-verified locally, x86_64 on CI);
   2 unit tests (differing-length runtime read + straight-line static). Deferred
   (E4d-4b, like E4d-2b/E4d-3b): runtime `str_concat`/`str_slice`/`str_index` over
   promoted operands — not needed by the foothold. *(needs 1, 1a)*
5. **Frontend payoffs** *(each needs 1–4 for the backends it targets)*:
   - **E4d-AL — ALGOL string procedures.** ✅ **Landed on ALL SEVEN backends**
     (`algol-iir-compiler` 0.28.0 / `lang-aot` 0.178.0). Lifted the
     `string procedures` `Unsupported` (algol-iir-compiler:886): a `string
     procedure` returns a runtime `str` (its result slot), and `print` gained a
     general string-expression path so `print(pick(1))` prints a call result.
     Matrix cell: a string procedure whose branch-selected result is printed —
     runs on **all 7 backends**. **Discovery:** a runtime string arriving as a
     *call result / return value* is a NEW path beyond the E4-dyn foothold (which
     only printed a branch-selected *local*): a backend must map `str` to its
     handle type at function boundaries and take the runtime header-read path for
     ANY non-foldable string, not only a promoted slot. Per-backend:
     **LLVM (E4d-2b, iir-to-llvm 0.31.0)** `str`→`i64` at boundaries;
     `print_str`/`str_len` runtime path keyed on `!str_lens.contains(src)`;
     `ret` of a literal-global str `ptrtoint`s to the handle.
     **WASM (E4d-3b, iir-to-wasm 0.30.0)** `str` types as i32; validator accepts
     `str` on `call`/`ret`; `print_str`/`str_len` `i32.load` the header.
     **JVM (iir-to-jvm-class-file 0.28.0)** `str` is a `java.lang.String`;
     validator accepts `str` on `call`/`ret`. **CLR** `str` is a `System.String`;
     already accepted `str` call/ret + lowered the returned string (no code change,
     only added to the cell). **NativeAot / VM / JIT** already carried a call-result
     runtime string. Bringing E4d-AL up also fixed a **latent native miscompile**
     (`strip_dead_aot_string_allocs` dropped all but the last buffer of a
     multi-block string alias → the not-last branch printed `""`; twig-aot 0.28.0).
   - **E4d-BA-input — BASIC string `INPUT`.** ✅ **Landed on all 7 backends**
     (`dartmouth-basic-iir-compiler` 0.36.0). `INPUT A$` reads a whole stdin line
     as a runtime string via `call_builtin "input_str"` (the `str` sibling of
     numeric `input_i64`); `PRINT A$` echoes it. Two matrix cells: `INPUT A$` →
     `"OK"`, and runtime concat `INPUT A$ / INPUT B$ / PRINT A$ + B$` → `"OK!"`.
     Per-backend `input_str`: native `__twig_input_str` C helper, WASM
     `env.__input_str` linear-memory writer, LLVM `@__twig_input_str`, JVM
     `BasicRuntime.readLine()`, CLR `Console.ReadLine()`, VM/JIT `input_str`
     closures returning a tagged `Value::Str`.
   - **E4d-BA-arr — BASIC string arrays.** ✅ **COMPLETE — all 7 backends**
     (`dartmouth-basic-iir-compiler` 0.37.0 / `iir-to-wasm` 0.36.0 /
     `iir-to-llvm` 0.36.0 / `iir-to-jvm-class-file` 0.30.0 /
     `iir-to-cil-bytecode` 0.39.0 / `x86_64-backend` 0.24.0 /
     `aarch64-backend` 0.23.0 / `lang-aot` 0.194.0). `DIM A$(n)` allocates an
     `array<str>` (the E5 aggregate substrate carrying an E4-dyn string handle per
     element); `A$(i) = s` → a `str`-typed `array_set`, `A$(i)` read → a `str`-typed
     `array_get` feeding PRINT / `+` concat. Matrix cell
     `DIM A$(2); A$(0)="O"; A$(1)="K"; PRINT A$(0)+A$(1)` → `OK` on all 7 backends.
     Per backend: **VM/JIT** tagged `Value::Str` element; **WASM** a 4-byte i32
     handle per element (`wasm_array_elem` `str` branch + a folded-literal-into-
     `array_set` promotion); **LLVM** an i64 handle (`str`→`i64`) with an `array_set`
     `ptrtoint` guard for folded literals; **NativeAot** an 8-byte handle
     (`native_array_elem_size` accepts `str`); **JVM** a `java.lang.String[]`
     reference array (`anewarray` + `aaload`/`aastore`); **CLR** a
     `System.String[]` (`newarr …System.String` + `ldelem.ref`/`stelem.ref`).

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
