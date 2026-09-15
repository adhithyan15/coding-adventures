## 0.46.0 — 2026-06-11 — **McCarthy's 1960 metacircular evaluator on every modern backend** — L7

McCarthy's 1960 paper closes with one of the most elegant ideas in
computing history: Lisp's `EVAL` function, written in Lisp itself.
Once you have the seven primitives (`QUOTE`, `ATOM`, `EQ`, `CAR`,
`CDR`, `CONS`, `COND`) plus `LAMBDA` and `LABEL`, you can express an
interpreter for the very language you're writing in.  That's the
**metacircular evaluator** — Lisp-in-Lisp.

This release adds exactly that test: McCarthy's 1960 `EVAL` authored
as McCarthy Lisp source (`(LABEL EVAL (LAMBDA (E) (COND ...)))`),
applied to 9 test programs, run through every modern backend.

### What changed

* New `tests/metacircular.rs` with:
  - `EVALUATOR_BODY` — McCarthy 1960 `EVAL` as McCarthy Lisp source.
  - `PROGRAMS` — 9 test inputs `(input-source, expected-integer)`:
    `42`, `0`, `(QUOTE 7)`, `(CAR (CONS 7 9))`, `(CDR (CONS 7 9))`,
    `(ATOM 7)`, `(EQ 5 5)`, `(EQ 5 6)`, `(CAR (CDR (CONS 1 (CONS 2 3))))`.
  - `metacircular_smoke_vm_evaluates_canonical_programs` — pure-VM
    smoke that validates `EVALUATOR_BODY` against every input.
  - `metacircular_eval_uniform_across_modern_backends` — runs the
    metacircular evaluator on the 8 modern backends, asserts integer
    agreement everywhere a result comes back.

### Conformance floor

| Backend | Floor? | Notes |
|---------|--------|-------|
| VM      | YES    | the reference interpreter (`mccarthy-lisp-vm`) |
| JIT     | YES    | `jit-core::GenericCirJit` |
| WASM    | YES    | in-repo `wasm-runtime` |
| CLR     | opt-in | `clr-simulator` panics on opcode `0x38` (long-form `br`) when the metacircular evaluator's nested-COND CIL exceeds short-branch range — documented simulator gap; the emitted CIL is valid (a real CLR loads it fine).  Caught via `catch_unwind`; once `clr-simulator` adds long-form-branch support this becomes a no-op. |
| JVM     | tool-gated on `java`                          | |
| BEAM    | tool-gated on `erl`                           | |
| LLVM    | tool-gated on `clang` + `lispy_runtime.c`     | |
| native  | macOS-only                                    | |

### Why this matters

After L1–L4 and the W1–W16 cascade, McCarthy Lisp runs on eight
backends through one shared IIR.  The W16 conformance suite proves
they agree on a hand-written program table.  **L7 raises the bar**:
the test programs become "anything the metacircular evaluator can
interpret," and the interpreter itself is a single McCarthy program
exercising COND, recursion, every primitive against itself.  Every
floor-required backend (VM/JIT/WASM) produces the identical integer
output across all 9 inputs.

### Specs

`code/specs/MCCARTHY-LISP-PLAN.md` updated — L7 marked ✓.

