# Changelog — iir-to-llvm

## 0.47.0 - 2026-08-02 - boolean procedure call results

Boolean user-function calls now retain their LLVM `i1` companion value, and
typed boolean procedure result slots use `i1` storage. A subsequent logical
operation or conditional branch therefore does not truncate an `i1` result as
if it were an `i64` slot.

## 0.46.0 - 2026-07-31 - boolean array elements

`array<bool>` now loads and stores LLVM `i1` elements. Boolean array reads
also retain their `i1` companion value so ALGOL `not` and boolean branches do
not widen the loaded bit to an invalid `i64` comparison.

## 0.45.0 - 2026-07-30 - ALGOL captured-array globals

`global_load` and `global_store` now infer a module-global storage type instead
of assuming every global is an `i64`. Scalar globals retain their historical
word slot; `array<T>` globals use `ptr`, preserving the array handle when an
ALGOL procedure captures an enclosing array. The generated LLVM emits
`internal global ptr null`, `load ptr`, and `store ptr` for those globals.

## 0.44.0 - 2026-07-30 - LANG-FULL E4-dyn runtime string ordering

`str_cmp` now mirrors the established runtime `str_eq` path. Literal pairs still
fold to `-1`/`0`/`1`, but a parameter, procedure result, or branch-selected local
now lowers to `@__twig_str_cmp(i64, i64)`. Literal operands mixed with a runtime
handle are converted with `ptrtoint`, so both arguments follow the same
length-prefixed-string ABI. The same mixed-handle conversion now covers runtime
`str_concat`, which ALGOL's snapshot-copy lowering uses with an empty literal
suffix; without it LLVM emitted invalid `i64 @__twig_str_N` call arguments.
Focused backend tests lock in both runtime calls and their on-demand declarations.

## 0.43.0 - 2026-07-20 — builtin whitelist: dyn_null_p (native `null?`)

Part of the fix restoring McCarthy-lisp list programs on the native-AOT / LLVM backends (`lang-aot` `lang_matrix`). See the umbrella commit for the full story: `null?` was never routed to a runtime call on the tagged native/LLVM path (breaking every cons-walk helper), `list-ref`/`assoc` unboxed a raw-int index/key (→ wrong element), a top-level `(null? …)` predicate result was unboxed instead of truthy-coerced, and cons-cell field access failed the JVM verifier. Verified end-to-end: native list-ref/assoc/length/reverse/append/null? all correct.
## 0.42.0 - 2026-07-14 (E6d-6-LLVM: structural heap ops + special-char name quoting → records run on LLVM)

Two additions give the LLVM column the word-granular heap model the native
backend already has, so a Twig **record** (its constructor + field accessors)
runs on LLVM — the last language feature that built for native but not LLVM.

1. **Structural heap ops.** `SUPPORTED_OPS` + `lower_instr` gain `alloc`,
   `field_store`, `field_load`, and `is_null`, mirroring the aarch64/x86_64
   memory model exactly:
   - `alloc [<size>]` → `call i64 @__twig_gc_alloc(i64 <size>)` (default 16, a
     2-word `LispyPair`); the extern is declared once per module that allocates.
   - `field_store ptr, idx, val` → `inttoptr`; `getelementptr i64, ptr, i64 <idx>`
     (the i64 element type scales the raw field index by 8); `store i64`.
   - `field_load ptr, idx -> dest` → same GEP + `load i64`.
   - `is_null x -> dest` → `icmp eq i64 x, 0` + `zext`, with the i1 kept in
     `env_i1` so a downstream `jmp_if_*` uses it directly.
   The object handle and every field are raw 64-bit words (tagged `DynValue`s),
   identical to the native backend, so the two columns agree byte-for-byte.

2. **Special-char function-name quoting.** `llvm_fn_ident` quotes any function
   name outside LLVM's unquoted set (`[-a-zA-Z$._][-a-zA-Z$._0-9]*`) — the Twig
   record accessor `point-x` and the union predicate `Some?` need `@"point-x"` /
   `@"Some?"`, which an unquoted `@Some?` mis-parses as a hard error. Applied at
   the `define` site and every `call` site so the reference always resolves; a
   plain identifier is emitted unquoted (no output churn).

Run-verified: `(record Point (x : int) (y : int)) (point-x (Point 42 7))` and the
`point-y` sibling exit 42 on native, LLVM, and WASM (new `e6d6_llvm_records`
integration test + the existing matrix record cells).

Not covered — a documented follow-up (E6d-6b): union `match` on the tagged
backends (native/LLVM). The union constructor stores raw words while `match`
reads them boxed; that only round-trips on the structural backends
(Wasm/Jvm/Clr), where the call boundary boxes `int → any`. The two union matrix
cells are scoped to `[Wasm, Jvm, Clr]` accordingly.

## 0.41.0 - 2026-07-14 (fix: dynamic comparison width — a `bool`-typed cmp compares i64s)

`lower_cmp` typed the `icmp`/`fcmp` operand width from the comparison's
`type_hint`. `lower_dynamic_arith` tags a dynamic `DynValue` comparison result
`"bool"` though its operands are the unboxed i64s, so LLVM emitted `icmp i1 %x`
on a 64-bit `%x` (`'%x' defined with type 'i64' but expected 'i1'`) — blocking
every dynamic `=`/`<` on the LLVM column (E6d-7 closure dispatch, E6d-6 match tag
tests). Fix: map a `"bool"` cmp hint to `i64` for the operand width, resolution,
and predicate; the i1 RESULT is unchanged (still threaded to `jmp_if_*` and
zext'd). The distinct legitimate `"i1"` hint (produce-i1, skip-zext) is left
untouched. Latent — the dynamic comparison path was never run on the LLVM column.

## 0.40.0 - 2026-07-11 (E6d-2b: ref<any> is a tagged i64)

E6d-2b: `llvm_type_for` now maps `ref<any>` -> `i64` (a tagged word, exactly like `ref<LispyPair>`), so a `dyn_box_int` result (a re-boxed dynamic-arithmetic value) validates and lowers on the LLVM backend. The `DYN_BUILTINS` table already routes `dyn_box_int`/`dyn_unbox_int` to `@__dyn_box_int`/`@__dyn_unbox_int`.

## 0.39.0 - 2026-07-11 (DVAL01-2: rename IIR builtin names lispy_* -> dyn_*)

DVAL01-2: the LLVM name->runtime-symbol table `LISPY_BUILTINS` is renamed `DYN_BUILTINS` and its first column de-lisped (`lispy_cons`->`dyn_cons`, ... -> the unchanged `__dyn_*` C symbols). `lispy_builtin()` lookup -> `dyn_builtin()`. Pure rename; LLVM lowering unchanged.

## 0.38.0 - 2026-07-11 (DVAL01-1b: rename C runtime file lispy_runtime.c -> dynval_runtime.c)

DVAL01-1b: the shared C runtime file is renamed `lispy_runtime.c` -> `dynval_runtime.c` (and the golden test `lispy_runtime_golden.rs` -> `dynval_runtime_golden.rs`), continuing the de-lisp of the generic dynamic-value substrate (spec DVAL01). Pure file/path rename -- no symbol, ABI, or behaviour change; the link/build path strings that reference the runtime are updated to match. The `lispy-runtime` Rust crate rename follows in DVAL01-1c.

## 0.37.0 - 2026-07-11 (DVAL01-1a: dynamic-value runtime ABI __twig_lispy_* -> __dyn_*)

De-lisp the tagged dynamic-value runtime ABI: every `__twig_lispy_*` C symbol (box_int/unbox_int/cons/car/cdr/pair_p/equal/not/nil/make_symbol/truthy/to_exit_code/tag_*) is renamed to the language-neutral `__dyn_*` (per spec DVAL01). Pure rename -- the 3-bit tag layout, encodings, and runtime behaviour are byte-for-byte unchanged, so any dynamic frontend (not just lisp) can target the same primitives. The GC ABI (`__twig_gc_*`) is untouched.

## [0.36.0] — 2026-07-10 (LANG-FULL E4-dyn — E4d-BA-arr: `array<str>` elements)

BASIC string arrays (`DIM A$(n)`) store a `str` element as an i64 handle
(`array_elem_llvm("str")` → `llvm_type_for("str")` = `i64`, 8-byte stride). No
new validator or element-type code was needed — `str` already maps to `i64` — but
`lower_array_set` had a latent bug: a folded `str` literal's value is tracked as
its `{i64 len,[N×i8]}` **global pointer** (`@__twig_str_N`), so storing it directly
emitted `store i64 @__twig_str_N` — a `ptr` constant in an i64 slot, which clang
rejects. Fix: `array_set` now `ptrtoint`s the literal's global to an i64 handle
before the store (the exact mirror of the existing call-arg and `ret` guards). A
runtime str element (branch-selected / read from another `array_get`) already
carries an i64, so the guard is scoped to the `@__twig_str` global.

Test: `str_array_set_ptrtoints_the_literal_handle` (asserts the `ptrtoint` and the
absence of a bare `store i64 @__twig_str`).

## [0.35.0] — 2026-07-08 (LANG-FULL tail: runtime `str_eq` over non-literal operands)

`lower_str_eq` gains a runtime path. It keeps the both-operands-literal compile-time
fold (constant 1/0), but when either operand is a runtime string handle — a param, a
call result, a `str_concat`/`str_slice` result — it calls the archive helper
`@__twig_str_eq(i64, i64)` over the two i64 handles (each operand resolved from `env`,
`ptrtoint`ing a literal's `@__twig_str` global pointer to a handle). Previously
`lower_str_eq` errored (`"a" is not a string literal value`) on a `str` parameter,
which is why `(define (same a b) (if (string=? a b) …)) (same …)` failed on LLVM.

- Declared under a `used_str_eq` flag (set whenever a `str_eq` op appears). Also fixes
  a latent declare-guard bug: the extern block's outer condition now includes
  `used_str_eq`, so a str_eq-only module (no `str_concat`/input/array ops) still emits
  the `declare` — mirroring the earlier `input_i64`/`input_str` declare-guard fix.
- Test: `str_eq_over_params_calls_twig_str_eq`. Run-verified end-to-end via clang.

## [0.34.0] — 2026-07-07 (LANG-FULL tail: `ptrtoint` a string literal passed as a call arg)

Fixes invalid IR when a string LITERAL is passed to a function. A single-assignment
`str_const` literal is tracked as its `{i64 len,[N×i8]}` GLOBAL POINTER
(`@__twig_str_N`), and a `str` argument is passed as an i64 handle — so passing the
literal directly emitted `call i64 @strlen(i64 @__twig_str_0)`, a `ptr` constant in an
`i64` argument slot, which clang rejects.

- `lower_call` now `ptrtoint`s any argument that resolves to a `@__twig_str` global
  pointer to an i64 handle before the call (the exact mirror of the `ret` path). The
  callee reads the length header via `inttoptr`+`load` in its runtime `str_len`.
- Test: `str_literal_call_arg_is_ptrtoint_to_i64`. Run-verified end-to-end via clang.

## [0.33.0] — 2026-07-07 (LANG-FULL E4-dyn: runtime `str_concat` → `@__twig_str_concat`)

`lower_str_concat` gains a runtime-operand path. It keeps the compile-time literal
fold (reuse the interned joined symbol so the result stays a known literal), but
when either operand is a runtime string handle (not in `str_values`), it reads each
operand's i64 handle from `env` and emits
`%r = call i64 @__twig_str_concat(i64 a, i64 b)`, storing the result only in `env`
(no `str_lens`/`str_values`) so `str_len`/`print_str` read the header at run time.
The extern is declared under a `used_str_concat` flag (set whenever a `str_concat`
op appears; an unused declare is legal LLVM and creates no undefined-symbol
reference when everything folds).

- Test: `runtime_str_concat_lowers_to_twig_str_concat_call` (two `input_str` handles
  → `str_concat` → `str_len` asserts the call + declare are emitted). Run-verified
  end-to-end via clang in the `PRINT A$ + B$` matrix cell.

## [0.32.0] — 2026-07-07 (LANG-FULL E4-dyn: BASIC string `INPUT A$`)

- `input_str` added to `SUPPORTED_BUILTINS`; `call_builtin "input_str"` lowers to
  `%v = call i64 @__twig_input_str()` (the AOT runtime helper) with a matching
  `declare i64 @__twig_input_str()`. The i64 result is the runtime-string handle
  (str→i64 at boundaries, E4d-2b), so a later `mov`/`print_str` reads the length
  header at run time.
- **Latent-bug fix:** the input/runtime `declare` block was gated on
  `used_alloc_bytes || … || used_putchar || used_getchar`, which omitted
  `used_input_i64`/`used_input_str` — so `@__twig_input_i64`/`@__twig_input_str`
  were only declared as a side effect of an unrelated helper (e.g. `putchar`)
  being present. A program that *only* reads input (no print/alloc) would emit
  the `call` without the `declare`. The guard now includes both input flags.
- Test `input_str_lowers_to_twig_input_str_call` (a minimal input-only function)
  locks in both the lowering and the declare-guard fix.

## [0.31.0] — 2026-07-03 (LANG-FULL E4-dyn E4d-2b: runtime string as return value / call result)

Extended the E4-dyn runtime-string support so a runtime string that arrives as a
function **return value**, **call result**, or **parameter** — not only a
branch-selected local slot — is a first-class value. This is what lets an ALGOL
`string procedure` (which returns a runtime string) run on the LLVM column.

Three changes:

- **`llvm_type_for` maps `"str"` → `"i64"`.** A string value is an i64 handle
  (the address of a `[i64 len][bytes…]` block), so it can now flow through a
  function boundary: a `str` parameter, a `str` return type, and a `call` whose
  result is `str` all type-check as `i64`. Previously `"str"` was rejected, so a
  string-returning function failed to lower at all.
- **`print_str` / `str_len` take the runtime header-read path for ANY string
  without a compile-time length**, not only promoted slots. The discriminator is
  now `!str_lens.contains(src)` (a slot is never in `str_lens`, so this subsumes
  the old `slots.contains` check and additionally covers a call result / return
  value / parameter). `env[src]` holds the i64 handle in every case;
  `inttoptr` + `load i64` recover the length. `str_len` gained its runtime branch
  (it previously only folded a compile-time constant).
- **`ret` of a `str` converts a literal global pointer with `ptrtoint`.** A
  single-assignment string is tracked as its `@__twig_str_N` global pointer;
  returning it directly would emit `ret i64 @global` (a type error), so the
  pointer is converted to the i64 handle first (branch-selected / call-result
  strings already carry an i64).

New unit tests `e4dyn_string_procedure_return_and_call_result_print` and
`e4dyn_str_len_of_runtime_string_reads_header`; the validator test that used
`str` as an "unsupported param type" now uses `ref<Foo>`, and a positive
`validate_accepts_str_param_and_return` was added. The `lang-aot` ALGOL
string-procedure matrix cell adds the `Llvm` column (verified end-to-end via
`clang`).

## [0.30.0] — 2026-07-03 (LANG-FULL E4-dyn E4d-2: runtime branch-selected strings)

First LLVM step of the **E4-dyn** arc (`code/specs/lang-full-e4-dyn-strings.md`):
a **runtime** string — one whose value is chosen by control flow, so it can't be
folded to a compile-time constant — now lowers on LLVM.

- **`collect_slot_vars`**: a `str` variable assigned in **more than one basic
  block** is promoted to a stack slot (an `i64` **handle**). A str reassigned
  twice *straight-line* keeps the compile-time literal fast path (linear
  last-write tracking is exact there); only cross-block assignment needs a
  runtime handle in memory. Basic-block boundaries are `label` (starts a block)
  and terminators `jmp`/`jmp_if_false`/`jmp_if_true`/`ret`/`ret_void` (end one).
- **Slot store**: a str value is carried in `env` as a global-symbol pointer
  (`@.str.N`); an `i64` slot stores the **handle** (block address), so the
  slot-store protocol emits `ptrtoint ptr @.str.N to i64` first. A literal
  string's `{i64 len, [N x i8]}` global address **is** a valid string handle, so
  a literal and a runtime heap string share one representation.
- **`print_str`** of a runtime (slot) string: `inttoptr` the handle, `load i64`
  the length from the block header (offset 0), `getelementptr … i64 8` to the
  bytes, `call @__print_str(ptr, i64)` — no compile-time length. A
  single-assignment literal keeps the folded fast path.

Proven end-to-end by the E4-dyn foothold matrix cell (a BASIC branch-selected
string), which now runs on the **`Llvm`** column (real `clang`) in addition to
VM/JIT/JVM/CLR. Two new unit tests assert the emitted runtime IR (`ptrtoint`
slot store, `inttoptr`+`load` print) and that a single-assignment string is
unchanged.

**Deferred to E4d-2b:** runtime paths for `str_len`/`str_concat`/`str_slice`/
`str_index`/`str_cmp` over slot operands (needed by the frontend payoffs that
*build* runtime strings; the foothold only observes one via `print_str`).

## [0.29.0] — 2026-06-30 (BA-INPUT: `input_i64` → `@__twig_input_i64`)

Added `"input_i64"` to `SUPPORTED_BUILTINS` and wired it to the `@__twig_input_i64`
function from the AOT runtime archive (`twig_runtime.c`).  `@__twig_input_i64` reads
one line from stdin and parses it as `int64_t`; returns 0 on EOF or parse failure.

Implementation pattern mirrors `@getchar`: a `used_input_i64` flag declares the
extern once at the top of the module when the builtin is encountered:

```llvm
declare i64 @__twig_input_i64()
```

The `call_builtin "input_i64"` instruction then emits:

```llvm
%dest = call i64 @__twig_input_i64()
```

Enables `10 INPUT X\n20 PRINT X` to run on the LLVM backend in
`matrix_every_proven_cell_agrees`.

## [0.28.0] — 2026-06-30 (LANG-FULL — seed `env_i1` for bool/i1 function parameters)

Fixed a regression in `lower_function` where ALGOL programs with boolean
parameters passed to functions that used bitwise logic would emit a bogus
`trunc i64 %param to i1` instruction, corrupting the i1 form in `env_i1`.

**Root cause**: `lower_bitwise` retrieves the i1 form of a value from
`env_i1` before emitting boolean `and`/`or`/`xor`.  For comparison
results (e.g. `icmp eq`), the i1 result is stored in `env_i1` naturally.
For *function parameters* typed "bool"/"i1", the parameter arrives in LLVM
IR already as an `i1` SSA value, but `env_i1` was not seeded at function
entry.  `lower_bitwise` then fell through to the truncation fallback and
emitted `trunc i64 %p to i1` — treating an `i1` as `i64`, which LLVM
rejects.

**Fix** (`lower_function` initialization loop, `lib.rs`):
After inserting each parameter into `state.env`, check whether its declared
type is "bool" or "i1".  If so, also insert the same `%pname` string into
`state.env_i1`.  `lower_bitwise` then finds it directly and skips the
truncation path entirely.

Regression test: `bitwise_bool_ops_lower_as_i1_logic` now passes cleanly.

## [0.26.0] — 2026-06-29 (LANG-FULL BA-pow — `f64_pow` LLVM lowering)

Added `lower_f64_pow`: detects use of `f64_pow` during the scan pass
(`used_f64_pow` flag), emits `declare double @pow(double, double)` once at the
top of the module when needed (direct libm call — no `@llvm.pow.f64` intrinsic
exists), and lowers each `f64_pow` instruction to a `call double @pow(...)`
with two f64 operands.  Added `"f64_pow"` to `SUPPORTED_OPS`.

All notable changes to this crate are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.27.0] — 2026-06-29 — `f64_atan/f64_tan` via direct libm declarations (LANG-FULL AL8-arctan)

LLVM has no `@llvm.atan.f64` or `@llvm.tan.f64` intrinsics (unlike sin/cos/log/exp).
Solution: emit direct libm declarations and call them:

- `f64_atan` → `declare double @atan(double)` + `call double @atan(double %src)`
- `f64_tan`  → `declare double @tan(double)`  + `call double @tan(double %src)`

Both use the existing `lower_f64_intrinsic` helper (which emits a `call double` instruction
for any function pointer, whether LLVM intrinsic or external libm symbol).
Two new `used_f64_atan` / `used_f64_tan` booleans gate the declarations.
Both ops added to `SUPPORTED_OPS`.

## [0.26.0] — 2026-06-28 — `f64_sin/cos/ln/exp` LLVM intrinsics (LANG-FULL AL8-trig)

Added `lower_f64_intrinsic` helper and four dispatch arms:
- `f64_sin` → `@llvm.sin.f64`
- `f64_cos` → `@llvm.cos.f64`
- `f64_ln`  → `@llvm.log.f64` (LLVM uses `log` for natural log)
- `f64_exp` → `@llvm.exp.f64`

Each intrinsic is declared with a usage-gated `declare double @llvm.<op>.f64(double)` header
emitted only when the op appears.  All four added to `SUPPORTED_OPS`.

## [0.25.0] — 2026-06-28 — `f64_sqrt` lowers to `@llvm.sqrt.f64` intrinsic (LANG-FULL AL8-sqrt)

Added `lower_f64_sqrt` function emitting `%dest = call double @llvm.sqrt.f64(double %src)`.
A new `used_f64_sqrt` flag gates a module-level `declare double @llvm.sqrt.f64(double)` that
is emitted only when the op is present — identical pattern to `used_conversions`.
`"f64_sqrt"` is added to the supported-ops list so the module validates cleanly.
LLVM lowers `@llvm.sqrt.f64` to `sqrtsd` on x86_64 and `fsqrt` on aarch64 — no libm call.

## [0.24.0] — 2026-06-28 — literal string comparison metadata reaches LLVM (LANG-FULL E4)

Literal-only `str_cmp` now folds through the LLVM string metadata pass. The
lowered value follows the shared E4 three-way ordering convention.

## [0.23.0] — 2026-06-28 — literal string slice metadata reaches LLVM (LANG-FULL E4)

Literal-only `str_slice` now participates in the LLVM string metadata pass.
When the source string and start/end bounds are constant, the backend interns
the derived slice as a length-prefixed private constant and binds the slice
destination to that metadata. This lets `str_slice` feed `str_index`, `str_len`,
`str_eq`, or `print_str` without adding a runtime string API.

## [0.22.0] — 2026-06-28 — computed string index metadata reaches LLVM (LANG-FULL E4)

Literal-only `str_len` results can now feed typed `i64` arithmetic in the LLVM
lowering environment. That lets `str_index` consume a computed constant index
for programs such as:

```text
str_const s "ABCDE"
str_len n s
sub i n 1
str_index b s i
```

The lowered LLVM returns byte `69` without requiring a runtime string API.

## [0.21.0] — 2026-06-27 — literal string concat can feed print_str (LANG-FULL E4)

Literal-only `str_concat` now materializes its derived string in the same
length-prefixed constant table used by `str_const`, and binds the concat
destination to that storage. This lets `print_str` consume a concat result
instead of failing with an undefined variable after the compile-time fold.

The backend regression proves:

```text
str_const a "O"
str_const b "K"
str_concat s a b
print_str s
```

lowers to a derived `@__twig_str_2` constant with byte length `2` and a
runtime `@__print_str` call over the payload pointer.

## [0.20.0] — 2026-06-27 — literal string index OOB traps (LANG-FULL E4)

Direct-literal `str_index` now preserves the E4 trap contract when the index is
statically out of range. Instead of rejecting the module during lowering, LLVM
emits `call void @llvm.trap()` so the matrix can prove `(string-ref "ABC" 3)`
fails at runtime.

## [0.19.0] — 2026-06-27 — literal string index reaches LLVM (LANG-FULL E4)

Extends the direct-literal E4 metadata foothold with `str_index`.

- `str_index s, i` is accepted when both operands are variables.
- Lowering folds the indexed byte from the existing literal string table when
  `s` comes from a direct `str_const` and `i` is a constant integer in the same
  function.
- Twig `(string-ref "ABC" 1)` now lowers to `ret i64 66` on LLVM without a
  runtime string API.
- Dynamic string values and the out-of-bounds trap proof remain outside this
  literal-only slice.

## [0.18.0] — 2026-06-27 — literal string metadata reaches LLVM (LANG-FULL E4)

Extends the E4 literal-string foothold from output to direct literal metadata.

- `str_len`, `str_eq`, and literal `str_concat` are now accepted when their
  operands are direct `str_const` literal values in the same function.
- Lowering materialises the byte count, byte equality result, and concatenated
  literal bytes from the existing private string-literal side table, so
  `(string-length "HELLO")` returns `5`, `(string=? "HELLO" "HELLO")` returns
  `1`, and `(string-length (string-append "AB" "CDE"))` returns `5` without
  needing a runtime string API.
- Validation remains fail-closed for dynamic string algebra: `str_index` and
  non-literal string values are still outside this slice.
- Backend tests assert the emitted functions return `ret i64 5` / `ret i64 1`
  and that the richer string-op rejection still fires.

## [0.17.0] — 2026-06-27 — BASIC string literal PRINT reaches LLVM (LANG-FULL E4 / BA4)

Adds the static-backend literal-output foothold for `str_const` + `print_str`.

- `str_const` collects printable-ASCII string literals into private LLVM constants
  with the unmanaged E4 layout: `{ i64 len, [len x i8] bytes }`.
- `print_str` materialises `base + 8` with `getelementptr inbounds i8` and calls
  the generic C runtime declaration `declare void @__print_str(ptr, i64)`.
- Validation accepts only this literal-output subset; richer byte-string ops
  (`str_len`, `str_index`, `str_concat`, `str_eq`) still fail closed until the
  full string runtime lands.
- Tests assert the length-prefixed global, runtime call, ASCII gate, and explicit
  rejection of richer string ops.

## [0.16.0] — 2026-06-22 — numeric conversions integer↔real (LANG-FULL E8 PR-2)

LLVM lowering for the three E8 conversion opcodes (vm-core 0.9.0 gave the
reference semantics; spec `lang-full-e8-numeric-conversions.md`).

- **`int_to_real`** → `sitofp i64 … to double` (IEEE-754). The dest slot is
  already typed `double` by `collect_slot_types` (its `type_hint` is `f64`).
- **`real_to_int_trunc`** / **`real_to_int_floor`** → round first with
  `@llvm.trunc.f64` (toward zero) / `@llvm.floor.f64` (toward −∞), then a
  range-check, then `fptosi double … to i64`. Rounding before `fptosi` mirrors
  the VM's `real_to_i64_checked(f.trunc()/f.floor())` **exactly**.
- **Trap matches the VM (no UB).** A bare `fptosi` of an out-of-range/NaN
  `double` is LLVM poison (UB); instead `emit_real_range_check` traps
  (`@llvm.trap` + `unreachable`, the same block shape as the E5 array-bounds
  `emit_bounds_check`) unless the rounded value is in `[-2⁶³, 2⁶³)`. The bounds
  are the `double` hex literals for ∓2⁶³, and the comparisons are **ordered**
  (`fcmp oge`/`olt`, `false` for NaN) so NaN/±∞ trap through the same check.
- Intrinsics (`@llvm.trap`, `@llvm.floor.f64`, `@llvm.trunc.f64`) declared once,
  gated on `is_conversion`; `@llvm.trap` shared with arrays (declared once when
  both are present). Added the three ops to `SUPPORTED_OPS`.
- Verified by RUNNING on **real clang**: `floor(int_to_real(45) − 2.7)` =
  `floor(42.3)` = 42 ⇒ exit 42 (`conversions_round_trip_runs_on_real_clang`),
  plus textual-emit assertions for `sitofp`/floor/trunc/range-check/`fptosi`.

## [0.15.0] — 2026-06-22 — typed module globals (LANG-FULL E6 layer 1)

`global_load` / `global_store` were the last of the `LANG32b`-deferred
rejections on this backend (the validator's `SUPPORTED_OPS` whitelist excluded
them). They now lower, so a **function can read/write a module-level global** in
compiled LLVM IR.

### Added
- **`global_load` / `global_store`** in `SUPPORTED_OPS` + lowering:
  - Every distinct global name (read or written, in first-seen order) becomes a
    module-level **`@__twig_global_N = internal global i64 0`** — index-based
    symbols (not name-based) so an arbitrary source identifier can never form an
    invalid or colliding LLVM global, and zero-initialised to match every other
    backend's never-written-global-reads-0 convention.
  - `global_load "g" -> %d` → `%d = load i64, ptr @__twig_global_N`.
  - `global_store "g", %v` → `store i64 %v, ptr @__twig_global_N`.
  - The name is an `Operand::Str` literal (never a register); a non-string or
    uncollected name is an `InvalidOperand` error.
  - `collect_global_syms` builds the name→symbol map once per module; threaded
    through `FnState`.

### Verified
- `tests/test_backend.rs`: the emitted `.ll` carries the `internal global` def +
  the load/store targeting it, and `validate_for_llvm` now accepts the ops.
- **End-to-end on real `clang`**: a cross-function program (`main` seeds `g`; a
  separate `bump` reads/increments/writes it) compiles and runs to **exit 42**.

## [0.14.0] — 2026-06-21 — arrays via length-prefixed `@calloc` + explicit bounds-trap (LANG-FULL E5 PR-4a)

The four E5 array opcodes now lower to the **static** array representation — a
flat `@calloc` block with the length in a header and an **explicit** out-of-bounds
trap (the native/LLVM target has no managed runtime to bounds-check for it, unlike
the JVM/CLR backends). Layout, with the IIR *handle* pointing at the payload:

```
base ──► [ i64 length | element 0 | element 1 | … ]   (zero-filled by calloc)
         └─ 8 bytes ──┘ ▲ handle = base + 8
```

| IIR op | LLVM IR |
|--------|---------|
| `alloc_array dest <- count` (`array<T>`) | `mul`+`add` size; `call ptr @calloc`; `store i64 count`; `getelementptr i8 … 8` |
| `array_get dest <- handle, idx` | load len at `handle−8`; `icmp uge`; `br` to trap; `getelementptr <T>`; `load <T>` |
| `array_set handle, idx, val` | same bounds check; `getelementptr <T>`; `store <T>` |
| `array_len dest <- handle` | `getelementptr i8 … -8`; `load i64` |

- **Explicit bounds check**: one **unsigned** compare (`icmp uge i64 idx, len`)
  catches both a `>= len` index and a negative one (a negative `i64` is a huge
  unsigned value); out-of-range branches to a block that does `call void
  @llvm.trap(); unreachable` — the static-backend realisation of E5's "OOB → trap".
- Element type (`i64`/`double`/`i32`/`float`) comes from `T`; the handle is an
  opaque `ptr`, and the typed `getelementptr <T>` does the index scaling. The
  index is always `i64` (LLVM keeps the uniform word model — no `i64`→`i32`).
- New `declare ptr @calloc` / `declare void @llvm.trap()` emitted only when a
  module uses an array op. The validator now accepts an `array<T>` `type_hint` by
  checking its **element** type (the `alloc_array` handle is a `ptr`, not a scalar).
- 3 new unit tests (calloc+trap+GEP shape, `double` element ops, `array<T>`
  validates). Verified end to end: a straight-line ALGOL array program assembles
  with `clang` and runs → exit 42.

Scope note: the ALGOL *for-loop* array program (`lang_matrix.rs`, exit 55) runs on
VM/JIT/JVM/CLR but **not yet LLVM** — an ALGOL `for` loop hits a *separate*,
pre-existing LLVM-only lowering bug (an `i1`-typed loop-guard `icmp` over `i64`
operands, double-emitted) unrelated to E5; the LLVM array proof uses a
straight-line cell, and the for-loop bug is tracked as a follow-up.

## [0.13.0] — 2026-06-20 — `f64` variable slots (LANG-FULL enabler E3, code-gen backend 1)

### Fixed — `real` (f64) variables produced invalid IR

The backend already emitted `fadd`/`fmul`/`fcmp` for `f64` *ops*, but three
things still broke a real program that uses a *variable* (clang rejected the
module, so ALGOL reals ran only on the VM/JIT):

1. **Uniform-`i64` stack slots.** A variable assigned 2+ times is promoted to an
   `alloca`, and every slot was hard-coded `alloca i64` / `store i64` / `load i64`.
   An `f64` local therefore did `store i64 <double>` — a type error. Slots now
   carry a per-variable type: `collect_slot_types` marks a slot `double` when any
   instruction writing it has a float `type_hint`, and the `alloca`/`load`/`store`
   protocol uses it. (`FnState` gains `slot_types` + a `slot_ty()` helper.)
2. **Comparison result `zext` to the operand width.** A `cmp_*` result is a
   boolean, but the value form was `zext i1 … to <operand_ty>` — i.e. `zext i1 to
   double` for a float comparison, which is invalid. A float comparison now
   `zext`s its boolean to `i64` (integer comparisons keep their operand width).
3. **Float literal formatting.** `Operand::Float` was rendered with Rust's `{:e}`,
   which emits `2e0`/`0e0` (no decimal point) for round numbers — LLVM's
   assembler rejects a floating literal without a `.` ("integer constant must
   have integer type"). Floats now use LLVM's exact **hexadecimal** double form
   `0x<16 hex>` (the IEEE-754 bit pattern) — always valid and bit-exact.

**Verified by RUNNING** on real `clang`: ALGOL 60 `real` programs
(`r := 2.5 * 2.0; if r = 5.0 …` → exit 42; `r := 7.0 / 2.0; if r < 4.0 …` →
exit 1) now execute on the LLVM column of `lang-aot`'s `lang_matrix.rs`, joining
the VM and JIT. Four new structural tests (`double` slot, hex literal, float-cmp
zext-to-i64, integer-program-unaffected). Integer programs are byte-identical
(the float path is taken only on a float `type_hint`).

**Still pending (E3):** `f64` *parameters* reassigned across a back-edge stay
SSA (`param_slot_compatible` excludes floats — a separate, unexercised case);
the wasm/jvm backends need the same slot fix (E3-codegen-slots); native + CLR
need FP emission (E3-native / E3-clr).

## [0.12.0] — 2026-06-16 — bitwise NOT (`not`) op

### Added — `not` (synthesised as `xor x, -1`)

LLVM has no `not` instruction, so the IIR `not` op was absent from this backend's
whitelist — the one backend of seven that lacked it. It now lowers to `xor x, -1`
(flip every bit). For a narrow unsigned width (`u4`/`u8`/`u16`/`u32`) it reuses the
E2 compute-wide+mask path — `xor i64 x, -1` then `and i64 …, <mask>` — so `~0u8` is
`255` (`-1 & 0xFF`), not the i64 all-ones. A full-width `i64`/`u64` `not` is a plain
`xor`. Added to `SUPPORTED_OPS`.

This **unblocks Nib N3-`~` and Oct O2-`~`** (their `compile_unary` lowers `~` to an
IIR `not`, which previously could not run on LLVM). **Verified on real `clang`**:
`not 0 : u8` returns exit `255`. New structural tests `not_u8_is_xor_minus1_then_masked`
and `not_i64_is_plain_xor_no_mask`; iir-to-llvm consumers (algol-iir-compiler, lang-aot)
green.

## [0.11.0] — 2026-06-15 — narrow unsigned arithmetic wraps mod-2ⁿ (LANG-FULL E2)

### Added — `u4`/`u8`/`u16`/`u32` results are masked back into their width

LANG-FULL **E2 — register width & wrap**, the LLVM column. A narrow unsigned
binary op (`add`/`sub`/`mul`/`div`/`mod` and `and`/`or`/`xor`) now computes at
`i64` and AND-masks the result into its declared width, so `200u8 + 100u8`
wraps to `44`:

```llvm
  %__nw1 = add i64 200, 100     ; compute wide (operands are i64 slots)
  %v     = and i64 %__nw1, 255  ; 300 & 0xFF = 44  ✓ wrapped to u8
```

**Why a value-mask, not a narrow-typed op.** Every IIR value rides a 64-bit
slot in this backend — arithmetic operands are `i64` SSA values (consts emit
`i64`; reassigned params become i64 stack slots). Typing the op at its narrow
LLVM width — `add i8 %a, %b` over two `i64` SSA values — is **invalid IR that
`clang` rejects** (the same shape as the AL5 `cmp`-truncation bug). So, exactly
like the VM, JIT, wasm, JVM, and CLR backends (and like this backend's own
byte-tape `store_byte` at the memory boundary), we compute wide and mask the
*value*:

| type_hint | mask         | example                |
|-----------|--------------|------------------------|
| `u4`      | `0xF`        | `15u4 + 1u4` → `0`     |
| `u8`      | `0xFF`       | `200u8 + 100u8` → `44` |
| `u16`     | `0xFFFF`     | `~0u16` → `65535`     |
| `u32`     | `0xFFFFFFFF` | wraps mod-2³²          |
| `u64`/`i*`/`f*` | —      | full word / signed / float: unchanged |

Signed narrow widths (`i8`/`i16`/`i32`) are left alone — E2 models unsigned
wrap; a signed wrap needs `trunc`+`sext`, out of scope.

Also adds `u4` (Nib's 4-bit nibble) to the supported type set — it has no
native LLVM width, so it rides an `i8` and the `& 0xF` mask enforces the range.

This corrects the earlier roadmap assumption that "LLVM already wraps natively
(u8→i8)": that was never executed (no frontend emitted narrow hints), and the
i64-slot value model means it does **not** hold. Verified by RUNNING the
emitted `.ll` through real `clang`: `200u8 + 100u8` returns exit `44`. New unit
tests: `e2_u8_add_computes_at_i64_then_masks`, `e2_u16_and_u4_masks_match_width`,
`e2_bitwise_u8_xor_masks`, `e2_wide_widths_emit_no_mask` (and the existing
`arith_div_unsigned_emits_udiv` updated to the i64-slot value model).

## [0.10.0] — 2026-06-13 — reassigned parameters become stack slots (LANG-FULL — LLVM first-class)

### Fixed — a reassigned function parameter is no longer silently dropped

`collect_slot_vars` promoted a variable to an `alloca` stack slot only when it was
the `dest` of **two or more** instructions. A parameter reassigned in the body —
e.g. `acc = acc + 6`, the shape of a loop accumulator — is the `dest` of only one
instruction, so it stayed a pure SSA value. Across a loop back-edge the
straight-line `const`/`mov` side-map is invalid, and the update was silently
dropped: the emitted IR computed `add %acc, 6` but never stored it back, so the
loop returned the unmodified incoming argument. (A `let` local works because its
declaration is a second `dest`; only parameters had this hole.)

A parameter's incoming binding **is** its first assignment, so:

- `collect_slot_vars` now seeds each i64-slot-compatible parameter with a count of
  1, so a single later reassignment crosses the `>= 2` promotion threshold. The new
  `param_slot_compatible` helper gates this to values that fit the i64 slot model
  (every integer width, `bool`, `any`, `symbol`, lisp `ref<Lispy…>`); a `float`/
  `double` parameter is **not** promoted (the i64 slot can't represent it — that is
  a separate concern under enabler E3, and is no worse than before).
- `lower_function` initialises each promoted parameter's slot from its incoming SSA
  argument at function entry (`store i64 %p, ptr %p.slot`), zero-extending a narrow
  `i1`/`i8`/`i16`/`i32` argument to the i64 slot width first.

Verified by RUNNING on real `clang`: a Nib program accumulating into a **parameter**
across a loop (`fn run(acc: u8) { for i in 0..7 { acc = acc + 6 } return acc }`)
now returns 42 — and is added to `lang-aot`'s `lang_matrix` battery across every
backend. New unit tests: `reassigned_parameter_is_promoted_to_a_stack_slot`,
`narrow_reassigned_parameter_is_zero_extended_into_its_slot`,
`non_reassigned_parameter_stays_pure_ssa`.

## [0.9.0] — 2026-06-12 (LLVM05 — byte-tape ops + Brainfuck I/O; LANG-MATRIX LM-L Brainfuck)

Adds the byte-tape memory ops and character I/O that Brainfuck needs, so the
LLVM column now covers Brainfuck — the last code-gen gap in that language's row.
Verified by RUNNING the Brainfuck cell `++++++++[>++++++++<-]>+.` on real `clang`
in `lang-aot/tests/lang_matrix.rs`: it prints `A`.

**New IIR opcodes** (added to `SUPPORTED_OPS` and `lower_instr`):

- `alloc_bytes dest <- size` → `%dest = call ptr @calloc(i64 size, i64 1)` — a
  zero-filled tape (Brainfuck cells start at 0). Declared once as
  `declare ptr @calloc(i64, i64)`. The tape base is a single-assignment value,
  so it is never a promoted stack slot.
- `load_byte dest <- base, idx` → `getelementptr i8` + `load i8` + `zext i8…i64`.
  The 8-bit cell becomes the uniform `i64` register width.
- `store_byte base, idx, val` (no dest) → `getelementptr i8` + `trunc i64…i8` +
  `store i8`. The `trunc` is what makes Brainfuck's 8-bit cell wrap-around fall
  out even though the surrounding arithmetic runs at `i64` width — "byte width
  only at the tape boundary."

**New `call_builtin`s** (added to `SUPPORTED_BUILTINS`):

- `putchar` (Brainfuck `.`) → `trunc i64…i32` + `call i32 @putchar(i32)`. Maps
  to libc directly (no host-runtime shim like `print_i64`'s `@__print_i64`).
- `getchar` (Brainfuck `,`) → `call i32 @getchar()` + `sext i32…i64`. EOF (`-1`)
  lands as `0xFF` after a subsequent `store_byte` truncation — the conventional
  Brainfuck behaviour. Declared as `declare i32 @putchar(i32)` / `@getchar()`.

**Bug fix — slot-dest SSA rename.** A variable assigned in 2+ instructions is
promoted to an `alloca i64` stack slot. Previously a value-producing op wrote
`%<var> = …` using the variable's name verbatim, so a slot variable that is the
dest of a real op (rather than only `const`/`mov`) emitted `%v = …` twice — which
LLVM rejects (*"multiple definition of local value named 'v'"*). Brainfuck's
`ptr`/`v` (incremented every command) are the first such case. `lower_instr_with_slots`
now lowers a clone of the instruction with a fresh SSA dest name and stores the
result into the original variable's slot. `const`/`mov` slot-dests (which emit no
`%dest =` line) are unaffected.

Six new tests in `tests/test_backend.rs` cover each emit case and the rename
regression.

## [0.8.0] — 2026-06-10 (McCarthy W13b — lisp lambda (F7) — LLVM COMPLETE)

Registers the universal exit-coercion runtime helper so the LLVM backend can
declare + call it: `LISPY_BUILTINS` gains `("lispy_to_exit_code",
"__twig_lispy_to_exit_code", 1)`. A lambda result is a `call` typed `any` whose
runtime tag is unknown at compile time; the shared `lower_lisp_repr` now emits
`lispy_to_exit_code` for it, and this entry lets the backend lower that to a
`call i64 @__twig_lispy_to_exit_code(i64)`. With it, **LLVM is McCarthy-complete
(F1–F7)** — verified by RUNNING in `lang-aot` (`lang-aot/tests/llvm_lambda.rs`).

## [0.7.0] — 2026-06-10 (McCarthy W13a — lisp symbols (F6))

`llvm_type_for("symbol")` now maps to `i64` — an interned McCarthy symbol is a
tagged 64-bit immediate (from `iir_builtin_lowering::intern_symbols`), so it flows
as a tagged word like `any`/`ref<Lispy…>`. With this, `(QUOTE A)`, symbol `EQ`, and
symbols inside `COND` all validate and lower. Verified by RUNNING in `lang-aot`
(clang + `lispy_runtime.c`): `(EQ (QUOTE A) (QUOTE A))`→1, `(EQ (QUOTE A) (QUOTE B))`→0.

## [0.6.0] — 2026-06-10 (McCarthy W12b-3 — `COND` via alloca SSA-merge — LLVM core F1–F5)

Lowers McCarthy `COND` (a cross-block value merge) and completes the LLVM core
(F1–F5).

- **Stack-slot promotion (`collect_slot_vars` + `lower_instr_with_slots`):** a
  variable assigned in 2+ instructions (a `COND` result written per clause) gets an
  entry `alloca`; each assignment becomes a `store i64 …, ptr %v.slot`, each read a
  `load i64, ptr %v.slot`. Single-assignment vars keep the `const`/`mov` side-map
  (fast path, no slot). This is the naive-frontend / `opt -mem2reg` pattern, so no
  PHI-predecessor analysis is needed.
- **Block-terminator hygiene (`FnState::block_open`):** a `label` reached while the
  current block is still open (its body was all tracked-not-emitted `const`/`mov`)
  emits an explicit fallthrough `br` first — no two labels back-to-back.
- **`jmp_if` void-cond:** when the `jmp_if_*` carries no operand type (`void`) — its
  condition is the `i64` 0/1 from `lispy_truthy` — it lowers to `icmp ne i64 %c, 0`
  instead of an invalid `trunc void`.
- Verified by RUNNING in `lang-aot` (clang + `lispy_runtime.c`):
  `(COND ((ATOM 7) 11) ((ATOM 8) 22))`→11, second-clause→22, nested `COND`→44.

## [0.5.0] — 2026-06-10 (McCarthy W12b-1 — tagged-word lisp `cons`/`car`/`cdr` → `__twig_lispy_*`)

Lowers the **tagged-word lisp** builtins to `call`s into the shared C runtime
(`twig-aot/runtime/lispy_runtime.c`) — the SAME runtime the native AOT path links,
so any lisp-family frontend inherits it.

- `LISPY_BUILTINS` table maps the `lispy_*` IIR names (from
  `iir_builtin_lowering::lower_heap_builtins_runtime`/`lower_lisp_repr`) to the
  runtime's `__twig_lispy_*` symbols: `cons`/`car`/`cdr`/`pair_p`/`equal`/`not`/
  `truthy`/`box_int`/`unbox_int`/`nil`. Each is `i64 (i64 × arity)` — a lisp value
  is a tagged 64-bit word.
- `call_builtin "lispy_*"` lowers to `%d = call i64 @__twig_lispy_*(i64 …)`; one
  `declare` per used builtin is emitted in the module header (first-seen order, deduped).
- `llvm_type_for`: `any` and a lisp reference (`ref<Lispy…>`) map to `i64` (the
  tagged word). A NON-lisp `ref<Foo>` stays `UnsupportedType`.
- **Verified by RUNNING** end-to-end in `lang-aot` (clang links `lispy_runtime.c`):
  `(CAR (CONS 7 9))`→7, `(CDR …)`→9, nested→2. Predicates (pair?/equal?/not, COND)
  are emitted but their tagged-boolean result handling is W12b-2.

## [0.4.0] — 2026-06-01 (LLVM04 — `call` + `call_builtin print_i64` + `lang-aot --emit=llvm-ir`)

### Added — user-defined `call`

Per-arg LLVM types come from a pre-built callee-signature side map:
`lower_iir_to_llvm` walks every function in the module once at the
start and stashes a `name → FnSig { param_types, return_type }` map.
Each `call` site looks up its callee in that map, validates the arg
count against the signature, and emits:

```llvm
%dest = call <ret_ty> @<callee>(<arg_ty> <arg>, ...)   ; non-void
        call void     @<callee>(<arg_ty> <arg>, ...)   ; void
```

Why pre-scan rather than synthesize from each call site's `type_hint`:
IIR's `call` carries only the **return** type in `type_hint`; param
types live on the *callee*.  Without pre-scan we'd need a second pass
or some hacky heuristic.

#### Validation

* `call`'s callee must exist in the module (else `UndefinedVariable`).
* Arg count must match the callee's param count (else `InvalidOperand`
  with an `arg-count` discriminator string).

### Added — `call_builtin "print_i64"` → extern `@__print_i64`

Completes the print_i64 trio across the four backend targets:

| Backend            | print_i64 lowering                                    |
|--------------------|-------------------------------------------------------|
| iir-to-wasm        | `env.__print_i64` host import                         |
| iir-to-jvm-class-file | `invokestatic env/BasicRuntime.println(J)V`         |
| iir-to-cil-bytecode | `call void env.BasicRuntime::PrintI64(int64)`        |
| **iir-to-llvm (this)** | `declare void @__print_i64(i64)` + `call void @__print_i64(i64 …)` |

The extern `declare` is emitted exactly **once** per module, at the
top, after the header.  `lower_iir_to_llvm` pre-scans the whole module
to decide whether to emit it (so the unused-builtin case doesn't pay
the extern cost).

#### Whitelist gate

* `SUPPORTED_BUILTINS = ["print_i64"]`.  Any other builtin name fails
  with `UnsupportedOp` — defence in depth even though `call_builtin`
  is in the validator whitelist.

### Tests added (45 total, was 37)

* `call` (4): non-void user fn typed call, void-return omits LHS,
  unknown callee → UndefinedVariable, arg-count mismatch error.
* `call_builtin` (4): print_i64 emits extern + call, declare emitted
  exactly once per module, declare omitted when print_i64 unused,
  unknown builtin name → UnsupportedOp.

[plan]: ../../../specs/MULTILANG-BACKEND-PLAN.md

## [0.3.0] — 2026-06-01 (LLVM03 — typed arithmetic + comparison + branches)

### Added — three op families

Implements item LLVM03 of the [multi-language backend plan][plan].  After
this release, the LLVM backend covers the IIR subset that BASIC, Twig,
Nib, and Oct front-ends actually emit for straight-line and branching
code (everything except `call`, `call_builtin`, and heap/memory ops —
those land in LLVM04).

#### Arithmetic — five op-families × signedness / float

| IIR op | Signed int | Unsigned int | Float |
|--------|------------|--------------|-------|
| `add`  | `add`      | `add`        | `fadd` |
| `sub`  | `sub`      | `sub`        | `fsub` |
| `mul`  | `mul`      | `mul`        | `fmul` |
| `div`  | `sdiv`     | `udiv`       | `fdiv` |
| `rem`  | `srem`     | `urem`       | `frem` |

Signedness comes from the IIR type_hint prefix (`i*` = signed, `u*` =
unsigned).  `add`/`sub`/`mul` are signedness-agnostic at the bit level
so they share opcodes.

#### Comparison — `icmp`/`fcmp` + automatic zext

| IIR op | i32 | u32 | f64 |
|--------|-----|-----|-----|
| `eq`   | `eq` | `eq` | `oeq` |
| `ne`   | `ne` | `ne` | `one` |
| `lt`   | `slt` | `ult` | `olt` |
| `le`   | `sle` | `ule` | `ole` |
| `gt`   | `sgt` | `ugt` | `ogt` |
| `ge`   | `sge` | `uge` | `oge` |

Both naked (`eq`) and `cmp_`-prefixed (`cmp_eq`) opcodes are accepted —
the latter were introduced in gap G1 for the wasm backend and we accept
them here for cross-backend consistency.

Float predicates use `o<pred>` (ordered) — NaN compares false.  This
matches the most common language-level expectation.

LLVM `icmp`/`fcmp` always return `i1`.  When the IIR type_hint is wider
than `i1`, we automatically emit a `zext` to widen.  The original `i1`
form is preserved in a sidecar `env_i1` map so a downstream
`jmp_if_true` / `jmp_if_false` can consume it directly without a
redundant `trunc` round-trip.

#### Control flow — three opcodes + auto-fallthrough

* `label "name"`           → `name:`
* `jmp "name"`             → `br label %name`
* `jmp_if_true cond, name` → `br i1 <cond_i1>, label %name, label %__fallN`
* `jmp_if_false cond, name`→ `br i1 <cond_i1>, label %__fallN, label %name`

Conditional branches require both arms in LLVM IR; IIR's `jmp_if_*` only
names one target.  We synthesize a fresh `__fallN` block immediately
after the branch, so the next IIR instruction lands in a valid basic
block.  No structural changes upstream are required.

#### Type system additions

* `llvm_type_for` now accepts `i1` and `bool` (both → LLVM `i1`).
  Enables comparison results to be requested at i1 width directly, with
  no zext.

#### Tests added (37 total, was 22)

* Arithmetic (6): add-i32, fadd-double, sdiv, udiv, srem/urem same
  module, const-operand inlining.
* Comparison (5): icmp eq i32 + zext, ult for u32, fcmp olt for f64,
  `cmp_`-prefix alias, no-zext when type_hint=i1.
* Control flow (4): label block header, unconditional br, jmp_if_true
  with fallthrough block, jmp_if_false swaps arms.

[plan]: ../../../specs/MULTILANG-BACKEND-PLAN.md

## [0.2.0] — 2026-06-01 (LLVM02 — function signatures + ret/ret_void/const/mov)

### Added — function lowering and four instructions

Implements item LLVM02 of the [multi-language backend plan][plan].  This
release extends the v0.1.0 skeleton with the smallest set of instructions
that produces a runnable LLVM module:

| IIR op     | Lowering strategy                                      |
|------------|--------------------------------------------------------|
| `const`    | tracked in a name→operand map, no LLVM line emitted    |
| `mov`      | aliases dest to source's operand, no LLVM line emitted |
| `ret_void` | `  ret void`                                           |
| `ret`      | `  ret <ty> <operand>`                                 |

Sample output (`fn answer() -> i64 { const v = 42; ret v }`):

```llvm
; ModuleID = 'iir_module'
target triple = "x86_64-unknown-linux-gnu"

define i64 @answer() {
  ret i64 42
}
```

#### Design choices

* **`const`/`mov` are side-map operations, not LLVM lines.**  An obvious
  alternative is to emit `%dest = add <ty> 0, <src>` for both, but that
  produces no-op SSA assignments that `opt -mem2reg` would have to
  immediately clean up.  The side-map approach gives output that already
  looks like what hand-written `.ll` looks like.
* **Signless integer types.**  IIR's `u32` and `i32` both lower to LLVM
  `i32` — LLVM has no signedness in types.  The sign manifests in the
  opcode (`sdiv` vs `udiv`, `slt` vs `ult`) and will be picked up in
  LLVM03 when arithmetic lowering arrives.
* **Float literal format.**  We emit `{:e}` scientific notation (e.g.
  `1.5e0`), which round-trips through `f64::to_string` for finite values
  and is unambiguously parsed by LLVM.

#### Public surface added

* `IIRLlvmError::UndefinedVariable { function, name }` — surfaced when
  `ret` references a name that was never `const`/`mov`/param-bound.

#### Validator rules (`validate_for_llvm`)

* `SUPPORTED_OPS` whitelist: `["const", "mov", "ret", "ret_void"]`.
  Anything else → `UnsupportedOp`.
* Type rules: `void`, `i{8,16,32,64}`, `u{8,16,32,64}`, `f32`, `f64`.
  Anything else (incl. `ref<…>`, `str`, `bool`, `any`, `polymorphic`)
  → `UnsupportedType`.
* Checks run on: return type, every param type, every instruction's
  `type_hint`.  Errors aggregate; the lowerer fails fast with
  `ValidationFailed(Vec<String>)` if any are present.

#### Tests added (22 total, was 7)

* Function signature lowering (4): void/no-params, i32 with 2 params,
  float types, u32+i32 → i32 mapping.
* ret_void / ret (4): emission, const-inlined, param-register,
  undefined-var error.
* const / mov (3): no LLVM line for `const`, mov chains, mov of a param.
* Validator (4): accept-supported, reject-op, reject-ret-type, reject-param-type.

#### Not yet in v0.2.0

* Arithmetic, comparisons, branches — LLVM03.
* `call` and `call_builtin print_i64` extern decl — LLVM04.
* `lang-aot --backend=llvm` wiring — LLVM04.

[plan]: ../../../specs/MULTILANG-BACKEND-PLAN.md

## [0.1.0] — 2026-06-01 (LLVM01 — crate skeleton)

### Added — empty-module emission

First release.  Implements item LLVM01 of the
[multi-language backend plan][plan]: a crate skeleton that emits a valid
**empty** LLVM textual IR (`.ll`) module — a `; ModuleID = '<name>'`
comment plus a `target triple = "<triple>"` directive.

#### Public surface

```rust
pub struct IIRLlvmConfig {
    pub module_name: String,
    pub target_triple: String,
}
impl IIRLlvmConfig {
    pub fn new(module_name: impl Into<String>) -> Self;
    pub fn with_target(self, triple: impl Into<String>) -> Self;
}

pub enum IIRLlvmError {
    ValidationFailed(Vec<String>),
    UnsupportedOp     { function: String, op: String },
    UnsupportedType   { function: String, type_hint: String },
    InvalidOperand    { function: String, detail: String },
}

pub fn validate_for_llvm(module: &IIRModule) -> Vec<String>;
pub fn lower_iir_to_llvm(
    module: &IIRModule,
    cfg: &IIRLlvmConfig,
) -> Result<String, IIRLlvmError>;
```

#### What is NOT in v0.1.0

- **No instruction lowering.**  Function bodies in the input `IIRModule`
  are ignored.  v0.2.0 (LLVM02) starts lowering `ret_void` / `ret` /
  `const` / `mov`.
- **No `lang-aot --backend=llvm` wiring.**  Deferred to LLVM04.
- **No `llvm-sys` dependency.**  Textual `.ll` only — see the README and
  spec for the rationale.

#### Why textual `.ll`?

- Zero build-time dep: CI doesn't need LLVM installed.
- The output is the human-readable form — `assert!`-able in tests.
- Adding a sibling `llvm-sys` emitter later is a non-breaking change.

#### Why a fixed default `target_triple`?

The default is the literal string `"x86_64-unknown-linux-gnu"` rather
than a host-derived value.  Reasons:

- Test output is byte-identical across CI runners.
- Cross-compilation footguns are avoided — the user opts into a host
  override via `.with_target(...)` rather than receiving it implicitly.

#### Tests added

* `validate_returns_empty_for_empty_module`
* `output_contains_module_id_comment`
* `output_contains_target_triple`
* `output_starts_with_comment_or_target` (LLVM01 acceptance criterion)
* `default_config_has_nonempty_triple`
* `new_sets_module_name_keeps_default_triple`
* `errors_display_without_panic`

[plan]: ../../../specs/MULTILANG-BACKEND-PLAN.md
