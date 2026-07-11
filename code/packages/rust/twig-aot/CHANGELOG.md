# Changelog — `twig-aot`

## 0.31.0 - 2026-07-11 (DVAL01-1a: dynamic-value runtime ABI __twig_lispy_* -> __dyn_*)

De-lisp the tagged dynamic-value runtime ABI: every `__twig_lispy_*` C symbol (box_int/unbox_int/cons/car/cdr/pair_p/equal/not/nil/make_symbol/truthy/to_exit_code/tag_*) is renamed to the language-neutral `__dyn_*` (per spec DVAL01). Pure rename -- the 3-bit tag layout, encodings, and runtime behaviour are byte-for-byte unchanged, so any dynamic frontend (not just lisp) can target the same primitives. The GC ABI (`__twig_gc_*`) is untouched.

## 0.30.0 — 2026-07-07 — E4-dyn: runtime `str_concat` → `call_builtin "str_concat"`

`lower_string_literals_for_aot`'s `str_concat` handler now mirrors the proven
`str_eq` path. It keeps the both-operands-literal **compile-time fold** (bake the
joined literal into the data segment; the result stays a known literal so
downstream `str_len`/`str_index` keep folding), but when either operand is a
runtime string handle — an `INPUT` result, a call result, a branch-selected
string — it emits `call_builtin "str_concat" a b → dest`, delegating to the
existing `__twig_str_concat(a, b)` archive helper (which reads both `[i64 len]
[bytes]` headers and returns a fresh joined block). `dest` is deliberately NOT
recorded as a literal, so `print_str`/`str_len` on it take their runtime
header-reading paths.

- Tests: `runtime_str_concat_lowers_to_call_builtin_str_concat` +
  `literal_str_concat_still_folds_to_data_segment` (fold fast-path regression guard).

## 0.29.0 — 2026-07-07 — E4-dyn: `__twig_input_str` runtime helper (BASIC string INPUT)

Adds `int64_t __twig_input_str(void)` to `runtime/twig_runtime.c` — the string
counterpart of `__twig_input_i64`. It reads one line from stdin and returns an
i64 **handle** to a fresh length-prefixed `[i64 len][bytes]` heap block (built on
`__twig_alloc_bytes`) — the same runtime-string repr `__twig_print_string` /
`__twig_str_eq` already consume — so BASIC's string `INPUT A$` runs on the native
AOT column (`PRINT A$` reads the header length at run time). A single line is
bounded by a 4096-byte stack buffer (a longer line is truncated, tail consumed
next read); EOF yields a handle to a zero-length `""` block (never NULL). No
backend change — the aarch64/x86_64 tables add `input_str` as a 0-arg/returns-i64
`V1_BUILTINS` entry (the pointer rides x0/RAX like any `alloc_bytes` result).

## 0.28.0 — 2026-07-03 — E4d-4 fix: keep every buffer of a multi-block string alias

Fixed a latent native miscompile in `strip_dead_aot_string_allocs`, surfaced by
ALGOL string procedures (a runtime string *returned* from a procedure and printed
by the caller).

`lower_string_literals_for_aot` emits an alias `mov s = buf` after each
`str_const`'s buffer so the str variable `s` can be used by `call`/`ret`. The
dead-alloc stripper collected these in a `HashMap<alias, buf>` keyed by the alias
name — assuming one buffer per string variable. But an E4d-4 promoted
(branch-selected) variable is the dest of `str_const` in **more than one basic
block**, so a single alias name (`s`) maps to **several** buffers (one per
branch). The map kept only the last, and the stripper then deleted the other
branches' `alloc_bytes`/`store_byte` blocks as "dead" even though a live
`mov s = buf` still referenced them. At run time the branch that selected an
earlier buffer read freed/empty memory (e.g. printed `""`). The E4-dyn foothold
only dodged this because the input it tested happened to select the *last-defined*
branch's buffer.

Fix: track every `(alias, buffer)` pair (a `Vec`, not an alias-keyed map). A
buffer is live iff it is directly referenced **or** its alias is live — so **all**
buffers of a live alias survive. Regression test
`multi_block_string_keeps_every_branch_buffer` builds a two-branch string
(`"LONGER"`/`"HI"`) and asserts both `alloc_bytes` and both alias-`mov`s survive.
This also hardens the E4d-4 foothold (a program that selects the earlier branch
now prints correctly) and is what lets an ALGOL `string procedure` return a
runtime string on NativeAot.

## 0.27.0 — 2026-07-03 — E4-dyn E4d-4: runtime (branch-selected) strings on native

Completes the E4-dyn backend ladder: the runtime branch-selected-string foothold
now runs on the **native aarch64 + x86_64** columns, so the E4-dyn foothold is
proven on **all seven backends**.

**How native strings already worked.** `lower_string_literals_for_aot` lowers each
`str_const` to a `push_aot_string_literal` block — an `alloc_bytes` heap buffer with
the E5/E4d-1 `[i64 len][bytes]` layout (`field_store` len at offset 0, `store_byte`s
at offset 8+) — followed by `mov dest = buf`, so the variable's stack slot already
holds the buffer's **address** (a runtime handle). And `print_str`/`str_len` already
have a runtime branch that reads the length header from the buffer at run time
(`field_load src[0]`), used today for runtime string *parameters*.

**The bug for a branch-selected local.** `str_const` also registered every `dest`
in the compile-time `strings` map (last-writer-wins). So for `A$` assigned `"LO"` in
one block and `"HI"` in another, `print_str A$` took the *static-length* path using
whichever literal was written last — printing the wrong length whenever the two
branches' strings differ in length.

**The fix.** Add `collect_runtime_str_vars_for_aot` (same basic-block promotion rule
as `iir-to-llvm`'s `collect_slot_vars` and `iir-to-wasm`'s `collect_runtime_str_vars`:
a `str`-typed dest appearing in >1 basic block). In `str_const`, a promoted var is
**not** registered in `strings`, so every downstream `print_str`/`str_len`/`str_eq`/
`str_cmp` on it takes its existing runtime path. No backend change is needed — the
buffer + slot address were already correct — so one change covers both aarch64 and
x86_64. Single-assignment (and straight-line-reassigned) strings are unchanged.

Two unit tests: `branch_selected_string_reads_length_at_runtime` (differing lengths
`"LONGER"`/`"HI"` → asserts the emitted `field_load A[0]` runtime length read) and
`straight_line_reassignment_keeps_static_length` (one block → no `field_load`). The
`lang-aot` E4-dyn foothold matrix cell adds `NativeAot` and now proves the runtime
string on all 7 backends (aarch64 run-verified locally; x86_64 on CI).

## 0.26.0 — 2026-07-03 — E4-dyn E4d-1: runtime string helpers

First step of the **E4-dyn** arc (`code/specs/lang-full-e4-dyn-strings.md`):
runtime (non-literal) strings on the static backends. This PR lands the
**runtime C helpers** — the shared substrate the LLVM/WASM/native backends will
lower to in E4d-2…4 — without touching any backend yet.

Added to `runtime/twig_runtime.c`, all operating on the **same length-prefixed
heap block E5 arrays use** (`offset 0: int64_t length`, `offset 8: bytes`) via
`__twig_alloc_bytes`:

| helper | semantics |
|--------|-----------|
| `__twig_str_len(s)` | byte length (offset-0 header); null/corrupt → 0 |
| `__twig_str_concat(a,b)` | fresh block = a's bytes then b's; null operand = empty; overflow/OOM `abort()` |
| `__twig_str_slice(s,start,end)` | fresh block = bytes `[start,end)`; **traps** (`abort()`) on out-of-range or backwards range (E4 bounds contract) |
| `__twig_str_index(s,i)` | unsigned byte `0..255` at `i`; **traps** on out-of-range |
| `__twig_str_cmp(a,b)` | lexicographic byte compare → `-1/0/1`; shorter string is "less" on a tie |

Every producing helper allocates a **fresh** block and copies — it never writes
through an operand handle, upholding E4's immutable-value contract. Null/corrupt
handles are guarded (a negative length header is treated as empty, never a
size_t over-read), matching the existing `__twig_str_eq`. `__twig_str_eq`
already reads these blocks and is unchanged.

**Tests** (`tests/e4d_str_helpers.rs`, Unix-gated): a C driver compiled with the
runtime via the system `cc` validates concat/len/slice/index/cmp on valid inputs
(incl. empty operands, boundary slices, unsigned-byte index, and operand
immutability), plus three trap cases (out-of-range index, out-of-range slice,
backwards slice range) that must `abort()`.

No backend or IIR change; the static-backend lowering that calls these helpers
is E4d-2 (LLVM), E4d-3 (WASM), E4d-4 (native). VM/JIT already run runtime strings.

## 0.25.0 — 2026-07-01 — TWIG-GC: conservative mark-and-sweep GC for native AOT

**TWIG-GC** (native-aot-substrate Layer 1, `code/specs/native-aot-substrate.md`):
Added `runtime/twig_gc.c` — a portable conservative mark-and-sweep garbage
collector for every language that targets the native AOT backend.

Key design points:

- **Conservative stack scanning**: registers flushed via `setjmp`; every stack
  word (and `word & ~0x7` for NaN-boxed Lispy pointers) is tested against the
  live-object table.  No GC maps or safepoint metadata needed from the compiler.
- **32-byte `gc_header_t`** prepended before each allocation; payload is always
  16-byte–aligned, satisfying the Lispy HEAP-tag requirement (low 3 bits clear).
- **Adaptive threshold**: starts at 1 MB; doubles when >50% of heap survives;
  halves otherwise (floor 1 MB) — matches Go/JVM ergonomic GC heuristics.
- **Public API**: `__twig_gc_alloc(n)`, `__twig_gc_collect()`,
  `__twig_gc_safepoint()`, `__twig_gc_live_bytes()`, `__twig_gc_collection_count()`.
- **Stack-base detection**: macOS via `pthread_get_stackaddr_np`; Linux via
  `pthread_getattr_np + pthread_attr_getstack`; Windows via
  `__readgsqword(0x08)` (TEB.StackBase).

**`lispy_runtime.c` update**: `__twig_lispy_cons` now calls `__twig_gc_alloc(16)`
instead of `calloc(1, 16)`.  Cons cells are now GC-tracked and freed when
unreachable; McCarthy Lisp programs no longer leak heap on every `CONS`.

**`build.rs` update**: `twig_gc.c` added to the `cc::Build` compilation alongside
`twig_runtime.c` and `lispy_runtime.c`; `cargo:rerun-if-changed` wired up.

## 0.24.0 — 2026-07-01 — LANG-STR-RT: string function parameters on NativeAot

**Root cause fixed:** `str_const dest = "HELLO"` was removed from the IIR
instruction stream after `lower_string_literals_for_aot`, leaving `dest`
undefined.  When `call strlen(dest)` followed, the backend loaded the
uninitialized stack slot and passed 0 to the callee — causing
`str_len(param)` to read 0 from the buffer header instead of the actual length.

**Fix — alias mov:** After every `push_aot_string_literal` block, now emit
`mov dest = buf_var` (type `i64`) so `dest` is always a defined, live pointer
to the LANG-STR-RT buffer.  `strings[dest] = (dest, len_var, literal)` so
subsequent string ops still fold statically.

**Fix — dead-stripping alias tracking:** `strip_dead_aot_string_allocs` was
updated to understand the alias pattern:
- `alias_movs` — collects `{dest → buf_var}` from `mov` instructions whose
  src is an `__aot_str{N}_buf` variable.
- `live_alias_dests` — dests that appear in srcs of any non-write-only,
  non-alias-mov instruction (e.g. `call strlen(dest)` makes `dest` live).
- A buf is live if directly referenced OR its alias dest is live.
- Dead buf blocks AND their alias-movs are stripped together when the alias
  is not observed outside already-folded string ops.

This ensures `FrameTooLarge` cannot be triggered by fold-only strings
(their buffers are still dead-stripped), while passing strings to function
calls now works correctly.

**New unit tests** (in `tests` module):
- `string_param_len_lowers_to_field_load` — `str_len` on a function parameter
  generates `field_load(s, 0)` runtime fallback.
- `string_param_eq_lowers_to_call_builtin_str_eq` — `str_eq` with a non-literal
  operand generates `call_builtin "str_eq" left right`.
- `string_literal_buffer_has_length_header` — buffer for "hello" allocates 13
  bytes, stores `field_store buf, 0, 5`, and writes 5 bytes at offsets 8–12.

**`__twig_str_eq` C helper** (`runtime/twig_runtime.c`): reads the 8-byte
length prefix from each LANG-STR-RT buffer and does a `memcmp` over the data
region.  Returns 1 (equal) or 0 (not equal).

## 0.23.0 — 2026-06-30 — strip dead AOT string-literal allocation blocks

Added `strip_dead_aot_string_allocs` pass (called from `prepare_module_for_aot`
after `lower_string_literals_for_aot`):

After `lower_string_literals_for_aot` folds literal-only `str_eq`/`str_cmp`
ops to constant integers, the `push_aot_string_literal` blocks that allocated
the buffer (`alloc_bytes` + `store_byte` writes) are left with no live
consumer — their `__aot_str{N}_buf` variable is written but never read.

On aarch64, each such dead block uses 8+ frame slots. With multiple string
comparisons (e.g. the ALGOL `s = 'ALPHA' and s != 'OMEGA'` cell), the
accumulated frame size exceeded the 504-byte limit of the
`stp_pre`/`ldp_post` 7-bit signed immediate, causing
`BackendError::FrameTooLarge` in the NativeAot backend.

The pass scans all `alloc_bytes` instructions whose dest starts with
`__aot_str` and ends with `_buf`, checks whether any instruction other than
`store_byte` references that var as a source, and removes the entire dead
block (the `alloc_bytes` + every `store_byte` into it) when no live
consumer exists.

## 0.22.0 — 2026-06-28 — native string comparison metadata folding (LANG-FULL E4)

`prepare_module_for_aot` now folds literal-only `str_cmp` to the shared `-1`,
`0`, or `1` integer convention before direct native lowering.

## 0.21.0 — 2026-06-28 — native substring metadata folding (LANG-FULL E4)

`prepare_module_for_aot` now folds literal-only `str_slice` results into the
same native byte-buffer metadata used by `str_const` and `str_concat`. This lets
Twig `(let ((s "ABCDE")) (string-ref (substring s 1 4) 1))` fold the slice to
`BCD` and the final `str_index` to byte `67` before direct native lowering.

## 0.20.0 — 2026-06-28 — native computed string index metadata (LANG-FULL E4)

`prepare_module_for_aot` now records folded `str_len` results as integer
metadata and propagates that metadata through typed integer arithmetic. This
lets Twig `(let ((s "ABCDE")) (string-ref s (- (string-length s) 1)))` fold the
native `str_index` to byte `69` instead of leaving an unsupported E4 string op
for the direct native backend.

## 0.19.0 — 2026-06-27 — native string literal metadata through local moves (LANG-FULL E4)

`prepare_module_for_aot` now propagates literal string and integer metadata
through `mov`, so lexical Twig bindings such as `(let ((s "ABC") (i 2))
(string-ref s i))` still fold the native `str_index` to a byte constant instead
of leaving an unsupported string op in the direct native backend.

## 0.18.0 — 2026-06-27 — native literal string index OOB trap (LANG-FULL E4)

`prepare_module_for_aot` now preserves the E4 trap contract for direct-literal
`str_index` when the index is statically out of range. The native rewrite emits
an unconditional `type_assert` trap before a dummy destination seed, so
`(string-ref "ABC" 3)` reaches native machine code and traps at runtime instead
of being rejected before execution.

## 0.17.0 — 2026-06-27 — native literal string index lowering (LANG-FULL E4)

`prepare_module_for_aot` now folds direct-literal `str_index` when both the
string value and index are statically known. Twig `(string-ref "ABC" 1)` now
runs through the native AOT column and returns byte/codepoint `66`, matching the
shared E4 byte-string semantics for printable ASCII. Dynamic string indexing and
the out-of-bounds trap matrix proof remain follow-up runtime slices.

## 0.16.0 — 2026-06-27 — native literal string metadata lowering (LANG-FULL E4)

`prepare_module_for_aot` now folds `str_len`, `str_eq`, and literal
`str_concat` metadata over direct string literals before native machine-code
lowering. This lets Twig `(string-length "HELLO")`,
`(string=? "HELLO" "HELLO")`, and
`(string-length (string-append "AB" "CDE"))` run through the native AOT column
without adding rodata string objects or a dynamic string runtime. The existing
`str_const` + `print_str` rewrite still handles literal output, and non-literal
byte-string ops remain deferred.

## 0.15.0 — 2026-06-27 — native string literal PRINT lowering (LANG-FULL E4 / BA4)

`prepare_module_for_aot` now lowers the landed E4 literal-output pair
`str_const` + `print_str` into the native runtime path that already existed:
`alloc_bytes`, one `store_byte` per printable-ASCII byte, and
`call_builtin "print_string"` (`__twig_print_string(ptr,len)`). This covers the
direct native AOT column without adding object-file rodata support or duplicating
machine-backend logic.

Added unit coverage for the rewrite shape and a Mach-O object compile test that
proves the transformed IIR reaches the aarch64 backend/packager.

## 0.14.0 — 2026-06-10 — `lispy_runtime.c`: universal exit coercion (LANG77 / McCarthy W13b)

Adds `__twig_lispy_to_exit_code(uint64_t)` to the shared tagged-word C runtime: it
coerces ANY `LispyValue` to a raw exit code by dispatching on its runtime tag
(integer → `>> 3`; `#t`/`#f`/nil → `1`/`0`/`0`; symbol/pair → the tagged word
verbatim). This is the program-exit boundary for a value whose tag the compiler
cannot know statically — a **lambda** result (F7), typed `any`. It is a safe
superset of the static `unbox_int`/`truthy` helpers (they agree on every tag they
each cover). Reusable by every tagged-word backend that links this runtime (LLVM
today; native AOT W14 and JIT W15 inherit it). No Rust source change; this is a
runtime-asset addition compiled into the AOT executable.

## 0.13.0 — 2026-06-04 — native symbols (LANG77 / McCarthy L3b-2c-3)

`prepare_module_for_aot` now runs `iir_builtin_lowering::intern_symbols`
(between the heap-builtin rename and `lower_lisp_repr`): each
`const Var(name):symbol` becomes the tagged immediate `(id << 32) | TAG_SYMBOL`,
with module-wide ids. So a McCarthy program's symbols compile to native and
`EQ` on them is word equality — `(CAR '(A B C))` produces the symbol `A`,
observable via `(EQ … 'A)` driving a `COND` branch. No backend change (a
symbol is just a tagged `const_i64`; `EQ` reuses `lispy_equal`).

No symbol-name *printing* yet (that needs string-literal emission, deferred) —
static programs observe symbol identity via `EQ`.

## 0.12.0 — 2026-06-04 — ATOM/EQ + COND truthiness (LANG77 / McCarthy L3b-2c-2)

Adds `__twig_lispy_truthy` to `runtime/lispy_runtime.c` — normalises a tagged
`LispyValue` to a **raw** machine `0`/`1` (false iff `#f` or nil) so the
backend's `jmp_if_false` branches correctly on a `COND` predicate that
produced a tagged boolean. A golden-test assertion pins its truth table.

With the lowering changes in `iir-builtin-lowering` 0.6.0 (predicate renames +
`COND` truthiness wrapping + bidirectional `mov` boxing), McCarthy `ATOM`/`EQ`
now drive a native branch: `(COND ((ATOM 5) 7) (5 9))` → 7, and
`(COND ((ATOM (CONS 1 2)) 7) (5 9))` → 9. No changes to `prepare_module_for_aot`
beyond what 0.11.0 already wired.

## 0.11.0 — 2026-06-04 — tag native lisp integers (LANG77 / McCarthy L3b-2c-1)

`prepare_module_for_aot` now runs `iir_builtin_lowering::lower_lisp_repr`
right after `lower_heap_builtins_runtime`. This **type-directed** pass boxes
the integer atoms that flow into `lispy_*` calls (`n << 3`, so their NaN-box
tag is `000` rather than the heap tag a raw int's low bits would collide
with), tags the nil sentinel, and inserts an unbox at the program-exit
boundary — so `(CAR (CONS 7 9))` still exits 7, now through fully **tagged**
`LispyValue`s. This lays the representation the `pair?`/`ATOM`/`EQ`
predicates (L3b-2c-2) require.

Gate-free: the pass keys on use-sites, not the source language, so every
Twig/Nib/Brainfuck program (whose integers feed `add`/`print_i64`, never a
`lispy_*` call) is left byte-for-byte unchanged. No new opcodes — the unbox
is a `call_builtin "lispy_unbox_int"` resolved from the runtime archive.

## 0.10.0 — 2026-06-04 — route native cons through the lisp runtime (LANG77 / McCarthy L3b-2b)

`prepare_module_for_aot` now calls
`iir_builtin_lowering::lower_heap_builtins_runtime` instead of
`lower_heap_builtins`, so a lisp frontend's `call_builtin
"cons"/"car"/"cdr"` is routed to the LANG77 C runtime
(`__twig_lispy_cons`/`car`/`cdr`, shipped in the runtime archive since
0.9.0) rather than expanded to a raw-word `alloc`/`field_*` cell. Cons
cells are now proper NaN-box **tagged** `LispyValue`s — the prerequisite for
`pair?`/`ATOM`/`EQ`/symbols (L3b-2c).

`(CAR (CONS 7 9))` still compiles to a native executable that exits 7 (Linux/
Windows; the macOS AOT-exe runtime-helper gap is unchanged) — now through
the tagged runtime instead of raw words. Integer payloads inside cells stay
raw in this slice; boxing lands with the predicates in L3b-2c, where the tag
is first inspected.

The change only touches the cons/car/cdr builtin names, so every
Twig/Nib/Brainfuck program is unaffected. The L3b-1 `alloc`/`field_*`
backend emitters remain available as general-purpose heap ops.

## 0.9.0 — 2026-06-04 — the shared lisp-native runtime (LANG77 / McCarthy L3b-2a)

Adds `runtime/lispy_runtime.c` — a portable C implementation of
`lispy-runtime`'s tagged-value model (`cons`/`car`/`cdr`/`pair?`/`equal?`/
`not`/interned `symbol`s/`nil`, plus int box/unbox) — to the existing runtime
archive.  `build.rs` compiles it into `libtwig_aot_runtime` alongside
`twig_runtime.c` with one extra `.file(...)`, reusing the whole
embed/link path unchanged.

This is the **reusable primitive** that lets *any* lisp-family frontend
(Twig and McCarthy Lisp today, future lisps tomorrow) compile its heap +
symbol value model to a native executable — not a McCarthy-specific feature.
It supersedes the raw-word cons of 0.8.0 (which had no type tag, so `pair?`/
`ATOM`/`EQ`/symbols were impossible): values are now NaN-box **tagged**,
exactly as the VM/JIT sees them.

**This release ships the runtime + its divergence guard only — no lowering
or backend changes**, so existing native compilation is byte-for-byte
unchanged.  A new lib unit-test module `lispy_runtime_golden` links the C
archive into the test binary and asserts every tag constant and encoding
matches `lispy-runtime`'s canonical `pub const`s and constructors
(`LispyValue::int(_).bits()`, `TAG_INT`, …).  If the Rust ABI ever changes,
the C runtime fails `cargo test` — the two implementations cannot silently
drift.  `lispy-runtime` is added as a **dev-dependency** for that test only;
the AOT binary never links the Rust crate.

Subsequent slices (L3b-2b/c, tracked in
`code/specs/LANG77-lisp-native-runtime.md`) wire the shared
`lower_heap_builtins` pass + the backends' `V1_BUILTINS` tables to *call*
these `__twig_lispy_*` symbols, so `(CAR (CONS 7 9))` runs through tagged
values and `(CAR '(A B C))` → `A` lights up.

## 0.8.0 — 2026-06-04 — run heap-builtin lowering (McCarthy Lisp L3b)

`prepare_module_for_aot` now runs `iir_builtin_lowering::lower_heap_builtins`
(right after `lower_global_io`), so a Lisp frontend's `call_builtin
"cons"/"car"/"cdr"/"null?"` is rewritten to `alloc`/`field_store`/
`field_load`/`is_null` before infer/specialise.  The native backends
(aarch64 0.5.0 / x86_64 0.7.0) lower those to a `__twig_alloc_bytes` cell +
word loads/stores, so a McCarthy cons-of-integers program — e.g.
`(CAR (CONS 7 9))` — compiles to a native executable that exits 7.

The pass only rewrites those exact builtin names, so a module without them
(every Twig / Nib / Brainfuck program today) is left byte-for-byte
unchanged — no regression (the existing `macos_arm64_smoke` Twig native
tests still pass).

> **Note:** the macOS-executable path still can't link the runtime
> archive's C helpers (`__twig_alloc_bytes` etc.) — a pre-existing
> limitation shared with Brainfuck's tape — so cons programs run natively
> on Linux/Windows; macOS native + runtime helpers is a separate
> build-system fix.  See `lessons.md`.

## 0.7.0 — 2026-05-20 (LANG76 — byte memory ops + heap allocation)

Runtime archive gains one new helper:

| Symbol                | Purpose                                              |
|-----------------------|------------------------------------------------------|
| `__twig_alloc_bytes`  | `calloc(1, n)` — zero-initialised heap allocation.   |

V1 leaks (no `__twig_free`); per the LANG76 spec, that's fine for AOT'd
command-line scripts.  Negative or zero `n` returns NULL (`0`).

**End-to-end smoke tests:** `tests/{windows,linux}_x86_64_smoke.rs`
each grow a new `end_to_end_lang76_heap_byte_io_writes_hi` test that
hand-builds an `IIRModule` calling `alloc_bytes 4`, writes `'H','i','\n'`
via three `store_byte` instructions, then calls `print_string` over
the buffer.  Asserts stdout is exactly `"Hi\n"`.

## 0.6.0 — 2026-05-20 (LANG75 — runtime-archive expansion)

**Runtime archive grows the V1 helper table.**

`runtime/twig_runtime.c` adds five new helpers to support the LANG75
`call_builtin` opcode that both backends now emit:

| Symbol                | Purpose                                      |
|-----------------------|----------------------------------------------|
| `__twig_putchar`      | Write one byte to stdout.                    |
| `__twig_getchar`      | Read one byte from stdin (-1 on EOF).        |
| `__twig_print_string` | Write `len` bytes from `ptr` to stdout.      |
| `__twig_input_i64`    | Read a line and parse a signed int64.        |
| `__twig_exit`         | Terminate the program with the given code.   |

The existing `__twig_print_i64` is unchanged.  No changes to the
build-time archive packaging — `cc::Build::compile` rebuilds the
single C file and embeds the resulting archive bytes the same way it
always has.

**End-to-end smoke tests:** `tests/windows_x86_64_smoke.rs` and
`tests/linux_x86_64_smoke.rs` each grow a new
`end_to_end_call_builtin_putchar_writes_hi` test that hand-builds an
`IIRModule` emitting three `call_builtin "putchar"` instructions and
asserts the linked executable writes exactly `"Hi\n"` to stdout.

**Compatibility.**  Existing Twig programs that compile today (which
use only `io_out`, lowering to `__twig_print_i64`) produce
byte-identical output — the new helpers are additive only.

## 0.5.0 — 2026-05-16 (`--emit-object` for cross-OS workflows)

**Cross-OS object emission via a new `--emit-object` flag.**

The third follow-up from PR #3203.  The `.o` / `.obj` object format
is fully portable — only the *link step* is bound to the target
host's toolchain.  This release exposes that asymmetry: produce the
object file on any host, then copy it to a target machine and link
it there.

```
# On Windows: produce a Linux ELF .o
twig-aot foo.twig --target=linux-x86_64 --emit-object -o out/foo

# Output:
# twig-aot: emitted object: out/foo.o
# twig-aot: NOTE: runtime archive for LinuxX86_64 was not built on
#           this host (1-byte stub).  Build twig-aot on a
#           LinuxX86_64 host or rebuild the runtime from
#           `twig-aot/runtime/twig_runtime.c` on the target machine.
```

When the runtime archive *is* available for the target (i.e. the
twig-aot binary was built on a matching host), `--emit-object` also
writes the archive alongside and prints the exact link command:

```
# On Windows: produce a Windows .obj + .lib (host == target)
twig-aot foo.twig --target=windows-x86_64 --emit-object -o out/bar

# Output:
# twig-aot: emitted object: out/bar.obj
# twig-aot: emitted runtime archive: out/bar_runtime.lib
# twig-aot: link on the target host with:
#   link.exe /OUT:<exe>.exe /ENTRY:main /SUBSYSTEM:CONSOLE \
#            out/bar.obj out/bar_runtime.lib libcmt.lib legacy_stdio_definitions.lib
```

### New API

- **`EmitObjectTarget`** enum — `MacosArm64`, `LinuxX86_64`,
  `WindowsX86_64`.  Selects which object format and runtime archive
  the helper produces.
- **`EmittedObject`** struct — `{ object_path, runtime_archive_path,
  target }` returned from `emit_object_to_disk`.  Callers (e.g. the
  CLI) print human-readable paths.
- **`emit_object_to_disk(src, out_base, target) -> EmittedObject`**
  — writes the relocatable object and (if available on this build
  host) the runtime archive next to it.  Works from any (host,
  target) combination because object emission doesn't need the
  target's toolchain.

### CLI changes

- New `-c` / `--emit-object` boolean flag.  When set, the binary
  writes the object + (optional) runtime archive instead of
  invoking the system linker.  Combines with `--target` so the
  user can write a Linux `.o` on a Windows host (or vice versa).

### Tests

- `emit_object_to_disk_writes_linux_o`: verifies the `.o` extension
  and ELF magic.
- `emit_object_to_disk_writes_windows_obj`: verifies the `.obj`
  extension and `IMAGE_FILE_MACHINE_AMD64` (0x8664) at byte 0.
- `emit_object_runtime_path_is_none_when_archive_is_stub`: iterates
  the three targets and asserts at least one yields a real archive
  (the host) and at least one yields a stub.

### Out of scope (deferred to V2)

Full cross-OS *linking* — taking a Linux ELF source on a Windows
host all the way to a runnable `<exe>` without copying — would need
either a bundled `clang+lld` + sysroot toolchain, or a `zig cc`
dependency.  Both are substantial.  `--emit-object` covers the
common case (build farm produces objects, target machine links)
without that complexity.

## 0.4.0 — 2026-05-16 (`--target` CLI flag)

**Expose the LANG46 multi-target driver to end users via a `--target`
CLI flag on the `twig-aot` binary.**

Previously the CLI only invoked `compile_file_macos_arm64`; the
Linux/Windows entry points existed but were unreachable from the
command line.  This release adds a `--target` flag and host-aware
dispatch:

```
twig-aot foo.twig                      # auto-picks the host target
twig-aot foo.twig --target=linux-x86_64
twig-aot foo.twig --target=windows-x86_64
twig-aot foo.twig --target=macos-arm64
```

Accepted values (and full target-triple aliases):
| Short | Triple |
|---|---|
| `auto` (default) | (build host) |
| `macos-arm64` | `aarch64-apple-darwin` |
| `linux-x86_64` | `x86_64-unknown-linux-gnu` |
| `windows-x86_64` | `x86_64-pc-windows-msvc` |

Cross-OS dispatch (e.g. `--target=linux-x86_64` on a Windows host)
errors out cleanly:

```
$ twig-aot --target=linux-x86_64 foo.twig    # on Windows
twig-aot: --target=linux-x86_64 requires a Linux x86-64 host in V1
         (cross-OS compilation is a separate follow-up)
```

Unknown targets produce an enumerated error:

```
$ twig-aot --target=bogus foo.twig
twig-aot: unknown target "bogus"; expected one of: auto, macos-arm64,
         linux-x86_64, windows-x86_64
```

## 0.3.1 — 2026-05-16 (multi-function x86_64 cross-fn patching)

**Patch cross-function `call` sites in place during the x86_64
two-pass compile.**

Previous v0.3.0 release noted that multi-function programs were
deferred — every `call` instruction surfaced as a `PltRel32`
external relocation, which only resolved correctly when the callee
was a runtime helper (e.g. `__twig_print_i64`).  Cross-module call
sites resolved fine because the system linker still found the
symbol via the function's exported symbol-table entry, but the
extra reloc overhead and the dependency on every internal function
having a global symbol were both incidental.

`compile_module_x86_64_to_text` now mirrors `aarch64-backend`'s
Pass 2 strategy:

- After concatenating per-function bytes via `aot_core::link::link`,
  walk every per-function reloc.
- If the reloc names another function in the same module
  (`offsets.contains_key`) AND is a `PltRel32` (CALL rel32),
  resolve in place: write `callee_off - patch_offset - 4` into the
  disp32 slot.  The reloc is consumed; the linker never sees it.
- Everything else (runtime helpers, possibly-external globals)
  passes through to the packager unchanged.

This unblocks real Twig programs (mutual-recursion, helpers, etc.)
on both Linux and Windows hosts.

### Tests

- `x86_64_cross_function_call_patched_in_place` — compiles a
  two-function module (`main` calls `helper`), verifies the CALL
  site's disp32 was patched to the correct PC-relative offset, and
  confirms no external reloc for `helper` is emitted.
- `x86_64_external_call_remains_in_relocs` — verifies that calls
  to runtime helpers like `__twig_print_i64` still surface as
  external relocs even when multi-function patching is otherwise
  active.

## 0.3.0 — 2026-05-14 (LANG46 phase 2 — multi-target driver)

**End-to-end Twig source → native binary on Linux x86-64 and Windows
x86-64.** This is the final piece of the x86-64 port — after this
release, the same Twig programs that compile on macOS ARM64 compile
and run on Linux x86-64 and Windows x86-64 hosts.

### New entry points

- `compile_module_linux_x86_64_object(module)` / `compile_linux_x86_64_object(source, name)`
  — emit an ELF64 `ET_REL` object file via `x86_64-backend` (System V
  AMD64 ABI) + `code-packager::pack_elf64_object_x86_64`.
- `compile_module_windows_x86_64_object(module)` / `compile_windows_x86_64_object(source, name)`
  — emit a PE/COFF `IMAGE_FILE_MACHINE_AMD64` object file via
  `x86_64-backend` (Microsoft x64 ABI) +
  `code-packager::pack_pe_object_x86_64`.
- `compile_file_linux_x86_64(src, out)` (`#[cfg(target_os = "linux")]`)
  — full pipeline: source → IR → x86_64 bytes → ELF object → `cc` →
  runnable ELF executable.
- `compile_file_windows_x86_64(src, out)` (`#[cfg(target_os = "windows")]`)
  — full pipeline: source → IR → x86_64 bytes → PE/COFF object →
  linker probe (`link.exe` → `lld-link.exe` → `gcc.exe`) → runnable
  `.exe`.

### Windows linker probe

The Windows path detects an actual MSVC `link.exe` by parsing the
banner ("Microsoft" + "Linker") rather than just checking program
spawnability — git-bash hosts ship a POSIX `link(1)` utility with the
same name on `PATH`, which would otherwise be (incorrectly) chosen.

### End-to-end smoke tests

- `tests/linux_x86_64_smoke.rs` (`#[cfg(target_os = "linux")]`):
  compiles small typed Twig programs (`42`, `(+ 30 12)`, `(* 6 7)`,
  branches), links via `cc`, runs the resulting ELF executable,
  asserts the exit code matches `main`'s return value.
- `tests/windows_x86_64_smoke.rs` (`#[cfg(target_os = "windows")]`):
  same suite via `link.exe` and a `.exe` output.  Each test
  gracefully skips when no real Windows linker is detected on
  `PATH` (e.g. MSVC dev env not activated).
- `tests/macos_arm64_smoke.rs` (existing): unchanged and still
  passes; verifies the macOS path didn't regress.

Each smoke test runs only on its respective CI runner; the suite
covers Linux + macOS + Windows end-to-end without cross-compilation.

## 0.2.0 — 2026-05-14 (LANG46 phase 1 — per-host runtime archives)

**Extend `build.rs` to produce per-host runtime archives plus stubs for
non-host targets.**

Sets up the runtime-archive layer that phase 10's multi-target driver
will consume.  After this release, `twig-aot` compiled on any of the
three V1-supported hosts exports three env vars
(`TWIG_RUNTIME_ARCHIVE_MACOS_ARM64`,
`TWIG_RUNTIME_ARCHIVE_LINUX_X86_64`,
`TWIG_RUNTIME_ARCHIVE_WINDOWS_X86_64`), each pointing at either the
real archive (for the build host's target) or a 1-byte stub (for
other targets).

The phase 10 driver uses these env vars with `include_bytes!` to bake
all three runtime archives into the `twig-aot` binary; at AOT compile
time, it picks the right one based on `--target` and refuses to emit
for a target whose archive is a stub with a clear "no runtime archive
for X on this host" error.

### Host-targets-host policy

V1 supports only host-targets-host AOT.  Each CI runner builds for
its own host and verifies its respective smoke test.  Cross-OS
compilation is deferred — adding it requires bundling cross
toolchains with `twig-aot` or detecting them on the host.

### Backwards compatibility

The existing `TWIG_RUNTIME_ARCHIVE` env var is preserved as an alias
for the host's archive (or a legacy stub on unsupported hosts), so
the existing `compile_file_macos_arm64` entry point continues to
work without changes.

## 0.1.9 — 2026-05-13 (LANG42)

**Wire the refinement obligation checker into the AOT pipeline.**

LANG23 built a complete refinement-type infrastructure (solver, checker, type
annotations on `IIRFunction`), but the IIR never reached the checker —
annotations silently did nothing.  LANG42 fixes this by adding a pre-codegen
pass that runs immediately after `twig-ir-compiler` emits the `IIRModule`,
before any lowering, and discharges every proof obligation through the existing
`lang-refinement-checker` API.

### New dependency

- **`iir-refinement-pass = { path = "../iir-refinement-pass" }`** — new crate
  that implements `check_module(module, mode) -> Vec<RefinementError>`.

### New `AotError` variant

- **`AotError::RefinementViolations(Vec<iir_refinement_pass::RefinementError>)`** —
  returned when one or more proof obligations are `ProvenUnsafe` (Lenient mode)
  or `ProvenUnsafe | Unknown` (Strict mode).

### Changed

- **`compile_module_macos_arm64_object`** now calls `check_refinements` before
  `compile_module_to_text`.  In `Lenient` mode (default) only `ProvenUnsafe`
  outcomes abort compilation.

- **`compile_module_macos_arm64_object_with_mode`** — new public function
  accepting an explicit `RefinementMode`.  The old function delegates to it
  with `Lenient`.

### Tests added

- `refinement_violation_becomes_aot_error` — a literal that violates a
  `(Int 0 128)` annotation returns `Err(AotError::RefinementViolations)`.
- `safe_annotated_program_compiles_ok` — a literal within range compiles
  normally.

---

## 0.1.8 — 2026-05-13 (LANG41)

**Replace macOS-specific `emit_print_helper` injection with a portable C
runtime archive linked via the system linker.**

LANG40 injected a 208-byte ARM64 subroutine with hardcoded macOS `write(2)`
syscall numbers (`x16=4`, `SVC #0x80`) into user code before linking.
LANG41 removes that approach entirely; `__twig_print_i64` is now defined in
a portable C file compiled at `cargo build` time and embedded in the
`twig-aot` binary, then written to a temp file and passed to `ld` for each
AOT compilation.

### New files

- **`runtime/twig_runtime.c`** — defines `__twig_print_i64(int64_t val)` using
  `printf("%lld\n", (long long)val)` + `fflush(stdout)`.  Pure POSIX — no raw
  syscall numbers, no platform ifdefs.  On macOS, `printf` routes through
  `libSystem`; on Linux, it routes through `libc`.  The same source file works
  on both platforms without change.

- **`build.rs`** — uses the `cc` crate to compile `runtime/twig_runtime.c`
  into `$OUT_DIR/libtwig_aot_runtime.a` at `cargo build` time.
  Exports `cargo:rustc-env=TWIG_RUNTIME_ARCHIVE=<path>` so the archive path
  is available to `include_bytes!` at compile time.
  `cargo:rerun-if-changed=runtime/twig_runtime.c` invalidates only when the
  C source changes.

### Changed

- **`[build-dependencies]`**: `cc = "1"` added to `Cargo.toml`.

- **`RUNTIME_ARCHIVE`** static: `include_bytes!(env!("TWIG_RUNTIME_ARCHIVE"))`
  embeds the archive in the binary.  Zero disk overhead at runtime (extracted
  only during AOT compilation).

- **`compile_module_to_text_raw`** return type is now a 5-tuple:
  `(text, offsets, n_global_slots, global_byte_relocs, extern_branch_relocs)`.
  The fifth element replaces the old "fail on unresolved external" logic;
  unresolved `BL` targets are now collected and forwarded to the packager.

- **`compile_module_macos_arm64_object`** always calls
  `pack_object_with_globals_and_externals` (no more conditional on whether
  globals are present).

- **`invoke_ld`** writes `RUNTIME_ARCHIVE` to a temp file (`twig_aot_runtime_<pid>.a`)
  and passes it as an argument to `ld` before cleanup.

- **`emit_print_helper` injection removed** — the old "inject helper if any
  function references `__twig_print_i64`" block in
  `compile_module_to_text_raw` is gone.

### Tests

All existing tests pass.  Integration tests in `tests/macos_arm64_smoke.rs`
exercise the full pipeline including `end_to_end_object_through_ld_returns_42`.

---

## 0.1.7 — 2026-05-13 (LANG40)

**AOT `io_out` — integer print to stdout via `__twig_print_i64`.**

Twig programs that use `(print n)` now compile to native ARM64 code without
`BackendRefused`.  Previously the `io_out` CIR opcode had no ARM64 handler;
LANG40 adds end-to-end support across three crates.

### Pipeline change — helper injection

`compile_module_to_text_raw` gains a new step between Pass 1 and the linker:

```rust
let needs_print_helper = fn_results.iter().any(|(_, _, relocs, _)| {
    relocs.iter().any(|r| r.symbol == "__twig_print_i64")
});
if needs_print_helper {
    fn_results.push(("__twig_print_i64".to_string(), emit_print_helper(), vec![], vec![]));
}
```

If any compiled function contains a `BL __twig_print_i64` placeholder (from
the `io_out` handler in `aarch64-backend`), the 208-byte self-contained print
helper is appended to `fn_results` before `link()` runs.  The existing
two-pass BL patcher then resolves the symbol and patches the correct
PC-relative offset automatically — zero new linker infrastructure needed.

The helper is **not** emitted when no `io_out` instructions are present,
so programs without printing incur zero overhead.

### Tests (2 new)

| Test | Asserts |
|------|---------|
| `print_program_compiles_ok` | `(print 42)` compiles to a valid `MH_OBJECT` Mach-O |
| `print_program_is_valid_macho` | compiled object is ≥ 400 bytes (helper present) |

### Upstream dependency versions

| Crate | Old | New |
|-------|-----|-----|
| `aarch64-encoder` | 0.2.1 | 0.2.2 (adds `strb_pre_neg1`) |
| `aarch64-backend` | 0.2.1 | 0.2.2 (adds `io_out` handler + `emit_print_helper`) |

## 0.1.6 — 2026-05-13 (LANG39)

**First-class global variable support — `(define x 5) x` now compiles to native code.**

Twig programs that use top-level value defines (`(define x 5)`, `(define counter 0)`)
previously failed with `AotError::BackendRefused` because the V1 ARM64 backend didn't
know how to handle `global_set` / `global_get` builtins.  LANG39 closes that gap
end-to-end across all four affected crates.

### New dependency

- `iir-builtin-lowering` added to `Cargo.toml` (provides `lower_global_io`).

### Pipeline changes

#### `prepare_module_for_aot`

Two new phases prepend the existing four-step AOT preparation pipeline:

**Phase 0 — `lower_global_io(module)`**
Converts `call_builtin "global_set"/%n/%v` → `global_store Str("name") Var(val_reg)` and
`call_builtin "global_get"/%n` → `global_load Str("name")` (imported from `iir-builtin-lowering`).
Must run before arithmetic pre-lowering so the const-string look-back can see the full instruction list.

**Phase 0b — `strip_dead_string_consts(func)`**
The twig-ir-compiler emits `const %n = Var("x")` (name-register) before each
`global_set`/`global_get` call.  After Phase 0, those call_builtins are gone
but the `const %n` instruction remains dead in the list.  `aot_specialise` would
convert it to `const_str` which the ARM64 backend cannot lower.

This new pass removes every `const` instruction whose source is `Operand::Var(_)`
(the string-literal-as-Var encoding) **and** whose dest register is never referenced
in any other instruction's `srcs`.  Registers that are still read (e.g. name args to
un-lowered `call_builtin "make_closure"`) are retained.

#### `compile_module_to_text` / `compile_module_to_text_raw`

Return type extended from `(Vec<u8>, HashMap<String, usize>)` to
`(Vec<u8>, HashMap<String, usize>, usize, Vec<GlobalByteReloc>)`.

New fields:
- `n_global_slots` — number of unique globals found by `collect_global_slots`.
- `global_byte_relocs` — `Vec<GlobalByteReloc>` containing the byte offsets of
  every `ADRP + ADD` instruction pair in the linked text section (for `ld`'s
  ARM64 relocation records).

#### `compile_module_macos_arm64_object`

When `n_global_slots > 0`, now calls `pack_object_with_globals` (from
`code-packager 0.2.1`) instead of `pack_object`.  This emits a two-section
Mach-O object file (`__TEXT/__text` + `__DATA/__data`) with:
- A zero-initialised `__data` section (8 bytes per global slot).
- An exported `_twig_globals` symbol pointing to the start of that section.
- `ARM64_RELOC_PAGE21` + `ARM64_RELOC_PAGEOFF12` relocation records per `GlobalByteReloc`.

When `n_global_slots == 0`, the original single-section `pack_object` path is used unchanged.

#### `collect_global_slots(module)`

New internal helper.  Scans all `global_load`/`global_store` instructions in a
post-`lower_global_io` module for `Operand::Str(name)` in `srcs[0]`.  Assigns each
unique global name a consecutive 0-based slot index (slot `i` lives at `_twig_globals + i*8`).

### Test changes

- `untyped_twig_returns_backend_refused` → renamed to `global_define_compiles_ok`.
  The old test expected `(define x 5) x` to fail.  With LANG39 it must now succeed
  and produce a valid `MH_OBJECT` Mach-O.

### Upstream dependency versions

| Crate | Old | New |
|-------|-----|-----|
| `aarch64-encoder` | 0.2.0 | 0.2.1 (adds `adrp_placeholder`) |
| `aarch64-backend` | 0.2.0 | 0.2.1 (adds `compile_with_globals`, `GlobalWordReloc`) |
| `code-packager` | 0.2.0 | 0.2.1 (adds `pack_object_with_globals`, `GlobalByteReloc`) |

## 0.1.5 — 2026-05-13

**Default integer type changed from `u64` to `i64` — all typing states now correct.**

Twig integers are semantically signed 64-bit values.  The previous default of
`"u64"` for untyped params caused `(< x 0)` to emit an unsigned ARM64 `CMP`,
which treated `-5` as a very large positive number and returned wrong results
for programs that compared against negative numbers.

### Changes

- **`normalize_params_to_i64`** (was `normalize_params_to_u64`): promotes
  `"any"` / `"polymorphic"` params to `"i64"` instead of `"u64"`.
- **`default_any_to_i64`** (was `default_any_to_u64`): defaults remaining
  `"any"` arithmetic/mov hints to `"i64"` instead of `"u64"`.
- **`infer_aot_type`**: integer literal constants (`Operand::Int(_)`) now infer
  `"i64"` instead of `"u64"`, so constant expressions propagate signed types.
- **`compile_typed_module_to_arm64_bytes`**: now calls `normalize_params_to_i64`
  before `propagate_aot_types`, ensuring that unannotated params (still `"any"`
  after the caller has set annotation-derived types) also get `"i64"` semantics.
  Previously this function relied entirely on the caller to set all param types,
  which left unannotated functions with unsigned comparisons.

### Effect

All three optional-typing states (untyped / partially typed / fully typed) now
produce correct results for programs that compare against negative numbers.
Type annotations are purely additive: they document intent and may enable future
optimisations, but are never required for correctness.

## 0.1.4 — 2026-05-13

**In-process ARM64 execution + typed i64 pipeline.**

### New public APIs

#### `compile_module_to_arm64_bytes(module) → Result<(Vec<u8>, HashMap<String, usize>), AotError>`

Returns raw ARM64 machine code bytes and a function-name→byte-offset map.
Uses the full preparation pipeline (builtin pre-lowering + i64 param
normalisation + type propagation + default-any-to-i64).  Suitable for
in-process execution via `call_arm64_function_in_process`.

#### `compile_typed_module_to_arm64_bytes(module) → Result<(Vec<u8>, HashMap<String, usize>), AotError>`

Like `compile_module_to_arm64_bytes` but uses caller-supplied type
annotations.  The caller pre-lowers builtins and may set params to `"i64"`;
this function first normalises any remaining `"any"` params to `"i64"`, then
propagates types.  Comparison instructions emit `cmp_lt_i64` (signed ARM64
condition code).  Correct for negative numbers whether or not the caller
pre-annotated params.

#### `pre_lower_aot_builtins_on_module(module: &mut IIRModule)`

Exposes the `pre_lower_aot_builtins` pass at the module level so callers
can pre-lower before running their own type-inference pass.

#### `call_arm64_function_in_process(code, offsets, fn_name, arg) → Result<i64, AotError>`

*macOS/ARM64 only.*  Execute compiled ARM64 code in-process:
1. Allocates an anonymous `PROT_READ | PROT_WRITE` mapping.
2. Copies code bytes into it.
3. `mprotect`s to `PROT_READ | PROT_EXEC` (no `MAP_JIT` entitlement required).
4. Calls `fn_name(arg)` via AAPCS64 (`x0` in/out) and returns the result.

This avoids the full `ld` + subprocess path (~200ms ld + ~30ms exec),
bringing per-call overhead to <1ms.

### Bug fix: comparison type inference (`infer_aot_type`)

Previously `infer_aot_type` always returned `"bool"` for `cmp_*`
instructions.  This produced `cmp_lt_bool` in the CIR, which the ARM64
backend lowered with an **unsigned** condition code.  For non-negative
values this is harmless, but `cmp_lt_u64(-5, 0)` evaluates false because
`-5` is stored as `0xFFFFFFFFFFFFFFFF` — a large unsigned number.

The fix: `infer_aot_type` for `cmp_*` now returns the **operand type**
(resolved from the first source via `resolve_src_aot_type`), falling
back to `"bool"` only when operands are still unresolved.

- Untyped path (u64 params): `cmp_lt` → `cmp_lt_u64` (unsigned, same as before).
- Typed path (i64 params): `cmp_lt` → `cmp_lt_i64` (signed, correct for negatives).

### Internal: `compile_module_to_text_raw`

The existing `compile_module_to_text` (clone + prepare + compile) was
split into two functions: `compile_module_to_text` (prep delegates to
`compile_module_to_text_raw`) and `compile_module_to_text_raw` (raw
two-pass compile + link, no prep).  The typed API uses `_raw` directly.

### Why `compile_typed_module_to_arm64_bytes` still runs propagation

`iir-type-checker::infer_function` seeds its SSA environment only from
instruction dests — **not** from `func.params`.  Instructions of the
form `sub dest, param_var, const` therefore stay `"any"` after the type
checker.  `propagate_aot_types` (seeded from `func.params`) fills these
in.  The propagation pass deliberately does NOT call
`normalize_params_to_u64`, so typed i64 params propagate as `i64`.

## 0.1.3 — 2026-05-13

**AOT preparation pipeline — cross-function fib compiles and runs.**

The AOT pipeline previously failed to compile recursive Twig programs
(like fibonacci) because:
1. `call_builtin "+"` / `call_builtin "_move"` instructions were left
   unresolved when the ARM64 backend received the CIR — both are
   `UnsupportedOp` in V1.
2. All function parameters had type `"any"`, which blocked
   `aot_specialise`'s type-specialisation logic (it can only lower
   `call_builtin "+"` → `add_u64` when it knows the operand types).
3. The two-pass linker for cross-function `BL` patching was implemented
   but not yet exercised.

### New: `prepare_module_for_aot` pipeline

A three-step IIR preparation pass now runs before `aot_specialise`:

1. **`pre_lower_aot_builtins`** — converts `call_builtin "+" a b` →
   `add a b`, `call_builtin "_move" n` → `mov n`, etc.  (mirrors the
   JVM/CLR/WASM pre-lowering passes).
2. **`normalize_params_to_u64`** — promotes every `"any"` param type
   to `"u64"` so `infer_types` can seed the type environment from
   params and propagate concrete types through arithmetic chains.
3. **`propagate_aot_types` + `default_any_to_u64`** — fixed-point type
   propagation (seeds from params, handles `const`, `cmp_*`,
   arithmetic, `mov`) followed by defaulting any remaining `"any"`
   arithmetic instructions to `"u64"`.  This ensures `aot_specialise`
   never emits `type_assert` guards (which the ARM64 backend lowers to
   `udf` hard-traps).

### New: `"mov"` handling in `aot-core::specialise`

`aot_specialise` now lowers `mov dest, src` (produced by
`pre_lower_aot_builtins`) to `mov_<ty>` so the ARM64 backend can emit
a typed stack-spill load/store pair.

### End-to-end result

`fib(10)` compiles and executes natively, returning `55`.

```text
AOT (ARM64 native)    224 ms    55  ✅ PASS
```

The two-pass cross-function BL linker (landed in 0.1.2) is now
exercised for real by the mutual recursion in `fib` → `fib`.

New test: `fib_compiles_ok` — asserts the full fib program compiles to
a valid Mach-O object without error.

## 0.1.2 — 2026-05-10

**LANG25-25A — Windows compilation hygiene.**

- `compile_file_macos_arm64` is now defined on all platforms.  On non-Unix
  hosts (Windows) the function returns `AotError::Linker` with a clear
  "requires Unix host" message.  Previously the `#[cfg(unix)]` gate made the
  function undefined on Windows, causing the `twig-aot` binary to fail
  `cargo check` on that platform.

- `tests/macos_arm64_smoke.rs`: wrapped `use std::os::unix::fs::PermissionsExt`
  in `#[cfg(unix)]` so the test file compiles on Windows (all callers are
  already `#[cfg(all(target_os = "macos", ...))]` which is a strict subset
  of unix).

## 0.1.1 — 2026-05-05

Real Twig source programs now compile and run on Apple Silicon — not
just hand-built IIR.  This release does NOT touch `twig-aot` itself
but pulls in upstream improvements that turn typed Twig source into
fully-resolved CIR + native code:

- `aot-core::specialise` now lowers `call_builtin "+ / - / * / / / = /
  != / < / <= / > / >= / _move"` to typed CIR ops (`add_<ty>`,
  `cmp_eq_<ty>`, `mov_<ty>`) when operand types are known, eliminating
  runtime calls for primitive arithmetic.
- `aarch64-backend` adds `mov_<ty>` lowering and fixes a stack-frame
  bug where virtual register slot 0 collided with the saved `fp/lr`
  (binaries previously SIGSEGV'd at function return).

End-to-end demonstration:

```
$ cat hello.twig
(+ 30 12)
$ twig-aot hello.twig -o hello && ./hello; echo $?
42
```

The integration test suite now runs 8 typed Twig programs through the
full pipeline and asserts their exit codes (see
`tests/macos_arm64_smoke.rs::end_to_end_typed_twig_arithmetic_and_branches`).

## 0.1.0 — 2026-05-05

Initial release.  End-to-end ahead-of-time compiler for Twig: source
file in, runnable native ARM64 Mach-O executable out.

### Pipeline

```
Twig source
   ↓ twig-ir-compiler
IIRModule
   ↓ aot-core (infer + specialise) → CIR
   ↓ aarch64-backend (compile_function) → ARM64 bytes
Vec<(fn, bytes)>
   ↓ aot-core::link → (text, offsets)
   ↓ code-packager::macho_object → MH_OBJECT
.o object file
   ↓ ld -arch arm64 -platform_version macos 15.0 15.0 -e _main -lSystem
runnable Mach-O executable
```

### Why we shell out to `ld`

On macOS 15+ (Sequoia / Tahoe) the kernel attaches a "provenance" tag
to every executable file, recording which process wrote it.  Files
written by Apple's system linker (`/usr/bin/ld`) inherit a trusted
provenance and run normally; files written by random user code are
SIGKILL'd by `AppleSystemPolicy` regardless of how well-formed the
Mach-O is.  Delegating the final link to `ld` solves that — and as a
bonus `ld` handles dyld setup, ad-hoc code signing, and SDK
versioning for us.

### CLI

Argument parsing is driven by `cli-builder` with a JSON spec
(`twig_aot.cli.json`) embedded at compile time.  `--help` and
`--version` are auto-generated.

```
twig-aot <FILE.twig> [-o <OUT>]
twig-aot --help
twig-aot --version
```

### Test coverage

- `module_with_no_entry_point_errors` — error path unit test
- `untyped_twig_returns_backend_refused` — surfaces unsupported opcodes
- `empty_main_compiles_to_object_bytes` — object-file structure
- **`end_to_end_object_through_ld_returns_42`** — real `ld` invocation,
  binary writes to disk, kernel `exec()`s it, asserts exit code 42
- **`end_to_end_typed_twig_returns_42`** — typed-IIR-via-API flow

The two E2E tests are gated to `aarch64-darwin`.

### Known limitation

The V1 ARM64 backend (PR #2156) doesn't yet lower `global_set` /
closure / property opcodes, so any Twig source that uses top-level
value defines (`(define x 5)`) or closures fails with
`AotError::BackendRefused`.  Hand-built typed IIR (function defines)
works end-to-end today.
