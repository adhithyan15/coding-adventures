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
- ☐ **C2 — cons / car / cdr (F2).** `(CONS a b)` → `newarr object` + `stelem.ref`
  ×2; `CAR`/`CDR` → `ldelem.ref` index 0/1; integer atoms `box`/`unbox.any
  [System.Int32]`. Verify `(CAR (CONS 7 9))`→7, `(CDR …)`→9 on real `dotnet`.
- ☐ **C3 — predicates + COND (F3–F5).** `pair?`→`isinst object[]`, `not`→`xor 1`,
  `equal?`→`unbox;unbox;ceq`, `COND` truthiness → branch. Verify `(ATOM 7)`→1,
  `(EQ 7 7)`→1, `(COND …)`→11.
- ☐ **C4 — symbols (F6).** Interned symbol ids (the shared
  `intern_symbols_structural`); `(EQ (QUOTE A) (QUOTE A))`→1, distinct→0.
- ☐ **C5 — lambda / LABEL / recursion (F7).** Each hoisted lambda as its own
  `.method`; application via `call <Method>`; recursion. Verify
  `((LAMBDA (X) (CAR X)) (CONS 7 9))`→7 and a recursive `LABEL`.
- ☐ **C6 — wire real-CoreCLR CLR into the conformance suite.** Add a `run_clr_real`
  arm to `lang-aot/tests/conformance.rs` (gated on `dotnet`+`ilasm`) so the W16
  table runs the CLR column on **real .NET**; ensure CI installs `ilasm`
  (`dotnet restore` of the ILAsm pack) so the upgrade holds on every PR.

## End state

CLR joins JVM/BEAM/LLVM/native as a backend verified on its **real runtime**, and
the cross-backend conformance matrix is genuinely stronger — the CLR column proven
by real CoreCLR, not an in-house simulator. The `clr-simulator` remains for fast,
zero-dependency unit checks; the real-runtime path is the verification of record.
