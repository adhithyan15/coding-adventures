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
| RCPU-005 / RCPU-006 | 1961 | GE-225 | Complete: `ge225-simulator` | In progress: `ge225-gatelevel` |
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

Current selection: **RCPU-P006B1**, the GE-225 gate-level central decimal and
clock option. RCPU-005 is complete after the AAU/final-audit slice added separate
40-bit AX/BX/QX/IX state, all three calculation modes, exact general/arithmetic/
data-transfer and plug-7 status words, deterministic integer floating-point,
transient/hold alerts, modification, and fail-closed preflight. The functional
oracle has 91 simulator tests and 88.81% core line coverage (2,152/2,423), above
the 80% completion floor.

RCPU-006 is split into gate-auditable slices. P006A establishes flip-flop
memory/registers, gate decode, the 20-bit and one-sign-plus-38-data-bit central
binary datapaths, modification, control flow, shifts, compares, and lifecycle
differentials. P006B1 adds central decimal/clock state. P006B2 adds direct I/O,
and P006B3 adds controller-selector and API signals/state. P006C adds the separate AAU
register file and fixed/normalized/unnormalized datapaths, then runs the final
instruction-family differential and coverage audit. The chronological queue
does not advance to the CDC 6600 until all three close RCPU-006.

P006A merged in PR #13330: its 17 tests cover gate-backed
lifecycle, one-hot fixed/opcode decode, core-memory X groups, automatic
modification, all central single/double binary operations, multiply/divide,
all twelve shift/normalize paths, `MOV`, and atomic bounds failures. Core line
coverage was 86.11% (682/792), above the completion floor. P006B1 is next; its
acceptance boundary is DFF-backed decimal mode/carry and 19-bit clock state,
gate-only single/double BCD arithmetic and clock advancement, exact fixed-word
decode, oracle differentials, fail-closed validation, and above-floor coverage.
P006B1 is implementation-complete locally with 23 combined tests, including 48
seeded decimal vectors; core line coverage is 89.91% (1,257/1,398).

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
| 2026-08-27 | P006B fidelity review found two independently auditable hardware domains inside the original optional-I/O slice: decimal/clock is a combinational central datapath, while direct devices and selector/API are bounded event-driven state machines. Combining them would make gate provenance and atomic error review unnecessarily difficult. | P0, scope clarity, blocks RCPU-006 | Split P006B into P006B1 decimal/clock, P006B2 direct card/paper-tape/typewriter, and P006B3 selector/API. Preserve chronological order and keep all three ahead of the AAU/final audit. |
| 2026-08-27 | P006B1 implementation review found that the deterministic clock API accepts a 64-bit tick count, including `u64::MAX`; host modulo would violate the gate-level arithmetic contract even though instruction stepping remained gate-backed. | P0, gate fidelity, blocks P006B1 | Reduce and advance the external tick vector through a 65-bit restoring-division/add/subtract gate network, preserving both daily wrap and the documented 19-bit out-of-day recovery path. |
| 2026-08-27 | P006A pre-push security review found that `run(max_steps)` passed the caller-controlled bound directly to `Vec::with_capacity`, so `usize::MAX` panicked before executing even one fail-closed instruction. | P0, allocation/panic safety, blocks P006A | Grow traces only as accepted instructions execute, and pin an oversized bound that must report the first unknown instruction instead of preallocating from the bound. |
| 2026-08-27 | AAU pre-push security review found that normalized floating divide tried to normalize an exact zero quotient by left-shifting zero forever. A caller could load a noncanonical AX/QX pair and make one `FDV` step consume an unbounded CPU loop. | P0, denial-of-service safety, blocks AAU publication | Stop zero before the normalization loop, preserve its deterministic quotient/remainder result, and pin the externally loaded zero-quotient case. |
| 2026-08-27 | Propagating the P005C atomic core preflight into the AAU slice exposed the same late-failure shape in all sixteen AAU status branches: a not-taken skip beyond installed memory could update IX or clear overflow/underflow holds before failing. | P0, state-corruption safety, blocks AAU/final audit | Share a pure AAU status predicate between preflight and execution, validate the exact skip target before accepting the instruction, and pin full-state end-of-memory atomicity. |
| 2026-08-27 | Each CPU feature-branch push and pull-request event starts a separate full CI workflow, but branch protection accepts only the pull-request `CI gate`; the redundant push matrices can remain queued after merge and delay the next chronological slice. | P1, CI throughput, non-blocking | During the simulator wave, cancel only superseded push-event runs after confirming the corresponding pull-request run is retained. Schedule a dedicated workflow-trigger/concurrency audit without displacing architecture-correctness work. |
| 2026-08-27 | Final P005C fail-closed review found that an out-of-range second word or decision skip was detected only after I/P and, for some instructions, operand state had changed. The affected paths included even double-word operations, BXL/BXH, CAB/DCB, fixed readiness/register branches, and controller `BCS`; raw LDX/STX, SPB, and MOV also relied on execution-time checks. | P0, state-corruption safety, blocks RCPU-P005C | Add a single pre-execution core preflight for pair/raw/X/branch/MOV destinations and the exact taken skip, share pure branch predicates with execution, and pin full-state atomicity at installed-memory boundaries before publishing P005C. |
| 2026-08-27 | P006A lifecycle review found that its initial gate loader rejected a zero-word load exactly at installed-memory end, while the functional oracle accepts that empty half-open range; the same boundary matters to zero-length `MOV`. | P0, lifecycle parity, blocks P006A | Accept exactly-at-end empty loads and empty checked ranges, continue rejecting non-empty or beyond-end ranges without mutation, and pin end/oversized-origin regressions before publication. |
| 2026-08-27 | RCPU-006 inherits a much larger fidelity surface than the earlier gate-level machines: the completed GE-225 oracle includes central single/double arithmetic, modification and shifts, decimal/clock options, three direct devices, selector/API control, and a separate AAU with 40-bit fixed/floating datapaths. Treating that as one PR would make gate provenance and differential completeness difficult to audit. | P0, scope clarity, blocks RCPU-006 | Split RCPU-006 into P006A gate storage/decode and central binary core, P006B optional central/direct-I/O/controller/API state, and P006C AAU plus final full-family differential audit. Every persistent architectural bit is flip-flop-backed; arithmetic and logic results use `logic-gates`/`arithmetic`; host integers remain limited to addresses, loop/control sequencing, queues, and trace bookkeeping. |
| 2026-08-27 | The AAU manual makes the final functional slice a distinct coprocessor model rather than aliases over central A/Q. It has separate 40-bit AX/BX/QX/IX registers; fixed, normalized floating, and unnormalized floating modes; exact `30`/`31`/`32`/`33`/`35`/`36` arithmetic/data-transfer opcodes; eleven general words; and sixteen plug-7 `BAR` words. Overflow/underflow indicators are transient, their hold indicators persist, and hold tests conditionally clear the corresponding hold. | P0, architecture completeness, blocks RCPU-005 and RCPU-006 | Implement the separate public AAU state and exact encodings, paired/odd memory behavior, CPU X modification and IX capture, deterministic integer mantissa/exponent arithmetic, readiness/mode/address preflight, normalization and exponent alerts, all status branches, reset behavior, and signed regression vectors. Close RCPU-005 only after the combined package remains above the coverage floor. |
| 2026-08-27 | The API hardware description adds two control-flow constraints beyond basic ready-event vectoring: the optional priority X group is the special group 32 at core 0200-0203 (not encodable by ordinary `SXG`), and an interrupt may not occur between any `BRU` and its target—so even `BRU *` is uninterruptible. Priority return requires `SET PST` followed by a modified `BRU`; inserting `SET PBK` after `SET PST` must still leave priority mode while returning to the main program with API disabled. | P0, architecture/control-flow correctness, blocks RCPU-P005C | Reserve and select special group 32 only during API service, save P at 0201 and vector to 0204, restore the interrupted X group on the armed modified branch, retain the return arm across `SET PBK`, and suppress interrupt recognition for the first target access after every `BRU`. Pin immediate, deferred, disabled-return, and `BRU *` regressions. |
| 2026-08-27 | The primary manual makes RCPU-P005C a controller-selector **and API** slice, not just a generic device bus. The selector has eight fixed-priority plugs (0 highest), must alert-halt if `SEL P,X` (`2500P20`) is issued while busy, transmits the following two words without CPU execution, leaves P at the third sequential word, and clears controller errors on selection. `BCS` conditions are controller-specific. Optional API must remember enabled devices' not-ready-to-ready transitions even while interrupts are disabled, interrupt only at instruction boundaries, select X-group 32, store the main-program continuation at octal 0201, branch to octal 0204, enter priority mode with interrupts disabled, and require `SET PST` plus a modified branch to leave priority mode; card reader and punch participate, while typewriter and paper tape do not. | P0, architecture/control-flow correctness, blocks RCPU-005 | Make selection/busy/error behavior, opaque two-word command delivery, per-plug priority and readiness, controller-specific `BCS` predicates, API masks/latches/modes, exact group-32 save/vector behavior, deferred interrupts, and card ready transitions the acceptance boundary of RCPU-P005C. Keep controller timing deterministic through explicit service events and expose a public generic controller adapter before adding device-specific controller manuals. |
| 2026-08-27 | The GE-200 punched-card and GE-225 paper-tape subsystem manuals make RCPU-P005B larger and more exact than the current host-record abstraction. Cards require `RCD`/`RCB` continuous modes, `RCF` single-card mode, `HCR`, `WCD`/`WCB`/`WCF`, reader/punch ready branches, fixed data and synchronization-word layouts, and fail-closed not-ready behavior. Paper tape is a streaming N-register peripheral: `RON`/`PON` select mutually exclusive power paths, `RPT` streams frames and asserts N-ready, `HPT` stops motion, `WPT` emits one frame, and `OFF` powers the path down; unread frames can overrun N. The same `2500006` word means `TYP`, `RPT`, or `WPT` according to the powered N-register device, and `HPT` also enables the optional typewriter keyboard-input path. | P0, architecture and peripheral correctness, blocks RCPU-005 | Make those public contracts the acceptance boundary of RCPU-P005B. Preserve deterministic bounded host queues and output capture, decode the shared command from explicit device-selection state, model timing as explicit readiness/events rather than wall-clock sleeps, and pin all six card data layouts plus synchronization/status words, power switching, readiness branches, atomic memory failures, paper-tape overrun, typewriter input, and output bounds before selecting RCPU-P005C. |
| 2026-08-27 | The RCPU-P005A reprioritization audit found that the corrected programming manual treats direct M/N-register devices separately from controller-selector peripherals, delegates full punched-card and paper-tape behavior to subsystem manuals, and gives controller operations their own selection, three-word command, status, and interrupt model. A single combined I/O item would conceal two independently testable public contracts. | P0, scope clarity, blocks RCPU-005 | Split the remaining I/O work chronologically into RCPU-P005B for deterministic direct card/paper-tape/typewriter contracts and RCPU-P005C for controller-selector selection, status, command-block, interrupt, and generic device-controller contracts. Keep both ahead of AAU and the final functional audit. |
| 2026-08-27 | RCPU-P005A audit found that `SET DECMODE` only toggled exposed state: `ADD`, `SUB`, `DAD`, `DSU`, `ADO`, and `SBO` continued to execute binary arithmetic. The optional real-time-clock `LAC`/`LCA` instructions were absent, and opcode 24 was exposed as noncanonical `MOY` despite the corrected manual's `MOV` definition. | P0, architecture correctness, blocks RCPU-005 | Implement the documented three-digit-per-word BCD layout, ten's-complement signed fields, end-of-field overflow and carried lower fields; expose bounded deterministic clock control with exact `LAC`/`LCA` transfers; rename opcode 24 and its diagnostics to `MOV`; pin the manual's single/double arithmetic and clock examples. |
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
