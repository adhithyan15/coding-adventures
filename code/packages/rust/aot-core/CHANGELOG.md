# Changelog — aot-core

## 0.3.0 - 2026-07-18 (`mov` keeps its `any` type — GC-root correctness)

`translate()`'s `mov` arm retyped the CIR instruction to `"u64"` whenever the spec
type was `"any"`, while the *mnemonic* already fell back to `mov_u64` independently.
The type string is load-bearing metadata, not a codegen selector — backends dispatch
on the mnemonic and a move is a 64-bit bit-copy regardless — so the retype bought
nothing and actively lied.

That lie was a latent use-after-free for the GC precise-root work: `any` is the
normal type of a boxed dynamic value (a `mov` between boxed registers is an expected
`lower_dyn_repr` pattern), and the stack-map builder classifies a frame slot as a GC
root from this string. Claiming `u64` — the most confidently scalar type there is —
made an aliasing slot look like a plain integer, so a collection at a later safepoint
would not root it and would free a live cell out from under the mutator.

`mov` now carries `sp` unchanged (`"any"` stays `"any"`); the `mov_u64` mnemonic is
untouched, so emitted code is identical. Verified across aot-core, aarch64-backend,
x86_64-backend and twig-aot suites.


## 0.2.2 — 2026-06-15 — `u4` in the AOT type pipeline (LANG-FULL E2)

Added `"u4"` (Nib's 4-bit nibble type) to the two ALLOWED_TYPES whitelists so
the native CIR pipeline accepts `u4`-typed IIR instructions without refusing them.

- **`infer.rs`**: `ALLOWED_TYPES` now includes `"u4"`. `numeric_rank` assigns
  `u4` rank 1 (between `bool` = 0 and `u8` = 2), making the promotion table
  correct: `u4 + u8` promotes to `u8`, and `u4 + bool` promotes to `u4`.
  Previously `"u4"` was absent from both tables, so a `u4` type_hint emitted by
  `nib-iir-compiler` 0.14.0 was treated as an unknown type and fell back to the
  unspecialised path.
- **`specialise.rs`**: `ALLOWED_TYPES` now includes `"u4"` so
  `specialise::translate` emits typed CIR mnemonics (`add_u4`, `not_u4`, …)
  for u4-typed ops. Without this, a `u4` op silently collapsed to the generic
  `any` path and the aarch64/x86_64 backends rejected it with `UnsupportedOp`.

Also aligns the Cargo.toml version with the CHANGELOG (was accidentally frozen at
0.1.0 while the CHANGELOG had advanced to 0.2.1).

## 0.2.1 — 2026-05-13

### Added

- `specialise::translate` now handles the `"mov"` opcode, lowering it to
  `mov_<ty>` (or `mov_u64` when the type is still `"any"`).  This opcode is
  emitted by the AOT preparation pipeline's builtin pre-lowering step as a
  replacement for `call_builtin "_move"`.  Previously `"mov"` fell through to
  the generic CIR passthrough and the ARM64 backend rejected it with
  `UnsupportedOp("mov")`.

## 0.2.0 — 2026-05-11

### Changed (LANG32 — Operand::Str exhaustiveness)

- `infer.rs`: `resolve_operand` and `literal_type` now handle `Operand::Str`
  (compile-time string literal, LANG32 global variable names).  Both return
  `"str"` — the same sentinel already used for `Operand::Var` string-shapes.
- `specialise.rs`: `operand_concrete_ty` returns `None` for `Operand::Str`
  (not a concrete numeric type).
- `vm_runtime.rs`: `operand_to_json` serialises `Operand::Str(s)` as a JSON
  string, matching the `Var` convention.

## 0.1.0 — 2026-04-28

### Added

- **`AOTCore`** — ahead-of-time compilation controller that compiles an entire
  `IIRModule` to a `.aot` binary; configurable optimization level (0/1/2).
- **`infer_types()`** — flow-insensitive static type inference pass over
  `IIRFunction` instructions; seeds from declared parameter types and propagates
  through arithmetic, bitwise, comparison, and unary ops with numeric promotion.
- **`aot_specialise()`** — AOT analog of `jit-core`'s `specialise()`, producing
  typed `Vec<CIRInstr>` from an `IIRFunction` and a pre-computed type environment.
  Identical structure to the JIT pass; only the type-resolution step differs
  (env lookup vs. observed_type from the profiler).
- **`link()`** + **`entry_point_offset()`** — concatenate per-function binary
  blobs into a single code section with byte-offset table.
- **`snapshot::write()`** + **`snapshot::read()`** — 26-byte little-endian
  `.aot` binary format: magic `b"AOT\0"` + version + flags + entry_point_offset
  + IIR-table offset/size + native-code size, followed by code section and
  optional IIR-table section.
- **`VmRuntime`** — wraps a pre-compiled vm-runtime library; provides
  `serialise_iir_table()` (compact JSON via `serde_json`) and
  `deserialise_iir_table()` for inspection and testing.
- **`AOTStats`** — cumulative compilation statistics (functions compiled/untyped,
  time, binary size, optimization level).
- **`AOTError`** — `Backend` and `Snapshot` error variants.
- 110 unit tests + 12 doc-tests.

## 0.1.1 — 2026-05-05

### Added — `try_specialize_builtin` in `specialise`

`aot-core::specialise::aot_specialise` now lowers `call_builtin "<op>"
arg1 arg2` to typed CIR ops (`add_<ty>`, `sub_<ty>`, `mul_<ty>`,
`div_<ty>`, `cmp_<rel>_<ty>`, `mov_<ty>`) when the operands have
known types.  Maps the eleven Twig primitive names (`+`, `-`, `*`,
`/`, `=`, `==`, `!=`, `<`, `<=`, `>`, `>=`, `_move`) to typed
mnemonics.

This is what unlocks the user-visible promise of the LANG-runtime
pipeline: write a statically-typed program in IIR's interpreter
flavour, and the AOT compiler resolves all primitive operations to
native CPU instructions instead of runtime calls.

### Test coverage

- 112 existing tests still pass (no regressions).
- New end-to-end coverage in `twig-aot/tests/macos_arm64_smoke.rs`
  exercises the full lowering chain on real Twig source.
