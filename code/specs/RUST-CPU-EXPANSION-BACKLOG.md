# Rust CPU Expansion Backlog

**Status:** active
**Last reprioritized:** 2026-10-26

This is the live expansion queue after the original 07-series architecture
matrix. The older `RUST-CPU-SIMULATOR-BACKLOG.md` remains the audit record for
that matrix and PORT-001. This file tracks newly discovered physical CPUs and
the functional/gate cells needed before SoC, interconnect, and SPICE descent.

## Loop rules

1. Re-fetch `origin/main` and search open and merged pull requests before
   starting a cell.
2. Required-check failures, merge conflicts, and correctness/security defects
   outrank new work.
3. A functional cell and its gate partner are ordered, but unrelated CPU
   functional cells are independent. Slow CI on one cell does not block
   implementation of another.
4. Every PR gets squash auto-merge immediately. A human never manually merges
   it. Only a confirmed redundant same-head push run may be cancelled.
5. Functional cells own checked architectural state, typed atomic lifecycle,
   bounded complete traces, docs/build integration, strict checks, and more
   than 80% line coverage. Gate cells additionally require stable-bit topology,
   repository gate/arithmetic networks, and full-state differential tests
   against the functional oracle.
6. A public ISA is enough for a functional model. A gate claim requires a
   published datapath/RTL/netlist or an explicitly educational ISA-level gate
   topology. Proprietary Apple M-series internals are not invented: public
   AArch64 behavior can advance, but M5/M6 transistor topology remains
   evidence-blocked.

## Active publication wave

| Priority | Target | Functional | Gate | Dependency / next action |
|---:|---|---|---|---|
| P0 | DEC PDP-8 / PDP-8/E | merged | merged | Complete; PR #18120 landed. |
| P0 | EDSAC | merged | merged | Complete; gate PR #18166 landed. |
| P0 | IAS machine | merged | merged | Complete; gate PR #18266 landed. |
| P0 | IBM System/360 | merged | merged | Complete; gate PR #18181 landed. |
| P0 | Motorola MC6800 | merged | merged | Complete; gate PR #18182 landed. |
| P0 | Intel 8085 | merged | merged | Complete; gate PR #18257 landed. |
| P0 | DEC VAX-11/780 | merged | merged | Complete; gate PR #18263 landed. |
| P0 | Motorola 6809 | merged | merged | Complete; gate PR #18269 landed. |
| P0 | OpenRISC 1000 | merged | merged | Complete; gate PR #18276 landed. |
| P0 | OpenSPARC T1 | merged | merged | Complete; gate PR #18295 landed. |
| P0 | Arm Cortex-M0 | merged | merged | Complete; gate PR #18296 landed. |
| P0 | Arm Cortex-M3 | merged | merged | Complete; gate PR #18297 landed. |
| P0 | Arm Cortex-M23, Security Extension disabled | merged | local complete | Spec 07ao2 is implemented with exactly 524,898 authoritative stable-Q DFFs and independent gate-derived decode/data/flags/control. All 35 release tests pass, including exhaustive decode, every condition and shift-boundary outcome, arithmetic/multiply edges, memory-addressing overlaps, full-state differential, typed allocation/lifecycle rollback, exact locks over every stable-Q mutation route, compile-valid direct-Q hostile fixtures, and out-of-range decoder boundaries; measured line coverage is 93.64% (1,827/1,951). TrustZone/SAU, NVIC, MPU/IDAU, buses, timing, and proprietary silicon topology remain separate cells. |

## P0 gate-integrity repair queue

Completed gate packages are re-audited when source inspection finds a host
operation deciding an architecturally visible datapath result or a public host
field shadowing physical storage. These bounded
repairs outrank new completeness work because an existing gate claim is
already public.

| Priority | Target | Status | Required boundary |
|---:|---|---|---|
| P0 | Cortex-M APSR MSR masks | local complete | Repaired inherited Cortex-M3 functional/gate mask classification and Cortex-M7 GE/NZCVQ group isolation before Cortex-M7 gate composition. Non-DSP Cortex-M3 accepts only mask `10`; Cortex-M7 accepts `01`, `10`, and `11` with group isolation; `00`, unavailable-extension masks, and non-APSR aliases fault atomically. All 82 affected release tests, strict checks, docs, affected builds, exact functional/gate differentials, and 93.24% combined line coverage (4,251/4,559) are green; corrected independent exact-head review remains before publication. |
| P0 complete locally | x86-64 multiply/divide qualification | ready | The fixed-round multiplier/divider returns gate-qualified wide results; exact signed/unsigned fit boundaries, atomic divide faults, discarded high wires, parsed-syntax anti-bypass fixtures including inactive decoys, duplicate definitions, and exact production module routing, strict checks, all affected builds, and 88.40% line coverage are green. |
| P0 | Motorola 68000 gate | local complete | All 560 register/PC/SR DFF Q outputs and the HALT DFF Q output are authoritative; public fields are synchronized observers. Concrete inventory, deliberate observer-divergence lifecycle regressions, 71 unit tests, all 82 differential vectors, 6 lifecycle tests, 7 doctests, strict checks, lessons, and 83.39% combined line coverage are green. |
| P1 audit | Remaining completed gate packages | queued | Search architecturally visible arithmetic, flags, fault qualification, decode, state selection, and storage authority for host-language shortcuts; record and repair one bounded cell at a time. Address/control bookkeeping permitted by the educational boundary is not itself a defect. |

## Independent functional queue

This queue is re-costed after every merge or discovery. Dates indicate the
architecture/core introduction era, not a claim about a particular chip.

Cortex-M4 is merged. Its checked Armv7E-M
boundary explicitly classifies all 20 inherited Cortex-M3 instruction
families, differentially witnesses inherited integer, privilege, IT, memory,
exclusive, branch, power, and lifecycle state, and covers the bounded DSP and
optional FPv4-SP extension. All 30 release tests, strict Clippy/rustdoc, and
92.81% line coverage (994/1071) are green. Cortex-M7 and Cortex-M23 are also
merged, while Cortex-M33 is already in review.

| Order | Target | Public basis | Planned boundary |
|---:|---|---|---|
| 1 | Arm Cortex-M4 (merged) | Armv7E-M manuals | Cortex-M3 integer base plus documented DSP/FPU boundary. |
| 2 | Arm Cortex-M7 (merged) | Armv7E-M manuals | M7 programmer-visible core; cache/bus timing stays in SoC cells. |
| 3 | Arm Cortex-M23 (merged) | Armv8-M manuals | Bounded baseline/security-state cell complete; exclusive encodings and exceptions remain follow-up work. |
| 4 | Arm Cortex-M33 (in review) | Armv8-M manuals | Functional cell complete locally: bounded Mainline integer core, optional Security Extension, banked control/stack-limit state, eight-region SAU with disabled-only ALLNS and most-secure overlap reduction, distinct fetch/data policy, checked transition hooks, all-bank stack-limit closure, and exact inherited-family differential witnesses. Publish after independent review; encoded security instructions, DSP/FPU, exceptions, MPU/NVIC/SoC, and evidence-backed gate topology remain separate cells. |
| 5 | DEC PDP-10 family | published architecture manuals | One model-selected checked 36-bit functional oracle. |
| 6 | Data General Nova | published architecture manuals | Checked 16-bit accumulator/minicomputer boundary. |
| 7 | IBM System/370 | published Principles of Operation | Functional merged in PR #18336. P0 source-correctness repair complete locally: destructive-overlap `MVCL` suppresses storage movement, preserves both lengths and low 24-bit addresses, and clears the ignored high byte of both address registers for condition-code-3 completion. The gate partner remains blocked until this repair merges. |
| 8 | Motorola 68020 | published programmer reference | 32-bit 68k extension boundary. |
| 9 | PA-RISC 1.1 | published architecture manuals | Functional merged: checked bounded integer core, big-endian transport, both IA queues, PSW nibble carry/borrow state, delayed control, nullification, calls/returns, complete traces, typed atomic lifecycle, three allocation-seam unit tests, 19 integration tests, one doctest, and 92.80% line coverage (580/625). Gate Spec 07au2 was independently accepted; its complete local implementation is awaiting independent implementation re-review with 525,436 authoritative DFFs, independent strict decode/execution, gate-derived datapath/control, exact functional lifecycle/fault differentials, post-progress allocation rollback, compiler-checked full-crate hostile fixtures, 25 release tests, and 97.65% line coverage (1,040/1,065). No gate publication PR exists yet. |
| 10 | SuperH-2 | published programming manuals | Checked compact RISC core boundary. |
| 11 | SPARC V9 | published architecture manual | Complete and independently reviewed locally: bounded problem-state integer core implements 72-register complete state, four windows, icc/xcc/Y, the admitted V9 and compatibility integer/memory/control families, exact delay/annul/chaining traces, and typed transactional lifecycle. 48 tests, strict checks/docs, BUILD integration, and 91.22% line coverage (1,216/1,333) are green. Publication is next. |
| 12 | ARM7TDMI | published ARMv4T manuals | Complete locally: bounded ARMv4T ARM/Thumb integer core; all 31 physical GPRs and five SPSRs; pipeline-visible PC, bank/state exchange, checked memory and full-span control targets; exact transactional traces/lifecycle; 39 release tests; and 88.30% line coverage (1,389/1,573). Gate partner and SoC composition remain queued. |
| 13 | AVR classic (PR #18344) | published instruction-set manual | Complete: ATmega328P/`avr5` Harvard core, mapped register/I/O/data state, checked stack/control flow, all 26 typed undefined-overlap refusals, transactional traces/lifecycle, terminal-state restore round trips, whole-span two-byte call preflight, all-family terminal skip-target validation, 34 release tests with exhaustive add/subtract flag oracles and bug-sensitive fractional multiply flag regressions, and 97.43% line coverage (949/974). Squash auto-merge is armed; the gate partner remains queued. |
| 14 | Xtensa LX | publicly documented configurable ISA subset | Configuration-pinned functional core; no proprietary extensions. |
| 15 | RCA CDP1802 (`RCPU-081` / `RCPU-082`) | Renesas/Intersil FN1441 and archived RCA/Intersil data sheet | Complete locally: functional merged; gate Spec 07bc2 independently reviewed; independent gate decoder/executor with gate-authoritative legality/family/control predicates, repository-gate ALU/control, exact 524,667-DFF state, exact full-trace/full-state 255-transition and focused lifecycle/bus differentials, explicit I/O/DMA/interrupt seams, eleven compiler-valid hostile structural fixtures, typed atomic lifecycle including post-first-step allocation rollback, 40 release tests plus one doctest, and 95.62% line coverage (1,442/1,508). Exact-head review remains before publication. |
| 16 | LatticeMico32 (`RCPU-083` / `RCPU-084`) | official LatticeMico32 Processor Reference Manual 3.8 plus vendor-origin open RTL at `84b3e3ca0ad9535acaef201c1482342871358b08` | `RCPU-083` is complete locally: full pinned base configuration, 34 release tests, 712 checked-in evidence rows including 308 independent manual transitions, 183 distinct source-topology transitions, and 56 lifecycle/fault rows, strict checks, and 94.33% line coverage (882/935). Independent exact-head review is next. `RCPU-084` remains merge-dependent and must review the manual/RTL EBA bit-8 discrepancy before claiming state equality. |

CDP1802 is complete locally from its independently reviewed specification.
The functional cell covers the complete register matrix, instruction table,
reset/inhibit lifecycle, flags, ports, DMA, and interrupt response. Its
explicit bus operations make it a high-value bridge from CPU execution toward
the requested SoC data-movement simulations. A different reviewer's exact-head
implementation PASS is the remaining publication gate. The independently
specified educational gate partner is complete locally.
Source-backed SoC composition remains a separate follow-up.
The gate evidence is 40 release tests plus one doctest, including a 256-byte
decode sweep, exact full traces/states for all 255 accepted transitions and
focused alias/control/bus/lifecycle/fault/allocation matrices, exhaustive
256x256x2 ALU relations, eleven compiler-valid hostile fixtures, strict
Clippy/rustdoc, affected BUILD, and 95.62% line coverage (1,442/1,508).

LatticeMico32 was selected after a duplicate and evidence audit found no
pre-existing package, branch, worktree, open pull request, or merged pull
request for LM32.
It ranks ahead of evidence-blocked Itanium and LoongArch because Lattice
publishes a complete programmer reference while the pinned vendor-origin fork
retains the original decoder, register file, ALU, shifter, multiplier,
multi-cycle divider, interrupt logic, top-level CPU, and instruction tests.
The functional boundary is deliberately configuration-pinned; the dependent
gate cell is not authorized to claim transistor accuracy or a complete SoC.
The completed local functional cell owns all 32 mutable GPRs, complete 64 KiB
memory/image state, every pinned instruction family, exact CFG/CSR behavior,
DC.RE exception remapping, all 32 latched-pending interrupt inputs, complete
post-state and bus-transaction traces, and atomic lifecycle/allocation seams.
Its 32 release integration tests consume 747 checked-in evidence rows,
including 530 independently generated exact full-state/full-transaction
transitions, 52 lifecycle/fault rollback rows, five immutable RTL source
hashes represented by eight source facts, and 32 operand seeds that drive 672
additional complete-state ALU transitions; strict Clippy/rustdoc and 94.18%
measured release line coverage (874/928) are green. A different
reviewer's exact-clean-head implementation PASS is the remaining publication
gate.

## Discovered follow-up work

### Gate-integrity audit queue

| Priority | Target | Finding / next action |
|---:|---|---|
| P0 complete locally | SPARC V8 gate HALT storage | HALT DFF Q is now the sole architectural authority; the public boolean is only a synchronized compatibility observer. The published storage inventory derives 524,288 memory bits, 1,896 register/control bits, and one HALT bit from owned containers. Deliberate mirror-divergence, restore, run/result, and typed-fault atomicity tests pass alongside 42 unit tests, the 248-vector differential, and eight lifecycle/integrity tests. Strict Clippy/rustdoc/fmt, lessons validation, the six-package affected build, and clean all-target 94.68% line coverage (1,691/1,786) are green. |
| P1 complete locally | AArch64 gate 32-bit carry authority | The 32-bit ADD/SUB flag path now takes carry and overflow directly from exact-width ripple networks; host sum comparisons are gone. Width-edge tests, all seven topology/datapath tests, six lifecycle suites, all 836 Python differential vectors, strict checks, and 97.79% line coverage pass. Follow-up audits still need to cost host decode and fetch assembly separately. |
| P1 complete locally | PowerPC 601 conditional-branch authority | The host-language BO/CTR/CR predicate is replaced by explicit gate ALU, 32:1 CR mux, polarity, bypass-mux, and final-AND networks. Exhaustive tests cover every BO value and CR selector, both selected-bit polarities, and CTR zero/nonzero boundaries against an independent truth table. Three gate/topology suites, three lifecycle suites, all 239 differential vectors, strict checks, and 94.58% line coverage pass. The counted GPR/LR/CTR/XER/CR/CIA, memory, and HALT storage banks remain private stable-Q authority with no public host-state mirrors. |
| P1 complete locally | PowerPC 601 CR/XER update authority | Split signed/unsigned decoder lines gate-select compare relations, decoded compare and `mtcrf` lines gate-qualify CR commits, and gate-derived CR0/XER[CA] muxes preserve stable-Q SO/OV/opaque state. Excluded OE/arithmetic-Rc pairs remain atomic and the bounded six-bit `sraw` CA edge does not absorb the separate shift/count/divide cell. Exhaustive truth-table, modifier, lifecycle, producer-to-branch, corrected 239-vector differential, complete dispatcher AST locks, and `rustc`-checked hostile shadow/overwrite suites pass with strict checks and 95.38% gate-package line coverage (1,052/1,103), including 100% of the status network; the functional profile remains at 90.25% (639/708). |
| P1 queued | PowerPC 601 shift/count/divide authority | Replace host slice moves, iterator leading-zero counting, and conditional restoring-divide selection with fixed gate/mux networks, then audit multiply accumulation and record-form updates as one bounded datapath family at a time. |
| P0 complete locally | MIPS R2000 gate HALT storage | HALT DFF Q is now the sole architectural halt authority; the public boolean remains only as a source-compatible observation mirror and is repaired by simulator mutations. The published storage inventory derives 524,288 memory bits, 1,120 register bits, and one HALT bit from the owned physical containers. Deliberate mirror-divergence, restore, and typed-fault atomicity tests pass alongside 33 unit tests, the 218-vector differential suite, nine lifecycle/integrity tests, and 20 doctests. Strict Clippy/rustdoc, the six-package affected build, and 94.50% line coverage (1,581/1,673) are green. |
| P0 complete locally | Intel 4004 gate HALT storage | HALT Q is now the sole authority across observation, snapshot, reset/load/restore, step/run, `HLT`, and rollback; the Boolean observer is non-decisive under two-way divergence and repaired from Q. The container-derived inventory totals exactly 1,428 DFFs. Five compiler-valid hostile full-crate variants are rejected after type-checking. All 256 first-byte classifications, 90 unit tests, seven integration tests, four doctests, strict checks, the affected build, and 96.69% line coverage (1,607/1,662) pass. |
| P1 complete locally | IBM System/360 branch predicates | `BCR`/`BC`, `BALR`, `BCTR`/`BCT`, and `BXH`/`BXLE` now derive their select wires from gate mask muxes, target/count OR reductions, signed comparator condition-code wires, and one-hot opcode lines in both the executor and independent validator. Exhaustive mask/CC/register/count/index truth tables and token-normalized exact-body structural checks cover the complete branch-select, fixed-width-adder, signed-comparator, and top-level dispatch chain, including the exact four-byte router, RR/RX/RS dispatcher shells, complete validation router, branch-validator shell, public `step`/`run`/`execute` bridge, and AST-verified top-level file-backed executor/gates module identity, and reject hostile helper rewrites, dead anchors/dispatch, attributed/aliased/inline or macro-generated routing modules, macro-discarded declaration decoys, delimiter-shaped character-literal confusers, binding shadows, oracle substitution, expectation shadowing, and post-dispatch overwrites. All eleven existing differential/lifecycle suites, strict checks, and 93.01% package-source line coverage (1,662/1,787) pass. |
| P1 complete locally | IBM System/360 decode authority | One transition-boundary 256-line gate decode is now the sole RR/RX/RS/SI/SS/floating transport, legality, family, typed-operation, validator, and commit authority. Reserved holes retain their transport width and prior truncation precedence; inactive candidates cannot read storage, fault, or mutate. An independent literal 256-entry family/legality/width table is exercised at every 2/4/6-byte loaded length, with explicit short reserved-hole truncation witnesses. Parsed-token locks cover `DecodeAuthority::new`, width, the complete candidate map, every execution/validation router, gate branch/add/compare primitives, and the public commit chain. Anchored raw-match/range/equality/`trace.raw.first()` helper, dead/duplicate/secondary-decode, oracle-dispatch, shadow, inactive-access, and post-authorization/commit source mutations compile as disposable complete crates before rejection. |
| P1 queued | IBM System/360 divide qualification | The restoring divider uses gate subtraction per round but host equality detects zero and host `quotient > limit` decides the architectural divide fault. Replace both with exact-width gate reductions/comparison and pin signed fit boundaries. |
| P1 queued | IBM System/360 DFF transition discipline | Register banks are private stable-Q authority, but main-storage reset and snapshot restore write the packed Q bytes directly despite Spec 07ad2 promising sequential primitives. Decide and enforce a scalable clocked-restore boundary without weakening the topology claim. |
| P2 queued | IBM System/360 shifts and hexadecimal floating control | Shift movement/count bounds and substantial HFP zero/sign/exponent/normalization control still use host comparisons and native shifts. Split these broad surfaces into independently reviewable gate-network cells. |
| P1 audit | Remaining completed gate packages with public halt fields | Audit each package before changing it. A public observation field is acceptable only when execution, snapshots, restore, and faults are driven by DFF Q; otherwise add one bounded repair cell and a non-tautological storage inventory. Candidate packages discovered by source scan include ARM1, Intel 8051, Intel 8086, and OpenRISC 1000. |

### Functional follow-ups

- Add the repository-gate companion for the exact ATmega328P/`avr5` Classic
  AVR functional boundary before composing source-backed AVR buses and
  peripherals. Keep GPIO, timers, serial engines, ADC, interrupts, EEPROM,
  watchdog, clocking, and pins in later SoC cells rather than fabricating them
  inside the CPU oracle.
- Extend the bounded Cortex-M23 core with source-backed Armv8-M-exclusive
  instructions, encoded SG/BXNS/BLXNS transitions, exception stacking/return,
  stack-limit registers, and MPU/IDAU composition. Keep NVIC, bus transactions,
  debug, and timing in later SoC/integration cells.
- Expand Cortex-M3 beyond the bounded completed core with long multiply,
  bitfield, saturation, dual-register, byte/halfword exclusive, and remaining
  wide Thumb-2 encodings. This is non-blocking for the core lifecycle and SoC
  composition boundary, but it precedes declaring full Armv7-M ISA coverage.
- Extend Cortex-M33 only through separate witnessed cells: encoded
  SG/BXNS/BLXNS/TT transitions, Armv8-M-exclusive integer and DSP operations,
  optional FPU, exception entry/return and interrupt targeting, MPU/IDAU/NVIC,
  and a source-backed or explicitly educational gate-level topology. The
  bounded functional cell does not imply proprietary Cortex-M33 RTL.
- Extend PA-RISC 1.1 problem-state coverage in bounded cells: extract/deposit
  and variable shifts, unit operations, indexed/short/modifying memory forms,
  decimal correction, multiply/divide, MOVB/MOVIB, and remaining arithmetic
  completers. Privileged spaces/control registers, traps, MMU/TLB, caches,
  coprocessors/FPU, and implementation timing remain separate system cells.
- Build reusable SoC primitives after the initial missing-core wave: clock and
  reset domains, address-decoded buses, arbitration/backpressure, interrupt
  routing, DMA movement, timers, memory-mapped peripherals, and deterministic
  transaction traces.
- Compose the first source-backed SoCs around completed cores (Cortex-M
  microcontroller class and an OpenSPARC/OpenRISC platform), then add gate-level
  interconnect models.
- Descend below Boolean gates only when the SPICE/device-model wave lands.
  Transistor and analog timing claims must reference those lower-level models.
