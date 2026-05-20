# LANG74 — LANG VM frontend coverage roadmap

**Status:** Draft — 2026-05-20
**Index spec** — collects and sequences the six concrete specs needed
to bring Nib, Brainfuck, Dartmouth BASIC, and Oct fully online on the
shared LANG VM AOT chain.

## Backdrop

PR #3673 introduced the `lang-aot` driver, which routes any source
that compiles to `interpreter_ir::IIRModule` through the AOT chain
built for Twig (x86_64-backend / aarch64-backend → `elf_object` /
`pe_object` / `macho_object` → system linker → native executable).

The driver works today for **Twig and Nib (trivial subset).**  Three
gaps remain:

1. **Brainfuck** — frontend wires correctly, but emits IIR ops
   (`load_mem`, `store_mem`, `putchar`, `getchar`) the V1 backends
   don't lower.
2. **Dartmouth BASIC** — no IIR-emitting frontend exists; the existing
   `dartmouth-basic-ir-compiler` targets a different IR for the
   GE-225 simulator.
3. **Oct** — Python-only frontend; no Rust port; the Python IR
   targets the Intel 8008 simulator and doesn't map cleanly to IIR.

This index spec catalogues the six follow-up specs that close those
gaps and tracks their dependencies.

## Spec graph

```text
   LANG75 (call_builtin + runtime helpers)
       │            │            │            │
       ▼            ▼            ▼            ▼
   BF07          NIB04        PL05         OCT02 phase 3
       ▲            ▲            ▲
       │            │            │
   LANG76 (byte memory + heap)
```

### Layer 1 — backend / runtime extensions

| Spec | What it adds |
|---|---|
| [LANG75](LANG75-call-builtin-and-runtime-helpers.md) | Generic `call_builtin "<name>", <args>` CIR opcode; runtime archive grows `putchar`, `getchar`, `print_string`, `input_i64`, `exit` |
| [LANG76](LANG76-byte-memory-ops-and-heap.md) | `alloc_bytes`, `load_byte`, `store_byte`; runtime `__twig_alloc_bytes` |

These two are language-agnostic and pay back across every frontend.

### Layer 2 — per-language frontend work

| Spec | Frontend | Effort |
|---|---|---|
| [BF07](BF07-brainfuck-aot-via-lang-vm.md) | Brainfuck end-to-end on LANG VM (consumes LANG75 + LANG76) | small — frontend already emits the IR |
| [NIB04](NIB04-nib-aot-via-lang-vm.md) | Nib AOT audit + missing features (user-defined fns, while loops, `print`); strings/arrays deferred to LANG77 | medium |
| [PL05](PL05-dartmouth-basic-iir-compiler.md) | New `dartmouth-basic-iir-compiler` crate; covers LET / IF / FOR / GOTO / GOSUB / PRINT / INPUT for integer programs | medium-large |
| [OCT02](OCT02-oct-rust-frontend.md) | Port Oct (lexer + parser + type-checker + iir-compiler) from Python to Rust; four-phase plan | large |

## Recommended sequencing

| Phase | Specs | Outcome |
|---|---|---|
| 1 | LANG75 | Frontends can emit `call_builtin "<name>"` for runtime helpers; `io_out` becomes sugar. |
| 2 | LANG76 | Byte memory + heap unlock BF and future array work. |
| 3 | BF07 | First non-Twig, non-Nib language ships end-to-end.  Proves LANG75 + LANG76 work in practice. |
| 4 | NIB04 steps 1–3 | Real Nib programs (functions, loops, `print`) compile.  Strings deferred. |
| 5 | PL05 | BASIC integer programs compile. |
| 6 | OCT02 phases 1–4 | Oct integer subset compiles, ending the survey. |

Each phase is independent; phase 3 onwards can interleave once phases
1–2 land.  PL05 and OCT02 don't share infrastructure, so they can run
in parallel if labour permits.

## Deliberately deferred

- **LANG77 — strings and `.rodata`**.  Strings affect every frontend
  (Nib `print("hi")`, BASIC `PRINT "HI"`, Oct's character set).  But
  they require non-trivial packager changes (new section + reloc kind
  on ELF/PE/Mach-O) that V1 doesn't need.  Sequenced after this
  roadmap is complete.
- **Floating point in the AOT chain**.  Twig has it via the
  interpreter; AOT lowering requires SSE2 work in the backends.
- **Garbage collection**.  V1 leaks; not a problem for AOT'd command-
  line scripts; future spec.
- **8008 intrinsics, fixed-address vars, port I/O in Oct.**  Oct's
  Intel-8008 backend remains the right home for these; LANG VM has no
  equivalent abstraction.

## How to use this index

When a contributor asks "how do I get language X compiling natively?":

1. Point at the corresponding leaf spec (BF07 / NIB04 / PL05 / OCT02).
2. Confirm LANG75 and LANG76 are done; if not, those land first.
3. Land the leaf spec's V1 cut.  Defer non-V1 features to follow-up
   work; the leaf spec lists what's V1 and what's deferred.

When all six specs land, every language in this survey compiles to a
native binary on Linux x86-64, Windows x86-64, and macOS ARM64
through the same chain Twig uses.

## Out of scope for this index

- Per-language **runtime** semantics beyond what LANG75 already
  introduces.  If a frontend needs runtime helpers other than the
  LANG75 V1 table, that's a separate spec that extends the table.
- The lang-aot **CLI surface** — `--target`, `--emit-object` for
  multi-language compilation.  Already tracked as a small follow-up
  on the lang-aot crate; no LANG-numbered spec needed.
