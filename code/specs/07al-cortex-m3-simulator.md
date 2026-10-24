# Layer 07al — Arm Cortex-M3 Functional Simulator

## 1. Status and sources

This cell defines the checked scalar execution boundary for one Arm Cortex-M3
processor. It is the functional oracle for a later Cortex-M3 gate cell and for
SoC work that composes a core with buses, interrupt control, timers, memory,
and device peripherals.

The normative public references are Arm's *Cortex-M3 Devices Generic User
Guide*, ARM DUI 0552A, the *Armv7-M Architecture Reference Manual*, and the
*Cortex-M3 Technical Reference Manual*. DUI 0552A identifies Cortex-M3 as an
Armv7-M Thumb-only processor and defines its programmer's model, instruction
set, memory behavior, and special-register rules. Optional and
implementation-defined device features are outside this deterministic cell.

## 2. Bounded architecture

The package models one deterministic Cortex-M3 core with:

- R0-R12, LR, PC, separate MSP and PSP, and the active SP alias;
- APSR N/Z/C/V/Q flags, IPSR, EPSR Thumb/IT state, PRIMASK, BASEPRI,
  FAULTMASK, CONTROL, and Thread/Handler mode;
- 64 KiB of owned little-endian teaching memory, exact installed-image origin
  and length, and one local exclusive monitor;
- the Cortex-M3 integer Thumb and Thumb-2 execution surface required by the
  generic guide: shifts, arithmetic, logical and compare operations, bitfield
  and saturation operations, multiply/divide, scalar and multiple memory,
  dual-register memory, stack operations, conditional execution, branches,
  table branches, BL/BX/BLX, exclusive accesses, hints, barriers, CPS,
  MRS/MSR, SVC, and BKPT; and
- exact raw bytes, width, conventional mnemonic, and complete before/after
  snapshots for every committed instruction.

This cell does not claim a particular MCU. NVIC arbitration, exception entry
and return, SysTick, MPU regions, debug/trace, WIC, AHB timing, bit-band bus
translation, and vendor peripherals belong to later SoC/interconnect cells.
SVC and non-sentinel BKPT therefore produce typed atomic boundary events.
`BKPT #0` is the deterministic teaching halt sentinel.

## 3. State and memory rules

Reset enters privileged Thread mode, selects MSP, clears integer and status
state, sets EPSR.T, initializes MSP and PSP to the top of memory, clears the
exclusive monitor, and has no installed program. PC is authoritative; R15 is
not independently restorable. The public snapshot owns all memory and
installation metadata.

Instruction fetches are halfword aligned and wholly inside the installed
image. Data accesses must fit inside owned memory and never wrap. Byte,
halfword, and word accesses are supported; unaligned halfword and word
accesses are permitted for ordinary scalar transfers but not for exclusive,
multiple, stack, or doubleword transfers. PC-relative reads use instruction
address plus four and the alignment rules of the selected instruction.

Loads validate the complete image before replacing state. Restore validates
memory size, status reserved bits, Thumb state, mode and stack consistency,
exclusive-monitor alignment/range, and installed-image bounds before commit.

## 4. Execution contract

`step` is atomic. It fetches and fully validates one 16- or 32-bit instruction,
executes on a candidate snapshot, constructs the complete trace, and only then
commits. Unknown, reserved, unsupported, out-of-range, breakpoint, SVC, and
halted conditions preserve the entire prior state. Architecturally ignored
unprivileged special-register writes commit as no-ops.

The IT state applies conditions to the following one-to-four instructions and
advances after every committed member instruction. A failed member preserves
the IT state. Branches clear IT state. LDREX records an aligned word address;
STREX succeeds only for the matching address and always clears the monitor;
CLREX clears it explicitly. Ordinary writes conservatively clear the monitor.

Bounded `run` retains at most 64 complete traces under an 8 MiB snapshot budget
and rolls back the whole run on the first fault or step-limit exhaustion.
Nested state snapshots use fallible allocation. `execute` preflights retention
before cloning, loading, or allocating, then atomically performs load and run.

## 5. Completion gate

The cell is complete only when it has:

1. decode classification across supported 16-bit and 32-bit families with
   reserved-neighbor witnesses;
2. behavior tests for flags, low/high registers, both stack pointers,
   PC-relative semantics, IT execution, scalar and multiple memory, exclusive
   accesses, control flow, special registers, and typed atomic faults;
3. transactional load, restore, step, run, execute, fetch, bounds, and
   retention tests;
4. release line coverage above 80%; and
5. Rustfmt, strict Clippy, rustdoc, release tests, BUILD integration, lessons
   validation, affected-package checks, and security review before publication.
