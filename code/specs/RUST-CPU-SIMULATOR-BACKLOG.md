# Rust CPU Simulator Backlog

**Status:** active

**Scope:** functional and gate-level Rust simulators for every CPU target in the
07-series, followed by a complete cross-language port wave
**Last reprioritized:** 2026-08-27

## Definition of the matrix

The architecture inventory is the CPU portion of the 07-series roadmap. It
contains every named physical CPU/ISA target from 07a through 07z and excludes
07c (WebAssembly), which is a virtual instruction set rather than a CPU. The
07b educational ARMv7 lane and the later 07x ARMv7-A/Thumb-2 lane remain
separate deliverables because they have different scope and public contracts.

Each target needs two Rust packages:

1. A **functional simulator** that performs instruction-level state changes
   with host-language operations.
2. A **gate-level simulator** whose data-path arithmetic and logic flow through
   the repository's Rust logic-gate and arithmetic primitives. Host integers
   may still be used for control flow, addresses, and trace bookkeeping. This
   is an ISA-level educational gate model, not a transistor-accurate die model.

An existing package is not automatically complete. Every existing Rust package
has a conformance/completeness audit in the backlog. An audit must compare the
package with its architecture spec, document the supported ISA surface, add
missing tests, and either close the row or add concrete follow-up items.

## Completion contract

A matrix cell is complete only when all of the following are true:

- The architecture and supported instruction surface are specified.
- Reset, load, single-step, bounded execution, and immutable state inspection
  are deterministic and tested.
- Invalid programs and out-of-range memory operations fail closed without
  panics or unbounded execution.
- Instruction families and architectural edge cases have tests, with at least
  80% coverage and a target of 95%.
- The package has a working `BUILD` recipe, is included in the Rust workspace,
  passes formatting and Clippy with warnings denied, and is documented in its
  README and changelog.
- A gate-level package has differential tests against its functional partner.

## Prioritization policy

The queue is recalculated after every merged pull request and whenever new work
is discovered. The ordering rules are:

1. Security, data-corruption, broken-main, and CI-blocking work comes first.
2. Otherwise proceed chronologically by CPU year, oldest first.
3. Within an architecture, finish or audit the functional simulator before the
   gate-level simulator so the latter has a behavioral oracle.
4. A discovered prerequisite is recorded immediately and placed before the
   item it blocks; non-blocking follow-up work is ranked by severity and age.
5. The cross-language wave begins only after every Rust matrix cell is complete.

## Rust coverage matrix and ordered queue

`Missing` means no matching Rust package exists. `Audit` means a package exists
but has not yet passed the completion contract above. Backlog IDs are ordered
according to the current prioritization run.

| IDs (functional / gate) | Year | Target | Functional Rust | Gate-level Rust |
|---|---:|---|---|---|
| RCPU-001 / RCPU-002 | 1948 | Manchester Baby (SSEM) | Complete: `manchester-baby-simulator` | Complete: `manchester-baby-gatelevel` |
| RCPU-003 / RCPU-004 | 1954 | IBM 704 | Complete: `ibm704-simulator` | Complete: `ibm704-gatelevel` |
| RCPU-005 / RCPU-006 | 1961 | GE-225 | Audit: `ge225-simulator` | Missing |
| RCPU-007 / RCPU-008 | 1964 | CDC 6600 | Missing | Missing |
| RCPU-009 / RCPU-010 | 1970 | DEC PDP-11 | Missing | Missing |
| RCPU-011 / RCPU-012 | 1971 | Intel 4004 | Audit: `intel4004-simulator` | Audit: `intel4004-gatelevel` |
| RCPU-013 / RCPU-014 | 1972 | Intel 8008 | Audit: `intel8008-simulator` | Audit: `intel8008-gatelevel` |
| RCPU-015 / RCPU-016 | 1974 | Intel 8080 | Audit: `intel8080-simulator` | Audit: `intel8080-gatelevel` |
| RCPU-017 / RCPU-018 | 1975 | MOS 6502 | Audit: `mos6502-simulator` | Audit: `mos6502-gatelevel` |
| RCPU-019 / RCPU-020 | 1976 | Zilog Z80 | Audit: `z80-simulator` | Audit: `z80-gatelevel` |
| RCPU-021 / RCPU-022 | 1978 | Intel 8086 | Audit: `intel8086-simulator` | Audit: `intel8086-gatelevel` |
| RCPU-023 / RCPU-024 | 1979 | Motorola 68000 | Audit: `m68k-simulator` | Audit: `motorola68k-gatelevel` |
| RCPU-025 / RCPU-026 | 1980 | Intel 8051 | Audit: `intel8051-simulator` | Audit: `intel8051-gatelevel` |
| RCPU-027 / RCPU-028 | 1985 | ARM1 / ARMv1 | Audit: `arm1-simulator` | Audit: `arm1-gatelevel` |
| RCPU-029 / RCPU-030 | 1985 | MIPS R2000 | Audit: `mips-r2000-simulator` | Audit: `mips-r2000-gatelevel` |
| RCPU-031 / RCPU-032 | 1987 | SPARC V8 | Audit: `sparc-v8-simulator` | Audit: `sparc-v8-gatelevel` |
| RCPU-033 / RCPU-034 | 1992 | DEC Alpha AXP 21064 | Missing | Missing |
| RCPU-035 / RCPU-036 | 1992 | PowerPC 601 | Missing | Missing |
| RCPU-037 / RCPU-038 | 2003 | x86-64 (AMD64) | Audit: `x86-simulator` | Missing |
| RCPU-039 / RCPU-040 | 2004 | ARMv7 educational baseline (07b) | Audit: `arm-simulator` | Missing |
| RCPU-041 / RCPU-042 | 2004 | ARMv7-A / Thumb-2 (07x) | Missing | Missing |
| RCPU-043 / RCPU-044 | 2010 | RISC-V RV32I (07a) | Audit: `riscv-simulator` | Missing |
| RCPU-045 / RCPU-046 | 2010 | RISC-V RV64I + M | Missing | Missing |
| RCPU-047 / RCPU-048 | 2011 | AArch64 (ARMv8-A) | Missing | Missing |
| RCPU-049 / RCPU-050 | 2020 | Apple M1 (AArch64 + NEON) | Missing | Missing |

Current selection: **RCPU-P004**, the GE-225 single-length indicator, shift,
and automatic-modification prerequisite. The slice corrects the `2506YY3` SXG
encoding and encoded group selection, applies core-resident X words to fixed
and shift instruction operands, enforces the 31-place shift limit and N-ready
precondition, preserves latched single-length overflow, and raises current core
coverage to 82.64%. RCPU-005 remains open for optional CPU and I/O, AAU, and a
final completion-coverage audit after those families land.

## Cross-language wave

After RCPU-050, freeze the Rust APIs and golden conformance vectors, then port
the completed pairs architecture by architecture in the same chronological
order. The initial port matrix is the fourteen non-Rust languages supported by
the scaffold generator: Python, Go, Ruby, TypeScript, Elixir, Perl, Lua, Swift,
Haskell, OCaml, Java, Kotlin, C, and C++. Existing implementations are audited
rather than overwritten. Before that wave starts, inventory additional package
ecosystems present in the repository (currently C#, Dart, F#, and Starlark) and
add scaffold support or an explicit exception so "all languages" remains an
auditable statement rather than an assumption.

The port wave has three infrastructure items before its generated per-target
queue:

- PORT-001: publish language-neutral instruction/state golden vectors for every
  completed Rust pair.
- PORT-002: inventory every repository language and record its scaffold/build
  support or explicit exception.
- PORT-003: generate the chronological architecture-by-language queue and mark
  existing packages as audit items.

## Discovery log

| Date | Item | Priority | Disposition |
|---|---|---|---|
| 2026-08-27 | RCPU-P004 manual audit found that `SXG` was hard-coded as `2506013` and selected a group from A, while General Electric's corrected manual specifies the variable `2506YY3` form and selects encoded group Y. Fixed/shift automatic modification was not decoded, the I register retained the unmodified operand, invalid modified targets advanced P, non-overflowing single operations cleared the latched overflow indicator, and N-input shifts ignored N readiness. | P0, architecture correctness, blocks RCPU-005 | Implement encoded SXG groups 00-31, operand-field modification through selected core X words with the 31-place shift bound and modified I value, fail-closed target preflight, latched single overflow through `BOV`/`BNO`, N-ready preflight, corrected-manual regressions, and an above-floor coverage audit. |
| 2026-08-27 | RCPU-P003 manual audit found that A/Q was combined as a conventional signed 40-bit integer even though the GE-225 uses one sign plus two 19-bit data fields and ignores or replaces Q's duplicated sign. This corrupted DAD, DSU, DCB, MPY, DVD, and SRD; zero-count double shifts skipped required sign transfers; NOR/DNO wrote their remainder into the selected X group instead of absolute location 0000. | P0, architecture correctness, blocks RCPU-005 | Implement the 39-bit architectural conversion, correct arithmetic/divide-overflow and A/Q shift/normalize semantics, and pin the manual's published octal examples before continuing the remaining integer/automatic-modification audit. |
| 2026-08-27 | RCPU-004 fidelity audit found that an initial floating implementation delegated FAD/FSB/FMP/FDH/FDP results to host `f64`, violating the gate-level completion contract even though simple differential tests passed. | P0, gate-level fidelity, blocks RCPU-004 | Replaced it before PR with exact bit-vector alignment, gate add/multiply/restoring divide, 53-bit round-to-nearest-even intermediates, and 512 seeded oracle comparisons including divide remainders. |
| 2026-08-27 | RCPU-005 primary-manual audit found that `07g` explicitly scoped the implementation as an MVP, while the Rust package silently wrapped effective addresses and multiword/device transfers, kept modification words in detached host arrays instead of reserved core, partially mutated state on range errors, and covered only 7 tests. The manual also exposes deferred automatic-modification, optional central-processor, controller-I/O, and AAU families plus incorrect double-length representation in the current model. | P0 memory/correctness prerequisite, then chronological architecture completeness | Add RCPU-P002 ahead of RCPU-005 for fail-closed installed-memory and architectural X-word storage. Continue RCPU-005 in manual-backed integer/shift, optional CPU and I/O, and AAU slices; do not start RCPU-006 until all slices close. |
| 2026-08-27 | RCPU-003 manual audit found five historical errors inherited by `07h` and the Python oracle: CAL targeted Q instead of P, HPR and DVH used transfer PCs, TNO retained a set overflow indicator, TNX was omitted, and +0240 FDH was mislabeled as +0241 FDP. | P0, architecture correctness, blocks RCPU-003 | Correct the Rust simulator and `07h` against IBM's 1955 manual, add targeted regressions, and record the Python implementation for repair during its later cross-language audit. |
| 2026-08-27 | RCPU-003 pre-push audit found that transport decoding could allocate an oversized temporary word vector before comparing it with configured memory, and an empty load with an unbounded origin could reach an invalid slice. | P0, allocation/panic safety, blocks RCPU-003 | Validate canonical length, origin, and decoded word count before allocation or slicing; add end-of-memory and `usize::MAX` origin regressions. |
| 2026-08-27 | RCPU-003: the repository already had a complete Python IBM 704 simulator and conformance suite. | P0, correctness aid | Use the Python implementation as the behavioral oracle while retaining the canonical Rust encoder as the transport authority; port its v1 semantics and architecture programs to Rust. |
| 2026-08-27 | RCPU-P001: the Rust IBM 704 encoder shifts an idealized 9-bit opcode into bits 35–27, labels `+0420` as HTR, emits little-endian words, and the backend treats a `CLA` address as an immediate. These conflict with the 1955 IBM Type B format and `07h`'s executable big-endian transport contract. | Resolved prerequisite | Corrected by merged PR #13234; RCPU-003 now consumes the canonical encoder and transport. |
| 2026-08-27 | The existing C and C++ IBM 704 encoders mirror the same legacy idealized layout. | P1, non-blocking for Rust | Preserve Rust-first ordering; add these packages to the IBM 704 cross-language port/audit item after the Rust matrix is complete. |
| 2026-08-27 | Current stable Clippy flags a collapsible ECALL condition in the already-affected `riscv-simulator`, blocking RCPU-P001's CI-equivalent graph lint. | P0, CI-blocking | Preserve behavior with a direct boolean assignment and verify the package tests in RCPU-P001. |
| 2026-08-27 | Current stable Clippy flags two nested phase-transition conditions in the already-affected `system-board`, blocking RCPU-P001's CI-equivalent graph lint. | P0, CI-blocking | Express the conditions as match guards without changing the phase transitions, then verify the package tests and affected graph in RCPU-P001. |
| 2026-08-27 | Pre-push security review found that independently compiled IBM 704 functions used function-local literal addresses after concatenation and did not enforce a module-wide 32K bound. | P0, correctness/data-corruption, blocks RCPU-P001 | Add absolute load-address relocation to the backend, enforce the remaining address space per function, and pin a two-function `lang-aot` regression test. |
| 2026-08-27 | Pre-push security review found that the Type A encoder could silently create Type B-discriminated words for prefixes `000` and `100`. | P0, encoding correctness, blocks RCPU-P001 | Reject non-Type-A and oversized prefixes through a typed, non-panicking encoder error and add boundary tests. |
| 2026-08-27 | Pre-push security review found that the backend applied its 32K guard only after using caller-controlled CIR length for allocation. | P0, allocation safety, blocks RCPU-P001 | Bound the minimum emitted word count before allocation and retain the exact post-lowering bound check. |
| 2026-08-27 | Existing Rust simulator APIs are not yet unified by a Rust equivalent of SIM00. | P1, non-blocking | Add an API-convergence design/audit before the cross-language golden-vector freeze; new crates meanwhile expose the five common lifecycle operations. |
| 2026-08-27 | The older CPU roadmap records Python completion, not Rust pair completion, and its gate-level list is stale. | P1, non-blocking | This file is the canonical Rust wave ledger; link it from the older roadmap in RCPU-001. |
| 2026-08-27 | Several existing Rust functional crates openly implement subsets (notably 07b ARMv7 and the x86-64 runtime lane). | P1 | Preserve their audit status and create precise follow-ups during their chronological audit items. |
