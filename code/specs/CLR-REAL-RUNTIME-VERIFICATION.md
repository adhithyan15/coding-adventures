# CLR backend — real CoreCLR verification

**Goal:** verify the CLR backend on a **real .NET runtime** (`dotnet`), not only the
in-repo `clr-simulator`. CLR is the one McCarthy backend never run on its actual
runtime — JVM uses real `java`, BEAM real `erl`, LLVM real `clang`, native a real
linker; CLR uses only an in-house CIL interpreter. A simulator can't *independently*
catch a backend bug (invalid CIL — stack imbalance, bad token, wrong `maxstack` —
that real CoreCLR's verifier rejects could pass a lenient simulator). This chapter
closes that gap.

## Approach — textual CIL, exactly like the LLVM backend

The CLR backend's binary path emits raw CIL method bodies (for the simulator). The
real-runtime path mirrors the **proven LLVM strategy**: emit **textual** code and
hand it to the **real toolchain**.

```
McCarthy source → IIR → managed value-model passes → emit_il (textual .il)
                                                          │  real ilasm
                                                          ▼
                                                     real PE assembly
                                                          │  real dotnet
                                                          ▼
                                                    printed result → assert
```

`iir-to-llvm` emits `.ll` text → real `clang`; `iir-to-cil-bytecode::emit_il` emits
`.il` text → real `ilasm`. `ilasm` owns all the metadata (PE headers, the
`#~`/`#Strings`/`#Blob` streams, token resolution) — no hand-rolled ECMA-335 tables.

`ilasm` ships as the NuGet runtime pack `runtime.<rid>.Microsoft.NETCore.ILAsm`; CI
fetches it via `dotnet restore`, locally it lives in the NuGet package cache. Both
`ilasm` and `dotnet` are gated — the tests skip gracefully when absent, like the
other external-tool backends.

## Status legend

`✅` done · `◑` in progress · `☐` not started.

## Worklist (one PR per item; slice further if large)

- ✅ **C1 — textual `.il` emitter + real-`dotnet` harness (scalar, F1).**
  `iir-to-cil-bytecode::emit_il` (new `il_text` module) emits an assemblable `.il`
  for the entry function (`const`/`mov`/`ret` → `ldc.i4`/`ldloc`/`stloc`/`ret`),
  wrapped in a `MccarthyEntry()` method + a printing `.entrypoint` launcher. New
  `lang_aot::compile_source_to_cil_text`. Verified by RUNNING on **real CoreCLR**
  (`lang-aot/tests/clr_real_scalar.rs`): `42`→42, `0`→0, `7`→7 — `.il` → real
  `ilasm` → real PE → real `dotnet`. Every other op returns `UnsupportedOp`, so the
  op match grows per slice below.
- ✅ **C2 — cons / car / cdr (F2).** `emit_il` gained `alloc` → `newarr
  [System.Runtime]System.Object` (a 2-element cons cell), `box`/`unbox.any
  [System.Runtime]System.Int32`, `field_store` → `stelem.ref`, `field_load` →
  `ldelem.ref`, and **mixed-type locals** (a cons cell is `object[]`, a boxed atom
  `object`, a raw int `int32`). The shared `ilasm`/`dotnet` harness was extracted to
  `tests/clr_support/mod.rs` (with a robust `find_ilasm` that searches *every*
  `*ilasm*` NuGet package — the binary lives only in the `runtime.<rid>.*` pack, not
  the ref-only `microsoft.netcore.ilasm`). Verified by RUNNING on **real CoreCLR**
  (`tests/clr_real_cons.rs`): `(CAR (CONS 7 9))`→7, `(CDR …)`→9,
  `(CAR (CDR (CONS 1 (CONS 2 3))))`→2.
- ✅ **C3 — predicates + COND (F3–F5).** `emit_il` gained `call_builtin "pair?"` →
  `isinst object[]; ldnull; ceq; ldc.i4.0; ceq` (the **textual** `isinst object[]`
  form — `ilasm` rejects an explicit `[System.Runtime]System.Object[]` scope there),
  `"not"` → `ldc.i4.1; xor`, `"equal?"` → `unbox.any int32` ×2 + `ceq`; and the
  `COND` control flow `label` → `<name>:`, `jmp` → `br`, `jmp_if_false` → `brfalse`
  (`jmp_if_true` → `brtrue`). A `const` of reference type (the `COND` nil
  fall-through) emits `ldnull`, not `ldc.i4 0`. Verified by RUNNING on **real
  CoreCLR** (`tests/clr_real_predicates.rs`): `(ATOM 7)`→1, `(ATOM (CONS 1 2))`→0,
  `(EQ 7 7)`→1, `(EQ 7 8)`→0, `(COND ((ATOM 7) 11) …)`→11,
  `(COND ((ATOM (CONS 1 2)) 11) ((EQ 5 5) 22))`→22.
- ✅ **C4 — symbols (F6).** **No new emit ops.** The shared
  `intern_symbols_structural` pass lowers each `(QUOTE S)` to a *tagged integer id*
  (`A` → `0x20000000`, `B` → `0x20000001`, …); on the CLR value model that id is
  just a boxed `System.Int32` atom, so `EQ`/`ATOM` on symbols reuse the C1–C3
  `const`/`box`/`equal?`/`pair?` path unchanged. Verified by RUNNING on **real
  CoreCLR** (`tests/clr_real_symbols.rs`): `(EQ (QUOTE A) (QUOTE A))`→1,
  `(EQ (QUOTE A) (QUOTE B))`→0, `(ATOM (QUOTE A))`→1,
  `(EQ (QUOTE FOO) (QUOTE FOO))`→1, `(EQ (QUOTE FOO) (QUOTE BAR))`→0. Unit test
  `symbol_eq_emits_tagged_id_consts_unboxed_and_compared` pins the value model.
- ✅ **C5 — lambda / LABEL / recursion (F7).** `emit_il` became a **multi-function**
  emitter: every IIR function is its own static `.method` (entry → `MccarthyEntry`,
  hoisted functions keep `lambda_<n>`/`label_<n>`), application is a by-name `call
  <ret> <Class>::<m>(<argtys>)` (so self-recursive `LABEL` is a method calling
  itself), parameters live in `ldarg`/`starg` slots (a new `FnRegs` model), `is_null`
  → `ldnull; ceq`, and a `field_*` on an `object`-typed operand (a lambda param)
  gets a `castclass object[]` before `ldelem.ref`/`stelem.ref` — real CoreCLR
  requires an array on the stack, which the lenient simulator never enforced.
  Function names join labels under the `checked_cil_ident` injection whitelist.
  Verified by RUNNING on **real CoreCLR** (`tests/clr_real_lambda.rs`):
  `((LAMBDA (X) X) 5)`→5, `((LAMBDA (X) (CAR X)) (CONS 7 9))`→7,
  `((LAMBDA (X Y) (EQ X Y)) 3 3)`→1, a COND-body lambda→100, and a recursive
  `LABEL` descending CARs→7. **This closes McCarthy F1–F7 on the CLR's real runtime.**
- ✅ **C6 — wire real-CoreCLR CLR into the conformance suite.** `lang-aot/tests/
  conformance.rs` gained a ninth backend column, **`CLR-real`** (`run_clr_real`),
  that runs each McCarthy program on real .NET via the shared `clr_support` harness
  (textual `.il` → real `ilasm` → real `dotnet`), gated on `dotnet`+`ilasm` so the
  in-process simulator `CLR` column stays the floor. Locally: **19 programs × 9
  backends, all agree.** CI (`.github/workflows/ci.yml`) installs `ilasm` whenever
  .NET is set up — a `<PackageDownload>` of the RID-specific
  `runtime.<rid>.Microsoft.NETCore.ILAsm` runtime pack into the NuGet cache where
  `find_ilasm()` searches — so the upgrade holds in CI, not just locally.

## End state — ACHIEVED ✅

CLR joins JVM/BEAM/LLVM/native as a backend verified on its **real runtime**, and
the cross-backend conformance matrix is genuinely stronger — the CLR column proven
by real CoreCLR, not an in-house simulator. The `clr-simulator` remains for fast,
zero-dependency unit checks; the real-runtime path is the verification of record.
All of C1–C6 are complete: McCarthy F1–F7 (scalar, cons, predicates+COND, symbols,
lambda/LABEL/recursion) all run on **real CoreCLR**, and the W16 capstone exercises
the CLR column on real .NET.
