# AOT00 — native-AOT robustness roadmap (toward GraalVM Native Image / .NET Native AOT)

**Status:** Draft — 2026-07-11 (spec-first north-star roadmap; sign-off = merge)
**Governs:** the direction of the `lang-aot` chain and every crate under it.
**North star (memory):** the AOT chain should eventually be as robust as **GraalVM
Native Image** or **.NET Native AOT**. Those are multi-MLOC systems; this is the
map that keeps each increment aimed there instead of at toy breadth.

## 0. One-paragraph summary

We have a shared `interpreter_ir` (IIR) lowered to **seven backends** (NativeAot
via aarch64/x86_64, LLVM, WASM, JVM, CLR, VM, JIT), a **cross-backend conformance
matrix** (159 cells that must *agree*), a growing **generic dynamic-value runtime
substrate** (DVAL01), and a **conservative** mark-sweep GC. That is the breadth of
a small language core. GraalVM/Native-AOT robustness is not "more opcodes" — it is
a **production runtime** (precise GC, exceptions, threads), **whole-program
discipline** (closed-world reachability + optimization), a **complete language +
stdlib**, and **conformance at spec-suite scale**. This roadmap defines a maturity
ladder and the parallel capability tracks that climb it, and it installs one gate:
**every increment must raise a robustness axis, not just add surface.**

## 1. The robustness bar, decomposed (what those systems actually give)

| Pillar | GraalVM NI / .NET AOT | Us today |
|---|---|---|
| **Precise GC** (stack maps, GC roots, generational/concurrent) | ✅ | ❌ conservative mark-sweep (`dynval_runtime.c` + `twig_gc.c`) |
| **Structured exceptions** (zero-cost unwinding tables) | ✅ | ❌ traps only |
| **Whole-program closed-world** (reachability, DCE, devirtualization) | ✅ | ❌ compile-what's-emitted |
| **Optimization pipeline** (inline, escape analysis, regalloc) | ✅ | ~ minimal (LLVM does some on that column; native/VM are naive) |
| **Complete language + large stdlib** (generics, vtables, closures, reflection-via-config, threads) | ✅ | ~ historical subsets + a dynamic-value substrate in progress |
| **Conformance at scale** (spec suite + differential + fuzzing) | ✅ | ~ 159-cell agreement matrix (the seed) |
| **Multi-platform + linking** (win/mac/linux × x64/arm64, static, PIE/TLS/signals) | ✅ | ~ mac-arm64 local + linux-x64 CI |
| **Debuggability** (DWARF/PDB, stack traces, profiling) | ✅ | ~ LANG14/LANG25 foothold |

Robustness = the **right column filled in with depth**, hardened against
adversarial input, and *measured*.

## 2. Maturity ladder

- **L0 — Core proven.** Shared IIR → N backends, each feature run-verified &
  cross-checked. *(where we are; the matrix is the evidence.)*
- **L1 — Generic runtime substrate.** A language-neutral value model + primitive
  ABI (dynamic values, heap objects, dispatch) any frontend targets — the
  Substrate-VM analogue. *(in progress: DVAL01.)*
- **L2 — One language, complete.** A single real language compiled AOT with its
  **full** semantics + a usable stdlib (not a subset): control flow, functions,
  closures, records/unions, dynamic dispatch, strings, collections, errors.
- **L3 — Production runtime.** Precise GC (stack maps) + structured exceptions +
  threads/safepoints. The point where "it runs" becomes "it runs *correctly under
  load and weird input*."
- **L4 — Whole-program + optimization.** Closed-world reachability → DCE +
  devirtualization; an optimization pass set (inline, escape analysis, regalloc).
  The point where binaries get *small and fast*.
- **L5 — Conformance at scale + platforms.** A real spec suite + differential
  oracle + fuzzing as blocking gates; win/mac/linux × x64/arm64, static linking,
  DWARF/PDB. The point where it is *trustworthy*.

Levels are not strictly serial — the **tracks** below advance in parallel — but a
level is "reached" only when its gate (§4) holds across the whole chain.

## 3. Capability tracks (parallel; each its own spec + PR ladder)

- **T1 Runtime — GC.** Conservative → **precise** GC: compiler emits stack maps /
  GC-root descriptors at safepoints; the collector uses them; then generational,
  then concurrent. Biggest single robustness lever. *(Depends on L1 substrate so
  the value model is uniform.)*
- **T2 Runtime — exceptions & unwinding.** Structured throw/catch, unwinding
  tables, stack traces; supersedes traps.
- **T3 Runtime — concurrency.** Threads, safepoints, memory model, a concurrent-GC
  handshake.
- **T4 Compiler — whole-program.** Reachability/points-to → DCE, devirtualization,
  build-time initialization; the closed-world assumption + its config surface.
- **T5 Compiler — optimization.** Inlining, escape analysis (stack allocation),
  constant folding, loop opts, better register allocation across the native/VM
  columns (LLVM already brings some).
- **T6 Language completeness.** Take one frontend all the way (generics/
  monomorphization, vtables/interfaces, closures, records/unions, reflection-via-
  config, a real stdlib). The DVAL substrate (L1) is what makes this affordable.
- **T7 Conformance-at-scale.** Grow the matrix into a real spec suite; add a
  reference-oracle differential harness + fuzzing as **blocking** gates. This is
  the mechanism that lets T1–T6 land safely.
- **T8 Platforms & artifacts.** windows + static linking + PIE/TLS/signals; DWARF/
  PDB debug info (extends LANG14/LANG25); profiling.

## 4. The robustness gate (applies to every PR from here)

A change earns its place only if it **raises a track's robustness axis** —
correctness under adversarial input, precision (GC/typing), coverage (conformance),
completeness (language), size/speed (optimization), or portability. "Adds an opcode
/ a matrix cell" counts **only** when it advances L2 language-completeness or T7
coverage. Pure breadth with no robustness axis is deferred. Cross-backend
**agreement** (the matrix invariant) remains mandatory: a feature that can't agree
across backends isn't done.

## 5. Sequencing (near-term, revisable)

1. **Finish L1 (DVAL substrate):** DVAL01-1c (crate rename) → -2 (builtin/pass
   rename) → **-3 (producer-agnostic dynamic-value classification — the real
   generalisation)** → resume E6 dynamic dispatch (arithmetic on native/LLVM →
   lists → symbols → records/unions → closures → dynamic globals) on the neutral
   substrate. *This is the Substrate-VM foundation everything above rests on.*
2. **Stand up T7 (conformance-at-scale) early**, in parallel: a reference-oracle
   differential harness + property/fuzz generation over the IIR, so T1–T6 land
   against a real safety net rather than hand-written cells.
3. **Open T1 (precise GC)** once L1 gives a uniform value model — the highest-value
   L3 robustness track — with T7 as its gate.
4. T2 (exceptions), T4/T5 (whole-program + optimization), T6 (one language
   complete), T8 (platforms) sequence behind, each gated by T7.

Each track gets its own detailed spec (this file is the index). Existing AOT specs
(LANG04 aot-core, LANG14 native-debug-info, LANG25 aot-debugger, LANG38 arithmetic-
completeness, BF03/BF07 native compiler) are absorbed as prior art under the
relevant track.

## 6. Non-goals / honesty

- **Not** claiming parity soon: this is a multi-year direction, and the spec's job
  is to keep increments pointed at it.
- **Not** a rewrite: the shared IIR + 7-backend + matrix architecture is the right
  spine; robustness is depth added to it, backend-agnostically where possible.
- **Not** breadth-for-its-own-sake: new historical-language cells are welcome only
  as T6/T7 progress, per §4.
