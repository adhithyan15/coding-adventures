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
| P0 | Arm Cortex-M4 | merged | merged | Complete; functional PR #18313 and gate PR #18387 landed. |
| P0 | Arm Cortex-M7 | merged | merged | Complete; functional PR #18307 and gate PR #18351 landed. |
| P0 | Arm Cortex-M23, Security Extension disabled | merged | local complete | Spec 07ao2 is implemented with exactly 524,898 authoritative stable-Q DFFs and independent gate-derived decode/data/flags/control. All 35 release tests pass, including exhaustive decode, every condition and shift-boundary outcome, arithmetic/multiply edges, memory-addressing overlaps, full-state differential, typed allocation/lifecycle rollback, exact locks over every stable-Q mutation route, compile-valid direct-Q hostile fixtures, and out-of-range decoder boundaries; measured line coverage is 93.64% (1,827/1,951). TrustZone/SAU, NVIC, MPU/IDAU, buses, timing, and proprietary silicon topology remain separate cells. |
| P0 | Arm Cortex-M33 | PR #18326 | blocked on functional | Functional cell is published with squash auto-merge armed. |
| P0 | DEC PDP-10 KA10 | in progress | blocked on functional | Spec 07aq and the checked bounded functional cell are under local completion review. |

## P0 gate-integrity repair queue

Completed gate packages are re-audited when source inspection finds a host
operation deciding an architecturally visible datapath result or a public host
field shadowing physical storage. These bounded
repairs outrank new completeness work because an existing gate claim is
already public.

| Priority | Target | Status | Required boundary |
|---:|---|---|---|
| P0 | Cortex-M APSR MSR masks | local complete | Repaired inherited Cortex-M3 functional/gate mask classification and Cortex-M7 GE/NZCVQ group isolation before Cortex-M7 gate composition. Non-DSP Cortex-M3 accepts only mask `10`; Cortex-M7 accepts `01`, `10`, and `11` with group isolation; `00`, unavailable-extension masks, and non-APSR aliases fault atomically. All 82 affected release tests, strict checks, docs, affected builds, exact functional/gate differentials, and 93.24% combined line coverage (4,251/4,559) are green; corrected independent exact-head review remains before publication. |
| P0 merged | x86-64 multiply/divide qualification | PR #18323 merged | The fixed-round multiplier/divider returns gate-qualified wide results; exact signed/unsigned fit boundaries, atomic divide faults, discarded high wires, parsed-syntax anti-bypass fixtures including inactive decoys, duplicate definitions, and exact production module routing, strict checks, all affected builds, and 88.40% line coverage are green. |
| P0 merged | Motorola 68000 gate | PR #18316 merged | All 560 register/PC/SR DFF Q outputs and the HALT DFF Q output are authoritative; public fields are synchronized observers. Concrete inventory, deliberate observer-divergence lifecycle regressions, 71 unit tests, all 82 differential vectors, 6 lifecycle tests, 7 doctests, strict checks, lessons, and 83.39% combined line coverage are green. |
| P0 in PR #18379 | ARM1 gate HALT storage | Publication in progress | The HALT DFF Q output is the sole architectural halt authority across observation, reset/load/restore, interrupts, execute, checked and legacy step/run, and rollback. The concrete inventory, deliberate two-way mirror divergence, exact transition locks, hostile anti-bypass fixtures, 23 unit tests, eight lifecycle tests, three integrity tests, the 599-vector differential, strict checks, and 93.27% line coverage (1,189/1,269) are green. Compiler-checked hostile full-crate fixtures close the final review gap. Do not duplicate this cell. |
| P0 in PR #18381 | MOS 6502 gate HALT storage | Publication in progress | The one-bit `StateRegister` Q output is the sole HALT authority across observation, lifecycle, execution, IRQ/NMI, and rollback. Do not duplicate this cell. |
| P0 complete locally | Z80 gate HALT storage | Awaiting independent review | The one-bit `halt_state` Q output is authoritative across observation, run/step/results, reset/load/restore, HALT, IRQ/NMI, and rollback; the host boolean is observer-only. The container-derived inventory is exactly 528,597 DFFs. Two-way divergence, already-halted, IRQ/NMI, typed rollback, all 1,160 differential vectors, exact transition-chain locks, five compiler-checked hostile full-crate rewrites, strict checks, and 98.16% line coverage (2,621/2,670) are green. |
| P1 audit | Remaining completed gate packages | queued | Search architecturally visible arithmetic, flags, fault qualification, decode, state selection, and storage authority for host-language shortcuts; record and repair one bounded cell at a time. Address/control bookkeeping permitted by the educational boundary is not itself a defect. |

## Independent functional queue

This queue is re-costed after every merge or discovery. Dates indicate the
architecture/core introduction era, not a claim about a particular chip.

Cortex-M4 functional merged in PR #18313. Its checked Armv7E-M
boundary explicitly classifies all 20 inherited Cortex-M3 instruction
families, differentially witnesses inherited integer, privilege, IT, memory,
exclusive, branch, power, and lifecycle state, and covers the bounded DSP and
optional FPv4-SP extension. All 30 release tests, strict Clippy/rustdoc, and
92.81% line coverage (994/1071) are green. Its educational gate companion now
has a locally complete independent 526,013-DFF companion with every bounded
DSP/FP family differential, exact inherited full-trace/full-state matrices,
post-progress `run`/`execute` allocation rollback, seven compiler-checked
complete hostile crates, exact production source locks, and 91.66% line
coverage (1,627/1,775); independent implementation review remains required.
Cortex-M7 and Cortex-M23 are merged, and Cortex-M33 merged in PR #18326.

| Order | Target | Public basis | Planned boundary |
|---:|---|---|---|
| 1 | Arm Cortex-M4 (functional merged; gate complete locally, review pending) | Armv7E-M manuals | Cortex-M3 gate base plus gate-derived DSP and optional FPv4-SP boundary; no vendor MCU or timing claims. |
| 2 | Arm Cortex-M7 (merged) | Armv7E-M manuals | M7 programmer-visible core; cache/bus timing stays in SoC cells. |
| 3 | Arm Cortex-M23 (merged) | Armv8-M manuals | Bounded baseline/security-state cell complete; exclusive encodings and exceptions remain follow-up work. |
| 4 | Arm Cortex-M33 (PR #18326) | Armv8-M manuals | Functional cell complete locally: bounded Mainline integer core, optional Security Extension, banked control/stack-limit state, eight-region SAU with disabled-only ALLNS and most-secure overlap reduction, distinct fetch/data policy, checked transition hooks, all-bank stack-limit closure, and exact inherited-family differential witnesses. Publish after independent review; encoded security instructions, DSP/FPU, exceptions, MPU/NVIC/SoC, and evidence-backed gate topology remain separate cells. |
| 5 | DEC PDP-10 KA10 (PR #18331) | published architecture manuals | Complete bounded checked 36-bit functional oracle; squash auto-merge armed. |
| 6 | Data General Nova (in publication) | published architecture manuals | Checked common Nova/Nova 1200 core, all 32,768 operate words, atomic lifecycle, 17 release tests, and 91.53% line coverage (540/590). |
| 7 | IBM System/370 | published Principles of Operation | Functional merged in PR #18336. P0 source-correctness repair complete locally: destructive-overlap `MVCL` suppresses storage movement, preserves both lengths and low 24-bit addresses, and clears the ignored high byte of both address registers for condition-code-3 completion. The gate partner remains blocked until this repair merges. |
| 8 | Motorola 68020 | published programmer reference | 32-bit 68k extension boundary. |
| 9 | PA-RISC 1.1 | published architecture manuals | Functional merged: checked bounded integer core, big-endian transport, both IA queues, PSW nibble carry/borrow state, delayed control, nullification, calls/returns, complete traces, typed atomic lifecycle, three allocation-seam unit tests, 19 integration tests, one doctest, and 92.80% line coverage (580/625). Gate Spec 07au2 was independently accepted; its complete local implementation is awaiting independent implementation re-review with 525,436 authoritative DFFs, independent strict decode/execution, gate-derived datapath/control, exact functional lifecycle/fault differentials, post-progress allocation rollback, compiler-checked full-crate hostile fixtures, 25 release tests, and 97.65% line coverage (1,040/1,065). No gate publication PR exists yet. |
| 10 | SuperH-2 (in publication) | published programming manuals | Checked compact RISC core boundary. |
| 11 | SPARC V9 | published architecture manual | Complete and independently reviewed locally: bounded problem-state integer core implements 72-register complete state, four windows, icc/xcc/Y, the admitted V9 and compatibility integer/memory/control families, exact delay/annul/chaining traces, and typed transactional lifecycle. 48 tests, strict checks/docs, BUILD integration, and 91.22% line coverage (1,216/1,333) are green. Publication is next. |
| 12 | ARM7TDMI | published ARMv4T manuals | Functional merged. Educational gate partner repaired after independent review: exact 525,522-DFF stable-Q inventory; independent ARM/Thumb engine; gate-derived one-hot selectors through the complete Thumb ALU/high/BX/transfer/stack/conditional/write-enable/mnemonic surface; syntax-normalized whole-file token locks plus named parsed-body source-to-sink locks; compiler-valid hostile full-crate mutations across ARM and Thumb control; two-way candidate divergence; real public differential execution and complete-state rollback for every reachable typed functional fault; and explicit source/public-invariant narrowing for compatibility-only unreachable variants. All 59 release tests, strict checks, and 91.67% line coverage (2,168/2,365) are green; a different independent exact-head review is next, and SoC composition remains queued. |
| 13 | AVR classic (PR #18344) | published instruction-set manual | Complete: ATmega328P/`avr5` Harvard core, mapped register/I/O/data state, checked stack/control flow, all 26 typed undefined-overlap refusals, transactional traces/lifecycle, terminal-state restore round trips, whole-span two-byte call preflight, all-family terminal skip-target validation, 34 release tests with exhaustive add/subtract flag oracles and bug-sensitive fractional multiply flag regressions, and 97.43% line coverage (949/974). Squash auto-merge is armed; the gate partner remains queued. |
| 14 | Xtensa LX6 | public Tensilica ISA manual plus ESP32 manuals | Functional cell complete locally as `Esp32Lx6IntegerV1`, pending independent implementation review: little-endian, 64 physical windowed ARs, mixed 16/24-bit core+density, lazy window checks, loops, complete traces/lifecycle, 29 release tests, and 90.09% line coverage (1,382/1,534). Proprietary/configuration-unknown extensions remain excluded. |
| 15 | RCA CDP1802 (`RCPU-081` / `RCPU-082`; functional merged in PR #18354) | Renesas/Intersil FN1441 and archived RCA/Intersil data sheet | Complete locally: functional merged; gate Spec 07bc2 independently reviewed; independent gate decoder/executor with gate-authoritative legality/family/control predicates, repository-gate ALU/control, exact 524,667-DFF state, exact full-trace/full-state 255-transition and focused lifecycle/bus differentials, explicit I/O/DMA/interrupt seams, eleven compiler-valid hostile structural fixtures, typed atomic lifecycle including post-first-step allocation rollback, 40 release tests plus one doctest, and 95.62% line coverage (1,442/1,508). Exact-head review remains before publication. |
| 16 | MSP430 classic (MSP430F2274 profile) | TI SLAU144K family guide and SLAS504G device data sheet | Functional cell repaired locally: source-pinned classic 16-bit von-Neumann CPU with the full documented 27-instruction core, source-only constant-generator enforcement, exact Format II trace roles, and injected allocation rollback; independent re-review/publication is next. MSP430X, peripherals, clocks, power, and timing stay outside this cell. |
| 17 | LatticeMico32 (`RCPU-083` / `RCPU-084`) | official LatticeMico32 Processor Reference Manual 3.8 plus vendor-origin open RTL at `84b3e3ca0ad9535acaef201c1482342871358b08` | `RCPU-083` is complete locally: full pinned base configuration, 34 release tests, 712 checked-in evidence rows including 308 independent manual transitions, 183 distinct source-topology transitions, and 56 lifecycle/fault rows, strict checks, and 94.33% line coverage (882/935). Independent exact-head review is next. `RCPU-084` remains merge-dependent and must review the manual/RTL EBA bit-8 discrepancy before claiming state equality. |
| 18 | ZipCPU (`RCPU-085` / `RCPU-086`) | official ZipCPU Specification revision 3.0 plus open RTL at `42606d2d6ef55df313772232977621b2d72f0159` | `RCPU-085` is complete locally from its independently reviewed spec-only predecessor: fixed-width big-endian execution, two complete register banks, CC/PC/interrupt/image state, exact transactions, typed atomic lifecycle, 31 completion tests, two corpus consumers over 3,970 checked-in upstream rows, one doctest, and 92.70% release line coverage (1,270/1,370). Exact-clean-head review is next; `RCPU-086` remains blocked until the functional oracle merges. |
| 19 | PIC16 classic mid-range (PIC16F84A profile) | Microchip DS35007C device data sheet and DS33023A family reference manual | Complete locally: source-pinned 14-bit Harvard CPU with all 35 documented instructions, exact banked file-register/PCL/stack behavior, typed peripheral boundaries, 14 release tests, one doctest, strict checks, and 98.67% package-source line coverage (670/679). |

CDP1802 merged in PR #18354 after independent specification and implementation
review. The functional cell covers the complete register matrix, instruction
table, reset/inhibit lifecycle, flags, ports, DMA, and interrupt response. Its
explicit bus operations make it a high-value bridge from CPU execution toward
the requested SoC data-movement simulations. The functional publication
evidence is 31 release integration tests plus one doctest, including 256-byte
independent decode and execution sweeps, exhaustive 256x256x2 ALU relations,
strict Clippy/rustdoc, lessons validation, affected BUILD, and 87.76% line
coverage (832/948).
The independently specified educational gate partner is complete locally; a
different reviewer's exact-head implementation PASS is its remaining
publication gate. Source-backed SoC composition remains a separate follow-up.
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

MSP430 follows Xtensa and CDP1802 rather than displacing an active item. TI's public
SLAU144K chapter 3 gives a complete classic encoding, addressing, flag, and
stack contract, while SLAS504G pins a real non-MSP430X implementation with 16
registers and a 16-bit address space. That unusually complete primary-source
boundary makes the CPU independently implementable now. Its educational gate
and MSP430F2274 SoC compositions remain later cells so neither undocumented
silicon topology nor peripheral behavior is inferred.

The completed local functional cell exhaustively classifies all 65,536 first
words, spans every Format I and II register/addressing/width combination, all
N/Z/C/V jump inputs, exhaustive byte ADD/SUB-with-carry/borrow and packed-BCD
domains, source-only constant-generator destinations, and typed complete-state
lifecycle/allocation rollback. Twenty-five release
unit tests and one doctest, strict Clippy/rustdoc, the package BUILD, lessons
validation, and all-target release package-source line coverage of 98.56%
(1,777/1,803; `cargo llvm-cov -p msp430-simulator --all-targets --release
--summary-only`) are green.

LatticeMico32 remains immediately ahead of ZipCPU because its independently
reviewed implementation work was already active when the ZipCPU duplicate
audit completed. ZipCPU `RCPU-085` is now complete locally and awaits a
different reviewer's exact-head PASS. Its official revision-3 ISA,
independently written upstream C++ simulator, and immutable open RTL provide
the bounded functional evidence and the later educational-gate boundary.
Both cells are configuration-pinned CPU cores; neither may claim a complete
SoC, physical FPGA realization, transistor topology, or timing accuracy.

PIC16F84A follows MSP430, LatticeMico32 and ZipCPU in the independent queue. Microchip's public
DS35007C pins the exact 1K-by-14 program memory, 68-byte GPR file, core SFR
map, eight-entry 13-bit stack, and 35-instruction profile; DS33023A supplies
the detailed mid-range PC, stack, and instruction rules. The first cell keeps
Timer0, ports, watchdog, EEPROM, interrupts, clocks, and electrical behavior
behind typed boundaries instead of inventing peripheral semantics. A later
educational gate cell and a source-backed PIC16F84A SoC composition remain
separate queued follow-up cells after independent implementation review and
publication of the functional core.

## Discovered follow-up work

- Independently re-review the repaired local Spec 07ax2 ARM7TDMI educational
  gate partner at its final exact head. Real exception delivery and SoC buses
  remain separate bounded follow-ups.

### Gate-integrity audit queue

| Priority | Target | Finding / next action |
|---:|---|---|
| P0 merged | SPARC V8 gate HALT storage | Repair PR #18309 landed. HALT DFF Q is now the sole architectural authority; the public boolean is only a synchronized compatibility observer. The published storage inventory derives 524,288 memory bits, 1,896 register/control bits, and one HALT bit from owned containers. Deliberate mirror-divergence, restore, run/result, and typed-fault atomicity tests pass alongside 42 unit tests, the 248-vector differential, and eight lifecycle/integrity tests. Strict Clippy/rustdoc/fmt, lessons validation, the six-package affected build, and clean all-target 94.68% line coverage (1,691/1,786) are green. |
| P0 merged | Motorola 68000 gate storage | Repair PR #18316 landed. |
| P0 complete locally; review fix | Intel 8051 gate HALT and PC storage | HALT DFF Q and all 16 PC DFF Q outputs are now the sole architectural authorities; the public boolean and PC integer are synchronized compatibility observers. The storage inventory derives 524,288 code bits, 524,288 XDATA bits, 2,048 IRAM/SFR bits, 16 PC bits, and one HALT bit from owned containers. Deliberate HALT/PC mirror-divergence, snapshot, step, restore, run/result, and typed-fault rollback tests pass alongside 71 unit tests, the all-256-opcode differential, nine lifecycle/integrity tests, and 23 doctests. Strict Clippy/rustdoc/fmt, lessons validation, the five-package affected build, and 97.34% line coverage (1,937/1,990) are green. |
| P0 complete locally | Intel 8086 counted DFF authority | Every counted bank is now Q-authoritative across execution, snapshot, restore, rollback, and port I/O; public fields and arrays are synchronized compatibility observers only. Register/FLAGS public read APIs sample Q, explicit setters clock Q, instruction evaluation starts Q-to-wire and commits result wires, and IN/OUT use port Q directly. Adversarial divergence tests cover ADC, snapshot, restore, rollback, IN, and OUT. Release tests cover 62 unit cases, all 461 differential vectors, 10 lifecycle cases, and 14 doctests; line coverage is 95.34% (2,906/3,048). |
| P0 complete locally | SPARC V8 gate HALT storage | HALT DFF Q is now the sole architectural authority; the public boolean is only a synchronized compatibility observer. The published storage inventory derives 524,288 memory bits, 1,896 register/control bits, and one HALT bit from owned containers. Deliberate mirror-divergence, restore, run/result, and typed-fault atomicity tests pass alongside 42 unit tests, the 248-vector differential, and eight lifecycle/integrity tests. Strict Clippy/rustdoc/fmt, lessons validation, the six-package affected build, and clean all-target 94.68% line coverage (1,691/1,786) are green. |
| P1 merged | AArch64 gate 32-bit carry authority | PR #18317 landed. The 32-bit ADD/SUB flag path now takes carry and overflow directly from exact-width ripple networks; host sum comparisons are gone. Width-edge tests, all seven topology/datapath tests, six lifecycle suites, all 836 Python differential vectors, strict checks, and 97.79% line coverage pass. Follow-up audits still need to cost host decode and fetch assembly separately. |
| P1 merged | PowerPC 601 conditional-branch authority | PR #18319 landed. The host-language BO/CTR/CR predicate is replaced by explicit gate ALU, 32:1 CR mux, polarity, bypass-mux, and final-AND networks. Exhaustive tests cover every BO value and CR selector, both selected-bit polarities, and CTR zero/nonzero boundaries against an independent truth table. Three gate/topology suites, three lifecycle suites, all 239 differential vectors, strict checks, and 94.58% line coverage pass. The counted GPR/LR/CTR/XER/CR/CIA, memory, and HALT storage banks remain private stable-Q authority with no public host-state mirrors. |
| P1 complete locally | PowerPC 601 CR/XER update authority | Split signed/unsigned decoder lines gate-select compare relations, decoded compare and `mtcrf` lines gate-qualify CR commits, and gate-derived CR0/XER[CA] muxes preserve stable-Q SO/OV/opaque state. Excluded OE/arithmetic-Rc pairs remain atomic and the bounded six-bit `sraw` CA edge does not absorb the separate shift/count/divide cell. Exhaustive truth-table, modifier, lifecycle, producer-to-branch, corrected 239-vector differential, complete dispatcher AST locks, and `rustc`-checked hostile shadow/overwrite suites pass with strict checks and 95.38% gate-package line coverage (1,052/1,103), including 100% of the status network; the functional profile remains at 90.25% (639/708). |
| P1 queued | PowerPC 601 shift/count/divide authority | Replace host slice moves, iterator leading-zero counting, and conditional restoring-divide selection with fixed gate/mux networks, then audit multiply accumulation and record-form updates as one bounded datapath family at a time. |
| P0 merged | MIPS R2000 gate HALT storage | PR #18304 landed. HALT DFF Q is now the sole architectural halt authority; the public boolean remains only as a source-compatible observation mirror and is repaired by simulator mutations. The published storage inventory derives 524,288 memory bits, 1,120 register bits, and one HALT bit from the owned physical containers. Deliberate mirror-divergence, restore, and typed-fault atomicity tests pass alongside 33 unit tests, the 218-vector differential suite, nine lifecycle/integrity tests, and 20 doctests. Strict Clippy/rustdoc, the six-package affected build, and 94.50% line coverage (1,581/1,673) are green. |
| P0 complete locally | MOS 6502 gate HALT storage | The one-bit `StateRegister` Q output is the sole HALT authority across observation, reset/load/restore, step/run, BRK, IRQ/NMI, and rollback; the private host Boolean is observer-only. The exact 528,184-bit inventory is derived from owned containers. Two-way divergence and five compiler-checked hostile full-crate variants pass alongside 83 unit tests, the complete 256-opcode differential, seven lifecycle/integrity tests, seven doctests, strict checks, and 98.75% package-source line coverage (1,834/1,857). |
| P0 complete locally | Intel 4004 gate HALT storage | HALT Q is now the sole authority across observation, snapshot, reset/load/restore, step/run, `HLT`, and rollback; the Boolean observer is non-decisive under two-way divergence and repaired from Q. The container-derived inventory totals exactly 1,428 DFFs. Five compiler-valid hostile full-crate variants are rejected after type-checking. All 256 first-byte classifications, 90 unit tests, seven integration tests, four doctests, strict checks, the affected build, and 96.69% line coverage (1,607/1,662) pass. |
| P1 complete locally | IBM System/360 branch predicates | `BCR`/`BC`, `BALR`, `BCTR`/`BCT`, and `BXH`/`BXLE` now derive their select wires from gate mask muxes, target/count OR reductions, signed comparator condition-code wires, and one-hot opcode lines in both the executor and independent validator. Exhaustive mask/CC/register/count/index truth tables and token-normalized exact-body structural checks cover the complete branch-select, fixed-width-adder, signed-comparator, and top-level dispatch chain, including the exact four-byte router, RR/RX/RS dispatcher shells, complete validation router, branch-validator shell, public `step`/`run`/`execute` bridge, and AST-verified top-level file-backed executor/gates module identity, and reject hostile helper rewrites, dead anchors/dispatch, attributed/aliased/inline or macro-generated routing modules, macro-discarded declaration decoys, delimiter-shaped character-literal confusers, binding shadows, oracle substitution, expectation shadowing, and post-dispatch overwrites. All eleven existing differential/lifecycle suites, strict checks, and 93.01% package-source line coverage (1,662/1,787) pass. |
| P1 complete locally | IBM System/360 decode authority | One transition-boundary 256-line gate decode is now the sole RR/RX/RS/SI/SS/floating transport, legality, family, typed-operation, validator, and commit authority. Reserved holes retain their transport width and prior truncation precedence; inactive candidates cannot read storage, fault, or mutate. An independent literal 256-entry family/legality/width table is exercised at every 2/4/6-byte loaded length, with explicit short reserved-hole truncation witnesses. Parsed-token locks cover `DecodeAuthority::new`, width, the complete candidate map, every execution/validation router, gate branch/add/compare primitives, and the public commit chain. Anchored raw-match/range/equality/`trace.raw.first()` helper, dead/duplicate/secondary-decode, oracle-dispatch, shadow, inactive-access, and post-authorization/commit source mutations compile as disposable complete crates before rejection. |
| P1 queued | IBM System/360 divide qualification | The restoring divider uses gate subtraction per round but host equality detects zero and host `quotient > limit` decides the architectural divide fault. Replace both with exact-width gate reductions/comparison and pin signed fit boundaries. |
| P1 queued | IBM System/360 DFF transition discipline | Register banks are private stable-Q authority, but main-storage reset and snapshot restore write the packed Q bytes directly despite Spec 07ad2 promising sequential primitives. Decide and enforce a scalable clocked-restore boundary without weakening the topology claim. |
| P2 queued | IBM System/360 shifts and hexadecimal floating control | Shift movement/count bounds and substantial HFP zero/sign/exponent/normalization control still use host comparisons and native shifts. Split these broad surfaces into independently reviewable gate-network cells. |
| P1 audit | Remaining completed gate packages with public halt fields | Audit each package before changing it. A public observation field is acceptable only when execution, snapshots, restore, and faults are driven by DFF Q; otherwise add one bounded repair cell and a non-tautological storage inventory. Candidate packages discovered by source scan include ARM1 and OpenRISC 1000. |

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
- Add an independently bounded classic-MSP430 interrupt/reset extension before
  the SoC cell: reset-vector entry, maskable/NMI acceptance, stack frame,
  RETI interaction, CPUOFF wake behavior, and deterministic priority remain
  intentionally outside the completed instruction-only oracle.
- Build the classic-MSP430 educational gate cell from the accepted functional
  oracle, then compose an MSP430F2274 SoC only from public device data: explicit
  buses, address decode, Flash/RAM, interrupt routing, timers, DMA, GPIO, and
  serial peripherals must remain visible components rather than CPU callbacks.
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
- Independently review and publish the completed bounded Xtensa LX6 functional
  cell before its educational gate or ESP32-class SoC work. Follow-ups are
  ordered: publicly specified
  multiply/divide and conditional-store options; exception/interrupt delivery;
  then dual-core memory aliases, cache/MMU boundary, interrupt routing, DMA,
  and peripherals. Any later gate package is an explicitly educational,
  repository-owned synthesized topology, never a claimed proprietary LX6
  netlist.
- Descend below Boolean gates only when the SPICE/device-model wave lands.
  Transistor and analog timing claims must reference those lower-level models.
