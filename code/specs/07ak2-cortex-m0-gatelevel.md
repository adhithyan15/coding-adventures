# Layer 07ak2 — Arm Cortex-M0 Gate-Level Simulator

## Purpose and evidence boundary

`cortex-m0-gatelevel` is the architectural gate-level companion to the
checked `cortex-m0-simulator`. The normative public references are Arm's
*Cortex-M0 Devices Generic User Guide* (ARM DUI 0497A), the *Armv6-M
Architecture Reference Manual* (ARM DDI 0419), and the *Cortex-M0 Technical
Reference Manual* (ARM DDI 0432). They define the programmer-visible Armv6-M
Thumb instruction, register, alignment, privilege, and low-power behavior
represented by this cell.

This package models every persistent architectural bit with repository D
flip-flops and reconstructs every committed transition through repository
logic gates and arithmetic networks. It is an educational architectural gate
model, not a claim about Arm's proprietary netlist, three-stage pipeline
timing, AHB-Lite cycles, NVIC/SysTick integration, clock gating, physical
placement, or transistor layout. The checked functional package is a
development and test oracle only; production code has no dependency on it and
never calls it to produce a transition.

## Persistent topology

Every mutable architectural bit changes through
`logic_gates::sequential::register`. Packed stable-Q memory is permitted, and
may keep a derived host-side set of nonzero byte addresses so reset remains
proportional to changed state. That set is reconstructible metadata, not
architectural storage.

| Component | D flip-flops |
|---|---:|
| 65,536-byte memory | 524,288 |
| R0-R12 | 416 |
| MSP, PSP, LR, and PC | 128 |
| APSR N/Z/C/V | 4 |
| IPSR | 6 |
| PRIMASK | 1 |
| CONTROL nPRIV/SPSEL | 2 |
| Thread/Handler mode latch | 1 |
| Halt, sleep, and event latches | 3 |
| Installed-program origin | 32 |
| Installed-program length, including 65,536 | 17 |
| **Total** | **524,898** |

EPSR.T is architecturally fixed to one in this Thumb-only boundary and
therefore is a constant wire rather than mutable storage. At six primitive
gates per master/slave DFF, persistent storage alone represents 3,149,388 gate
equivalents. Decode, selection, comparison, add/subtract, shift/rotate,
multiply, byte-reversal, branch, flag, address, and memory-control networks add
further simulated combinational circuitry.

## Gate transition engine

- Gate-built prefix and terminal decoders classify every supported 16-bit
  family plus 32-bit BL, barrier, MRS, and MSR encodings. Host orchestration
  dispatches only after a decoder line settles.
- Ripple adders, two's-complement subtraction, Boolean networks, staged
  barrel shifters, rotate networks, comparisons, sign/zero extension, and a
  shift-add multiplier derive integer and flag results.
- PC sequencing, visible-PC calculations, branch targets, stack addressing,
  register write selection, active-stack selection, and special-register
  masks are gate-derived before candidate state is committed.
- Natural alignment, owned-memory range, load sign extension, and byte write
  enables settle before any memory DFF is clocked. Loads, direct writes,
  stack/multiple transfers, restore, reset, and instruction commits all use
  the same DFF-backed storage path.
- CPS, MRS/MSR privilege rules, CONTROL stack selection, BKPT/SVC boundaries,
  WFE/WFI/SEV event state, barriers, and the deterministic `BKPT #0` halt
  convention preserve the checked functional contract.

The engine may build a value-level candidate while evaluating those networks,
but persistent state commits only by clocking the corresponding DFF banks.
Complete-state differential tests make an unclocked bit, wrong selector, or
incorrect memory write observable.

## Checked lifecycle

The public machine owns complete state, exposes reset, validated restore,
image load, register and naturally aligned memory access, event signalling,
single-step execution, bounded loaded execution, and atomic load-and-execute.
It preserves exact little-endian raw bytes, instruction width, mnemonic, PC
transition, and complete before/after snapshots.

Every failure is typed and atomic. Failed image validation, restore, fetch,
decode, alignment, range, Thumb-state transfer, breakpoint/SVC boundary,
sleep/halt step, trace allocation, or run budget preserves every DFF and
derived memory index. Complete-state traces are bounded by the same fixed
8 MiB retention budget as the functional cell and every nested 64 KiB memory
snapshot is allocated fallibly before architectural mutation.

## Completion criteria

1. Pin the topology to exactly 524,898 DFFs and test the combinational
   primitives independently.
2. Compare exact success traces, typed faults, and complete architectural state
   against `cortex-m0-simulator` for every implemented decode subfamily,
   including both branch outcomes and reserved neighbors.
3. Cover flags, high and low registers, MSP and PSP selection, every scalar
   memory width/sign mode, stack and multiple-transfer ordering, control flow,
   privilege, barriers, low-power/event behavior, load/restore/reset, external
   writes, arbitrary origins, rollback, and retention bounds.
4. Keep the functional package exclusively in development dependencies and
   prove production source contains no oracle call path.
5. Pass release tests, Rustfmt, strict Clippy, rustdoc warnings, package BUILD,
   workspace and affected-package integration, lessons validation, independent
   security review, and more than 90 percent Rust line coverage.
