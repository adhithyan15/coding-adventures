## 0.47.0 — 2026-06-11 — **McCarthy Lisp arc COMPLETE** — L6: byte-identical to Twig on every historical-arch backend

The closing PR of the McCarthy Lisp arc.  One table-driven test
asserts that the integer-literal program `7`, compiled from **both**
Twig and McCarthy Lisp source, emits **byte-for-byte identical
machine code** on all six historical-arch backends — GE-225,
Intel 4004, Intel 8008, ARMv7, RV32I, IBM 704.

The IIR convergence proof on the historical lanes: one IR layer
(IIR/CIR/Backend trait), two surface languages (typed Twig vs.
dynamic-typed McCarthy 1960), six machine architectures spanning
71 years (1954 vacuum-tube IBM 704 → 2025 ARMv7), six bit-identical
outputs.

### What changed

* `lang-aot` v0.47.0.
* New `tests/historical_round_trip.rs`:
  - One `#[test] fn mccarthy_byte_identical_to_twig_on_every_historical_arch`.
  - Table of 6 backends; for each, compile `7` from `.twig` and
    `.mcl` sources to a temp dir, read both `.bin` outputs,
    `assert_eq!` the bytes.
  - Floor: every backend must be exercised.  Anything less is a
    regression.

### Why `7` and not the canonical `42`?

The Intel 4004 (1971) is a *4-bit* microprocessor; its
immediate-load instruction (`LDM`) holds a value in `[0, 15]`.
The canonical `42` literal that the IBM 704 / GE-225 / Intel 8008
/ ARMv7 / RV32I per-arch e2e tests use overflows that 4-bit
window, so the Intel 4004 backend rejects it on **both** languages
(consistent refusal, but trivially so).  Choosing `7` lets us
exercise all six backends with a single non-trivial byte sequence,
strengthening the convergence claim to "all 6 historical-arch
lanes agree byte-for-byte" rather than "5 agree, 1 jointly
refuses."

### Specs

`code/specs/MCCARTHY-LISP-PLAN.md` updated: **L6 ✓ — McCARTHY LISP
ARC COMPLETE**.

### McCarthy Lisp arc — full summary

| Phase | What | Status |
|-------|------|--------|
| L1 | mccarthy-lisp-lexer + mccarthy-lisp-parser | ✓ |
| L2a/b/c | IIR compiler + mccarthy-lisp-vm + closures | ✓ |
| L3a | Language::McCarthyLisp in lang-aot | ✓ |
| L3b-1..2c | Native tagged-word cons/symbol/predicate via lispy_runtime.c | ✓ |
| L3b-3 / W1–W11 | wasm + JVM + CLR + BEAM (all 7 features) | ✓ |
| W12–W13 | LLVM (all 7 features) | ✓ |
| W14a/b | Native AOT macOS Mach-O link gap + lambda | ✓ |
| W15a/b | Universal JIT (all 7 features) | ✓ |
| W16 | Cross-backend conformance suite (8 modern × 19 programs) | ✓ |
| L4 + L5 | IBM 704 encoder + backend + --emit=ibm704 wiring | ✓ |
| L7 | Metacircular evaluator on every modern backend | ✓ |
| **L6** | **Cross-backend byte-pinned matrix on all 6 historical-arch backends** | **✓ (this release)** |

**McCarthy Lisp now runs on 14 backends — 8 modern (VM, JIT, WASM,
CLR, JVM, BEAM, LLVM, native AOT) and 6 historical-arch (GE-225,
Intel 4004, Intel 8008, ARMv7, RV32I, IBM 704) — through one shared
IIR/CIR layer.**

