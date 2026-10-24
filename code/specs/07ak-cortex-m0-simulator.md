# Layer 07ak — Arm Cortex-M0 Functional Simulator

## 1. Status and sources

This cell defines the checked scalar execution boundary for one Arm Cortex-M0
processor. It is the functional oracle for the later Cortex-M0 gate cell and
for SoC work that composes a core with AHB-Lite memory, NVIC, timers, and device
peripherals.

The normative public references are Arm's *Cortex-M0 Devices Generic User
Guide*, ARM DUI 0497A (2009), the *Armv6-M Architecture Reference Manual*, ARM
DDI 0419, and the *Cortex-M0 Technical Reference Manual*, ARM DDI 0432. The
generic guide identifies Cortex-M0 as an Armv6-M, three-stage, Von Neumann core
and gives the supported Thumb instruction set, programmer's model, alignment,
branch, and special-register rules.

## 2. Bounded architecture

The package models one deterministic Cortex-M0 core with:

- R0-R12, LR, PC, separate MSP and PSP, and the active SP alias;
- APSR N/Z/C/V flags, IPSR, EPSR T, PRIMASK, CONTROL, and Thread/Handler mode;
- 64 KiB of owned little-endian teaching memory plus exact installed-image
  origin and length;
- the complete Cortex-M0 integer Thumb surface listed in DUI 0497A Table 3-1:
  shifts, arithmetic, logic, compares, byte reversal and extension, scalar and
  multiple memory operations, stack operations, branches, BL/BX/BLX, hints,
  barriers, CPS, MRS/MSR, SVC, and BKPT; and
- exact raw bytes, width, conventional mnemonic, and complete before/after
  snapshots for every committed instruction.

This cell does not claim a particular MCU. NVIC arbitration, exception entry
and return, SysTick, debug attachment, WIC, AHB-Lite timing, bus faults, and
vendor peripherals belong to later SoC/interconnect cells. SVC and non-sentinel
BKPT therefore produce typed atomic boundary events. `BKPT #0` is the
deterministic semihosting halt sentinel.

## 3. State and memory rules

Reset enters privileged Thread mode, selects MSP, clears R0-R12/LR and all
flags, sets EPSR.T, initializes MSP and PSP to the top of the owned memory, and
has no installed program. R15 is not separately restorable: PC is authoritative.
The public snapshot owns all memory and installation metadata.

Instruction fetches must be halfword aligned and wholly inside the installed
image. Data accesses must be naturally aligned, because Cortex-M0 does not
support unaligned data accesses. They must also fit inside the owned memory.
No access wraps. PC-relative reads use the architectural visible PC value
(`instruction address + 4`) and word-align it where required.

Loads validate the complete image before replacing state. Restore validates
memory size, alignment, register and status reserved bits, mode/stack
consistency, and installed-image bounds before commit.

## 4. Execution contract

`step` is atomic. It fetches and fully validates an instruction, executes on a
candidate snapshot, builds a complete trace, and only then commits. Unknown,
reserved, unsupported, misaligned, out-of-range, privilege, breakpoint, SVC,
and halted conditions preserve the entire prior state.

Bounded `run` retains at most 1,024 complete traces and rolls back the whole
run on the first fault or on step-limit exhaustion. `execute` preflights the
retention bound before cloning, loading, or allocating, then atomically performs
load plus run.

Control-transfer targets must retain the Thumb-state indication where the
architecture requires it. BX/BLX with target bit zero are typed state faults.
Committed PC values are halfword aligned. BL and BLX write an odd LR return
address.

## 5. Completion gate

The cell is complete only when it has:

1. decode classification across every supported 16-bit family and the Armv6-M
   32-bit BL, barrier, MRS, and MSR families, with reserved-neighbor witnesses;
2. behavioral tests for flags, high and low registers, both stack pointers,
   PC-relative semantics, every scalar width/sign mode, stack/multiple ordering,
   control flow, special registers, and typed atomic faults;
3. transactional load, restore, step, run, execute, fetch, alignment, and
   retention tests;
4. release line coverage above 80% (completion baseline: 82.28%, 887/1,078);
   and
5. Rustfmt, strict Clippy, rustdoc, release tests, BUILD integration, lessons
   validation, affected-package checks, and security review before publication.
