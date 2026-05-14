# Changelog — `twig-aot`

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
