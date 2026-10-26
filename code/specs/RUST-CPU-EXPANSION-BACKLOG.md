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
| P0 | Arm Cortex-M0 | merged | PR #18296 | Conflict reconciliation in progress; squash auto-merge remains armed. |
| P0 | Arm Cortex-M3 | merged | merged | Complete; gate PR #18297 landed. |

## P0 gate-integrity repair queue

Independent review of the completed architecture matrix found a recurring
storage-authority defect: some packages counted DFF banks in their physical
topology while executing from parallel public host fields. These repairs
outrank new architecture cells because a shadow bank makes the published gate
claim unsound even when ISA differentials remain green.

| Priority | Target | Status | Required boundary |
|---:|---|---|---|
| P0 | Motorola 68000 gate | local complete | All 560 register/PC/SR DFF Q outputs and the HALT DFF Q output are authoritative; public fields are synchronized observers. Concrete inventory, deliberate observer-divergence lifecycle regressions, 71 unit tests, all 82 differential vectors, 6 lifecycle tests, 7 doctests, strict checks, lessons, and 83.39% combined line coverage are green. |

## Independent functional queue

This queue is re-costed after every merge or discovery. Dates indicate the
architecture/core introduction era, not a claim about a particular chip.

| Order | Target | Public basis | Planned boundary |
|---:|---|---|---|
| 1 | Arm Cortex-M4 | Armv7E-M manuals | Cortex-M3 integer base plus documented DSP/FPU boundary. |
| 2 | Arm Cortex-M7 | Armv7E-M manuals | M7 programmer-visible core; cache/bus timing stays in SoC cells. |
| 3 | Arm Cortex-M23 | Armv8-M manuals | Baseline profile and security-state boundary. |
| 4 | Arm Cortex-M33 | Armv8-M manuals | Mainline profile, TrustZone, DSP/FPU options made explicit. |
| 5 | DEC PDP-10 family | published architecture manuals | One model-selected checked 36-bit functional oracle. |
| 6 | Data General Nova | published architecture manuals | Checked 16-bit accumulator/minicomputer boundary. |
| 7 | IBM System/370 | published Principles of Operation | System/360-compatible extension boundary. |
| 8 | Motorola 68020 | published programmer reference | 32-bit 68k extension boundary. |
| 9 | PA-RISC 1.1 | published architecture manuals | Checked integer core and delayed-control boundary. |
| 10 | SuperH-2 | published programming manuals | Checked compact RISC core boundary. |
| 11 | SPARC V9 | published architecture manual | 64-bit SPARC functional extension. |
| 12 | ARM7TDMI | published ARMv4T manuals | ARM/Thumb state and pipeline-visible architecture boundary. |
| 13 | AVR classic | published instruction-set manual | Harvard core boundary suitable for later MCU SoCs. |
| 14 | Xtensa LX | publicly documented configurable ISA subset | Configuration-pinned functional core; no proprietary extensions. |

## Discovered follow-up work

### Gate-integrity audit queue

| Priority | Target | Finding / next action |
|---:|---|---|
| P1 complete locally | PowerPC 601 conditional-branch authority | The host-language BO/CTR/CR predicate is replaced by explicit gate ALU, 32:1 CR mux, polarity, bypass-mux, and final-AND networks. Exhaustive tests cover every BO value and CR selector, both selected-bit polarities, and CTR zero/nonzero boundaries against an independent truth table. Three gate/topology suites, three lifecycle suites, all 239 differential vectors, strict checks, and 94.58% line coverage pass. The counted GPR/LR/CTR/XER/CR/CIA, memory, and HALT storage banks remain private stable-Q authority with no public host-state mirrors. |
| P1 queued | PowerPC 601 CR/XER update authority | Replace host `Ordering`/Boolean control in CR-field, CR0, CA, and SO propagation with explicit relation wires, masks, muxes, and gates while preserving the public lifecycle. |
| P1 queued | PowerPC 601 shift/count/divide authority | Replace host slice moves, iterator leading-zero counting, and conditional restoring-divide selection with fixed gate/mux networks, then audit multiply accumulation and record-form updates as one bounded datapath family at a time. |
| P0 complete locally | MIPS R2000 gate HALT storage | HALT DFF Q is now the sole architectural halt authority; the public boolean remains only as a source-compatible observation mirror and is repaired by simulator mutations. The published storage inventory derives 524,288 memory bits, 1,120 register bits, and one HALT bit from the owned physical containers. Deliberate mirror-divergence, restore, and typed-fault atomicity tests pass alongside 33 unit tests, the 218-vector differential suite, nine lifecycle/integrity tests, and 20 doctests. Strict Clippy/rustdoc, the six-package affected build, and 94.50% line coverage (1,581/1,673) are green. |
| P1 audit | Remaining completed gate packages with public halt fields | Audit each package before changing it. A public observation field is acceptable only when execution, snapshots, restore, and faults are driven by DFF Q; otherwise add one bounded repair cell and a non-tautological storage inventory. Candidate packages discovered by source scan include ARM1, SPARC V8, Motorola 68000, Intel 8051, Intel 8086, and OpenRISC 1000. |

- Expand Cortex-M3 beyond the bounded completed core with long multiply,
  bitfield, saturation, dual-register, byte/halfword exclusive, and remaining
  wide Thumb-2 encodings. This is non-blocking for the core lifecycle and SoC
  composition boundary, but it precedes declaring full Armv7-M ISA coverage.
- Build reusable SoC primitives after the initial missing-core wave: clock and
  reset domains, address-decoded buses, arbitration/backpressure, interrupt
  routing, DMA movement, timers, memory-mapped peripherals, and deterministic
  transaction traces.
- Compose the first source-backed SoCs around completed cores (Cortex-M
  microcontroller class and an OpenSPARC/OpenRISC platform), then add gate-level
  interconnect models.
- Descend below Boolean gates only when the SPICE/device-model wave lands.
  Transistor and analog timing claims must reference those lower-level models.
