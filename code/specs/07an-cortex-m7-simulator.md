# Layer 07an — Arm Cortex-M7 Functional Simulator

## 1. Status and sources

This cell defines the checked scalar execution boundary for one Arm Cortex-M7
processor. It is a functional oracle for a later Cortex-M7 gate cell and for
SoC work that composes a processor with memory, interrupt control, buses,
caches, TCM, DMA, timers, and peripherals.

The normative public references are Arm's *Cortex-M7 Devices Generic User
Guide* (DUI 0646), *Cortex-M7 Technical Reference Manual* (DDI 0489), and the
*Armv7-M Architecture Reference Manual*. They define Cortex-M7 as an
Armv7E-M, Thumb-only processor with DSP extensions and implementation options
including floating point, caches, TCM, and external bus interfaces. This
package models only the deterministic programmer-visible core boundary stated
below; it does not invent a particular licensed implementation or MCU.

## 2. Bounded architecture

The package composes the completed Cortex-M3 functional model and adds a
bounded Cortex-M7/Armv7E-M surface:

- all Cortex-M3 integer registers, MSP/PSP, LR, PC, APSR/IPSR/EPSR state,
  privilege and masks, IT state, local exclusive monitor, power/event state,
  installed-image metadata, and 64 KiB little-endian teaching memory;
- APSR GE[3:0] state in addition to the inherited N/Z/C/V/Q flags;
- signed saturating QADD and QSUB instructions, including sticky APSR.Q; and
- signed and unsigned four-lane byte addition (SADD8 and UADD8), including
  the architected per-lane GE results.

Every committed instruction reports exact raw bytes, width, a stable mnemonic,
and complete before/after snapshots. The inherited Cortex-M3 decode and
execution surface remains the compatibility foundation. The added DSP
encodings are decoded before delegation, so an Armv7E-M instruction never
falls through to the Cortex-M3 unknown-instruction boundary.

The Cortex-M7's optional floating-point unit, double-precision option, MPU,
NVIC, exception entry/return, debug and trace, and the remaining Armv7E-M DSP
families are explicit later functional-surface extensions. SVC and non-sentinel
BKPT remain typed atomic boundaries; `BKPT #0` is the teaching halt sentinel.

## 3. Cache, TCM, bus, and timing boundary

Instruction/data caches, branch prediction, ITCM/DTCM, AXI/AHB interfaces,
write buffers, arbitration, coherency, and cycle timing do not change the
architectural result of an instruction in this functional cell. They are
therefore deliberately absent from restorable state and traces. Later SoC and
gate-level cells may place those structures around this core and model traffic,
latency, stalls, contention, and cache maintenance without changing this
package's instruction semantics. Barrier instructions retain the inherited
architectural ordering marker but claim no bus-cycle timing.

## 4. State, memory, and lifecycle rules

Reset uses the Cortex-M3 reset state and clears GE. Loads are halfword aligned,
fit wholly within owned memory, replace the complete machine state, and clear
GE. Instruction fetch must be halfword aligned and wholly inside the installed
image. Data accesses use the inherited checked non-wrapping memory contract.

Restore validates the complete inherited state and rejects GE values outside
four bits before committing. Register and memory access errors, invalid restore,
invalid program installation, fetch/decode faults, halted/sleeping state, SVC,
breakpoint boundaries, and retention exhaustion are typed and atomic.

`step` executes against a candidate complete state and commits only after a
complete trace exists. DSP saturation clamps to signed 32-bit range and sets
sticky Q; a non-saturating operation never clears Q. SADD8 writes each signed
lane sum modulo 256 and sets GE for non-negative full-precision lane results.
UADD8 writes each unsigned lane sum modulo 256 and sets GE for lane carries.

Bounded `run` retains at most 64 complete-state traces under the inherited
8 MiB teaching budget and rolls back the whole run on a fault or step-limit
exhaustion. `execute` preflights retention and atomically performs load plus
run.

## 5. Completion gate

The cell is complete only when it has:

1. explicit classification of every inherited Cortex-M3 family plus each
   bounded Cortex-M7 DSP family and reserved-neighbor witnesses;
2. behavioral tests for inherited execution and lifecycle, saturating QADD and
   QSUB, sticky Q, signed/unsigned parallel byte addition, GE flags, exact
   traces, and cache/bus boundary documentation;
3. transactional load, restore, step, run, execute, fetch, bounds, and
   retention tests;
4. release line coverage above 80 percent; and
5. Rustfmt, strict Clippy, rustdoc, release tests, BUILD integration, lessons
   validation, affected-package checks, and independent security review before
   publication.
