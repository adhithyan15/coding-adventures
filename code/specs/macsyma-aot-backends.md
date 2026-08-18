# Macsyma → the other 6 IIR backends (Wave 4)

**Status:** Delivered — 2026-08-18
**Depends on:** [`macsyma-iir-vm.md`](macsyma-iir-vm.md) — this is Wave 4. Read
that spec first for the full v0 value-representation design (integer-only,
inert `cons`-chain for symbolic data, the `/` exactness rule); this leaf spec
only covers the delta of widening *where* that same v0 IIR runs.
**Unblocks:** nothing further named here — see `macsyma-iir-vm.md` §6 for
Waves 2 (Rational/Float) and 3 (control flow), the two remaining named waves.

## 1. Summary

Wires `Language::Macsyma` into `lang-aot`, the crate that already drives
McCarthy Lisp and Twig through NativeAOT (arm64/x86_64), LLVM, WASM, JVM, and
CLR. Macsyma's IIR (from `macsyma-iir-compiler`, Wave 1) needed **zero new
codegen** to reach these five backends — every one of `lang-aot`'s
`compile_source_to_llvm`/`_wasm`/`_jvm_class`/`_cil_artifact` and the native
`compile_file_to_*_executable` functions already runs the same generic,
language-agnostic `iir-builtin-lowering` pass sequence before invoking the
actual backend, driven purely by which ops/type-hints are present in a
module — not by which language produced it. BEAM and the universal JIT are
explicitly out of scope (see §3).

What this wave actually did:

1. Added the `Language::Macsyma` variant + one `compile_source_to_iir` match
   arm (`code/packages/rust/lang-aot/src/lib.rs`) — the entire frontend-wiring
   half of the work.
2. Added a cross-backend conformance suite
   (`lang-aot/tests/macsyma_conformance.rs`) proving 21 arithmetic/assignment
   Macsyma programs compute the identical integer result on all six backends
   (VM + NativeAOT + LLVM + WASM + JVM + CLR).
3. That suite immediately surfaced two real, previously-latent bugs (§4) —
   genuine engineering work, not just wiring.

## 2. Precedent

`macsyma-iir-compiler`'s emitted IIR (`const`/`"symbol"`/`"ref<LispyPair>"`
nil / `call_builtin "+"/"-"/"*"/"/"/"cons"/"car"/"cdr"`) is byte-identical in
shape to `mccarthy-lisp-iir-compiler`'s own emission (confirmed by direct
comparison of both crates' `emit_int`/`emit_nil`/`emit_symbol`/`emit_builtin`
helpers) — and McCarthy Lisp is proven complete on all 8 IIR backends
(`MCCARTHY-LISP-PLATFORM-MATRIX.md`, `lang-aot/tests/conformance.rs`'s W16
capstone). Twig's own dynamic-arithmetic matrix cells
(`e6d2b_dynamic_arith.rs`, `lang_matrix.rs`) already proved
`iir-builtin-lowering::lower_dynamic_arith` end-to-end on NativeAOT + LLVM —
but only for the case where at least one operand is already boxed (a `car`
result). This precedent is why the wiring itself needed no design work; §4
covers the one genuinely new case Macsyma's shape exercises.

## 3. Scope

**In scope:** NativeAOT (arm64/x86_64 — proven cross-platform via
`compile_file_to_{linux,macos,windows}_executable`, verified for real on this
Windows box), LLVM, WASM, JVM, CLR. Matches `macsyma-iir-vm.md` §6's Wave 4
definition exactly.

**Out of scope:**
- **BEAM.** Scoped out repo-wide for non-McCarthy languages
  (`code/specs/LANG-PLATFORM-MATRIX.md`'s own explicit BEAM exclusion); not
  named in Wave 4's own spec text.
- **The universal JIT.** `lang_aot::run_mccarthy_on_jit` is McCarthy-hardcoded
  today — there is no generic `run_on_jit(language, source)` yet
  (`LANG-PLATFORM-MATRIX.md` notes the same gap for its own 6-language
  worklist). Generalizing it is separate, larger work, not named in Wave 4.
- **Wave 2 (Rational/Float) and Wave 3 (control flow)** — untouched. This
  wave only widens where the *existing* v0 value model runs.
- **The other 5 CAS languages** (Wolfram/Derive/Reduce/Maple/Axiom) reaching
  these same 5 backends — natural future follow-up, each needing its own
  `Language` variant wiring, not decided or started here.

## 4. The pipeline, and the one genuine risk it surfaced

No backend changes were anticipated going in — `compile_source_to_llvm`,
`compile_source_to_wasm`, `compile_source_to_jvm_class`,
`compile_source_to_cil_artifact`, and `twig_aot::compile_module_to_*_executable`
(the native path, confirmed to run the identical pass sequence internally via
`prepare_module_for_aot`) already run `lower_global_io` →
`lower_closures_to_heap` → `lower_heap_builtins[_runtime]` →
`lower_dynamic_arith` → `intern_symbols[_structural]` →
`lower_dyn_repr[_structural]` → a `concretize_scalar_any_for_<backend>` pass,
generically, for every `Language`.

Unlike Twig (whose literal arithmetic lowers straight to a raw `add`, never
touching `call_builtin`), Macsyma's lowerer **always** emits `call_builtin
"+"/"-"/"*"` — even for two already-concrete literal operands
(`macsyma-iir-compiler/src/lower.rs`'s `combine`/`emit_builtin`). Running the
new conformance suite immediately found two real bugs in this previously
unexercised combination:

1. **Unary `call_builtin "-"` (Macsyma's `-x` on a concrete operand) was
   never rewritten by `iir_builtin_lowering::lower_dynamic_arith_function`.**
   That pass only matched the binary (2-operand) shape; a 1-operand
   `call_builtin "-"` fell through untouched to each backend's own
   `call_builtin` whitelist, which only knows heap/predicate builtins, not
   arithmetic names — surfaced first as a WASM `UnsupportedOp` validation
   failure. **Fix:** `dynamic_arith.rs` gained a unary-negate case
   (`unbox` → `neg` → `box`), reusing the raw typed `neg` IIR op every
   backend already implements (`interpreter_ir::opcodes::is_arithmetic`
   already lists it; `iir-to-wasm`/`iir-to-llvm`/`iir-to-jvm-class-file`/
   `iir-to-cil-bytecode`/the native backends all already lower it) — no
   backend change needed for this half of the fix.
2. **`clr-simulator` (the in-repo CLR interpreter used by
   `compile_source_to_cil_artifact`'s in-process verification) had no
   dispatch case for the CIL `neg` opcode (`0x65`) at all**, even though
   `iir-to-cil-bytecode` already emitted it correctly (standard CIL; real
   CoreCLR via `ilasm`/`dotnet` already handles it natively). **Fix:** added
   `OP_NEG` to `clr-simulator`'s opcode table and its `step()` dispatch
   (pop one `i32`, `wrapping_neg`, push).

Both fixes are narrow and additive — no existing backend behavior changed,
confirmed by running the full pre-existing `lang-aot`/`iir-builtin-lowering`/
`clr-simulator` test suites after the fix (only the pre-existing, unrelated
`e6d7a_wasm_closures::closure_identity_returns_captured_value` failure
remains, confirmed present identically on a clean checkout before this
wave's changes).

## 5. Conformance suite

`lang-aot/tests/macsyma_conformance.rs::macsyma_is_uniform_across_every_backend`
— mirrors `conformance.rs`'s McCarthy W16 capstone structure exactly
(per-backend runner functions, a `PROGRAMS` table, a floor-vs-gated assertion
policy that fails loudly rather than letting an absent-tool `None` mask a
real regression). 21 programs covering: integer literals; all 4 binary ops;
operator precedence/chains; the `/` exactness rule; unary `-`/`+`; assignment
and later reference; multi-statement chains with both `$`/`;` terminators.
VM/WASM/CLR are the always-run floor; JVM/LLVM/native-AOT are tool-gated and
— on this development box, with `java`/`clang`/a Windows linker present — all
three also ran and agreed. Bare-symbol/inert-cons results are intentionally
not exercised here (their representation differs by backend), matching
McCarthy's own W16 scoping; that surface already has VM-level oracle coverage
from Wave 1 (`macsyma-iir-compiler/tests/oracle.rs`).

## 6. Verification

- `cargo test -p lang-aot --test macsyma_conformance` — green, all 6 backends
  exercised on this box.
- `cargo test -p lang-aot -p iir-builtin-lowering -p clr-simulator` — full
  suites green except the pre-existing, unrelated `e6d7a_wasm_closures`
  failure (confirmed present on a clean checkout).
- `cargo clippy -p lang-aot -p iir-builtin-lowering -p clr-simulator
  --all-targets` / `cargo fmt --check` clean.

## References

[`macsyma-iir-vm.md`](macsyma-iir-vm.md) (the root spec),
[`LANG-PLATFORM-MATRIX.md`](LANG-PLATFORM-MATRIX.md) (the general
"prove every (language, backend) cell by running" methodology this wave
follows, for a different 6-language worklist),
[`MCCARTHY-LISP-PLATFORM-MATRIX.md`](MCCARTHY-LISP-PLATFORM-MATRIX.md) (the
proven 8-backend precedent), `lang-aot/tests/conformance.rs` (the McCarthy
W16 capstone this wave's own suite structurally mirrors).
