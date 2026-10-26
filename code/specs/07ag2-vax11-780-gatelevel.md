# Layer 07ag2 — DEC VAX-11/780 Educational Gate-Level Simulator

## Purpose

This package is the gate-level partner for Layer 07ag's checked DEC
VAX-11/780 integer simulator.  It models the published programmer-visible
architecture as a stable educational Boolean topology; it does **not** claim
to reconstruct DEC's proprietary 780 board schematics, timing, microcode, or
individual semiconductor devices.

The functional simulator remains the lifecycle and fault oracle.  Successful
instructions are also executed by an independent gate transition engine.  A
transition is clocked into persistent state only when the gate result, trace,
and complete state agree with the functional oracle.

## Persistent topology

Every architecturally persistent bit is stored in a simulated master/slave D
flip-flop:

- 65,536 bytes of memory: 524,288 DFFs;
- sixteen 32-bit registers: 512 DFFs;
- N, Z, V, and C: 4 DFFs;
- halt: 1 DFF;
- installed-program origin: 32 DFFs; and
- installed-program length, including 65,536: 17 DFFs.

The exact total is **524,854 persistent DFFs**.  Packed stable-Q storage is
permitted for memory because a settled master/slave clock state is determined
by Q.  Every write must still clock the selected byte through repository DFF
primitives.  The stable storage lower bound is six primitive gates per DFF;
the package separately accounts for its finite combinational decode and
datapath network.

## Combinational transition engine

The independent transition engine implements the complete Layer 07ag integer
surface and operand-specifier grammar.  It must use repository Boolean gates
for:

- one-hot opcode and full operand-specifier decode, with host orchestration
  dispatching only the value reconstructed from the asserted gate lines;
- 8-, 16-, and 32-bit add, subtract, negate, increment, and decrement;
- bitwise BIS, BIC, XOR, complement, mask, and zero detection;
- fixed-width partial-product multiplication and restoring signed division;
- unsigned borrow/carry and signed-overflow generation;
- branch-condition evaluation and PC/address arithmetic; and
- gate-derived packed-memory write enables.

Byte and word partial-register writes preserve upper bits through gate masks.
Memory is little-endian and permits unaligned accesses, matching VAX.  The
bounded 64 KiB arena remains a checked modelling boundary.

## Checked lifecycle

The public machine shares `VaxState`, `Psl`, `StepTrace`, `ExecutionResult`,
and `VaxError` with the functional package.  It provides reset, snapshot,
restore, checked program loading, direct register/memory access, one-step
execution, bounded loaded execution, and reset/load/run execution.

Every error is typed and atomic.  A step settles both engines privately and
commits only after exact agreement.  A bounded run is one transaction: a later
fault or exhausted step limit restores the complete pre-run DFF state.
Retained traces use the same explicit finite limit as the functional oracle.

## Conformance

Completion requires:

1. full-state and trace agreement for every valid opcode family and every
   general addressing mode already covered by Layer 07ag;
2. matching typed, atomic failures for the complete malformed-input and
   lifecycle surface;
3. focused gate-network and exact-topology tests;
4. release tests, strict Clippy, rustdoc warnings, formatting, package BUILD,
   and repository lessons validation passing; and
5. at least 95 percent Rust line coverage for the new package.

The functional oracle may diagnose or confirm a transition, but it is never
used as the gate engine's successful-transition implementation.
