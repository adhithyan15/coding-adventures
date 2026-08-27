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
| RCPU-001 / RCPU-002 | 1948 | Manchester Baby (SSEM) | Complete: `manchester-baby-simulator` | In progress: `manchester-baby-gatelevel` |
| RCPU-003 / RCPU-004 | 1954 | IBM 704 | Missing | Missing |
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

Current selection: **RCPU-002**, the Manchester Baby gate-level simulator. The
post-RCPU-001 prioritization run found no blocking discovery, so the paired
gate-level implementation remains ahead of RCPU-003.

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
| 2026-08-27 | Existing Rust simulator APIs are not yet unified by a Rust equivalent of SIM00. | P1, non-blocking | Add an API-convergence design/audit before the cross-language golden-vector freeze; new crates meanwhile expose the five common lifecycle operations. |
| 2026-08-27 | The older CPU roadmap records Python completion, not Rust pair completion, and its gate-level list is stale. | P1, non-blocking | This file is the canonical Rust wave ledger; link it from the older roadmap in RCPU-001. |
| 2026-08-27 | Several existing Rust functional crates openly implement subsets (notably 07b ARMv7 and the x86-64 runtime lane). | P1 | Preserve their audit status and create precise follow-ups during their chronological audit items. |
