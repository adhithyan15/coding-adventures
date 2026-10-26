# Layer 07am — Arm Cortex-M4 Functional Simulator

## 1. Status and sources

This cell defines the checked scalar execution boundary for one Arm Cortex-M4
processor. It is the functional oracle for a later Cortex-M4 gate cell and for
microcontroller/SoC work that composes a core with interrupt control, buses,
memory, DMA, timers, and device peripherals.

The normative public references are Arm's *Cortex-M4 Devices Generic User
Guide* (DUI 0553), *Cortex-M4 Technical Reference Manual* (DDI 0439), and the
*Armv7-M Architecture Reference Manual*. They define Cortex-M4 as an Armv7E-M
Thumb-only processor with DSP extensions and an optional FPv4-SP
single-precision floating-point unit. This package does not infer a vendor MCU
or implementation-defined timing from those programmer-visible manuals.

## 2. Bounded architecture

The package models one deterministic Cortex-M4 core with:

- the complete Cortex-M3 cell's R0-R12, MSP/PSP, LR, PC, Thumb/IT state,
  privilege/mask state, local exclusive monitor, sleep/event lifecycle,
  installed-image metadata, and 64 KiB little-endian teaching memory;
- APSR N/Z/C/V/Q and Armv7E-M GE[3:0] state;
- an explicit DSP surface containing saturating QADD/QSUB, SSAT/USAT,
lane-parallel SADD8/UADD8, dual-halfword SMUAD, and SMLAD;
- an explicit optional FPv4-SP surface containing S0-S31, FPSCR condition and
  cumulative exception state, VADD/VSUB/VMUL/VDIV, VMOV/VABS/VNEG/VSQRT,
  VCMP, VMRS, and VMSR; and
- exact raw bytes, width, mnemonic, and complete before/after snapshots for
  every committed instruction.

The FPU is a configuration boundary rather than an invented universal feature.
Reset starts with it disabled. A platform may enable the modeled
single-precision unit through the checked public configuration method, and
that choice is included in every restorable state snapshot. Executing an FPU
encoding while disabled produces a typed atomic fault. This models the core's
optional FP boundary without claiming CPACR/NVIC/system-control behavior that
belongs to an SoC.

The bounded FPU uses IEEE-754 binary32 host operations under round-to-nearest,
ties-to-even. It canonicalizes NaN results, records invalid/divide-by-zero/
overflow/underflow/inexact cumulative exception flags, implements FPSCR NZCV
comparison results, and rejects unsupported FPSCR control modes atomically.
Exception classification compares exact signed dyadic significands and
exponents rather than narrowing an exact binary32 expression through `f64`;
quiet NaN comparison is unordered without invalid, while signaling NaN
comparison is unordered and records invalid.
Denormal flushing, alternative rounding modes, fused multiply-add, conversion,
load/store, and lazy context stacking are explicit later extensions rather
than approximated behavior.

The cell does not model exception entry/return, NVIC arbitration, SysTick,
MPU, debug/trace, cache or bus timing, vendor peripherals, or implementation
timing. SVC and non-sentinel BKPT remain typed atomic boundary events;
`BKPT #0` is the deterministic teaching halt sentinel.

## 3. State and memory rules

Reset enters privileged Thread mode, selects MSP, clears integer, DSP, and FPU
state, disables the optional FPU, initializes MSP/PSP to the top of memory,
clears event/exclusive state, and has no installed program. The public snapshot
owns all architectural state, configuration, memory, and image metadata.

Instruction fetch is halfword aligned and wholly inside the installed image.
Data accesses fit within owned memory and never wrap. Restore validates memory
size, reserved status/FPSCR bits, Thumb/IT state, mode/stack consistency,
exclusive-monitor alignment/range, finite installation metadata, and the
supported FPU-control boundary before committing.

## 4. Execution contract

`step` fetches, decodes, validates, and executes on a candidate snapshot. Only
after a complete trace has been constructed does it commit. Unknown, reserved,
disabled-FPU, invalid-control, out-of-range, lifecycle, breakpoint, and SVC
faults preserve the entire prior state.

DSP saturation sets sticky APSR.Q. Parallel byte arithmetic writes APSR.GE
lane results. Inherited MRS/MSR transfers include GE bits 19:16 for the bounded
APSR system-register variants, including unprivileged and conditional IT
execution. SMUAD/SMLAD perform signed dual-halfword products and set sticky Q
only when the 32-bit result overflows. Extension operand and destination
constraints are validated before an IT condition can skip execution. FPU
operations update FPSCR cumulative exception flags while preserving supported
control bits;
VCMP updates FPSCR.NZCV exactly. VMRS/VMSR transport the bounded FPSCR through
integer registers.

Bounded `run` retains at most 64 complete traces under an 8 MiB snapshot budget
and rolls back the whole run on a fault or step-limit exhaustion. `execute`
preflights retention and atomically performs load plus run. Nested state
snapshots use fallible allocation. Failure to allocate the successful run's
final-state snapshot also rolls the entire run back to its entry state.

## 5. Completion gate

The cell is complete only when it has:

1. decode classification for every inherited family and each bounded DSP/FPU
   family, including disabled and reserved-neighbor witnesses;
2. behavior tests for inherited integer, privilege, IT, memory, exclusive,
   branch, power, and lifecycle behavior plus DSP saturation/GE/multiply and
   FPU state/arithmetic/compare/status behavior;
3. transactional load, restore, step, run, execute, fetch, bounds, FPU
   configuration, and retention tests;
4. release line coverage above 80 percent (completion baseline: 92.81 percent,
   994/1071 lines); and
5. Rustfmt, strict Clippy, rustdoc, release tests, BUILD integration, lessons
   validation, affected-package checks, and independent security review before
   publication.
