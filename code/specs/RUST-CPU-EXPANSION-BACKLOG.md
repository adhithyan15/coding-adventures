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
| P0 | IBM System/360 | merged | merged | Complete. |
| P0 | Motorola MC6800 | merged | merged | Complete. |
| P0 | Intel 8085 | merged | merged | Complete. |
| P0 | DEC VAX-11/780 | merged | merged | Complete; gate PR #18263 landed. |
| P0 | Motorola 6809 | merged | merged | Complete; gate PR #18269 landed. |
| P0 | OpenRISC 1000 | merged | merged | Complete; gate PR #18276 landed. |
| P0 | OpenSPARC T1 | merged | merged | Complete; gate PR #18295 landed. |
| P0 | Arm Cortex-M0 | merged | PR #18296 | Gate cell is published with squash auto-merge armed. |
| P0 | Arm Cortex-M3 | merged | merged | Complete; gate PR #18297 landed. |
| P0 | Arm Cortex-M4 | PR #18313 | blocked on functional | Conflict resolved locally; exact-head review precedes the update. |
| P0 | Arm Cortex-M7 | PR #18307 | blocked on functional | Functional cell is published with squash auto-merge armed. |
| P0 | Arm Cortex-M23 | PR #18327 | blocked on functional | Functional cell is published with squash auto-merge armed. |
| P0 | Arm Cortex-M33 | PR #18326 | blocked on functional | Functional cell is published with squash auto-merge armed. |
| P0 | DEC PDP-10 KA10 | in progress | blocked on functional | Spec 07aq and the checked bounded functional cell are under local completion review. |

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
| 1 | DEC PDP-10 KA10 | published architecture manuals | Active: bounded checked 36-bit functional oracle. |
| 2 | Data General Nova | published architecture manuals | Checked 16-bit accumulator/minicomputer boundary. |
| 3 | IBM System/370 | published Principles of Operation | System/360-compatible extension boundary. |
| 4 | Motorola 68020 | published programmer reference | 32-bit 68k extension boundary. |
| 5 | PA-RISC 1.1 | published architecture manuals | Checked integer core and delayed-control boundary. |
| 6 | SuperH-2 | published programming manuals | Checked compact RISC core boundary. |
| 7 | SPARC V9 | published architecture manual | 64-bit SPARC functional extension. |
| 8 | ARM7TDMI | published ARMv4T manuals | ARM/Thumb state and pipeline-visible architecture boundary. |
| 9 | AVR classic | published instruction-set manual | Harvard core boundary suitable for later MCU SoCs. |
| 10 | Xtensa LX | publicly documented configurable ISA subset | Configuration-pinned functional core; no proprietary extensions. |

## Discovered follow-up work

### Gate-integrity audit queue

| Priority | Target | Finding / next action |
|---:|---|---|
| P0 merged | MIPS R2000 gate HALT storage | Complete; PR #18304 landed. HALT DFF Q is the sole architectural authority. |
| P0 PR #18309 | SPARC V8 gate HALT storage | Reviewed and published with squash auto-merge armed. |
| P0 PR #18316 | Motorola 68000 gate storage | Reviewed and published with squash auto-merge armed. |
| P0 local | Intel 8051 gate PC/HALT storage | Conflict resolved; exact-head review precedes the PR update. |
| P0 PR #18320 | Intel 8086 gate DFF bank | Conflict resolution is under mandatory exact-head review. |
| P1 audit | Remaining completed gate packages with public halt fields | Continue auditing ARM1 and OpenRISC 1000; a public mirror is acceptable only when execution, snapshots, restore, and faults are driven by DFF Q. |

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
