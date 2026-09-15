## 0.228.0 - 2026-08-18 (macsyma-iir-vm.md Wave 4 — Macsyma on NativeAOT/LLVM/WASM/JVM/CLR)

Wired `Language::Macsyma` into the frontend dispatch — one `compile_source_to_iir`
match arm calling `macsyma_iir_compiler::compile_source`, plus the enum variant/
`Display`/`parse`/`detect_language_from_path` (`.mac` extension) boilerplate. No
other `lang-aot` code changed: every backend entry point
(`compile_source_to_llvm`/`_wasm`/`_jvm_class`/`_cil_artifact`, and native AOT via
`twig_aot::compile_module_to_*_executable`) already runs the same generic,
language-agnostic `iir-builtin-lowering` pass sequence for every `Language` —
this was verified, not assumed, before writing any code.

New cross-backend conformance suite, `tests/macsyma_conformance.rs`, mirroring
`conformance.rs`'s McCarthy W16 capstone structure: 21 v0 arithmetic/assignment
Macsyma programs (all four binary ops, the `/` exactness rule, unary `-`/`+`,
assignment + later reference, multi-statement chains) compute the identical
integer result on VM + NativeAOT + LLVM + WASM + JVM + CLR. Unlike the W16
capstone's macOS-only native runner, this suite's native-AOT column is
cross-platform (Linux/macOS/Windows, mirroring `lang_matrix.rs`'s
`compile_native`) — verified for real as a compiled-and-run Windows executable
on this development box, the literal "compiled to an AOT binary" proof.

Running the suite immediately surfaced two real, previously-latent bugs — the
one genuine risk this wave's own design doc flagged going in (Macsyma's
lowerer always emits `call_builtin "+"/"-"/"*"`, even over two already-concrete
literals, unlike every prior frontend):

- `iir-builtin-lowering` 0.39.0: `lower_dynamic_arith_function` only ever
  matched the binary (2-operand) `call_builtin` shape; a unary `call_builtin
  "-"` (Macsyma's `-x` on a concrete literal) fell through unrewritten to a
  backend's `call_builtin` whitelist, which knows heap/predicate builtins but
  not arithmetic names — first surfaced as a WASM `UnsupportedOp` for `-7$`.
  Fixed with a unary-negate case reusing the raw typed `neg` IIR op every
  backend already implements.
- `clr-simulator` 0.5.0: had no dispatch case for the CIL `neg` opcode (0x65)
  at all, even though `iir-to-cil-bytecode` already emitted it correctly (real
  CoreCLR already handles it natively) — no prior language reaching the CLR
  column had exercised unary negation this way. Fixed.

Both fixes are narrow and additive; the full pre-existing `lang-aot`/
`iir-builtin-lowering`/`clr-simulator` test suites stay green afterward (the
only surviving failures — `e6d7a_wasm_closures::closure_identity_returns_captured_value`
and 8 ALGOL nested-procedure array-capture cells in `lang_matrix.rs` plus
`matrix_every_proven_cell_agrees` — are confirmed pre-existing and unrelated,
verified against a clean `origin/main` checkout before this change).

BEAM and the universal JIT are explicitly out of scope for Macsyma (JIT is
still McCarthy-hardcoded; BEAM is scoped out repo-wide for non-McCarthy
languages per `LANG-PLATFORM-MATRIX.md`). See `code/specs/macsyma-aot-backends.md`.

