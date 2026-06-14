# LANG77 — The shared lisp-native runtime (cons / symbols / `pair?` / `eq` on every native backend)

**Status:** Active. **L3b-2a (this PR): the runtime + its divergence-guard golden test land first**, with no lowering or backend changes, so existing native compilation is untouched and the new primitive is fully unit-tested on the host. L3b-2b wires the shared lowering + backends to call it; L3b-2c adds symbols / `ATOM` / `EQ` end-to-end.

> **One sentence.** Give the AOT pipeline a small, language-agnostic C
> implementation of `lispy-runtime`'s tagged-value model — `cons`, `car`,
> `cdr`, `pair?`, `equal?`, `not`, interned `symbol`s, `nil` — so that
> **any** lisp-family frontend (Twig today, McCarthy Lisp today, a future
> Scheme/Clojure/Lisp tomorrow) compiles its heap+symbol programs to a
> native executable *for free*, with zero language-specific code in the
> runtime, the lowering, or the backends.

---

## 1. Why this exists (and why it is not a McCarthy feature)

McCarthy Lisp's L3b-1 taught the native backends to allocate **raw-word**
cons cells (`__twig_alloc_bytes` + `field_store`/`field_load`). That ran
`(CAR (CONS 7 9))` → `7`, but it is a **structural dead-end**: raw words
carry no type tag, so there is no way to ask `pair?`, distinguish a symbol
from an integer, or implement `EQ`. The literal McCarthy worked example
`(CAR '(A B C))` → `A` needs *symbols*, and `ATOM`/`EQ` need a *tag*.

The naïve fix — bolt McCarthy-specific tagging into the backend or a
McCarthy-only lowering — would be a **shortcut that helps exactly one
language**. The repo already has *two* lisp-family frontends on the shared
`lispy-runtime` value model (Twig and McCarthy Lisp), and intends more.
The right investment is a **reusable primitive**: implement
`lispy-runtime`'s value model once, in a place every native backend can
link, and route the *shared* builtin-lowering pass through it. Then:

- **Twig** (already a lisp-family language) gets native `cons`/symbols the
  moment its frontend emits the same `call_builtin "cons"/"pair?"/…` IIR —
  no Twig-specific work.
- **McCarthy Lisp** gets `(CAR '(A B C))` → `A` natively.
- **Any future lisp** inherits native heap+symbol compilation by emitting
  the same builtin names. The cost of "make a new lisp compile to a native
  binary" drops to "write a frontend that lowers to `lispy-runtime`
  conventions" — which is already the L2 contract.

This is the same philosophy as LANG75's V1 helper table ("add a C function
+ a table row, no backend changes") extended from scalar I/O to the lisp
value model.

---

## 2. The ABI contract (the single source of truth)

`lispy-runtime`'s `value.rs` is written **as an ABI specification** — a
`LispyValue` is a `#[repr(transparent)] u64`, exactly 8 bytes, with a 3-bit
tag in the low bits, and crosses every `extern "C"` boundary as an opaque
machine word (`bits()` / `from_raw_bits`). The native runtime implements
*that documented contract*; it does not re-derive a scheme.

| Tag (low 3 bits) | Kind | Encoding | Decode |
|---|---|---|---|
| `0b000` `TAG_INT` | Integer | `(n << 3)` | arithmetic `>> 3` (sign-extends), range ±2⁶⁰ |
| `0b001` `TAG_NIL` | Nil singleton | whole word `== 0b001` | `x == 1` |
| `0b010` `TAG_SYMBOL` | Interned symbol | `(id << 32) \| 0b010`, id in high 32 bits | `(x >> 32) as u32` |
| `0b011` `TAG_FALSE` | `#f` singleton | whole word `== 0b011` | `x == 3` |
| `0b101` `TAG_TRUE` | `#t` singleton | whole word `== 0b101` | `x == 5` |
| `0b111` `TAG_HEAP` | Heap pointer | `ptr \| 0b111`, ptr 8-byte aligned | `x & ~0b111` |
| `0b100`,`0b110` | reserved | — | — |

Truthiness (Scheme/lispy): a value is **false** iff it is `#f` *or* `nil`;
everything else (including `0`) is true. `not(x)` returns `#t` iff `x` is
false, else `#f`.

**These constants are `pub` and re-exported at the `lispy-runtime` crate
root** (`TAG_INT`, `TAG_NIL`, …). The native runtime's notion of each is
pinned against them by a golden test (§6) — divergence becomes a failing
build, never a silent corruption.

---

## 3. The runtime: `twig-aot/runtime/lispy_runtime.c`

A portable C translation unit added to the **existing** `twig-aot` runtime
archive (the same archive that already carries `__twig_print_i64`,
`__twig_alloc_bytes`, …). It reuses **100%** of the embed/link plumbing:
`build.rs` adds one `.file("runtime/lispy_runtime.c")` to the existing
`cc::Build`, and `src/lib.rs`'s `include_bytes!`-the-archive →
write-to-temp → hand-to-`ld` path is unchanged.

Exported symbols (named `__twig_lispy_*` to fit the backends' uniform
`__twig_`-prefix rule, so the backend needs no special case):

| Symbol | Signature | Meaning |
|---|---|---|
| `__twig_lispy_box_int` | `(i64) -> u64` | `(n << 3)` — tag an integer |
| `__twig_lispy_unbox_int` | `(u64) -> i64` | arithmetic `>> 3` — untag at the program boundary (e.g. exit code) |
| `__twig_lispy_nil` | `() -> u64` | the `nil` singleton (`0b001`) |
| `__twig_lispy_cons` | `(u64, u64) -> u64` | allocate a 2-word cell (16 B, 8-aligned), store car/cdr, return `ptr\|0b111` |
| `__twig_lispy_car` | `(u64) -> u64` | `*(ptr & ~7)` |
| `__twig_lispy_cdr` | `(u64) -> u64` | `*((ptr & ~7) + 8)` |
| `__twig_lispy_pair_p` | `(u64) -> u64` | tagged `#t`/`#f`: `(x & 7) == 7` |
| `__twig_lispy_not` | `(u64) -> u64` | tagged `#t`/`#f`: true iff `x` is `#f` or `nil` |
| `__twig_lispy_equal` | `(u64, u64) -> u64` | tagged `#t`/`#f`: structural deep equality (atoms by bits, pairs by recursion) |
| `__twig_lispy_make_symbol` | `(const char*, i64) -> u64` | intern the name, return `(id << 32)\|0b010` |

Plus **tag-accessor** functions (`__twig_lispy_tag_int()`, `…_tag_nil()`,
`…_tag_symbol()`, `…_tag_false()`, `…_tag_true()`, `…_tag_heap()`,
`…_tag_mask()`) used **only** by the golden test to read what the C side
believes each tag is.

### Heap and intern

- **Cons heap:** `calloc(1, 16)` per cell (zero-init, ≥16-byte aligned on
  every libc, satisfying the 8-byte-alignment invariant the OR-with-tag
  scheme requires). V1 intentionally leaks (no `free`), matching
  `__twig_alloc_bytes`' "valid until process exit" contract — fine for
  AOT'd command-line programs.
- **Intern table:** a fixed-capacity open-addressing hash from name bytes
  → 32-bit id, so repeated `make_symbol("FOO")` returns the same id and
  `EQ`/`equal?` on symbols falls out as bitwise equality. Capacity is
  generous and overflow is a hard `__twig_exit`-style abort (no silent
  collision). This mirrors `lispy-runtime`'s interner *by contract* (same
  observable behaviour: same name ⇒ same id), not by sharing its table —
  the AOT executable is self-contained and never shares memory with the
  Rust VM.

### Is the C re-implementation a "second source of truth" smell?

No — it is the **AOT counterpart of the VM/JIT runtime sharing one
documented ABI**, exactly like `__twig_print_i64` is the C counterpart of
the VM's print. `lispy-runtime` is full-`std` (`Mutex`, `HashMap`,
`OnceLock`, `Box::leak`); linking *it* as a Rust staticlib would drag
`std` + panic machinery into every AOT binary (which today links only
libc), and the repo has **no** mechanism (and no precedent) for building a
Rust staticlib from a dependent's `build.rs` (cargo does not order a
`staticlib` dependency's artifact before a dependent's build script). The C
runtime adds **zero** new runtime dependency and reuses the proven archive
mechanism. The only real risk — the two implementations drifting — is
eliminated by the §6 golden test, which is **mandatory in the first PR**.

---

## 4. The shared lowering (L3b-2b)

`iir-builtin-lowering` is language-agnostic (it keys on builtin *name*
strings). For the native-runtime target it lowers the lisp builtins to
`call_builtin "lispy_*"` so the backend emits a call to the runtime:

| Builtin | Native-runtime lowering |
|---|---|
| `cons` | `call_builtin "lispy_cons"` |
| `car` / `cdr` | `call_builtin "lispy_car"` / `"lispy_cdr"` |
| `pair?` | `call_builtin "lispy_pair_p"` |
| `not` | `call_builtin "lispy_not"` |
| `equal?` | `call_builtin "lispy_equal"` |
| `make_symbol` | `call_builtin "lispy_make_symbol"` |
| integer literal | boxed at creation (`<< 3`) — inline in the backend or `call_builtin "lispy_box_int"` |
| program result | unboxed at the entry/exit boundary (`>> 3`) |

The **managed** backends (wasm/jvm/clr/beam) keep the existing structural
lowering (`alloc`/`field_*`/`is_null`) — they have native GC'd objects and
do not link a C runtime. So the lowering is **target-aware**: structural
for managed, runtime-call for native. This is a general distinction
("lower to host GC objects" vs "lower to the linked value runtime"), not a
per-language one. The box/unbox boundary lives at the AOT entry/exit (the
generic "raw machine int ↔ tagged lisp value" seam), so it is shared by all
lisp frontends; only frontends with *arithmetic* (e.g. Twig) also box/unbox
around numeric ops, which is itself generic IIR.

> McCarthy Lisp 1.0 has **no arithmetic** (its primitives are
> `ATOM EQ CAR CDR CONS COND QUOTE LAMBDA LABEL`), so its only box/unbox
> sites are int-literal creation and the program-exit boundary — the
> simplest possible exercise of the seam, which is why it is the first
> frontend wired.

---

## 5. The backends (L3b-2b)

The native backends already turn `call_builtin "<name>"` into a call to
`__twig_<name>` via `V1_BUILTINS` (name + arity + returns-bool) +
`bl_external`/`call_rel32`, with `code-packager` emitting the external
relocation. So the **entire** backend change is a handful of generic
`BuiltinSig` rows (`lispy_cons`/2, `lispy_car`/1, … → `__twig_lispy_*`) in
**both** x86_64 and aarch64. The backend gains **no** lisp-specific logic;
it stays "emit a call to a named runtime symbol." Truthiness handling for
`COND` (a tagged `#t`/`#f`/`nil` condition vs a 0/1 branch test) is done in
the lowering, not the backend.

---

## 6. The divergence guard (golden test — ships in L3b-2a)

`cc::Build::compile` makes cargo link the runtime archive into `twig-aot`'s
**own** test binary (host linker), so a Rust test calls the C functions
directly. The test (a) imports the `pub` tag constants and constructors
from `lispy-runtime` and (b) `extern "C"`-declares the runtime symbols, then
asserts the C side reproduces the Rust ABI exactly:

```rust
use lispy_runtime::{LispyValue, TAG_INT, TAG_NIL, TAG_SYMBOL, TAG_FALSE, TAG_TRUE, TAG_HEAP, TAG_BITS};
// tag accessors pinned to the canonical constants
assert_eq!(unsafe { __twig_lispy_tag_int() }, TAG_INT);          // and nil/symbol/false/true/heap/mask
// encodings pinned to the canonical constructors
assert_eq!(unsafe { __twig_lispy_box_int(7) }, LispyValue::int(7).bits());
assert_eq!(unsafe { __twig_lispy_nil() },       LispyValue::NIL.bits());
// behaviour: round-trip + predicates
let c = unsafe { __twig_lispy_cons(__twig_lispy_box_int(7), __twig_lispy_box_int(9)) };
assert_eq!(unsafe { __twig_lispy_car(c) }, LispyValue::int(7).bits());
assert_eq!(unsafe { __twig_lispy_pair_p(c) }, LispyValue::TRUE.bits());
assert_eq!(unsafe { __twig_lispy_pair_p(__twig_lispy_box_int(7)) }, LispyValue::FALSE.bits());
// symbols: same name ⇒ same id, correct tag
let a = unsafe { __twig_lispy_make_symbol("FOO".as_ptr().cast(), 3) };
let b = unsafe { __twig_lispy_make_symbol("FOO".as_ptr().cast(), 3) };
assert_eq!(a, b);
assert_eq!(a & TAG_BITS, TAG_SYMBOL);
```

If a later PR changes any `TAG_*` constant or encoding in `value.rs`, this
test fails at `cargo test` — the two implementations can never silently
diverge. This is what makes the C-runtime split safe rather than a
duplication hazard.

This test runs on the dev host (macOS arm64 here) and on every CI runner,
because linking the static archive into a *Rust test binary* uses the
normal host linker — it is unaffected by the separate, pre-existing macOS
limitation on linking runtime helpers into AOT-*produced* executables.

---

## 7. Slicing

| Slice | Scope | Verification |
|---|---|---|
| **L3b-2a** (this PR) | `lispy_runtime.c` (full runtime), `build.rs` `.file()`, golden test, `twig-aot` dev-dep on `lispy-runtime`, spec. **No lowering/backend changes** → no regression risk. | Golden test green on host + CI; existing smoke tests unchanged. |
| **L3b-2b ✓** | Target-aware native lowering of `cons`/`car`/`cdr` → `lispy_*` (`lower_heap_builtins_runtime`); `V1_BUILTINS` rows in both backends. (Int boxing + unbox-at-exit deferred to L3b-2c, where the tag is first inspected — for the pure cons/car/cdr data path, raw payloads round-trip identically through the runtime.) | `(CAR (CONS 7 9))` → native exe exits `7` **through the linked runtime** (Linux/Windows CI; macOS AOT-exe gap pre-existing). Backends emit external relocs to `__twig_lispy_cons`/`__twig_lispy_car` — verified host-independently in `aarch64-backend`/`x86_64-backend` unit tests. |
| **L3b-2c-1 ✓** | Type-directed lisp-value **representation** (`iir-builtin-lowering::lower_lisp_repr`): box integer atoms that flow into `lispy_*` calls (`n << 3`), tag the nil sentinel (`0b001`), unbox the program result at the exit boundary (`lispy_unbox_int`). Gate-free / use-site-directed — non-lisp programs untouched. | `(CAR (CONS 7 9))` → 7 through fully **tagged** values (was raw payloads in 2b). 7 unit tests + host-independent backend tests for the boxed cons/car/unbox sequence; the Linux/Windows e2e still exits 7. |
| **L3b-2c-2 ✓** | `pair?`/`not`/`equal?` (`ATOM`/`EQ`) renamed → `lispy_pair_p`/`lispy_not`/`lispy_equal`; a new `__twig_lispy_truthy` (tagged → raw `0`/`1`) normalises tagged `COND` predicates for `jmp_if_false`; `lower_lisp_repr` extended (predicate-arg boxing, **bidirectional `mov`** boxing for `COND` funnels, truthiness wrapping). | `(COND ((ATOM 5) 7) (5 9))` → 7; `(COND ((ATOM (CONS 1 2)) 7) (5 9))` → 9 (Linux/Windows e2e). Predicate + truthy relocs verified host-independently; truthy truth-table pinned in the golden test. (Explicit-`('T …)`-else COND needs symbols → 2c-3.) |
| **L3b-2c-3 ✓** | **Compile-time symbol interning** (`iir-builtin-lowering::intern_symbols`) — `const Var(name):symbol` → the tagged immediate `(id<<32)\|TAG_SYMBOL`, module-wide ids, so `EQ`/`equal?` on symbols is word equality. *Diverges from the original plan:* runtime `make_symbol` + string-literal emission is **not** needed for symbol *values* — only for printing a symbol's *name* / dynamic symbol creation, which the native backend has no string-constant support for, so it stays deferred. `lisp_repr` treats a symbol immediate as tagged-but-never-boxed. No backend change (a symbol is a tagged `const_i64`; `EQ` reuses `lispy_equal`). | `(CAR '(A B C))` → symbol `A`, observed via `EQ`: `(COND ((EQ (CAR '(A B C)) 'A) 7) ('T 9))` → 7; the `'B` variant → 9 (Linux/Windows e2e). 5 intern/guard unit tests. |
| **L3b-3** | Wire the managed backends (wasm/jvm/clr/beam) + the per-`--emit` acceptance matrix for symbols (they already lower cons). | The worked example emits a non-trivial artifact on every target. |

**Reusability acceptance:** once L3b-2b lands, a **Twig** `(car (cons 7 9))`
program compiles natively through the *same* runtime + lowering, with no
Twig-specific code — the proof that this is a primitive, not a McCarthy
feature. Captured as a parallel smoke test.

---

## 8. Why not the Rust staticlib (rejected alternative)

Linking `lispy-runtime` itself into AOT executables was considered and
rejected: (a) cargo gives no guarantee that a `staticlib`-typed dependency
is built before a dependent crate's `build.rs` runs, so a `build.rs`
embedding its `.a` would glob for a file that may not exist; (b)
`build.rs`-invokes-`cargo` (the only alternative) is the classic
target-dir-lock / profile-mismatch footgun and has **no precedent** in this
repo; (c) `cargo:rustc-link-*` directives resolve at *twig-aot's* build, but
twig-aot links at *its own runtime* (when compiling user lisp), so the
archive must be embedded bytes, not link-time-resolved; (d) `lispy-runtime`
is full-`std`, so its staticlib drags `std`/panic/`Mutex` into every AOT
binary. The C runtime sidesteps all four and reuses the existing,
proven archive mechanism, at the cost of one re-implementation that the
golden test pins to the canonical ABI.
