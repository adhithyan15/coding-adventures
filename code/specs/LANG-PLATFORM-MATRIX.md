# Language × Backend platform matrix — every language on every (non-BEAM) backend

**Goal:** verify, *by running*, that **every language frontend in the repo** executes
correctly on **every backend except BEAM** — the same completeness bar McCarthy Lisp
already clears, extended to the whole language family. LLVM coverage for every
language is an explicit priority.

This is the generalization of the McCarthy `MCCARTHY-LISP-PLATFORM-MATRIX.md` /
`CLR-REAL-RUNTIME-VERIFICATION.md` chapters: McCarthy was the reference language run
on all 8 backends; this chapter brings the other six languages up to the same
cross-backend bar (minus BEAM).

## Why this is mostly verification, not new backends

Every language frontend lowers to **one shared IIR** (`interpreter-ir::IIRModule`):

```
Twig / Nib / Brainfuck / Dartmouth BASIC / Oct / ALGOL 60   (+ McCarthy)
        │  each via <lang>_iir_compiler::compile_source
        ▼
                 IIRModule  (the lingua franca)
        │
        ├── mccarthy_lisp_vm::run            → VM        (generic IIR interpreter)
        ├── jit-core::GenericCirJit          → JIT       (⚠ McCarthy-only today)
        ├── twig-aot + aarch64/x86_64-backend→ native AOT
        ├── iir-to-llvm                      → LLVM  → real clang
        ├── iir-to-wasm                      → WASM  → wasm-runtime
        ├── iir-to-jvm-class-file            → JVM   → real java
        ├── iir-to-cil-bytecode              → CLR   → real ilasm + real dotnet
        └── iir-to-beam                      → BEAM  (OUT OF SCOPE — see below)
```

Each backend consumes the shared IIR, so a backend is *language-agnostic by
construction*: a frontend that lowers to IIR can in principle reach every backend
for free. The work here is therefore mostly **(a)** adding cross-language conformance
that proves each `(language, backend)` cell by running it, and **(b)** fixing the
real lowering / runtime gaps that running surfaces — not writing new code generators.

The two genuine exceptions, where new wiring (not just a test) is required:

- **JIT** — the only JIT entrypoint today is `run_mccarthy_on_jit` (McCarthy-only).
  A generic `run_on_jit(language, source)` must be wired (Phase I).
- **I/O languages on managed/LLVM backends** — Brainfuck (`putchar`/`getchar`) and
  Dartmouth BASIC (`PRINT`) produce results via **stdout**, not an exit code, so
  their conformance harness must capture stdout, and each backend's I/O intrinsics
  (`io_out`/`putchar`/`print_i64`) must be exercised end-to-end.

> **Note:** the `Language` enum doc comments in `lang-aot/src/lib.rs` are **stale**
> (they call DartmouthBasic / Oct "placeholders / no Rust frontend"); all six
> frontends are in fact wired into `compile_source_to_iir`. Fixing those comments is
> part of LM0.

## Scope

**Languages (6):** Twig, Nib, Brainfuck, Dartmouth BASIC, Oct, ALGOL 60.
(McCarthy Lisp is already complete — it is the reference, not a worklist item.)

**Backends (7):** VM, JIT, native AOT, LLVM, WASM, JVM, CLR.

**Out of scope — BEAM.** The Erlang VM is a purely-functional, immutable-term
runtime; languages with mutable imperative state (Brainfuck's tape, BASIC's
variables/`GOTO`) do not map cleanly onto it, so BEAM stays McCarthy-only. (If a
later chapter wants BEAM for the *expression* languages — Twig/Nib/Oct/ALGOL — it
can be added then; this chapter does not pursue it.)

## Methodology — prove every cell by RUNNING

Mirror the McCarthy W16 capstone (`lang-aot/tests/conformance.rs`):

1. Per language, a **battery** of small programs, each with a known result — an
   integer exit value for expression languages, or a stdout string for the I/O
   languages (Brainfuck/BASIC).
2. A backend-runner table (generalized from the McCarthy conformance runners to take
   a `Language`), each runner gated on its external tool (skip when absent).
3. For every `(program, backend)`: run and assert the result. The in-process
   backends (VM, and the simulators) are the floor; the external-tool backends
   (LLVM/clang, JVM/java, CLR/dotnet+ilasm, native/ld) upgrade the proof to the real
   runtime when installed.

A cell is **✅ only when a test actually runs the program through that backend and
asserts the result** — never on "the frontend lowers to IIR so it *should* work."

## Status legend

`✅` proven by a running test · `◑` in progress · `☐` not started.

## The matrix (target — every non-BEAM cell ✅)

| Language        | VM | JIT | native-AOT | LLVM | WASM | JVM | CLR |
|-----------------|----|-----|-----------|------|------|-----|-----|
| Twig            | ☐  | ☐   | ✅        | ◑    | ◑    | ◑   | ◑   |
| Nib             | ☐  | ☐   | ✅        | ☐    | ☐    | ☐   | ☐   |
| Brainfuck       | ☐  | ☐   | ✅        | ☐    | ☐    | ☐   | ☐   |
| Dartmouth BASIC | ☐  | ☐   | ✅        | ☐    | ☐    | ☐   | ☐   |
| Oct             | ☐  | ☐   | ✅        | ☐    | ☐    | ☐   | ☐   |
| ALGOL 60        | ☐  | ☐   | ☐         | ☐    | ☐    | ☐   | ☐   |

(`◑` = partial today: Twig is proven at *scalar* level on LLVM/WASM/JVM/CLR; the
slice promotes it to a full feature battery. The starting state is re-verified per
slice — the loop trusts running, not this table.)

## Worklist (one PR per item; slice further if large)

### Phase 0 — matrix harness

- ☐ **LM0 — cross-language conformance harness.** New `lang-aot/tests/lang_matrix.rs`:
  per-language program batteries + a `(Language, backend-runner)` table generalized
  from the McCarthy conformance runners (integer-exit and stdout variants). Wire only
  the **already-green** cells first (native-AOT for the five non-ALGOL languages, plus
  whatever VM/LLVM/etc. pass out of the box) so the grid exists and is honest. Fix the
  stale `Language` enum doc comments.

### Phase L — LLVM for every language (priority)

- ☐ **LM-L Twig** — full feature battery on real `clang` (beyond scalar).
- ☐ **LM-L Nib** — Nib on real `clang`.
- ☐ **LM-L Oct** — Oct (if/while/calls) on real `clang`.
- ☐ **LM-L Brainfuck** — tape + `putchar`/`getchar` via the LLVM C runtime; assert stdout.
- ☐ **LM-L BASIC** — `PRINT`/`LET`/`FOR`/`GOTO`/`IF` on `clang`; assert stdout.
- ☐ **LM-L ALGOL** — ALGOL 60 scalar/boolean on `clang`.

### Phase V — VM (generic IIR interpreter) for every language

- ☐ **LM-V** — Twig, Nib, Oct, Brainfuck, BASIC, ALGOL on `mccarthy_lisp_vm::run`
  (the generic IIR interpreter). Likely a single slice; split if a language surfaces
  an interpreter gap.

### Phase W — WASM for every language

- ☐ **LM-W** — each language on `iir-to-wasm` + `wasm-runtime` (Twig promote to full;
  Nib/Oct/BF/BASIC/ALGOL new). I/O languages: thread stdout through the wasm runtime.

### Phase J — JVM for every language

- ☐ **LM-J** — each language on `iir-to-jvm-class-file` + real `java` (the W16
  wrapper-launcher pattern). I/O via `System.out`.

### Phase C — CLR for every language

- ☐ **LM-C** — each language on `iir-to-cil-bytecode` (textual `.il` → real `ilasm` →
  real `dotnet`, the CLR-real path) + the `clr-simulator` floor. I/O via `Console`.

### Phase A — native AOT completeness

- ☐ **LM-A** — confirm/extend native-AOT to any language not yet proven there
  (ALGOL 60 especially), so the AOT column is uniformly ✅.

### Phase I — JIT (needs generic wiring)

- ☐ **LM-I0 — generic `run_on_jit(language, source)`.** Replace the McCarthy-only
  `run_mccarthy_on_jit` with a language-agnostic JIT entrypoint over the shared IIR
  (register the builtins each language needs, mirroring `jit_lisp.rs`).
- ☐ **LM-I** — each of the six languages on the JIT, verified by running.

## End state

Every language in the repo runs on every backend except BEAM, **verified by
running**, and the platform matrix is uniformly green (minus the deliberately-empty
BEAM column for the imperative languages). The capstone is a single
`lang_matrix.rs` suite asserting every `(language, backend)` cell agrees with the
known result — the cross-language analog of McCarthy's W16.
